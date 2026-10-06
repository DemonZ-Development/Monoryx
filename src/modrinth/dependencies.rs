use crate::error::{MonoryxError, Result};
use crate::modrinth::models::{DependencyType, ProjectVersion, VersionDependency};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct DependencyPlan {
    pub ordered: Vec<ResolvedDep>,

    pub optional: Vec<DepInfo>,

    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedDep {
    pub project_id: String,
    pub version_id: String,
    pub version: ProjectVersion,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct DepInfo {
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub file_name: Option<String>,
}

pub async fn resolve_required<F, Fut>(
    root: &ProjectVersion,
    installed: &HashMap<String, String>,
    mut fetch_version: F,
) -> Result<DependencyPlan>
where
    F: FnMut(&VersionDependency) -> Fut,
    Fut: std::future::Future<Output = Result<Option<ProjectVersion>>>,
{
    let mut selected = HashMap::from([(root.project_id.clone(), root.clone())]);
    let mut fetched = HashMap::new();
    let mut requests = 0;
    for _ in 0..1024 {
        let mut plan = DependencyPlan::default();
        let mut visited = HashSet::from([root.project_id.clone()]);
        let mut pins = HashMap::from([(root.project_id.clone(), root.id.clone())]);
        let mut incompatible = Vec::new();
        let mut conflicts = Vec::new();
        let mut failures = Vec::new();
        let mut changed = false;
        let mut stack = vec![(root.clone(), 0, 0)];
        while let Some((ver, depth, index)) = stack.pop() {
            if index == ver.dependencies.len() {
                if depth > 0 && installed.get(&ver.project_id) != Some(&ver.id) {
                    plan.ordered.push(ResolvedDep {
                        project_id: ver.project_id.clone(),
                        version_id: ver.id.clone(),
                        version: ver,
                        depth,
                    });
                }
                continue;
            }
            let dep = ver.dependencies[index].clone();
            stack.push((ver.clone(), depth, index + 1));
            match dep.dependency_type {
                DependencyType::Embedded => {
                    plan.warnings.push(format!(
                        "embedded dependency skipped ({})",
                        dep.project_id.as_deref().unwrap_or("unknown")
                    ));
                    continue;
                }
                DependencyType::Optional => {
                    plan.optional.push(DepInfo {
                        project_id: dep.project_id,
                        version_id: dep.version_id,
                        file_name: dep.file_name,
                    });
                    continue;
                }
                DependencyType::Incompatible => {
                    incompatible.push((ver.project_id, dep));
                    continue;
                }
                DependencyType::Required => {}
                DependencyType::Unknown => {
                    plan.warnings.push(format!(
                        "unknown dependency type skipped ({})",
                        dep.project_id.as_deref().unwrap_or("unknown")
                    ));
                    continue;
                }
            }
            if let Some(pid) = &dep.project_id {
                if !selected.contains_key(pid)
                    && installed
                        .get(pid)
                        .is_some_and(|vid| dep.version_id.as_ref().is_none_or(|pin| pin == vid))
                {
                    if let Some(pin) = &dep.version_id {
                        if pins.get(pid).is_some_and(|existing| existing != pin) {
                            conflicts.push(format!("conflicting pinned versions for {pid}"));
                        } else {
                            pins.insert(pid.clone(), pin.clone());
                        }
                    }
                    continue;
                }
            }
            let existing = dep
                .project_id
                .as_ref()
                .and_then(|pid| selected.get(pid))
                .filter(|version| dep.version_id.as_ref().is_none_or(|pin| pin == &version.id))
                .cloned();
            let best = if let Some(version) = existing {
                version
            } else {
                let key = (dep.project_id.clone(), dep.version_id.clone());
                if let Some(version) = fetched.get(&key) {
                    ProjectVersion::clone(version)
                } else {
                    let label = dep
                        .version_id
                        .as_deref()
                        .or(dep.project_id.as_deref())
                        .or(dep.file_name.as_deref())
                        .unwrap_or("unknown");
                    requests += 1;
                    if requests > 4096 {
                        return Err(MonoryxError::DependencyConflict(
                            "dependency resolution exceeds 4096 requests".into(),
                        ));
                    }
                    let version = match fetch_version(&dep).await {
                        Ok(Some(version)) => version,
                        Ok(None) => {
                            failures.push(format!(
                                "required dependency {label} has no compatible version"
                            ));
                            continue;
                        }
                        Err(error) => {
                            failures.push(format!(
                                "required dependency {label}: {}",
                                error.user_message()
                            ));
                            continue;
                        }
                    };
                    if dep
                        .version_id
                        .as_ref()
                        .is_some_and(|pin| pin != &version.id)
                        || dep
                            .project_id
                            .as_ref()
                            .is_some_and(|pid| pid != &version.project_id)
                    {
                        return Err(MonoryxError::DependencyConflict(format!("required dependency {label} resolved to a different project or version")));
                    }
                    fetched.insert(key, version.clone());
                    if fetched.len() > 4096 {
                        return Err(MonoryxError::DependencyConflict(
                            "dependency resolution exceeds 4096 versions".into(),
                        ));
                    }
                    version
                }
            };
            if let Some(pin) = &dep.version_id {
                if let Some(existing_pin) = pins.get(&best.project_id).filter(|value| *value != pin)
                {
                    conflicts.push(format!(
                        "conflicting versions for {}: {existing_pin} vs {pin}",
                        best.project_id
                    ));
                    continue;
                }
                pins.insert(best.project_id.clone(), pin.clone());
            }
            if selected
                .get(&best.project_id)
                .is_some_and(|version| version.id != best.id)
            {
                changed = true;
            }
            selected.insert(best.project_id.clone(), best.clone());
            if visited.insert(best.project_id.clone()) {
                if visited.len() > 1024 {
                    return Err(MonoryxError::DependencyConflict(
                        "dependency graph exceeds 1024 projects".into(),
                    ));
                }
                stack.push((best, depth + 1, 0));
            }
        }
        if changed {
            continue;
        }
        if !conflicts.is_empty() {
            return Err(MonoryxError::DependencyConflict(conflicts.join("; ")));
        }
        if !failures.is_empty() {
            return Err(MonoryxError::DependencyConflict(failures.join("; ")));
        }
        let mut final_versions = installed.clone();
        for pid in &visited {
            if let Some(version) = selected.get(pid) {
                final_versions.insert(pid.clone(), version.id.clone());
            }
        }
        for (owner, dep) in incompatible {
            let conflicts = if let Some(pid) = &dep.project_id {
                final_versions
                    .get(pid)
                    .is_some_and(|vid| dep.version_id.as_ref().is_none_or(|pin| pin == vid))
            } else if let Some(vid) = &dep.version_id {
                final_versions.values().any(|value| value == vid)
            } else {
                false
            };
            if conflicts {
                return Err(MonoryxError::DependencyConflict(format!(
                    "{owner} is incompatible with {}",
                    dep.project_id
                        .as_deref()
                        .or(dep.version_id.as_deref())
                        .unwrap_or("unknown")
                )));
            }
        }
        return Ok(plan);
    }
    Err(MonoryxError::DependencyConflict(
        "dependency constraints did not converge".into(),
    ))
}
#[must_use]
pub fn find_incompatibilities(
    version: &ProjectVersion,
    installed_project_ids: &HashSet<String>,
    id_to_name: &HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for dep in &version.dependencies {
        if dep.dependency_type != DependencyType::Incompatible {
            continue;
        }
        if let Some(pid) = &dep.project_id {
            if installed_project_ids.contains(pid) {
                let name = id_to_name.get(pid).map(String::as_str).unwrap_or(pid);
                out.push(format!("incompatible with installed {name}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modrinth::models::{ProjectVersion, VersionDependency};

    fn ver(pid: &str, vid: &str, deps: Vec<VersionDependency>) -> ProjectVersion {
        ProjectVersion {
            id: vid.into(),
            project_id: pid.into(),
            author_id: String::new(),
            featured: false,
            name: vid.into(),
            version_number: vid.into(),
            changelog: None,
            date_published: "2024-01-01T00:00:00Z".into(),
            downloads: 0,
            version_type: "release".into(),
            status: "listed".into(),
            requested_status: None,
            files: vec![],
            dependencies: deps,
            game_versions: vec!["1.21".into()],
            loaders: vec!["fabric".into()],
        }
    }

    fn req(pid: &str) -> VersionDependency {
        VersionDependency {
            version_id: None,
            project_id: Some(pid.into()),
            file_name: None,
            dependency_type: DependencyType::Required,
        }
    }

    #[tokio::test]
    async fn version_only_dependency_is_resolved_exactly() {
        let root = ver(
            "root",
            "root-v",
            vec![VersionDependency {
                version_id: Some("pinned".into()),
                project_id: None,
                file_name: None,
                dependency_type: DependencyType::Required,
            }],
        );
        let plan = resolve_required(&root, &HashMap::new(), |dep| {
            assert_eq!(dep.version_id.as_deref(), Some("pinned"));
            async { Ok(Some(ver("dependency", "pinned", vec![]))) }
        })
        .await
        .unwrap();
        assert_eq!(plan.ordered[0].version_id, "pinned");
        assert_eq!(plan.ordered[0].project_id, "dependency");
    }

    #[tokio::test]
    async fn installed_wrong_version_does_not_satisfy_pin() {
        let mut dep = req("a");
        dep.version_id = Some("required-v".into());
        let root = ver("root", "root-v", vec![dep]);
        let installed = HashMap::from([("a".into(), "old-v".into())]);
        let plan = resolve_required(&root, &installed, |_| async {
            Ok(Some(ver("a", "required-v", vec![])))
        })
        .await
        .unwrap();
        assert_eq!(plan.ordered.len(), 1);
        assert_eq!(plan.ordered[0].version_id, "required-v");
    }

    #[tokio::test]
    async fn conflicting_pins_are_rejected() {
        let mut first = req("a");
        first.version_id = Some("v1".into());
        let mut second = req("a");
        second.version_id = Some("v2".into());
        let root = ver("root", "root-v", vec![first, second]);
        assert!(resolve_required(&root, &HashMap::new(), |_| async {
            Ok(Some(ver("a", "v1", vec![])))
        })
        .await
        .is_err());
    }

    #[tokio::test]
    async fn incompatible_planned_project_is_rejected_regardless_of_order() {
        let mut incompatible = req("a");
        incompatible.dependency_type = DependencyType::Incompatible;
        let root = ver("root", "root-v", vec![incompatible, req("a")]);
        assert!(resolve_required(&root, &HashMap::new(), |_| async {
            Ok(Some(ver("a", "a-v", vec![])))
        })
        .await
        .is_err());
    }

    #[tokio::test]
    async fn later_transitive_pin_overrides_unpinned_choice_and_prunes_old_dependencies() {
        let root = ver("root", "root-v", vec![req("a"), req("b")]);
        let mut pin = req("a");
        pin.version_id = Some("a-pinned".into());
        let versions = HashMap::from([
            ("a-newest", ver("a", "a-newest", vec![req("orphan")])),
            ("a-pinned", ver("a", "a-pinned", vec![])),
            ("b", ver("b", "b-v", vec![pin])),
            ("orphan", ver("orphan", "orphan-v", vec![])),
        ]);
        let plan = resolve_required(&root, &HashMap::new(), |dep| {
            let key = dep.version_id.as_deref().unwrap_or_else(|| {
                if dep.project_id.as_deref() == Some("a") {
                    "a-newest"
                } else {
                    dep.project_id.as_deref().unwrap()
                }
            });
            let version = versions.get(key).cloned();
            async move { Ok(version) }
        })
        .await
        .unwrap();
        assert_eq!(plan.ordered.len(), 2);
        assert_eq!(plan.ordered[0].version_id, "a-pinned");
        assert_eq!(plan.ordered[1].project_id, "b");
    }

    #[tokio::test]
    async fn unavailable_dependency_of_a_replaced_unpinned_version_is_pruned() {
        let root = ver("root", "root-v", vec![req("a"), req("b")]);
        let mut pin = req("a");
        pin.version_id = Some("a-pinned".into());
        let versions = HashMap::from([
            (
                "a-newest",
                ver("a", "a-newest", vec![req("unavailable-orphan")]),
            ),
            ("a-pinned", ver("a", "a-pinned", vec![])),
            ("b", ver("b", "b-v", vec![pin])),
        ]);
        let plan = resolve_required(&root, &HashMap::new(), |dep| {
            let key = dep.version_id.as_deref().unwrap_or_else(|| {
                if dep.project_id.as_deref() == Some("a") {
                    "a-newest"
                } else {
                    dep.project_id.as_deref().unwrap()
                }
            });
            let version = versions.get(key).cloned();
            async move { Ok(version) }
        })
        .await
        .unwrap();
        assert_eq!(plan.ordered.len(), 2);
        assert_eq!(plan.ordered[0].version_id, "a-pinned");
    }

    #[tokio::test]
    async fn resolves_transitive_required() {
        let root = ver("root", "v-root", vec![req("a")]);
        let a = ver("a", "v-a", vec![req("b")]);
        let b = ver("b", "v-b", vec![]);
        let map: HashMap<String, ProjectVersion> = [("a".to_string(), a), ("b".to_string(), b)]
            .into_iter()
            .collect();
        let plan = resolve_required(&root, &HashMap::new(), |dep: &VersionDependency| {
            let m = dep
                .project_id
                .as_ref()
                .and_then(|pid| map.get(pid))
                .cloned();
            async move { Ok(m) }
        })
        .await
        .unwrap();
        let ids: Vec<_> = plan.ordered.iter().map(|r| r.project_id.as_str()).collect();
        assert!(ids.contains(&"a") && ids.contains(&"b"));

        assert!(ids.iter().position(|x| *x == "b") < ids.iter().position(|x| *x == "a"));
    }

    #[tokio::test]
    async fn cycle_within_deps_errors_or_terminates() {
        let root = ver("root", "v-root", vec![req("a")]);
        let a = ver("a", "v-a", vec![req("b")]);
        let b = ver("b", "v-b", vec![req("a")]);
        let map: HashMap<String, ProjectVersion> = [("a".to_string(), a), ("b".to_string(), b)]
            .into_iter()
            .collect();
        let plan = resolve_required(&root, &HashMap::new(), |dep: &VersionDependency| {
            let m = dep
                .project_id
                .as_ref()
                .and_then(|pid| map.get(pid))
                .cloned();
            async move { Ok(m) }
        })
        .await
        .unwrap();
        let ids: Vec<_> = plan.ordered.iter().map(|r| r.project_id.as_str()).collect();
        assert_eq!(ids.len(), 2);
    }

    #[tokio::test]
    async fn missing_required_errors() {
        let root = ver("root", "v-root", vec![req("ghost")]);
        let err = resolve_required(&root, &HashMap::new(), |_: &VersionDependency| async move {
            Ok(None)
        })
        .await
        .unwrap_err();
        assert!(matches!(err, MonoryxError::DependencyConflict(_)));
    }

    #[test]
    fn incompat_detected() {
        let v = ver(
            "x",
            "vx",
            vec![VersionDependency {
                version_id: None,
                project_id: Some("bad".into()),
                file_name: None,
                dependency_type: DependencyType::Incompatible,
            }],
        );
        let installed: HashSet<String> = ["bad".to_string()].into_iter().collect();
        let names: HashMap<String, String> = [("bad".to_string(), "BadMod".to_string())]
            .into_iter()
            .collect();
        let w = find_incompatibilities(&v, &installed, &names);
        assert_eq!(w.len(), 1);
    }
}
