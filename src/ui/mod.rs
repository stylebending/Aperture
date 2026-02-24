mod controller;
mod locker;
mod nexus;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Tabs},
    Frame,
};

use crate::app::{App, Modal, Tab};

pub fn render(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Length(1), // Tab description
            Constraint::Min(0),    // Content (will be split horizontally)
            Constraint::Length(1), // Status bar
        ])
        .split(f.area());

    render_header(f, app, chunks[0]);
    render_tab_description(f, app, chunks[1]);

    // Split content area into main panel + sidebar
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),     // Main content (flexible)
            Constraint::Length(22), // Sidebar (22 columns for keybindings)
        ])
        .split(chunks[2]);

    if app.search_mode {
        let inner_area = Rect::new(
            content_chunks[0].x,
            content_chunks[0].y,
            content_chunks[0].width,
            content_chunks[0].height.saturating_sub(3),
        );
        render_tab_content(f, app, inner_area);
        render_search_box(f, app, content_chunks[0]);
    } else {
        render_tab_content(f, app, content_chunks[0]);
    }

    // Render sidebar with keybindings
    render_keybindings_sidebar(f, app, content_chunks[1]);

    render_status_bar(f, app, chunks[3]);

    if app.modal.is_some() {
        render_modal(f, app);
    }
}

