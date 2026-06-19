use std::{collections::HashSet, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::domain::{
    BackendStatus, OrphanPackageSummary, ParuCacheSummary, SystemSnapshot, audit_orphan_packages,
    inspect_cache, matches_keyword, matches_orphan_keyword, preview_remove_orphans, remove_orphans,
    trash_paru_cache, validate_orphan_remove_targets,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Overview,
    CacheSearch,
    Orphans,
    Actions,
}

impl Focus {
    pub const ALL: [Focus; 4] = [
        Focus::Overview,
        Focus::CacheSearch,
        Focus::Orphans,
        Focus::Actions,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Focus::Overview => "Overview",
            Focus::CacheSearch => "Paru Cache",
            Focus::Orphans => "Orphans",
            Focus::Actions => "Actions",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    ConfirmDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    Cancel,
    Trash,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheEntryState {
    Ready,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrphanPackageState {
    Ready,
    Removed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteTarget {
    Cache,
    Orphans,
}

#[derive(Debug, Clone)]
pub struct VisibleCacheRow {
    pub index: usize,
    pub state: CacheEntryState,
}

#[derive(Debug, Clone)]
pub struct VisibleOrphanRow {
    pub index: usize,
    pub state: OrphanPackageState,
}

#[derive(Debug)]
pub struct App {
    running: bool,
    focus: usize,
    cache_index: usize,
    orphan_index: usize,
    actions_index: usize,
    snapshot: SystemSnapshot,
    orphan_packages: Vec<OrphanPackageSummary>,
    actions: Vec<&'static str>,
    logs: Vec<String>,
    input_mode: InputMode,
    search_query: String,
    orphan_query: String,
    filtered_cache: Vec<VisibleCacheRow>,
    filtered_orphans: Vec<VisibleOrphanRow>,
    deleted_paths: HashSet<PathBuf>,
    removed_orphans: HashSet<String>,
    selected_paths: HashSet<PathBuf>,
    selected_orphans: HashSet<String>,
    confirm_action: ConfirmAction,
    confirm_target: Option<DeleteTarget>,
    orphan_remove_preview: Vec<String>,
    skip_orphan_preview: bool,
}

impl App {
    pub fn new() -> Self {
        let snapshot = SystemSnapshot::detect();
        Self::new_with_snapshot(snapshot)
    }

    fn new_with_snapshot(snapshot: SystemSnapshot) -> Self {
        let (orphan_packages, orphan_log) = detect_orphan_packages();
        Self::new_with_snapshot_and_orphans(snapshot, orphan_packages, orphan_log)
    }

    fn new_with_snapshot_and_orphans(
        snapshot: SystemSnapshot,
        orphan_packages: Vec<OrphanPackageSummary>,
        orphan_log: String,
    ) -> Self {
        let logs = vec![
            "Welcome to pacc.".to_string(),
            "Goal: manage pacman, paru, and AUR flows from one TUI.".to_string(),
            "Press / to search the active package workspace.".to_string(),
            "Use arrow keys to move, Space to mark, Enter to inspect.".to_string(),
            orphan_log,
        ];
        let filtered_cache = (0..snapshot.paru_cache.len())
            .map(|index| VisibleCacheRow {
                index,
                state: CacheEntryState::Ready,
            })
            .collect::<Vec<_>>();
        let filtered_orphans = (0..orphan_packages.len())
            .map(|index| VisibleOrphanRow {
                index,
                state: OrphanPackageState::Ready,
            })
            .collect::<Vec<_>>();

        Self {
            running: true,
            focus: 1,
            cache_index: 0,
            orphan_index: 0,
            actions_index: 0,
            snapshot,
            orphan_packages,
            actions: vec![
                "Sync databases",
                "Upgrade system",
                "Review orphan packages",
                "Clean cache",
                "Inspect AUR updates",
            ],
            logs,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            orphan_query: String::new(),
            filtered_cache,
            filtered_orphans,
            deleted_paths: HashSet::new(),
            removed_orphans: HashSet::new(),
            selected_paths: HashSet::new(),
            selected_orphans: HashSet::new(),
            confirm_action: ConfirmAction::Cancel,
            confirm_target: None,
            orphan_remove_preview: Vec::new(),
            skip_orphan_preview: false,
        }
    }

    #[allow(dead_code)]
    fn new_with_detected_orphans(snapshot: SystemSnapshot) -> Self {
        let (orphan_packages, orphan_log) = match audit_orphan_packages() {
            Ok(packages) => {
                let count = packages.len();
                (
                    packages,
                    format!("Audited orphan packages: {count} package(s) found."),
                )
            }
            Err(error) => (
                Vec::new(),
                format!("Failed to audit orphan packages: {error}"),
            ),
        };
        Self::new_with_snapshot_and_orphans(snapshot, orphan_packages, orphan_log)
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    pub fn snapshot(&self) -> &SystemSnapshot {
        &self.snapshot
    }

    pub fn actions(&self) -> &[&'static str] {
        &self.actions
    }

    pub fn logs(&self) -> &[String] {
        &self.logs
    }

    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn current_query(&self) -> &str {
        match self.focus() {
            Focus::Orphans => &self.orphan_query,
            _ => &self.search_query,
        }
    }

    pub fn focus(&self) -> Focus {
        Focus::ALL[self.focus]
    }

    pub fn selected_cache(&self) -> usize {
        self.cache_index
    }

    pub fn selected_action(&self) -> usize {
        self.actions_index
    }

    pub fn orphan_packages(&self) -> &[OrphanPackageSummary] {
        &self.orphan_packages
    }

    pub fn filtered_cache(&self) -> Vec<(&ParuCacheSummary, CacheEntryState)> {
        self.filtered_cache
            .iter()
            .filter_map(|row| {
                self.snapshot
                    .paru_cache
                    .get(row.index)
                    .map(|entry| (entry, row.state.clone()))
            })
            .collect()
    }

    pub fn filtered_orphans(&self) -> Vec<(&OrphanPackageSummary, OrphanPackageState)> {
        self.filtered_orphans
            .iter()
            .filter_map(|row| {
                self.orphan_packages
                    .get(row.index)
                    .map(|entry| (entry, row.state.clone()))
            })
            .collect()
    }

    pub fn selected_cache_entry(&self) -> Option<&ParuCacheSummary> {
        self.filtered_cache
            .get(self.cache_index)
            .and_then(|row| self.snapshot.paru_cache.get(row.index))
    }

    pub fn selected_orphan_entry(&self) -> Option<&OrphanPackageSummary> {
        self.filtered_orphans
            .get(self.orphan_index)
            .and_then(|row| self.orphan_packages.get(row.index))
    }

    pub fn selected_cache_state(&self) -> Option<CacheEntryState> {
        self.filtered_cache
            .get(self.cache_index)
            .map(|row| row.state.clone())
    }

    pub fn selected_orphan_state(&self) -> Option<OrphanPackageState> {
        self.filtered_orphans
            .get(self.orphan_index)
            .map(|row| row.state.clone())
    }

    pub fn confirm_action(&self) -> ConfirmAction {
        self.confirm_action
    }

    pub fn confirm_target(&self) -> Option<DeleteTarget> {
        self.confirm_target
    }

    pub fn is_selected(&self, entry: &ParuCacheSummary) -> bool {
        self.selected_paths.contains(&entry.path)
    }

    pub fn is_orphan_selected(&self, entry: &OrphanPackageSummary) -> bool {
        self.selected_orphans.contains(&entry.name)
    }

    pub fn selected_count(&self) -> usize {
        self.selected_paths.len()
    }

    pub fn selected_orphan_count(&self) -> usize {
        self.selected_orphans.len()
    }

    pub fn pending_delete_entries(&self) -> Vec<ParuCacheSummary> {
        self.selected_entries_for_delete()
    }

    pub fn pending_orphan_delete_entries(&self) -> Vec<OrphanPackageSummary> {
        self.selected_orphans_for_delete()
    }

    pub fn orphan_remove_preview(&self) -> &[String] {
        &self.orphan_remove_preview
    }

    pub fn selected_orphan(&self) -> usize {
        self.orphan_index
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Search => {
                self.handle_search_key_event(key);
                return;
            }
            InputMode::ConfirmDelete => {
                self.handle_confirm_delete_key_event(key);
                return;
            }
            InputMode::Normal => {}
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Tab => self.next_focus(),
            KeyCode::BackTab => self.prev_focus(),
            KeyCode::Down => self.move_next(),
            KeyCode::Up => self.move_prev(),
            KeyCode::Char(' ') => self.toggle_selected_current(),
            KeyCode::Enter => self.activate(),
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('/') => self.enter_search_mode(),
            KeyCode::Char('d') => self.begin_delete_flow(),
            _ => {}
        }
    }

    fn next_focus(&mut self) {
        self.focus = (self.focus + 1) % Focus::ALL.len();
    }

    fn prev_focus(&mut self) {
        self.focus = (self.focus + Focus::ALL.len() - 1) % Focus::ALL.len();
    }

    fn move_next(&mut self) {
        match self.focus() {
            Focus::Overview => {}
            Focus::CacheSearch => {
                if !self.filtered_cache.is_empty() {
                    self.cache_index = (self.cache_index + 1) % self.filtered_cache.len();
                }
            }
            Focus::Orphans => {
                if !self.filtered_orphans.is_empty() {
                    self.orphan_index = (self.orphan_index + 1) % self.filtered_orphans.len();
                }
            }
            Focus::Actions => {
                if !self.actions.is_empty() {
                    self.actions_index = (self.actions_index + 1) % self.actions.len();
                }
            }
        }
    }

    fn move_prev(&mut self) {
        match self.focus() {
            Focus::Overview => {}
            Focus::CacheSearch => {
                if !self.filtered_cache.is_empty() {
                    self.cache_index = (self.cache_index + self.filtered_cache.len() - 1)
                        % self.filtered_cache.len();
                }
            }
            Focus::Orphans => {
                if !self.filtered_orphans.is_empty() {
                    self.orphan_index = (self.orphan_index + self.filtered_orphans.len() - 1)
                        % self.filtered_orphans.len();
                }
            }
            Focus::Actions => {
                if !self.actions.is_empty() {
                    self.actions_index =
                        (self.actions_index + self.actions.len() - 1) % self.actions.len();
                }
            }
        }
    }

    fn activate(&mut self) {
        match self.focus() {
            Focus::Overview => self
                .logs
                .push("Overview is informational for now.".to_string()),
            Focus::CacheSearch => {
                if let Some(pkg) = self.selected_cache_entry() {
                    if self.selected_cache_state() == Some(CacheEntryState::Deleted) {
                        self.logs.push(format!(
                            "{} was moved to trash. Press r to reload from disk.",
                            pkg.name
                        ));
                        self.trim_logs();
                        return;
                    }

                    let details = inspect_cache(pkg);
                    self.logs.push(format!(
                        "Cache hit: {} | pkgbuild={} git={} pkg archives={} source archives={} files={} size={}.",
                        pkg.name,
                        yes_no(pkg.has_pkgbuild),
                        yes_no(pkg.has_git_metadata),
                        details.package_archives,
                        details.source_archives,
                        details.total_files,
                        human_bytes(details.total_bytes)
                    ));

                    if !details.sample_files.is_empty() {
                        self.logs
                            .push(format!("Sample files: {}", details.sample_files.join(", ")));
                    }
                }
            }
            Focus::Orphans => {
                if let Some(pkg) = self.selected_orphan_entry() {
                    if self.selected_orphan_state() == Some(OrphanPackageState::Removed) {
                        self.logs
                            .push(format!("{} was removed. Press r to audit again.", pkg.name));
                        self.trim_logs();
                        return;
                    }

                    self.logs.push(format!(
                        "Orphan: {} | version={} size={} reason={}.",
                        pkg.name,
                        pkg.version.as_deref().unwrap_or("-"),
                        pkg.installed_size.as_deref().unwrap_or("-"),
                        pkg.install_reason.as_deref().unwrap_or("-")
                    ));
                }
            }
            Focus::Actions => {
                if let Some(action) = self.actions.get(self.actions_index) {
                    self.logs.push(format!("Queued action stub: {}.", action));
                }
            }
        }
        self.trim_logs();
    }

    fn refresh(&mut self) {
        self.snapshot = SystemSnapshot::detect();
        self.deleted_paths.clear();
        self.selected_paths.clear();
        self.apply_search();
        self.logs
            .push("Refreshed backend detection and paru cache index.".to_string());

        match audit_orphan_packages() {
            Ok(packages) => {
                let count = packages.len();
                self.orphan_packages = packages;
                self.removed_orphans.clear();
                self.selected_orphans.clear();
                self.apply_orphan_search();
                self.logs.push(format!(
                    "Audited orphan packages: {count} package(s) found."
                ));
            }
            Err(error) => {
                self.logs
                    .push(format!("Failed to audit orphan packages: {error}"));
            }
        }

        self.trim_logs();
    }

    fn trim_logs(&mut self) {
        const MAX_LOGS: usize = 8;
        if self.logs.len() > MAX_LOGS {
            let overflow = self.logs.len() - MAX_LOGS;
            self.logs.drain(0..overflow);
        }
    }

    fn handle_search_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.logs
                    .push("Exited search mode. Current filter kept.".to_string());
            }
            KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                self.logs.push(self.search_applied_message());
            }
            KeyCode::Backspace => {
                self.current_query_mut().pop();
                self.apply_current_search();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.current_query_mut().clear();
                self.apply_current_search();
            }
            KeyCode::Char(ch) => {
                self.current_query_mut().push(ch);
                self.apply_current_search();
            }
            _ => {}
        }
        self.trim_logs();
    }

    fn enter_search_mode(&mut self) {
        if !matches!(self.focus(), Focus::CacheSearch | Focus::Orphans) {
            self.focus = Focus::ALL
                .iter()
                .position(|focus| *focus == Focus::CacheSearch)
                .unwrap_or(self.focus);
        }
        self.input_mode = InputMode::Search;
        self.current_query_mut().clear();
        self.apply_current_search();
        self.logs.push(format!(
            "Search mode: type keyword(s) to filter {}.",
            self.focus().title()
        ));
        self.trim_logs();
    }

    fn handle_confirm_delete_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') => {
                self.confirm_action = self.confirm_execute_action();
                self.execute_confirm_action();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.confirm_action = ConfirmAction::Cancel;
                self.execute_confirm_action();
            }
            KeyCode::Left => self.confirm_action = ConfirmAction::Cancel,
            KeyCode::Right => self.confirm_action = self.confirm_execute_action(),
            KeyCode::Enter => self.execute_confirm_action(),
            _ => {}
        }
    }

    fn execute_confirm_action(&mut self) {
        match self.confirm_action {
            ConfirmAction::Cancel => {
                self.input_mode = InputMode::Normal;
                self.confirm_target = None;
                self.orphan_remove_preview.clear();
                self.logs.push("Canceled deletion.".to_string());
            }
            ConfirmAction::Trash => self.confirm_selected_cache(),
            ConfirmAction::Remove => self.confirm_selected_orphans(),
        }
        self.trim_logs();
    }

    fn begin_delete_flow(&mut self) {
        match self.focus() {
            Focus::CacheSearch => self.begin_cache_delete_flow(),
            Focus::Orphans => self.begin_orphan_delete_flow(),
            _ => {
                self.logs
                    .push("Delete is available in Paru Cache and Orphans.".to_string());
                self.trim_logs();
            }
        }
    }

    fn begin_cache_delete_flow(&mut self) {
        if self.selected_count() > 0 {
            self.input_mode = InputMode::ConfirmDelete;
            self.confirm_action = ConfirmAction::Cancel;
            self.confirm_target = Some(DeleteTarget::Cache);
            return;
        }

        let Some(entry) = self.selected_cache_entry() else {
            self.logs
                .push("No cache entry selected for deletion.".to_string());
            self.trim_logs();
            return;
        };

        if self.selected_cache_state() == Some(CacheEntryState::Deleted) {
            self.logs.push(format!(
                "{} is already marked deleted. Press r to reload from disk.",
                entry.name
            ));
            self.trim_logs();
            return;
        }

        self.input_mode = InputMode::ConfirmDelete;
        self.confirm_action = ConfirmAction::Cancel;
        self.confirm_target = Some(DeleteTarget::Cache);
    }

    fn begin_orphan_delete_flow(&mut self) {
        if self.selected_orphan_count() > 0 {
            self.prepare_orphan_delete_confirmation();
            return;
        }

        let Some(entry) = self.selected_orphan_entry() else {
            self.logs
                .push("No orphan package selected for removal.".to_string());
            self.trim_logs();
            return;
        };

        if self.selected_orphan_state() == Some(OrphanPackageState::Removed) {
            self.logs.push(format!(
                "{} is already removed. Press r to audit again.",
                entry.name
            ));
            self.trim_logs();
            return;
        }

        self.prepare_orphan_delete_confirmation();
    }

    fn prepare_orphan_delete_confirmation(&mut self) {
        self.orphan_remove_preview.clear();
        let targets = self
            .selected_orphans_for_delete()
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();

        if self.skip_orphan_preview {
            self.orphan_remove_preview = targets.clone();
        } else {
            match validate_orphan_remove_targets(&targets, &self.orphan_packages)
                .and_then(|targets| preview_remove_orphans(&targets))
            {
                Ok(preview) => {
                    self.orphan_remove_preview = preview;
                    self.logs.push(format!(
                        "Removal preview ready for {} orphan package(s).",
                        targets.len()
                    ));
                }
                Err(error) => {
                    self.logs
                        .push(format!("Could not preview orphan removal: {error}"));
                }
            }
        }

        self.input_mode = InputMode::ConfirmDelete;
        self.confirm_action = ConfirmAction::Cancel;
        self.confirm_target = Some(DeleteTarget::Orphans);
    }

    fn confirm_selected_cache(&mut self) {
        let targets = self.selected_entries_for_delete();
        if targets.is_empty() {
            self.input_mode = InputMode::Normal;
            self.confirm_target = None;
            self.logs
                .push("No cache entry selected for deletion.".to_string());
            return;
        }
        let Some(clone_dir) = self.snapshot.paru_clone_dir.clone() else {
            self.input_mode = InputMode::Normal;
            self.confirm_target = None;
            self.logs
                .push("Cannot delete cache because paru clone dir is unavailable.".to_string());
            return;
        };

        let total = targets.len();
        let mut success_count = 0usize;

        for entry in targets {
            match trash_paru_cache(&entry, &clone_dir) {
                Ok(()) => {
                    self.deleted_paths.insert(entry.path.clone());
                    self.selected_paths.remove(&entry.path);
                    success_count += 1;
                    if total == 1 {
                        self.logs.push(format!(
                            "Moved {} cache to trash. Press r to reload from disk.",
                            entry.name
                        ));
                    }
                }
                Err(error) => {
                    self.logs
                        .push(format!("Failed to trash {}: {error}", entry.name));
                }
            }
        }

        if total > 1 && success_count > 0 {
            self.logs.push(format!(
                "Moved {} cache entries to trash. Press r to reload from disk.",
                success_count
            ));
        }

        self.apply_search();
        self.move_cache_selection_to_next_ready();
        self.input_mode = InputMode::Normal;
        self.confirm_target = None;
    }

    fn confirm_selected_orphans(&mut self) {
        let targets = self
            .selected_orphans_for_delete()
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();

        let targets = match validate_orphan_remove_targets(&targets, &self.orphan_packages) {
            Ok(targets) => targets,
            Err(error) => {
                self.input_mode = InputMode::Normal;
                self.confirm_target = None;
                self.logs
                    .push(format!("Cannot remove orphan package(s): {error}"));
                return;
            }
        };

        match remove_orphans(&targets) {
            Ok(()) => {
                for name in &targets {
                    self.removed_orphans.insert(name.clone());
                    self.selected_orphans.remove(name);
                }
                self.logs.push(format!(
                    "Removed {} orphan package(s). Press r to audit again.",
                    targets.len()
                ));
                self.apply_orphan_search();
                self.move_orphan_selection_to_next_ready();
            }
            Err(error) => {
                self.logs
                    .push(format!("Failed to remove orphan package(s): {error}"));
            }
        }

        self.input_mode = InputMode::Normal;
        self.confirm_target = None;
        self.orphan_remove_preview.clear();
    }

    fn move_cache_selection_to_next_ready(&mut self) {
        if self.filtered_cache.is_empty() {
            self.cache_index = 0;
            return;
        }

        self.cache_index = next_ready_index(self.cache_index, self.filtered_cache.len(), |index| {
            self.filtered_cache[index].state == CacheEntryState::Ready
        });
    }

    fn move_orphan_selection_to_next_ready(&mut self) {
        if self.filtered_orphans.is_empty() {
            self.orphan_index = 0;
            return;
        }

        self.orphan_index =
            next_ready_index(self.orphan_index, self.filtered_orphans.len(), |index| {
                self.filtered_orphans[index].state == OrphanPackageState::Ready
            });
    }

    fn toggle_selected_cache(&mut self) {
        let Some(entry) = self.selected_cache_entry().cloned() else {
            return;
        };

        if self.selected_cache_state() == Some(CacheEntryState::Deleted) {
            self.logs.push(format!(
                "{} is already deleted and cannot be selected.",
                entry.name
            ));
            self.trim_logs();
            return;
        }

        if self.selected_paths.contains(&entry.path) {
            self.selected_paths.remove(&entry.path);
            self.logs.push(format!("Unselected {}.", entry.name));
        } else {
            self.selected_paths.insert(entry.path.clone());
            self.logs.push(format!("Selected {}.", entry.name));
        }
        self.trim_logs();
    }

    fn toggle_selected_orphan(&mut self) {
        let Some(entry) = self.selected_orphan_entry().cloned() else {
            return;
        };

        if self.selected_orphan_state() == Some(OrphanPackageState::Removed) {
            self.logs.push(format!(
                "{} is already removed and cannot be selected.",
                entry.name
            ));
            self.trim_logs();
            return;
        }

        if self.selected_orphans.contains(&entry.name) {
            self.selected_orphans.remove(&entry.name);
            self.logs.push(format!("Unselected {}.", entry.name));
        } else {
            self.selected_orphans.insert(entry.name.clone());
            self.logs.push(format!("Selected {}.", entry.name));
        }
        self.trim_logs();
    }

    fn toggle_selected_current(&mut self) {
        match self.focus() {
            Focus::CacheSearch => self.toggle_selected_cache(),
            Focus::Orphans => self.toggle_selected_orphan(),
            _ => {}
        }
    }

    fn selected_entries_for_delete(&self) -> Vec<ParuCacheSummary> {
        if !self.selected_paths.is_empty() {
            return self
                .snapshot
                .paru_cache
                .iter()
                .filter(|entry| self.selected_paths.contains(&entry.path))
                .cloned()
                .collect();
        }

        self.selected_cache_entry().cloned().into_iter().collect()
    }

    fn selected_orphans_for_delete(&self) -> Vec<OrphanPackageSummary> {
        if !self.selected_orphans.is_empty() {
            return self
                .orphan_packages
                .iter()
                .filter(|entry| self.selected_orphans.contains(&entry.name))
                .filter(|entry| !self.removed_orphans.contains(&entry.name))
                .cloned()
                .collect();
        }

        self.selected_orphan_entry()
            .filter(|entry| !self.removed_orphans.contains(&entry.name))
            .cloned()
            .into_iter()
            .collect()
    }

    fn current_query_mut(&mut self) -> &mut String {
        match self.focus() {
            Focus::Orphans => &mut self.orphan_query,
            _ => &mut self.search_query,
        }
    }

    fn apply_current_search(&mut self) {
        match self.focus() {
            Focus::Orphans => self.apply_orphan_search(),
            _ => self.apply_search(),
        }
    }

    fn search_applied_message(&self) -> String {
        match self.focus() {
            Focus::Orphans => format!(
                "Applied orphan package filter: \"{}\" ({} matches).",
                self.orphan_query,
                self.filtered_orphans.len()
            ),
            _ => format!(
                "Applied paru cache filter: \"{}\" ({} matches).",
                self.search_query,
                self.filtered_cache.len()
            ),
        }
    }

    fn confirm_execute_action(&self) -> ConfirmAction {
        match self.confirm_target {
            Some(DeleteTarget::Orphans) => ConfirmAction::Remove,
            _ => ConfirmAction::Trash,
        }
    }

    fn apply_search(&mut self) {
        self.filtered_cache = self
            .snapshot
            .paru_cache
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                matches_keyword(entry, &self.search_query).then_some(VisibleCacheRow {
                    index,
                    state: if self.deleted_paths.contains(&entry.path) {
                        CacheEntryState::Deleted
                    } else {
                        CacheEntryState::Ready
                    },
                })
            })
            .collect();

        if self.filtered_cache.is_empty() {
            self.cache_index = 0;
        } else if self.cache_index >= self.filtered_cache.len() {
            self.cache_index = self.filtered_cache.len() - 1;
        }
    }

    fn apply_orphan_search(&mut self) {
        self.filtered_orphans = self
            .orphan_packages
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                matches_orphan_keyword(entry, &self.orphan_query).then_some(VisibleOrphanRow {
                    index,
                    state: if self.removed_orphans.contains(&entry.name) {
                        OrphanPackageState::Removed
                    } else {
                        OrphanPackageState::Ready
                    },
                })
            })
            .collect();

        if self.filtered_orphans.is_empty() {
            self.orphan_index = 0;
        } else if self.orphan_index >= self.filtered_orphans.len() {
            self.orphan_index = self.filtered_orphans.len() - 1;
        }
    }
}

