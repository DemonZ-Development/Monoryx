use crate::app::{AppState, Page};
use crate::instance::{InstanceConfig, LoaderKind};
use crate::modrinth::updates::UpdateScan;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateTarget {
    pub id: String,
    pub minecraft_version: String,
    pub loader: LoaderKind,
    pub loader_version: String,
}

impl From<&InstanceConfig> for UpdateTarget {
    fn from(instance: &InstanceConfig) -> Self {
        Self {
            id: instance.id.clone(),
            minecraft_version: instance.minecraft_version.clone(),
            loader: instance.loader,
            loader_version: instance.loader_version.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InstanceUpdateReport {
    pub content: Result<UpdateScan, String>,
    pub loader: Result<Option<String>, String>,
}

impl InstanceUpdateReport {
    fn notification_signature(&self) -> Option<u64> {
        let mut versions: Vec<_> = self
            .content
            .as_ref()
            .ok()
            .into_iter()
            .flat_map(|scan| &scan.updates)
            .map(|update| (&update.file_name, &update.new_version_id))
            .collect();
        versions.sort_unstable();
        let loader = self.loader.as_ref().ok().and_then(Option::as_ref);
        if versions.is_empty() && loader.is_none() {
            return None;
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        versions.hash(&mut hash);
        loader.hash(&mut hash);
        Some(hash.finish())
    }

    fn retry_interval(&self) -> Duration {
        if self.content.is_err()
            || self.loader.is_err()
            || self
                .content
                .as_ref()
                .is_ok_and(|scan| !scan.errors.is_empty())
        {
            Duration::from_secs(300)
        } else {
            Duration::from_secs(1800)
        }
    }
}

struct CachedCheck {
    target: UpdateTarget,
    report: InstanceUpdateReport,
    checked_at: Instant,
    notified: Option<u64>,
}

struct PendingCheck {
    target: UpdateTarget,
    automatic: bool,
    abort: tokio::task::AbortHandle,
}

pub struct InstanceUpdateState {
    enabled: bool,
    generation: u64,
    pending: Option<PendingCheck>,
    cache: HashMap<String, CachedCheck>,
    view: Option<UpdateTarget>,
}

impl InstanceUpdateState {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            generation: 0,
            pending: None,
            cache: HashMap::new(),
            view: None,
        }
    }

    fn cancel(&mut self) {
        if let Some(pending) = self.pending.take() {
            pending.abort.abort();
            self.generation = self.generation.wrapping_add(1);
        }
    }
}

impl AppState {
    pub fn poll_instance_updates(&mut self) {
        if !self.instance_updates.enabled {
            return;
        }
        let Some(cfg) = self.selected() else {
            self.instance_updates.cancel();
            self.instance_updates.view = None;
            self.updates_loading = false;
            self.updates.clear();
            self.loader_update_candidate = None;
            self.loader_update_checking = None;
            return;
        };
        let target = UpdateTarget::from(&cfg);
        if self.instance_updates.view.as_ref() != Some(&target) {
            self.instance_updates.cancel();
            self.instance_updates.view = Some(target.clone());
            self.updates.clear();
            self.updates_loading = false;
            self.updates_checked = false;
            self.updates_summary.clear();
            self.updates_error.clear();
            self.updates_instance = Some(cfg.id.clone());
            self.loader_update_candidate = None;
            self.loader_update_checking = None;
            self.loader_update_error.clear();
            self.loader_update_instance = Some(cfg.id.clone());
            let report = self
                .instance_updates
                .cache
                .get(&cfg.id)
                .filter(|cached| cached.target == target)
                .map(|cached| cached.report.clone());
            if let Some(report) = report {
                self.apply_instance_update_report(&target, &report);
            }
        }
        if self.edit_instance.is_none() && self.loader_update_instance.as_ref() != Some(&cfg.id) {
            self.loader_update_candidate = None;
            self.loader_update_error.clear();
            self.loader_update_instance = Some(cfg.id.clone());
            let report = self
                .instance_updates
                .cache
                .get(&cfg.id)
                .filter(|cached| cached.target == target)
                .map(|cached| cached.report.clone());
            if let Some(report) = report {
                self.apply_instance_update_report(&target, &report);
            }
        }
        if !self.config.auto_check_instance_updates
            || self.launcher_hidden
            || self.playing.values().any(|playing| *playing)
            || self.page == Page::Onboarding
        {
            if self
                .instance_updates
                .pending
                .as_ref()
                .is_some_and(|pending| pending.automatic)
            {
                self.instance_updates.cancel();
                self.updates_loading = false;
                if self.loader_update_checking.as_deref() == Some(&cfg.id) {
                    self.loader_update_checking = None;
                }
            }
            return;
        }
        if self.instance_updates.pending.is_some()
            || self.edit_instance.is_some()
            || self
                .operations
                .values()
                .any(|operation| operation.instance_id.as_ref() == Some(&cfg.id))
        {
            return;
        }
        let fresh = self
            .instance_updates
            .cache
            .get(&cfg.id)
            .is_some_and(|cached| {
                cached.target == target
                    && cached.checked_at.elapsed() < cached.report.retry_interval()
            });
        if !fresh && self.instance_readiness(&cfg).is_ready() {
            self.check_instance_updates(true);
        }
    }

    pub fn check_instance_updates(&mut self, automatic: bool) {
        let Some(cfg) = self.selected() else {
            return;
        };
        if self.instance_updates.pending.is_some() {
            return;
        }
        if self
            .operations
            .values()
            .any(|operation| operation.instance_id.as_ref() == Some(&cfg.id))
        {
            if !automatic {
                self.notify("Wait for the current installation to finish before checking updates.");
            }
            return;
        }
        self.instance_updates.generation = self.instance_updates.generation.wrapping_add(1);
        let generation = self.instance_updates.generation;
        let target = UpdateTarget::from(&cfg);
        self.instance_updates.view = Some(target.clone());
        self.updates_instance = Some(cfg.id.clone());
        self.updates_loading = true;
        self.updates_error.clear();
        if cfg.loader != LoaderKind::Vanilla && self.edit_instance.is_none() {
            self.loader_update_checking = Some(cfg.id.clone());
        }
        let abort = crate::app::tasks::scan_instance_updates(self, cfg, generation, target.clone());
        self.instance_updates.pending = Some(PendingCheck {
            target,
            automatic,
            abort,
        });
        self.egui_ctx.request_repaint();
    }

    pub fn finish_instance_update_check(
        &mut self,
        generation: u64,
        target: UpdateTarget,
        report: InstanceUpdateReport,
    ) {
        if self.instance_updates.generation != generation
            || !self
                .instance_updates
                .pending
                .as_ref()
                .is_some_and(|pending| pending.target == target)
        {
            return;
        }
        let automatic = self.instance_updates.pending.take().unwrap().automatic;
        if self.loader_update_checking.as_deref() == Some(&target.id) {
            self.loader_update_checking = None;
        }
        self.updates_loading = false;
        let Some(cfg) = self
            .instance_list
            .iter()
            .find(|cfg| UpdateTarget::from(*cfg) == target)
            .cloned()
        else {
            return;
        };
        let old_signature = self
            .instance_updates
            .cache
            .get(&target.id)
            .and_then(|cached| cached.notified);
        let signature = report.notification_signature();
        if self.selected_instance.as_deref() == Some(&target.id) {
            self.apply_instance_update_report(&target, &report);
            self.refresh_library();
            if signature.is_some() && (!automatic || signature != old_signature) {
                self.notify(format!(
                    "Compatible updates are available for {}. Open Library to review them.",
                    cfg.name
                ));
            } else if !automatic
                && report
                    .content
                    .as_ref()
                    .is_ok_and(|scan| scan.errors.is_empty() && scan.untracked == 0)
                && matches!(report.loader, Ok(None))
            {
                self.notify(format!(
                    "{} is up to date for Minecraft {}.",
                    cfg.name, cfg.minecraft_version
                ));
            }
        }
        self.instance_updates.cache.insert(
            target.id.clone(),
            CachedCheck {
                target,
                report,
                checked_at: Instant::now(),
                notified: signature.or(old_signature),
            },
        );
        self.instance_updates
            .cache
            .retain(|id, _| self.instance_list.iter().any(|cfg| &cfg.id == id));
        while self.instance_updates.cache.len() > 8 {
            let oldest = self
                .instance_updates
                .cache
                .iter()
                .min_by_key(|(_, cached)| cached.checked_at)
                .map(|(id, _)| id.clone())
                .unwrap();
            self.instance_updates.cache.remove(&oldest);
        }
    }

    fn apply_instance_update_report(
        &mut self,
        target: &UpdateTarget,
        report: &InstanceUpdateReport,
    ) {
        self.updates_instance = Some(target.id.clone());
        self.updates_checked = true;
        match &report.content {
            Ok(scan) => {
                self.updates = scan.updates.clone();
                self.updates_summary = format!(
                    "Checked {} item(s). {} compatible update(s) available for Minecraft {}.",
                    scan.checked,
                    self.updates.len(),
                    target.minecraft_version
                );
                if scan.untracked > 0 {
                    self.updates_summary.push_str(&format!(
                        " {} manual item(s) could not be identified.",
                        scan.untracked
                    ));
                }
                self.updates_error = scan.errors.join("; ");
            }
            Err(error) => {
                self.updates_error = error.clone();
                self.updates_summary.clear();
            }
        }
        if self
            .edit_instance
            .as_ref()
            .is_none_or(|cfg| cfg.id == target.id)
        {
            self.loader_update_instance = Some(target.id.clone());
            match &report.loader {
                Ok(version) => {
                    self.loader_update_candidate =
                        version.clone().map(|version| (target.id.clone(), version));
                    self.loader_update_error.clear();
                }
                Err(error) => self.loader_update_error = error.clone(),
            }
        }
    }

    pub fn invalidate_instance_updates(&mut self, id: &str) {
        self.instance_updates.cache.remove(id);
        if self
            .instance_updates
            .pending
            .as_ref()
            .is_some_and(|pending| pending.target.id == id)
        {
            self.instance_updates.cancel();
            self.updates_loading = false;
            if self.loader_update_checking.as_deref() == Some(id) {
                self.loader_update_checking = None;
            }
        }
    }

    pub fn instance_update_count(&self) -> usize {
        let content = if self.updates_instance == self.selected_instance {
            self.updates.len()
        } else {
            0
        };
        content
            + usize::from(
                self.loader_update_candidate
                    .as_ref()
                    .is_some_and(|(id, _)| Some(id) == self.selected_instance.as_ref()),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modrinth::updates::UpdateInfo;
    use crate::storage::paths::MonoryxPaths;

    fn fixture() -> (tempfile::TempDir, AppState, InstanceConfig) {
        let dir = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let cc = eframe::CreationContext::_new_kittest(ctx);
        let mut state = AppState::new_for_preview(&cc, MonoryxPaths::new(dir.path().into()));
        let cfg = state
            .instances
            .create(
                "Survival".into(),
                "1.21.1".into(),
                LoaderKind::Fabric,
                "0.16.9".into(),
            )
            .unwrap();
        state.instance_list = vec![cfg.clone()];
        state.selected_instance = Some(cfg.id.clone());
        state.refresh_readiness_cache();
        state.page = Page::Home;
        state.instance_updates = InstanceUpdateState::new(true);
        (dir, state, cfg)
    }

    fn pending(state: &mut AppState, cfg: &InstanceConfig, automatic: bool) -> (u64, UpdateTarget) {
        let target = UpdateTarget::from(cfg);
        state.instance_updates.generation += 1;
        state.instance_updates.pending = Some(PendingCheck {
            target: target.clone(),
            automatic,
            abort: state
                .runtime
                .spawn(std::future::pending::<()>())
                .abort_handle(),
        });
        (state.instance_updates.generation, target)
    }

    fn available() -> InstanceUpdateReport {
        InstanceUpdateReport {
            content: Ok(UpdateScan {
                updates: vec![UpdateInfo {
                    file_name: "fabric-api.jar".into(),
                    kind: crate::content::ContentKind::Mod,
                    project_id: "fabric-api".into(),
                    title: "Fabric API".into(),
                    current_version: "1".into(),
                    new_version: "2".into(),
                    new_version_id: "v2".into(),
                }],
                checked: 1,
                modrinth_checked: 1,
                ..Default::default()
            }),
            loader: Ok(Some("0.16.14".into())),
        }
    }

    #[test]
    fn ready_instances_start_one_automatic_check_without_a_button_click() {
        let (_dir, mut state, mut cfg) = fixture();
        cfg.loader = LoaderKind::Vanilla;
        cfg.loader_version.clear();
        cfg.resolved_version_id = "1.21.1".into();
        state.instances.save(&cfg).unwrap();
        state.instance_list = vec![cfg.clone()];
        let jar = crate::instance::readiness::client_jar_for(&state.paths, &cfg).unwrap();
        std::fs::create_dir_all(jar.parent().unwrap()).unwrap();
        std::fs::write(jar, []).unwrap();
        state.refresh_readiness_cache();
        state.poll_instance_updates();
        let generation = state.instance_updates.generation;
        assert!(state.updates_loading);
        assert!(state.instance_updates.pending.is_some());
        state.poll_instance_updates();
        assert_eq!(state.instance_updates.generation, generation);
        state.instance_updates.cancel();
    }

    #[test]
    fn automatic_checks_respect_preferences_gameplay_and_readiness() {
        let (_dir, mut state, cfg) = fixture();
        state.poll_instance_updates();
        assert!(state.instance_updates.pending.is_none());
        state.config.auto_check_instance_updates = false;
        state.poll_instance_updates();
        assert!(state.instance_updates.pending.is_none());
        state.config.auto_check_instance_updates = true;
        state.playing.insert(cfg.id, true);
        state.poll_instance_updates();
        assert!(state.instance_updates.pending.is_none());
    }

    #[test]
    fn results_for_an_older_minecraft_or_loader_snapshot_are_ignored() {
        for change_loader in [false, true] {
            let (_dir, mut state, cfg) = fixture();
            let (generation, target) = pending(&mut state, &cfg, true);
            if change_loader {
                state.instance_list[0].loader = LoaderKind::Quilt;
            } else {
                state.instance_list[0].minecraft_version = "1.20.1".into();
            }
            state.finish_instance_update_check(generation, target, available());
            assert_eq!(state.instance_update_count(), 0);
            assert!(state.instance_updates.cache.is_empty());
            assert!(state.notice.is_empty());
        }
    }

    #[test]
    fn available_updates_notify_once_and_restore_from_cache_on_selection() {
        let (_dir, mut state, cfg) = fixture();
        let (generation, target) = pending(&mut state, &cfg, true);
        state.finish_instance_update_check(generation, target, available());
        assert_eq!(state.instance_update_count(), 2);
        assert!(state.notice.contains("Compatible updates"));
        state.notice.clear();
        let (generation, target) = pending(&mut state, &cfg, true);
        state.finish_instance_update_check(generation, target, available());
        assert!(state.notice.is_empty());
        state.updates.clear();
        state.loader_update_candidate = None;
        state.instance_updates.view = None;
        state.poll_instance_updates();
        assert_eq!(state.instance_update_count(), 2);
        assert!(state.instance_updates.pending.is_none());
    }

    #[test]
    fn invalidating_an_install_discards_in_flight_update_results() {
        let (_dir, mut state, cfg) = fixture();
        let (generation, target) = pending(&mut state, &cfg, true);
        state.invalidate_instance_updates(&cfg.id);
        state.finish_instance_update_check(generation, target, available());
        assert!(state.updates.is_empty());
        assert!(state.instance_updates.cache.is_empty());
    }

    #[test]
    fn failed_checks_cannot_report_an_instance_as_up_to_date() {
        let (_dir, mut state, cfg) = fixture();
        let (generation, target) = pending(&mut state, &cfg, false);
        state.finish_instance_update_check(
            generation,
            target,
            InstanceUpdateReport {
                content: Err("Offline".into()),
                loader: Err("Offline".into()),
            },
        );
        assert!(state.notice.is_empty());
        assert_eq!(state.updates_error, "Offline");
        assert_eq!(
            state.instance_updates.cache[&cfg.id]
                .report
                .retry_interval(),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn automatic_results_cannot_overwrite_a_different_selected_instance() {
        let (_dir, mut state, cfg) = fixture();
        let (generation, target) = pending(&mut state, &cfg, true);
        state.selected_instance = Some("other".into());
        state.finish_instance_update_check(generation, target, available());
        assert_eq!(state.instance_update_count(), 0);
        assert!(state.notice.is_empty());
        assert!(state.instance_updates.cache.contains_key(&cfg.id));
    }

    #[test]
    fn clearing_selection_cancels_the_check_and_clears_its_loading_state() {
        let (_dir, mut state, cfg) = fixture();
        let (generation, target) = pending(&mut state, &cfg, true);
        state.updates_loading = true;
        state.selected_instance = None;
        state.poll_instance_updates();
        assert!(!state.updates_loading);
        assert!(state.instance_updates.pending.is_none());
        state.finish_instance_update_check(generation, target, available());
        assert!(state.instance_updates.cache.is_empty());
    }
}
