use crossterm::event::{KeyCode, KeyEvent};

use crate::domain::{BackendStatus, PackageSource, PackageSummary, SystemSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Overview,
    Packages,
    Actions,
}

impl Focus {
    pub const ALL: [Focus; 3] = [Focus::Overview, Focus::Packages, Focus::Actions];

    pub fn title(self) -> &'static str {
        match self {
            Focus::Overview => "Overview",
            Focus::Packages => "Packages",
            Focus::Actions => "Actions",
        }
    }
}

#[derive(Debug)]
pub struct App {
    running: bool,
    focus: usize,
    package_index: usize,
    actions_index: usize,
    snapshot: SystemSnapshot,
    actions: Vec<&'static str>,
    logs: Vec<String>,
}

impl App {
    pub fn new() -> Self {
        let snapshot = SystemSnapshot::detect();
        let logs = vec![
            "Welcome to pacc.".to_string(),
            "Goal: manage pacman, paru, and AUR flows from one TUI.".to_string(),
            "Next step: wire real package queries and actions.".to_string(),
        ];

        Self {
            running: true,
            focus: 0,
            package_index: 0,
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

    pub fn focus(&self) -> Focus {
        Focus::ALL[self.focus]
    }

    pub fn selected_package(&self) -> usize {
        self.package_index
    }

    pub fn selected_action(&self) -> usize {
        self.actions_index
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Tab => self.next_focus(),
            KeyCode::BackTab => self.prev_focus(),
            KeyCode::Down | KeyCode::Char('j') => self.move_next(),
            KeyCode::Up | KeyCode::Char('k') => self.move_prev(),
            KeyCode::Enter => self.activate(),
            KeyCode::Char('r') => self.refresh(),
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
            Focus::Packages => {
                if !self.snapshot.packages.is_empty() {
                    self.package_index = (self.package_index + 1) % self.snapshot.packages.len();
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
            Focus::Packages => {
                if !self.snapshot.packages.is_empty() {
                    self.package_index = (self.package_index + self.snapshot.packages.len() - 1)
                        % self.snapshot.packages.len();
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
            Focus::Packages => {
                if let Some(pkg) = self.snapshot.packages.get(self.package_index) {
                    self.logs.push(format!(
                        "Selected package: {} {} from {}.",
                        pkg.name,
                        pkg.version,
                        pkg.source.label()
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
        self.logs
            .push("Refreshed backend detection and demo package snapshot.".to_string());
        self.trim_logs();
    }

    fn trim_logs(&mut self) {
        const MAX_LOGS: usize = 8;
        if self.logs.len() > MAX_LOGS {
            let overflow = self.logs.len() - MAX_LOGS;
            self.logs.drain(0..overflow);
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

impl PackageSource {
    pub fn label(&self) -> &'static str {
        match self {
            PackageSource::Pacman => "pacman",
            PackageSource::Paru => "paru",
            PackageSource::Aur => "AUR",
        }
    }
}

impl PackageSummary {
    pub fn state_label(&self) -> &'static str {
        if self.has_update { "update" } else { "current" }
    }
}
