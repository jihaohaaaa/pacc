use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
        Wrap,
    },
};

use crate::app::{App, DeleteTarget, Focus, InputMode};

pub fn ui(frame: &mut Frame, app: &App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(14),
            Constraint::Length(8),
        ])
        .split(frame.area());

    render_header(frame, app, layout[0]);
    render_body(frame, app, layout[1]);
    render_logs(frame, app, layout[2]);
    if app.input_mode() == InputMode::ConfirmDelete {
        render_delete_confirm(frame, app);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(" pacc ", Style::default().fg(Color::Black).bg(Color::Green)),
        Span::raw("  Arch package workbench  "),
        Span::styled("pacman ", Style::default().add_modifier(Modifier::BOLD)),
        status_span(app.snapshot().pacman.badge()),
        Span::raw("  "),
        Span::styled("paru ", Style::default().add_modifier(Modifier::BOLD)),
        status_span(app.snapshot().paru.badge()),
        Span::raw("  "),
        Span::styled("mode ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(input_mode_label(app.input_mode())),
        Span::raw("  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" nav "),
        Span::styled("Arrows", Style::default().fg(Color::Yellow)),
        Span::raw(" move "),
        Span::styled("Space", Style::default().fg(Color::Yellow)),
        Span::raw(" mark "),
        Span::styled("/ d r q", Style::default().fg(Color::Yellow)),
    ]))
    .wrap(Wrap { trim: true })
    .block(Block::default().borders(Borders::ALL).title("Status"));

    frame.render_widget(title, area);
}

fn render_body(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(18),
            Constraint::Percentage(52),
            Constraint::Percentage(48),
        ])
        .split(area);

    render_navigation(frame, app, columns[0]);
    render_workspace(frame, app, columns[1]);
    render_details(frame, app, columns[2]);
}

fn render_navigation(frame: &mut Frame, app: &App, area: Rect) {
    let items = Focus::ALL.into_iter().map(|focus| {
        let marker = if focus == app.focus() { ">" } else { " " };
        let style = if focus == app.focus() {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        ListItem::new(Line::from(Span::styled(
            format!("{marker} {}", focus.title()),
            style,
        )))
    });

    let list = List::new(items)
        .block(panel_block("Workspace", true))
        .highlight_symbol("");

    frame.render_widget(list, area);
}

fn render_workspace(frame: &mut Frame, app: &App, area: Rect) {
    match app.focus() {
        Focus::Overview => render_overview(frame, app, area),
        Focus::CacheSearch => render_cache_workspace(frame, app, area),
        Focus::Orphans => render_orphans_workspace(frame, app, area),
        Focus::Actions => render_actions(frame, app, area),
    }
}

fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    match app.focus() {
        Focus::Overview => render_overview_details(frame, app, area),
        Focus::CacheSearch => render_cache_details(frame, app, area),
        Focus::Orphans => render_orphan_details(frame, app, area),
        Focus::Actions => render_action_details(frame, app, area),
    }
}

fn render_overview(frame: &mut Frame, app: &App, area: Rect) {
    let snapshot = app.snapshot();
    let lines = vec![
        Line::from(vec![
            Span::styled("pacman: ", Style::default().add_modifier(Modifier::BOLD)),
            status_span(snapshot.pacman.badge()),
        ]),
        Line::from(vec![
            Span::styled("paru:   ", Style::default().add_modifier(Modifier::BOLD)),
            status_span(snapshot.paru.badge()),
        ]),
        Line::raw(""),
        Line::from(format!(
            "clone dir: {}",
            snapshot
                .paru_clone_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unavailable".to_string())
        )),
        Line::from(format!("cached entries: {}", snapshot.paru_cache.len())),
        Line::from(format!(
            "filter query: {}",
            format_query(app.search_query())
        )),
        Line::from(format!("mode: {}", input_mode_label(app.input_mode()))),
        Line::from(format!("orphan packages: {}", app.orphan_packages().len())),
        Line::from(format!(
            "active query: {}",
            format_query(app.current_query())
        )),
        Line::raw(""),
        Line::raw("Tip: Tab into Paru Cache or Orphans, then / to filter."),
    ];

    let block = panel_block("System", app.focus() == Focus::Overview);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true }).block(block);
    frame.render_widget(paragraph, area);
}