fn detect_orphan_packages() -> (Vec<OrphanPackageSummary>, String) {
    match audit_orphan_packages() {
        Ok(packages) => {
            let count = packages.len();
            (
                packages,
                format!("Audited orphan packages: {count} package(s) found."),
            )
        }
        Err(error) => (
            Vec::new(),
            format!("Failed to audit orphan packages: {error}"),
        ),
    }
}

fn next_ready_index(current_index: usize, len: usize, is_ready: impl Fn(usize) -> bool) -> usize {
    let current_index = current_index.min(len - 1);

    if let Some(index) = (current_index + 1..len).find(|index| is_ready(*index)) {
        return index;
    }

    if let Some(index) = (0..=current_index).rev().find(|index| is_ready(*index)) {
        return index;
    }

    current_index
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl BackendStatus {
    pub fn badge(&self) -> &'static str {
        match self {
            BackendStatus::Available => "ready",
            BackendStatus::Missing => "missing",
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];

    let mut value = bytes as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{bytes} {}", UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::*;
    use crate::domain::{BackendStatus, OrphanPackageSummary, ParuCacheSummary, SystemSnapshot};

    #[test]
    fn slash_enters_search_mode_and_focuses_cache() {
        let mut app = test_app();

        app.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));

        assert_eq!(app.input_mode(), InputMode::Search);
        assert_eq!(app.focus(), Focus::CacheSearch);
        assert_eq!(app.search_query(), "");
    }

    #[test]
    fn typing_filters_cache_results() {
        let mut app = test_app();

        app.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));

        let names = app
            .filtered_cache()
            .iter()
            .map(|(entry, _)| entry.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["visual-studio-code-bin"]);
    }

    #[test]
    fn arrow_keys_navigate_cache_hits_in_normal_mode() {
        let mut app = test_app();

        assert_eq!(
            app.selected_cache_entry().map(|entry| entry.name.as_str()),
            Some("args")
        );

        app.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(
            app.selected_cache_entry().map(|entry| entry.name.as_str()),
            Some("visual-studio-code-bin")
        );

        app.handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(
            app.selected_cache_entry().map(|entry| entry.name.as_str()),
            Some("args")
        );
    }

    #[test]
    fn d_enters_confirm_delete_mode() {
        let mut app = test_app();

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert_eq!(app.input_mode(), InputMode::ConfirmDelete);
        assert_eq!(app.confirm_action(), ConfirmAction::Cancel);
    }

    #[test]
    fn n_cancels_delete_confirmation() {
        let mut app = test_app();

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));

        assert_eq!(app.input_mode(), InputMode::Normal);
        assert_eq!(app.selected_cache_state(), Some(CacheEntryState::Ready));
    }

    #[test]
    fn deleted_item_blocks_repeat_delete() {
        let mut app = test_app();
        app.deleted_paths
            .insert(PathBuf::from("/tmp/paru/clone/args"));
        app.apply_search();

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert_eq!(app.input_mode(), InputMode::Normal);
        assert_eq!(app.selected_cache_state(), Some(CacheEntryState::Deleted));
    }

    #[test]
    fn space_toggles_selected_cache_entry() {
        let mut app = test_app();

        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.selected_count(), 1);

        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.selected_count(), 0);
    }

    #[test]
    fn d_prefers_multi_select_targets() {
        let mut app = test_app();
        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

        assert_eq!(app.selected_count(), 2);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.input_mode(), InputMode::ConfirmDelete);
    }

    #[test]
    fn pending_delete_entries_returns_multi_select_targets() {
        let mut app = test_app();
        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

        let names = app
            .pending_delete_entries()
            .into_iter()
            .map(|entry| entry.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec!["args".to_string(), "visual-studio-code-bin".to_string()]
        );
    }

    #[test]
    fn tab_can_focus_orphans_and_arrow_keys_navigate() {
        let mut app = test_app();

        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.focus(), Focus::Orphans);
        assert_eq!(
            app.selected_orphan_entry().map(|entry| entry.name.as_str()),
            Some("old-lib")
        );

        app.handle_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(
            app.selected_orphan_entry().map(|entry| entry.name.as_str()),
            Some("unused-tool")
        );
    }

    #[test]
    fn slash_filters_orphans_when_orphans_are_focused() {
        let mut app = test_app();
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        app.handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
        app.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));

        let names = app
            .filtered_orphans()
            .iter()
            .map(|(entry, _)| entry.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["unused-tool"]);
    }

    #[test]
    fn space_toggles_selected_orphan_entry() {
        let mut app = test_app();
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.selected_orphan_count(), 1);

        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.selected_orphan_count(), 0);
    }

    #[test]
    fn d_enters_orphan_confirm_delete_mode() {
        let mut app = test_app();
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));

        assert_eq!(app.input_mode(), InputMode::ConfirmDelete);
        assert_eq!(app.confirm_action(), ConfirmAction::Cancel);
        assert_eq!(app.confirm_target(), Some(DeleteTarget::Orphans));
    }

    #[test]
    fn removed_orphan_blocks_repeat_delete_and_selection() {
        let mut app = test_app();
        app.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        app.removed_orphans.insert("old-lib".to_string());
        app.apply_orphan_search();

        app.handle_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert_eq!(app.selected_orphan_count(), 0);

        app.handle_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        assert_eq!(app.input_mode(), InputMode::Normal);
        assert_eq!(
            app.selected_orphan_state(),
            Some(OrphanPackageState::Removed)
        );
    }

    #[test]
    fn next_ready_index_prefers_next_then_previous() {
        assert_eq!(next_ready_index(1, 4, |index| index == 3), 3);
        assert_eq!(next_ready_index(2, 4, |index| index == 1), 1);
        assert_eq!(next_ready_index(2, 4, |_| false), 2);
    }

    fn test_app() -> App {
        let mut app = App::new_with_snapshot_and_orphans(
            SystemSnapshot {
                pacman: BackendStatus::Available,
                paru: BackendStatus::Available,
                paru_clone_dir: Some(PathBuf::from("/tmp/paru/clone")),
                paru_cache: vec![
                    ParuCacheSummary {
                        name: "args".to_string(),
                        version: Some("6.4.16".to_string()),
                        description: Some("A CLI parser".to_string()),
                        url: Some("https://example.com/args".to_string()),
                        path: PathBuf::from("/tmp/paru/clone/args"),
                        pkgbuild_path: Some(PathBuf::from("/tmp/paru/clone/args/PKGBUILD")),
                        has_pkgbuild: true,
                        has_git_metadata: true,
                        package_archives: 0,
                        source_archives: 1,
                    },
                    ParuCacheSummary {
                        name: "visual-studio-code-bin".to_string(),
                        version: Some("1.125.0".to_string()),
                        description: Some("Editor for modern apps".to_string()),
                        url: Some("https://code.visualstudio.com/".to_string()),
                        path: PathBuf::from("/tmp/paru/clone/visual-studio-code-bin"),
                        pkgbuild_path: Some(PathBuf::from(
                            "/tmp/paru/clone/visual-studio-code-bin/PKGBUILD",
                        )),
                        has_pkgbuild: true,
                        has_git_metadata: true,
                        package_archives: 8,
                        source_archives: 4,
                    },
                ],
            },
            vec![
                OrphanPackageSummary {
                    name: "old-lib".to_string(),
                    version: Some("1.0.0-1".to_string()),
                    description: Some("Old dependency".to_string()),
                    installed_size: Some("2.00 MiB".to_string()),
                    install_reason: Some(
                        "Installed as a dependency for another package".to_string(),
                    ),
                },
                OrphanPackageSummary {
                    name: "unused-tool".to_string(),
                    version: Some("2.0.0-1".to_string()),
                    description: Some("Unused helper tool".to_string()),
                    installed_size: Some("5.00 MiB".to_string()),
                    install_reason: Some(
                        "Installed as a dependency for another package".to_string(),
                    ),
                },
            ],
            "Audited orphan packages: 2 package(s) found.".to_string(),
        );
        app.skip_orphan_preview = true;
        app
    }
}
