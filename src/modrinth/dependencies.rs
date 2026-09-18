use crate::error::{MonoryxError, Result};
use crate::modrinth::models::{DependencyType, ProjectVersion};
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
    installed: &HashSet<String>,
    mut fetch_best: F,
) -> Result<DependencyPlan>
where
    F: FnMut(&str) -> Fut,
    Fut: std::future::Future<Output = Result<Option<ProjectVersion>>>,
{
    let mut plan = DependencyPlan::default();
    let mut visiting: HashSet<String> = HashSet::new();
    let mut done: HashSet<String> = installed.clone();

    done.insert(root.project_id.clone());

    let mut stack: Vec<(ProjectVersion, usize)> = vec![(root.clone(), 0)];

    let mut ordered_tmp: Vec<ResolvedDep> = Vec::new();

    while let Some((ver, depth)) = stack.pop() {
        for dep in &ver.dependencies {
            match dep.dependency_type {
                DependencyType::Embedded => {
                    plan.warnings.push(format!(
                        "embedded dependency skipped ({})",
                        dep.project_id.as_deref().unwrap_or("unknown")
                    ));
                    continue;
                }
                DependencyType::Incompatible => {
                    if let Some(pid) = &dep.project_id {
                        if done.contains(pid) {
                            return Err(MonoryxError::DependencyConflict(format!(
                                "{} is incompatible with installed {}",
                                ver.project_id, pid
                            )));
                        }
                    }
                    continue;
                }
                DependencyType::Optional => {
                    plan.optional.push(DepInfo {
                        project_id: dep.project_id.clone(),
                        version_id: dep.version_id.clone(),
                        file_name: dep.file_name.clone(),
                    });
                    continue;
                }
                DependencyType::Required => {}
            }

            if let Some(vid) = &dep.version_id {
                if done.contains(vid) {
                    continue;
                }

                if dep.project_id.is_none() {
                    plan.warnings.push(format!(
                        "required dependency {vid} has no project id; skipping"
                    ));
                    continue;
                }
            }
            let Some(pid) = dep.project_id.clone() else {
                continue;
            };
            if done.contains(&pid) {
                continue;
            }
            if !visiting.insert(pid.clone()) {
                return Err(MonoryxError::DependencyConflict(format!(
                    "dependency cycle detected at {pid}"
                )));
            }
            let Some(best) = fetch_best(&pid).await? else {
                return Err(MonoryxError::DependencyConflict(format!(
                    "required dependency {pid} has no compatible version"
                )));
            };

            if let Some(existing) = ordered_tmp.iter().find(|r| r.project_id == best.project_id) {
                if existing.version_id != best.id {
                    return Err(MonoryxError::DependencyConflict(format!(
                        "conflicting versions for {}: {} vs {}",
                        best.project_id, existing.version_id, best.id
                    )));
                }
                visiting.remove(&pid);
                continue;
            }
            done.insert(pid.clone());
            done.insert(best.id.clone());
            visiting.remove(&pid);
            ordered_tmp.push(ResolvedDep {
                project_id: best.project_id.clone(),
                version_id: best.id.clone(),
                version: best.clone(),
                depth: depth + 1,
            });
            stack.push((best, depth + 1));
        }
    }

    ordered_tmp.sort_by_key(|r| std::cmp::Reverse(r.depth));
    plan.ordered = ordered_tmp;
    Ok(plan)
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
    async fn resolves_transitive_required() {
        let root = ver("root", "v-root", vec![req("a")]);
        let a = ver("a", "v-a", vec![req("b")]);
        let b = ver("b", "v-b", vec![]);
        let map: HashMap<String, ProjectVersion> = [("a".to_string(), a), ("b".to_string(), b)]
            .into_iter()
            .collect();
        let plan = resolve_required(&root, &HashSet::new(), |pid: &str| {
            let m = map.get(pid).cloned();
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
        let plan = resolve_required(&root, &HashSet::new(), |pid: &str| {
            let m = map.get(pid).cloned();
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
        let err = resolve_required(&root, &HashSet::new(), |_: &str| async move { Ok(None) })
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