fn render_cache_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);

    render_search_box(frame, app, sections[0], "Paru Cache");

    let cache_entries = app.filtered_cache();
    let rows = cache_entries.iter().map(|(pkg, state)| {
        let selected_marker = if app.is_selected(pkg) { "[x]" } else { "[ ]" };
        let row = Row::new(vec![
            Cell::from(format!("{selected_marker} {}", pkg.name)),
            Cell::from(pkg.version.as_deref().unwrap_or("-")),
            Cell::from(match state {
                crate::app::CacheEntryState::Ready => "ready",
                crate::app::CacheEntryState::Deleted => "deleted",
            }),
        ]);

        if *state == crate::app::CacheEntryState::Deleted {
            row.style(Style::default().fg(Color::Red))
        } else {
            row
        }
    });

    let widths = [
        Constraint::Percentage(46),
        Constraint::Percentage(38),
        Constraint::Percentage(16),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Package", "Version", "State"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .row_highlight_style(selected_row_style())
        .block(panel_block("Cache Hits", app.focus() == Focus::CacheSearch));

    let selected = (!cache_entries.is_empty()).then_some(app.selected_cache());
    let mut state = TableState::default().with_selected(selected);
    frame.render_stateful_widget(table, sections[1], &mut state);
}

fn render_orphans_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);

    render_search_box(frame, app, sections[0], "Orphans");

    let orphan_entries = app.filtered_orphans();
    let rows = orphan_entries.iter().map(|(pkg, state)| {
        let selected_marker = if app.is_orphan_selected(pkg) {
            "[x]"
        } else {
            "[ ]"
        };
        let row = Row::new(vec![
            Cell::from(selected_marker),
            Cell::from(pkg.name.as_str()),
            Cell::from(pkg.version.as_deref().unwrap_or("-")),
            Cell::from(pkg.installed_size.as_deref().unwrap_or("-")),
            Cell::from(match state {
                crate::app::OrphanPackageState::Ready => "ready",
                crate::app::OrphanPackageState::Removed => "removed",
            }),
        ]);

        if *state == crate::app::OrphanPackageState::Removed {
            row.style(Style::default().fg(Color::Red))
        } else {
            row
        }
    });

    let widths = [
        Constraint::Length(6),
        Constraint::Percentage(36),
        Constraint::Percentage(26),
        Constraint::Percentage(18),
        Constraint::Percentage(14),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Mark", "Package", "Version", "Size", "State"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .row_highlight_style(selected_row_style())
        .block(panel_block("Orphan Audit", app.focus() == Focus::Orphans));

    let selected = (!orphan_entries.is_empty()).then_some(app.selected_orphan());
    let mut state = TableState::default().with_selected(selected);
    frame.render_stateful_widget(table, sections[1], &mut state);
}

fn render_search_box(frame: &mut Frame, app: &App, area: Rect, title: &str) {
    let mode = if app.input_mode() == InputMode::Search {
        "typing"
    } else {
        "idle"
    };

    let query = if app.current_query().is_empty() {
        "/ <type package keyword>"
    } else {
        app.current_query()
    };

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled("query ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(query),
        Span::raw("  "),
        Span::styled(format!("[{}]", mode), Style::default().fg(Color::LightBlue)),
    ]))
    .block(panel_block(title, app.input_mode() == InputMode::Search));

    frame.render_widget(paragraph, area);
}

fn render_overview_details(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from("Workbench map"),
        Line::raw(""),
        Line::from(format!(
            "Paru cache entries: {}",
            app.snapshot().paru_cache.len()
        )),
        Line::from(format!("Orphan packages: {}", app.orphan_packages().len())),
        Line::raw(""),
        Line::raw("Paru Cache: search clone dirs and trash cache safely."),
        Line::raw("Orphans: audit pacman -Qdtq and remove with pacman -Rns."),
        Line::raw("Actions: placeholder command queue for future workflows."),
    ];

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(panel_block("Details", false));
    frame.render_widget(paragraph, area);
}

