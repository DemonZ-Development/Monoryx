use std::path::PathBuf;

#[must_use]
pub fn archive_path(
    instances: &crate::instance::manager::InstanceManager,
    instance_id: &str,
) -> PathBuf {
    instances
        .instance_dir(instance_id)
        .join("appcds")
        .join("game.jsa")
}

#[must_use]
pub fn archive_exists(
    instances: &crate::instance::manager::InstanceManager,
    instance_id: &str,
) -> bool {
    archive_path(instances, instance_id).is_file()
}

#[must_use]
pub fn archive_size(
    instances: &crate::instance::manager::InstanceManager,
    instance_id: &str,
) -> Option<u64> {
    std::fs::metadata(archive_path(instances, instance_id))
        .ok()
        .map(|m| m.len())
}

#[must_use]
pub fn content_stamp(game_dir: &std::path::Path) -> String {
    let mut parts: Vec<String> = Vec::new();
    for dir in ["mods", "config"] {
        let path = game_dir.join(dir);
        let Ok(entries) = std::fs::read_dir(&path) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let len = e.metadata().ok().map(|m| m.len()).unwrap_or(0);
                format!("{dir}/{name}:{len}")
            })
            .collect();
        names.sort();
        parts.append(&mut names);
    }
    crate::utils::hash::sha1_bytes(parts.join("\n").as_bytes())
}

#[must_use]
pub fn classpath_stamp(cp_entries: &[std::path::PathBuf]) -> String {
    let joined = cp_entries
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    crate::utils::hash::sha1_bytes(joined.as_bytes())
}

fn stamp_path(instances: &crate::instance::manager::InstanceManager, id: &str) -> PathBuf {
    archive_path(instances, id)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("game.jsa.stamp")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveState {
    Current,
    Missing,
    Stale,
}

#[must_use]
pub fn archive_state(
    instances: &crate::instance::manager::InstanceManager,
    id: &str,
    cp_entries: &[std::path::PathBuf],
) -> ArchiveState {
    if !archive_exists(instances, id) {
        return ArchiveState::Missing;
    }
    let Ok(stored) = std::fs::read_to_string(stamp_path(instances, id)) else {
        return ArchiveState::Current;
    };
    let mut lines = stored.lines();
    let (Some(content), Some(classpath)) = (lines.next(), lines.next()) else {
        return ArchiveState::Current;
    };
    let game_dir = instances.instance_dir(id).join("game");
    let content_ok = content == content_stamp(&game_dir);
    let classpath_ok = classpath == classpath_stamp(cp_entries);
    if content_ok && classpath_ok {
        ArchiveState::Current
    } else {
        ArchiveState::Stale
    }
}

pub fn write_stamp(
    instances: &crate::instance::manager::InstanceManager,
    id: &str,
    cp_entries: &[std::path::PathBuf],
) {
    let game_dir = instances.instance_dir(id).join("game");
    let body = format!(
        "{}\n{}\n",
        content_stamp(&game_dir),
        classpath_stamp(cp_entries)
    );
    let _ = std::fs::write(stamp_path(instances, id), body);
}

pub fn request_archive(state: &mut crate::app::state::AppState, instance_id: &str) {
    if let Some(mut cfg) = state
        .instance_list
        .iter()
        .find(|c| c.id == instance_id)
        .cloned()
    {
        cfg.appcds_pending = true;
        let _ = state.instances.save(&cfg);
        state.refresh_instances();
        if state.selected_instance.as_deref() == Some(instance_id) {
            state.edit_instance = state.selected();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &std::path::Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn stamp_is_stable_for_unchanged_content() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        write(&game.join("mods/a.jar"), "a");
        write(&game.join("mods/b.jar"), "b");
        let first = content_stamp(&game);
        assert_eq!(first, content_stamp(&game));
    }

    #[test]
    fn stamp_changes_when_a_mod_is_added() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        write(&game.join("mods/a.jar"), "a");
        let before = content_stamp(&game);
        write(&game.join("mods/c.jar"), "c");
        assert_ne!(before, content_stamp(&game));
    }

    #[test]
    fn stamp_changes_when_a_mod_is_removed() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        write(&game.join("mods/a.jar"), "a");
        write(&game.join("mods/b.jar"), "b");
        let before = content_stamp(&game);
        std::fs::remove_file(game.join("mods/b.jar")).unwrap();
        assert_ne!(before, content_stamp(&game));
    }

    #[test]
    fn stamp_changes_when_a_mod_is_replaced_with_another_build() {
        let temp = tempfile::tempdir().unwrap();
        let game = temp.path().join("game");
        write(&game.join("mods/sodium.jar"), "old contents");
        let before = content_stamp(&game);
        write(&game.join("mods/sodium.jar"), "new, longer contents");
        assert_ne!(before, content_stamp(&game));
    }

    #[test]
    fn stamp_does_not_depend_on_directory_iteration_order() {
        let temp = tempfile::tempdir().unwrap();
        let one = temp.path().join("one/game");
        let two = temp.path().join("two/game");
        for (dir, names) in [
            (&one, vec!["a.jar", "b.jar", "c.jar"]),
            (&two, vec!["c.jar", "a.jar", "b.jar"]),
        ] {
            for n in names {
                write(&dir.join("mods").join(n), n);
            }
        }
        assert_eq!(content_stamp(&one), content_stamp(&two));
    }

    #[test]
    fn a_missing_mods_folder_stamps_without_error() {
        let temp = tempfile::tempdir().unwrap();
        let _ = content_stamp(&temp.path().join("nothing-here"));
    }
}

#[cfg(test)]
mod stamp_tests {
    use super::*;

    #[test]
    fn classpath_stamp_is_order_sensitive() {
        let a = std::path::PathBuf::from("/libs/a.jar");
        let b = std::path::PathBuf::from("/libs/b.jar");
        assert_ne!(
            classpath_stamp(&[a.clone(), b.clone()]),
            classpath_stamp(&[b, a])
        );
    }

    #[test]
    fn classpath_stamp_is_stable_for_the_same_order() {
        let cp: Vec<std::path::PathBuf> = ["/libs/a.jar", "/libs/b.jar", "/client.jar"]
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        assert_eq!(classpath_stamp(&cp), classpath_stamp(&cp));
    }

    #[test]
    fn classpath_stamp_changes_when_a_jar_is_added() {
        let before: Vec<std::path::PathBuf> = ["/libs/a.jar", "/libs/b.jar"]
            .iter()
            .map(std::path::PathBuf::from)
            .collect();
        let mut after = before.clone();
        after.push(std::path::PathBuf::from("/libs/c.jar"));
        assert_ne!(classpath_stamp(&before), classpath_stamp(&after));
    }
}
