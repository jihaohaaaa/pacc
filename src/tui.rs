use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, TableState,
    },
};

use crate::app::{App, Focus};

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
        Span::raw("j/k move"),
        Span::raw("  "),
        Span::raw("Enter act"),
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
            Constraint::Percentage(34),
            Constraint::Percentage(38),
            Constraint::Percentage(28),
        ])
        .split(area);

    render_overview(frame, app, columns[0]);
    render_packages(frame, app, columns[1]);
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
        Line::raw("Planned modules"),
        Line::raw("1. sync and upgrade"),
        Line::raw("2. local package search"),
        Line::raw("3. AUR inspection"),
        Line::raw("4. orphan and cache cleanup"),
    ];

    let block = panel_block("System", app.focus() == Focus::Overview);
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn render_packages(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.snapshot().packages.iter().map(|pkg| {
        Row::new(vec![
            Cell::from(pkg.name.as_str()),
            Cell::from(pkg.source.label()),
            Cell::from(pkg.state_label()),
        ])
    });

    let widths = [
        Constraint::Percentage(58),
        Constraint::Percentage(20),
        Constraint::Percentage(22),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["Package", "Source", "State"]).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .block(panel_block("Packages", app.focus() == Focus::Packages));

    let mut state = TableState::default().with_selected(Some(app.selected_package()));
    frame.render_stateful_widget(table, area, &mut state);
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