fn render_cache_details(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(entry) = app.selected_cache_entry() {
        if app.selected_cache_state() == Some(crate::app::CacheEntryState::Deleted) {
            vec![
                Line::from(format!("path: {}", entry.path.display())),
                Line::raw("Moved to trash"),
                Line::raw("Press r to reload from disk"),
            ]
        } else {
            vec![
                Line::from(format!("path: {}", entry.path.display())),
                Line::from(format!(
                    "pkgbuild: {}",
                    entry
                        .pkgbuild_path
                        .as_ref()
                        .map(|path| path.display().to_string())
                        .unwrap_or_else(|| "missing".to_string())
                )),
                Line::from(format!(
                    "git: {}  pkgbuild: {}",
                    bool_label(entry.has_git_metadata),
                    bool_label(entry.has_pkgbuild)
                )),
                Line::from(format!(
                    "pkg archives: {}  source archives: {}",
                    entry.package_archives, entry.source_archives
                )),
                Line::from(format!("url: {}", entry.url.as_deref().unwrap_or("-"))),
            ]
        }
    } else {
        vec![
            Line::raw("No cache entry matched the current query."),
            Line::raw("Try a package name, version, or description keyword."),
        ]
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(panel_block("Details", false));
    frame.render_widget(paragraph, area);
}

fn render_orphan_details(frame: &mut Frame, app: &App, area: Rect) {
    let lines = if let Some(entry) = app.selected_orphan_entry() {
        if app.selected_orphan_state() == Some(crate::app::OrphanPackageState::Removed) {
            vec![
                Line::from(format!("package: {}", entry.name)),
                Line::raw("Removed from system"),
                Line::raw("Press r to audit again"),
            ]
        } else {
            vec![
                Line::from(format!("package: {}", entry.name)),
                Line::from(format!(
                    "version: {}",
                    entry.version.as_deref().unwrap_or("-")
                )),
                Line::from(format!(
                    "size: {}",
                    entry.installed_size.as_deref().unwrap_or("-")
                )),
                Line::from(format!(
                    "reason: {}",
                    entry.install_reason.as_deref().unwrap_or("-")
                )),
                Line::raw(""),
                Line::from(format!(
                    "description: {}",
                    entry.description.as_deref().unwrap_or("-")
                )),
                Line::raw(""),
                Line::raw("Delete action: sudo -n pacman -Rns -- <targets>"),
            ]
        }
    } else {
        vec![
            Line::raw("No orphan package matched the current query."),
            Line::raw("Press r to re-run pacman orphan audit."),
        ]
    };

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(panel_block("Details", false));
    frame.render_widget(paragraph, area);
}

fn render_action_details(frame: &mut Frame, app: &App, area: Rect) {
    let action = app
        .actions()
        .get(app.selected_action())
        .copied()
        .unwrap_or("No action selected");
    let lines = vec![
        Line::from(format!("selected: {action}")),
        Line::raw(""),
        Line::raw("These are still action stubs."),
        Line::raw("Enter logs the selected action for now."),
    ];

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(panel_block("Details", false));
    frame.render_widget(paragraph, area);
}

fn render_actions(frame: &mut Frame, app: &App, area: Rect) {
    let items = app.actions().iter().map(|action| {
        ListItem::new(Line::from(vec![
            Span::styled("• ", Style::default().fg(Color::LightBlue)),
            Span::raw(*action),
        ]))
    });

    let list = List::new(items)
        .highlight_style(selected_row_style())
        .highlight_symbol(">")
        .block(panel_block("Action Stubs", app.focus() == Focus::Actions));

    let mut state = ListState::default();
    state.select(Some(app.selected_action()));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_logs(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let lines = app
        .logs()
        .iter()
        .map(|line| Line::from(Span::raw(line.as_str())))
        .collect::<Vec<_>>();

    let paragraph = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::ALL).title("Activity Log"));
    frame.render_widget(paragraph, area);
}

fn render_delete_confirm(frame: &mut Frame, app: &App) {
    match app.confirm_target() {
        Some(DeleteTarget::Orphans) => render_orphan_delete_confirm(frame, app),
        _ => render_cache_delete_confirm(frame, app),
    }
}