fn render_header(f: &mut Frame, app: &mut App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let (first, rest) = t.as_str().split_at(1);
            Line::from(vec![
                Span::styled(
                    first,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                Span::styled(rest, Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Aperture ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .select(
            Tab::all()
                .iter()
                .position(|&t| t == app.current_tab)
                .unwrap(),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    f.render_widget(tabs, area);
}

fn render_tab_description(f: &mut Frame, app: &mut App, area: Rect) {
    let description = match app.current_tab {
        Tab::Locker => "Find and kill processes holding file locks",
        Tab::Controller => "Start, stop, and manage Windows services",
        Tab::Nexus => "Monitor active network connections",
    };

    let desc_line = Line::from(vec![
        Span::styled("  → ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            description,
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
        ),
    ]);

    let paragraph = Paragraph::new(desc_line);
    f.render_widget(paragraph, area);
}

fn render_keybindings_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let header_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default().fg(Color::Cyan);
    let action_style = Style::default().fg(Color::White);
    let _muted_style = Style::default().fg(Color::Gray);

    let mut lines = vec![
        Line::from(Span::styled("Keys", header_style)),
        Line::from(""),
        Line::from(Span::styled("Navigation", header_style)),
        Line::from(vec![
            Span::styled("j/k", key_style),
            Span::styled("  Move", action_style),
        ]),
        Line::from(vec![
            Span::styled("↑/↓", key_style),
            Span::styled("  Move", action_style),
        ]),
        Line::from(vec![
            Span::styled("C-d/u", key_style),
            Span::styled(" Page", action_style),
        ]),
        Line::from(vec![
            Span::styled("Tab", key_style),
            Span::styled("  Switch", action_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("Actions", header_style)),
        Line::from(vec![
            Span::styled("/", key_style),
            Span::styled("     Search", action_style),
        ]),
        Line::from(vec![
            Span::styled("s/S", key_style),
            Span::styled("   Sort", action_style),
        ]),
        Line::from(vec![
            Span::styled("f", key_style),
            Span::styled("     FindLocks", action_style),
        ]),
    ];

    // Tab-specific keybindings
    match app.current_tab {
        Tab::Locker => {
            lines.push(Line::from(vec![
                Span::styled("K", key_style),
                Span::styled("     Kill", action_style),
            ]));
        }
        Tab::Controller => {
            lines.push(Line::from(vec![
                Span::styled("Enter", key_style),
                Span::styled(" Toggle", action_style),
            ]));
        }
        Tab::Nexus => {
            // Nexus has fewer specific actions
        }
    }

    // Common keybindings
    lines.extend(vec![
        Line::from(vec![
            Span::styled("r", key_style),
            Span::styled("     Refresh", action_style),
        ]),
        Line::from(vec![
            Span::styled("Esc", key_style),
            Span::styled("   ClearFilt", action_style),
        ]),
        Line::from(""),
        Line::from(Span::styled("System", header_style)),
    ]);

    // Show filter status
    if app.has_active_filter() {
        lines.push(Line::from(vec![Span::styled(
            "FILTER",
            Style::default().fg(Color::Yellow),
        )]));
    }

    // Show elevation status
    if !app.is_elevated {
        lines.push(Line::from(vec![Span::styled(
            "[!] Admin",
            Style::default().fg(Color::Red),
        )]));
    }

    lines.extend(vec![
        Line::from(""),
        Line::from(Span::styled("Quit", header_style)),
        Line::from(vec![
            Span::styled("q", key_style),
            Span::styled("     Exit", action_style),
        ]),
    ]);

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Shortcuts ")
            .title_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(paragraph, area);
}

fn render_tab_content(f: &mut Frame, app: &mut App, area: Rect) {
    match app.current_tab {
        Tab::Locker => locker::render(f, &mut app.state.locker, &app.search_query, area),
        Tab::Controller => {
            controller::render(f, &mut app.state.controller, &app.search_query, area)
        }
        Tab::Nexus => nexus::render(f, &mut app.state.nexus, &app.search_query, area),
    }
}

fn render_status_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let mut spans = vec![];

    // Show sort indicator
    let sort_info = match app.current_tab {
        Tab::Locker => format!(
            "Sort: {} {}",
            app.state.locker.sort_key.as_str(),
            app.state.locker.sort_order.as_str()
        ),
        Tab::Controller => format!(
            "Sort: {} {}",
            app.state.controller.sort_key.as_str(),
            app.state.controller.sort_order.as_str()
        ),
        Tab::Nexus => format!(
            "Sort: {} {}",
            app.state.nexus.sort_key.as_str(),
            app.state.nexus.sort_order.as_str()
        ),
    };
    spans.push(Span::styled(sort_info, Style::default().fg(Color::Cyan)));

    // Show filter status if active
    if app.has_active_filter() {
        spans.push(Span::styled(
            "  [FILTER ACTIVE]",
            Style::default().fg(Color::Yellow),
        ));
    }

    // Show status message if present
    if let Some(msg) = &app.status_message {
        spans.push(Span::styled("  ", Style::default()));
        spans.push(Span::styled(msg, Style::default().fg(Color::Yellow)));
    }

    // Show elevation warning
    if !app.is_elevated {
        spans.push(Span::styled(
            "  [!] No admin",
            Style::default().fg(Color::Red),
        ));
    }

    let status = Paragraph::new(Line::from(spans));
    f.render_widget(status, area);
}

fn render_search_box(f: &mut Frame, app: &mut App, area: Rect) {
    let search_area = Rect::new(area.x, area.bottom().saturating_sub(3), area.width, 3);
    let search = Paragraph::new(format!("Search: {}", app.search_query))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" / ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(search, search_area);
}

fn render_modal(f: &mut Frame, app: &mut App) {
    match &app.modal {
        Some(Modal::KillConfirmation { pid, name }) => {
            render_kill_confirmation(f, *pid, name);
        }
        Some(Modal::HandleSearch {
            input,
            results,
            selected,
            loading,
            error,
        }) => {
            render_handle_search_modal(
                f,
                input,
                results,
                *selected,
                *loading,
                error,
                app.is_elevated,
            );
        }
        _ => {}
    }
}

fn render_kill_confirmation(f: &mut Frame, pid: u32, name: &str) {
    let area = centered_rect(50, 9, f.area());

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Confirm Kill Process",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("  Kill \"{}\" (PID: {})?", name, pid)),
        Line::from("  This action cannot be undone."),
        Line::from(""),
        Line::from(vec![
            Span::styled("       [Y] Yes  ", Style::default().fg(Color::Green)),
            Span::styled("[N] No", Style::default().fg(Color::Red)),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirmation ")
                .title_style(Style::default().fg(Color::Red)),
        )
        .alignment(Alignment::Center);

    f.render_widget(Clear::default(), area);
    f.render_widget(paragraph, area);
}

fn render_handle_search_modal(
    f: &mut Frame,
    input: &str,
    results: &[crate::app::LockingProcess],
    selected: usize,
    loading: bool,
    error: &Option<String>,
    is_elevated: bool,
) {
    let area = centered_rect(70, 20, f.area());

    let mut lines = vec![
        Line::from(Span::styled(
            "Find Locking Processes",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Paths: {}", input.replace('\n', "; ")),
            Style::default().fg(Color::Gray),
        )),
        Line::from(""),
    ];

    if loading {
        lines.push(Line::from(Span::styled(
            "  Searching...",
            Style::default().fg(Color::Yellow),
        )));
    } else if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    } else if results.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No locking processes found.",
            Style::default().fg(Color::Green),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  Locking processes:",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(""));
        for (i, proc) in results.iter().enumerate() {
            let style = if i == selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(
                format!("    PID: {:6}  {}", proc.pid, proc.name),
                style,
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("[Enter] Search  ", Style::default().fg(Color::Gray)),
        Span::styled("[j/k] Navigate  ", Style::default().fg(Color::Gray)),
        if is_elevated {
            Span::styled("[K] Kill  ", Style::default().fg(Color::Red))
        } else {
            Span::styled("[K] Kill (admin)  ", Style::default().fg(Color::DarkGray))
        },
        Span::styled("[Esc] Close", Style::default().fg(Color::Gray)),
    ]));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Handle Search ")
            .title_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(Clear::default(), area);
    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
