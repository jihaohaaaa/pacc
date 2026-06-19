use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
    },
};

use crate::app::{App, Focus, InputMode};

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
    let spans = Focus::ALL.into_iter().map(|focus| {
        let active = focus == app.focus();
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        Span::styled(format!(" {} ", focus.title()), style)
    });

    let title = Paragraph::new(Line::from(vec![
        Span::styled(" pacc ", Style::default().fg(Color::Black).bg(Color::Green)),
        Span::raw("  Arch package assistant  "),
        Span::raw("Tab switch"),
        Span::raw("  "),
        Span::raw("Arrows move"),
        Span::raw("  "),
        Span::raw("Space mark"),
        Span::raw("  "),
        Span::raw("Enter act"),
        Span::raw("  "),
        Span::raw("/ search"),
        Span::raw("  "),
        Span::raw("d delete"),
        Span::raw("  "),
        Span::raw("r refresh"),
        Span::raw("  "),
        Span::raw("q quit"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(Line::from(spans.collect::<Vec<_>>())),
    );

    frame.render_widget(title, area);
}

fn render_body(frame: &mut Frame, app: &App, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(42),
            Constraint::Percentage(28),
        ])
        .split(area);

    render_overview(frame, app, columns[0]);
    render_cache_search(frame, app, columns[1]);
    render_actions(frame, app, columns[2]);
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
        Line::raw(""),
        Line::raw("Tip: press / to search paru cache"),
    ];

    let block = panel_block("System", app.focus() == Focus::Overview);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_cache_search(frame: &mut Frame, app: &App, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(8),
            Constraint::Length(8),
        ])
        .split(area);

    render_search_box(frame, app, sections[0]);

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
        .row_highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(panel_block("Cache Hits", app.focus() == Focus::CacheSearch));

    let selected = (!cache_entries.is_empty()).then_some(app.selected_cache());
    let mut state = TableState::default().with_selected(selected);
    frame.render_stateful_widget(table, sections[1], &mut state);

    render_cache_details(frame, app, sections[2]);
}

fn render_search_box(frame: &mut Frame, app: &App, area: Rect) {
    let mode = if app.input_mode() == InputMode::Search {
        "typing"
    } else {
        "idle"
    };

    let query = if app.search_query().is_empty() {
        "/ <type package keyword>"
    } else {
        app.search_query()
    };

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled("query ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(query),
        Span::raw("  "),
        Span::styled(format!("[{}]", mode), Style::default().fg(Color::LightBlue)),
    ]))
    .block(panel_block("Search", app.focus() == Focus::CacheSearch));

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

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Details"));
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
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
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

    let paragraph =
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title("Logs"));
    frame.render_widget(paragraph, area);
}

fn render_delete_confirm(frame: &mut Frame, app: &App) {
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

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Confirm Delete"),
    );
    frame.render_widget(paragraph, area);
}

fn panel_block<'a>(title: &'a str, focused: bool) -> Block<'a> {
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
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