fn render_cache_delete_confirm(frame: &mut Frame, app: &App) {
    let area = centered_rect(68, 12, frame.area());
    frame.render_widget(Clear, area);

    let targets = app.pending_delete_entries();
    let target_names = targets
        .iter()
        .map(|entry| entry.name.as_str())
        .take(4)
        .collect::<Vec<_>>();
    let extra_count = targets.len().saturating_sub(target_names.len());
    let path = if targets.len() == 1 {
        targets[0].path.display().to_string()
    } else {
        app.snapshot()
            .paru_clone_dir
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    };

    let cancel_style = if app.confirm_action() == crate::app::ConfirmAction::Cancel {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let trash_style = if app.confirm_action() == crate::app::ConfirmAction::Trash {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Red)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };

    let mut lines = vec![
        Line::from(if app.selected_count() > 0 {
            format!("Delete {} selected cache entries", app.selected_count())
        } else {
            format!(
                "Delete cache for: {}",
                targets
                    .first()
                    .map(|entry| entry.name.as_str())
                    .unwrap_or("unknown")
            )
        }),
        Line::from(format!("Path: {path}")),
    ];

    if targets.len() > 1 {
        lines.push(Line::from(format!("Targets: {}", target_names.join(", "))));
        if extra_count > 0 {
            lines.push(Line::from(format!("... and {extra_count} more")));
        }
    }

    lines.extend([
        Line::raw("This will move the entire cache directory to trash."),
        Line::raw("It is not a permanent delete."),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" Cancel ", cancel_style),
            Span::raw("  "),
            Span::styled(" Trash ", trash_style),
        ]),
        Line::raw("Use y/n, Left/Right, or Enter."),
    ]);

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Confirm Delete"),
    );
    frame.render_widget(paragraph, area);
}

fn render_orphan_delete_confirm(frame: &mut Frame, app: &App) {
    let area = centered_rect(72, 14, frame.area());
    frame.render_widget(Clear, area);

    let targets = app.pending_orphan_delete_entries();
    let target_names = targets
        .iter()
        .map(|entry| entry.name.as_str())
        .take(5)
        .collect::<Vec<_>>();
    let extra_count = targets.len().saturating_sub(target_names.len());

    let cancel_style = confirm_button_style(
        app.confirm_action() == crate::app::ConfirmAction::Cancel,
        Color::Yellow,
    );
    let remove_style = confirm_button_style(
        app.confirm_action() == crate::app::ConfirmAction::Remove,
        Color::Red,
    );

    let preview = if app.orphan_remove_preview().is_empty() {
        "preview unavailable".to_string()
    } else {
        format!("preview targets: {}", app.orphan_remove_preview().len())
    };

    let mut lines = vec![
        Line::from(if app.selected_orphan_count() > 0 {
            format!(
                "Remove {} selected orphan packages",
                app.selected_orphan_count()
            )
        } else {
            format!(
                "Remove orphan package: {}",
                targets
                    .first()
                    .map(|entry| entry.name.as_str())
                    .unwrap_or("unknown")
            )
        }),
        Line::from("Command: sudo -n pacman -Rns -- <targets>"),
        Line::from(preview),
    ];

    if !target_names.is_empty() {
        lines.push(Line::from(format!("Targets: {}", target_names.join(", "))));
    }
    if extra_count > 0 {
        lines.push(Line::from(format!("... and {extra_count} more")));
    }

    lines.extend([
        Line::raw("This removes packages from the system."),
        Line::raw("sudo must be usable without an interactive password prompt."),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" Cancel ", cancel_style),
            Span::raw("  "),
            Span::styled(" Remove ", remove_style),
        ]),
        Line::raw("Use y/n, Left/Right, or Enter."),
    ]);

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Confirm Orphan Removal"),
    );
    frame.render_widget(paragraph, area);
}

fn selected_row_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Green)
        .add_modifier(Modifier::BOLD)
}

fn confirm_button_style(active: bool, color: Color) -> Style {
    if active {
        Style::default()
            .fg(Color::Black)
            .bg(color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let border_style = if focused {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    Block::default()
        .title(title.bold())
        .borders(Borders::ALL)
        .border_style(border_style)
}

fn status_span(status: &str) -> Span<'static> {
    match status {
        "ready" => Span::styled(status.to_string(), Style::default().fg(Color::Green)),
        _ => Span::styled(status.to_string(), Style::default().fg(Color::Red)),
    }
}

fn bool_label(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn input_mode_label(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Normal => "normal",
        InputMode::Search => "search",
        InputMode::ConfirmDelete => "confirm-delete",
    }
}

fn format_query(query: &str) -> String {
    if query.is_empty() {
        "<none>".to_string()
    } else {
        query.to_string()
    }
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Percentage(percent_x),
            Constraint::Fill(1),
        ])
        .split(vertical[1]);

    horizontal[1]
}
