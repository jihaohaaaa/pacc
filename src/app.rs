use std::{collections::HashSet, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::domain::{
    BackendStatus, ParuCacheSummary, SystemSnapshot, inspect_cache, matches_keyword,
    trash_paru_cache,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Overview,
    CacheSearch,
    Actions,
}

impl Focus {
    pub const ALL: [Focus; 3] = [Focus::Overview, Focus::CacheSearch, Focus::Actions];

    pub fn title(self) -> &'static str {
        match self {
            Focus::Overview => "Overview",
            Focus::CacheSearch => "Paru Cache",
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheEntryState {
    Ready,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct VisibleCacheRow {
    pub index: usize,
    pub state: CacheEntryState,
}

#[derive(Debug)]
pub struct App {
    running: bool,
    focus: usize,
    cache_index: usize,
    actions_index: usize,
    snapshot: SystemSnapshot,
    actions: Vec<&'static str>,
    logs: Vec<String>,
    input_mode: InputMode,
    search_query: String,
    filtered_cache: Vec<VisibleCacheRow>,
    deleted_paths: HashSet<PathBuf>,
    selected_paths: HashSet<PathBuf>,
    confirm_action: ConfirmAction,
}

impl App {
    pub fn new() -> Self {
        let snapshot = SystemSnapshot::detect();
        Self::new_with_snapshot(snapshot)
    }

    fn new_with_snapshot(snapshot: SystemSnapshot) -> Self {
        let logs = vec![
            "Welcome to pacc.".to_string(),
            "Goal: manage pacman, paru, and AUR flows from one TUI.".to_string(),
            "Press / to search cached paru packages by keyword.".to_string(),
            "Use arrow keys to move through cache hits, Enter to inspect.".to_string(),
        ];
        let filtered_cache = (0..snapshot.paru_cache.len())
            .map(|index| VisibleCacheRow {
                index,
                state: CacheEntryState::Ready,
            })
            .collect::<Vec<_>>();

        Self {
            running: true,
            focus: 1,
            cache_index: 0,
            actions_index: 0,
            snapshot,
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
            filtered_cache,
            deleted_paths: HashSet::new(),
            selected_paths: HashSet::new(),
            confirm_action: ConfirmAction::Cancel,
        }
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

    pub fn focus(&self) -> Focus {
        Focus::ALL[self.focus]
    }

    pub fn selected_cache(&self) -> usize {
        self.cache_index
    }

    pub fn selected_action(&self) -> usize {
        self.actions_index
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

    pub fn selected_cache_entry(&self) -> Option<&ParuCacheSummary> {
        self.filtered_cache
            .get(self.cache_index)
            .and_then(|row| self.snapshot.paru_cache.get(row.index))
    }

    pub fn selected_cache_state(&self) -> Option<CacheEntryState> {
        self.filtered_cache
            .get(self.cache_index)
            .map(|row| row.state.clone())
    }

    pub fn confirm_action(&self) -> ConfirmAction {
        self.confirm_action
    }

    pub fn is_selected(&self, entry: &ParuCacheSummary) -> bool {
        self.selected_paths.contains(&entry.path)
    }

    pub fn selected_count(&self) -> usize {
        self.selected_paths.len()
    }

    pub fn pending_delete_entries(&self) -> Vec<ParuCacheSummary> {
        self.selected_entries_for_delete()
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
            KeyCode::Char(' ') => self.toggle_selected_cache(),
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
                self.logs.push(format!(
                    "Applied paru cache filter: \"{}\" ({} matches).",
                    self.search_query,
                    self.filtered_cache.len()
                ));
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.apply_search();
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search_query.clear();
                self.apply_search();
            }
            KeyCode::Char(ch) => {
                self.search_query.push(ch);
                self.apply_search();
            }
            _ => {}
        }
        self.trim_logs();
    }

    fn enter_search_mode(&mut self) {
        self.focus = Focus::ALL
            .iter()
            .position(|focus| *focus == Focus::CacheSearch)
            .unwrap_or(self.focus);
        self.input_mode = InputMode::Search;
        self.search_query.clear();
        self.apply_search();
        self.logs
            .push("Search mode: type keyword(s) to filter paru cache.".to_string());
        self.trim_logs();
    }

    fn handle_confirm_delete_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('y') => {
                self.confirm_action = ConfirmAction::Trash;
                self.execute_confirm_action();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.confirm_action = ConfirmAction::Cancel;
                self.execute_confirm_action();
            }
            KeyCode::Left => self.confirm_action = ConfirmAction::Cancel,
            KeyCode::Right => self.confirm_action = ConfirmAction::Trash,
            KeyCode::Enter => self.execute_confirm_action(),
            _ => {}
        }
    }

    fn execute_confirm_action(&mut self) {
        match self.confirm_action {
            ConfirmAction::Cancel => {
                self.input_mode = InputMode::Normal;
                self.logs.push("Canceled cache deletion.".to_string());
            }
            ConfirmAction::Trash => self.confirm_selected_cache(),
        }
        self.trim_logs();
    }

    fn begin_delete_flow(&mut self) {
        if self.selected_count() > 0 {
            self.input_mode = InputMode::ConfirmDelete;
            self.confirm_action = ConfirmAction::Cancel;
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
    }

    fn confirm_selected_cache(&mut self) {
        let targets = self.selected_entries_for_delete();
        if targets.is_empty() {
            self.input_mode = InputMode::Normal;
            self.logs
                .push("No cache entry selected for deletion.".to_string());
            return;
        }
        let Some(clone_dir) = self.snapshot.paru_clone_dir.clone() else {
            self.input_mode = InputMode::Normal;
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
        self.move_selection_to_next_ready();
        self.input_mode = InputMode::Normal;
    }

    fn move_selection_to_next_ready(&mut self) {
        if self.filtered_cache.is_empty() {
            self.cache_index = 0;
            return;
        }

        if let Some(index) = self
            .filtered_cache
            .iter()
            .enumerate()
            .find(|(_, row)| row.state == CacheEntryState::Ready)
            .map(|(index, _)| index)
        {
            self.cache_index = index;
            return;
        }

        self.cache_index = self.cache_index.min(self.filtered_cache.len() - 1);
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
    use crate::domain::{BackendStatus, ParuCacheSummary, SystemSnapshot};

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

    fn test_app() -> App {
        App::new_with_snapshot(SystemSnapshot {
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
        })
    }
}
