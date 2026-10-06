use crate::instance::config::{InstanceConfig, LoaderKind};
use crate::storage::paths::MonoryxPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    Ready,

    Installing,

    NotDownloaded,
}

impl Readiness {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Installing => "Installing",
            Self::NotDownloaded => "Not downloaded",
        }
    }

    #[must_use]
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}

#[must_use]
pub fn version_dir_for(paths: &MonoryxPaths, cfg: &InstanceConfig) -> Option<std::path::PathBuf> {
    let id = cfg.resolved_version_id.trim();
    if id.is_empty() || crate::utils::fs::safe_file_name(id).is_err() {
        return None;
    }
    Some(paths.versions_dir().join(id))
}

#[must_use]
pub fn client_jar_for(paths: &MonoryxPaths, cfg: &InstanceConfig) -> Option<std::path::PathBuf> {
    let dir = version_dir_for(paths, cfg)?;
    let id = cfg.resolved_version_id.trim();
    Some(dir.join(format!("{id}.jar")))
}

fn client_jar_installed(paths: &MonoryxPaths, cfg: &InstanceConfig) -> bool {
    let Some(dir) = version_dir_for(paths, cfg) else {
        return false;
    };
    if let Some(jar) = client_jar_for(paths, cfg) {
        if jar.is_file() {
            return true;
        }
    }
    std::fs::read_dir(&dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(".jar"))
        })
    })
}

#[must_use]
pub fn readiness(paths: &MonoryxPaths, cfg: &InstanceConfig, busy: bool) -> Readiness {
    if busy {
        return Readiness::Installing;
    }
    if client_jar_installed(paths, cfg) {
        Readiness::Ready
    } else {
        Readiness::NotDownloaded
    }
}

#[must_use]
pub fn installable_loaders<'a, I>(
    instances: I,
    paths: &MonoryxPaths,
    busy: &std::collections::HashMap<String, (String, usize, usize)>,
) -> Vec<LoaderKind>
where
    I: Iterator<Item = &'a InstanceConfig>,
{
    let mut seen: Vec<LoaderKind> = Vec::new();
    for cfg in instances {
        if !readiness(paths, cfg, busy.contains_key(&cfg.id)).is_ready() {
            continue;
        }
        if !seen.contains(&cfg.loader) {
            seen.push(cfg.loader);
        }
    }

    LoaderKind::all()
        .into_iter()
        .filter(|kind| seen.contains(kind))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(id: &str, resolved: &str) -> InstanceConfig {
        InstanceConfig {
            id: id.into(),
            resolved_version_id: resolved.into(),
            ..InstanceConfig::new(
                "Test".into(),
                "1.21.1".into(),
                LoaderKind::Fabric,
                String::new(),
            )
        }
    }

    #[test]
    fn an_unresolved_instance_has_no_client_jar() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        assert!(client_jar_for(&paths, &instance("a", "  ")).is_none());
    }

    #[test]
    fn a_traversal_attempt_in_the_version_id_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let cfg = instance("a", "../../evil");
        assert!(
            client_jar_for(&paths, &cfg).is_none(),
            "a version id must never escape the versions directory"
        );
    }

    #[test]
    fn missing_files_mean_not_downloaded() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let busy: std::collections::HashMap<String, (String, usize, usize)> =
            std::collections::HashMap::new();
        let cfg = instance("a", "1.21.1-abric");
        assert_eq!(
            readiness(&paths, &cfg, busy.contains_key("a")),
            Readiness::NotDownloaded
        );
    }

    #[test]
    fn a_present_client_jar_means_ready() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let cfg = instance("a", "1.21.1-fabric");
        let jar = client_jar_for(&paths, &cfg).expect("a resolved version has a jar path");
        std::fs::create_dir_all(jar.parent().unwrap()).unwrap();
        std::fs::write(&jar, b"not really a jar").unwrap();
        let busy: std::collections::HashMap<String, (String, usize, usize)> =
            std::collections::HashMap::new();
        assert_eq!(
            readiness(&paths, &cfg, busy.contains_key("a")),
            Readiness::Ready
        );
    }

    #[test]
    fn an_install_in_progress_reports_installing_even_with_files_present() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let cfg = instance("a", "1.21.1-fabric");
        let jar = client_jar_for(&paths, &cfg).unwrap();
        std::fs::create_dir_all(jar.parent().unwrap()).unwrap();
        std::fs::write(&jar, b"x").unwrap();
        let mut busy = std::collections::HashMap::new();
        busy.insert("a".to_string(), ("Installing".to_string(), 1, 10));
        assert_eq!(
            readiness(&paths, &cfg, busy.contains_key("a")),
            Readiness::Installing
        );
    }

    #[test]
    fn only_loaders_with_a_ready_instance_are_offered() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let mut fabric = instance("a", "f-1");
        fabric.loader = LoaderKind::Fabric;
        let mut forge = instance("b", "forge-1");
        forge.loader = LoaderKind::Forge;
        let mut pending = instance("c", "");
        pending.loader = LoaderKind::Neoforge;

        for cfg in [&fabric, &forge] {
            let jar = client_jar_for(&paths, cfg).unwrap();
            std::fs::create_dir_all(jar.parent().unwrap()).unwrap();
            std::fs::write(jar, b"x").unwrap();
        }
        let busy: std::collections::HashMap<String, (String, usize, usize)> =
            std::collections::HashMap::new();
        let loaders = installable_loaders([&fabric, &forge, &pending].into_iter(), &paths, &busy);
        assert!(loaders.contains(&LoaderKind::Fabric));
        assert!(loaders.contains(&LoaderKind::Forge));
        assert!(
            !loaders.contains(&LoaderKind::Neoforge),
            "an instance with no files must not advertise its loader"
        );
    }

    #[test]
    fn loader_order_is_stable_regardless_of_instance_order() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let mk = |id: &str, loader: LoaderKind| {
            let mut cfg = instance(id, "v");
            cfg.loader = loader;
            let jar = client_jar_for(&paths, &cfg).unwrap();
            std::fs::create_dir_all(jar.parent().unwrap()).unwrap();
            std::fs::write(jar, b"x").unwrap();
            cfg
        };
        let a = mk("a", LoaderKind::Quilt);
        let b = mk("b", LoaderKind::Fabric);
        let busy: std::collections::HashMap<String, (String, usize, usize)> =
            std::collections::HashMap::new();
        assert_eq!(
            installable_loaders([&b, &a].into_iter(), &paths, &busy),
            vec![LoaderKind::Fabric, LoaderKind::Quilt]
        );
        assert_eq!(
            installable_loaders([&a, &b].into_iter(), &paths, &busy),
            vec![LoaderKind::Fabric, LoaderKind::Quilt]
        );
    }

    #[test]
    fn a_version_whose_jar_name_differs_from_its_id_is_still_ready() {
        let temp = tempfile::tempdir().unwrap();
        let paths = MonoryxPaths::new(temp.path().into());
        let cfg = instance("1.21.5-pre1", "1.21.5-pre1");
        assert_eq!(readiness(&paths, &cfg, false), Readiness::NotDownloaded);
        let dir = version_dir_for(&paths, &cfg).unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("1.21.5-pre1-rc1.jar"), b"x").unwrap();
        assert_eq!(
            readiness(&paths, &cfg, false),
            Readiness::Ready,
            "a client jar whose name differs from the version id must count as installed"
        );
    }
}
