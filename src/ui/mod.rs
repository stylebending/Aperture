mod controller;
mod env;
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
        Tab::Env => "View user, system, and process environment variables",
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
            Span::styled("gg/G", key_style),
            Span::styled(" First/Last", action_style),
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
                Span::styled("t", key_style),
                Span::styled("     TreeView", action_style),
            ]));
            lines.push(Line::from(vec![
                Span::styled("SPC", key_style),
                Span::styled("   Expand", action_style),
            ]));
            lines.push(Line::from(vec![
                Span::styled("d", key_style),
                Span::styled("     Details", action_style),
            ]));
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
        Tab::Env => {
            lines.push(Line::from(vec![
                Span::styled("a", key_style),
                Span::styled("     Add", action_style),
            ]));
            lines.push(Line::from(vec![
                Span::styled("E", key_style),
                Span::styled("     Edit", action_style),
            ]));
            lines.push(Line::from(vec![
                Span::styled("D", key_style),
                Span::styled("     Delete", action_style),
            ]));
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
        Line::from(vec![
            Span::styled("e", key_style),
            Span::styled("     Export", action_style),
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
        Tab::Env => env::render(f, &mut app.state.env, &app.search_query, area),
    }
}

fn render_status_bar(f: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Max(42)])
        .split(area);

    // Left-aligned: sort, filter, messages, elevation
    let mut left_spans = vec![];

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
        Tab::Env => format!(
            "Sort: {} {}",
            app.state.env.sort_key.as_str(),
            app.state.env.sort_order.as_str()
        ),
    };
    left_spans.push(Span::styled(sort_info, Style::default().fg(Color::Cyan)));

    if app.has_active_filter() {
        left_spans.push(Span::styled(
            "  [FILTER ACTIVE]",
            Style::default().fg(Color::Yellow),
        ));
    }

    if let Some(msg) = &app.status_message {
        let color = if app.status_is_error { Color::Red } else { Color::Green };
        left_spans.push(Span::styled("  ", Style::default()));
        left_spans.push(Span::styled(msg, Style::default().fg(color)));
    }

    if !app.is_elevated {
        left_spans.push(Span::styled(
            "  [!] No admin",
            Style::default().fg(Color::Red),
        ));
    }

    f.render_widget(
        Paragraph::new(Line::from(left_spans)),
        chunks[0],
    );

    // Right-aligned: total system metrics
    let cpu_style = if app.total_cpu > 90.0 {
        Style::default().fg(Color::Red)
    } else if app.total_cpu > 70.0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let used_gb = app.total_memory_mb / 1024.0;
    let total_gb = app.total_system_memory_mb / 1024.0;
    let mem_pct = if app.total_system_memory_mb > 0.0 {
        (app.total_memory_mb / app.total_system_memory_mb) * 100.0
    } else {
        0.0
    };
    let mem_style = if mem_pct > 90.0 {
        Style::default().fg(Color::Red)
    } else if mem_pct > 70.0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let right_spans = vec![
        Span::styled("CPU", cpu_style),
        Span::styled(
            format!(":{:>6.1}%  ", app.total_cpu),
            Style::default().fg(Color::White),
        ),
        Span::styled("Mem", mem_style),
        Span::styled(
            format!(":{:>5.1}/{:>5.1}GB", used_gb, total_gb),
            Style::default().fg(Color::White),
        ),
    ];

    f.render_widget(
        Paragraph::new(Line::from(right_spans)),
        chunks[1],
    );
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
            is_directory,
            files_scanned,
        }) => {
            render_handle_search_modal(
                f,
                input,
                results,
                *selected,
                *loading,
                error,
                app.is_elevated,
                app.handle_search_input_mode,
                *is_directory,
                *files_scanned,
            );
        }
        Some(Modal::ProcessDetails(details)) => {
            render_process_details_modal(f, details, app.is_elevated);
        }
        Some(Modal::ExportFormat) => {
            render_export_format_modal(f);
        }
        Some(Modal::EnvVarEdit {
            name,
            value,
            scope,
            is_new,
            field,
            ..
        }) => {
            render_env_var_edit_modal(f, name, value, scope, *is_new, *field, app.is_elevated);
        }
        Some(Modal::EnvVarConfirmDelete { name, scope }) => {
            render_env_var_delete_modal(f, name, scope);
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

    f.render_widget(Clear, area);
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
    input_mode: bool,
    is_directory: bool,
    files_scanned: Option<usize>,
) {
    let area = centered_rect(70, 20, f.area());

    let input_display = if input.is_empty() {
        if input_mode {
            "_".to_string()
        } else {
            "(enter path)".to_string()
        }
    } else {
        input.to_string()
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "Find Locking Processes",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Path: {}", input_display.replace('\n', "; ")),
            Style::default().fg(if input_mode {
                Color::White
            } else {
                Color::Gray
            }),
        )),
        Line::from(""),
    ];

    if loading {
        let scan_msg = if is_directory {
            if let Some(count) = files_scanned {
                format!("  Scanning {} files...", count)
            } else {
                "  Scanning directory...".to_string()
            }
        } else {
            "  Searching...".to_string()
        };
        lines.push(Line::from(Span::styled(
            scan_msg,
            Style::default().fg(Color::Yellow),
        )));
    } else if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("  Error: {}", err),
            Style::default().fg(Color::Red),
        )));
    } else if results.is_empty() {
        let empty_msg = if is_directory {
            if let Some(count) = files_scanned {
                format!("  Scanned {} files - no locks found.", count)
            } else {
                "  No locking processes found.".to_string()
            }
        } else {
            "  No locking processes found.".to_string()
        };
        lines.push(Line::from(Span::styled(
            empty_msg,
            Style::default().fg(Color::Green),
        )));
    } else {
        let results_msg = if is_directory {
            if let Some(count) = files_scanned {
                format!("  Scanned {} files - Found {} locks:", count, results.len())
            } else {
                format!("  Found {} locks:", results.len())
            }
        } else {
            format!("  Locking processes ({}):", results.len())
        };
        lines.push(Line::from(Span::styled(
            results_msg,
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

    let hints = if input_mode {
        vec![
            Span::styled("[Enter] Search  ", Style::default().fg(Color::Gray)),
            Span::styled("[Esc] Cancel  ", Style::default().fg(Color::Gray)),
        ]
    } else {
        vec![
            Span::styled("[/] Edit Path  ", Style::default().fg(Color::Gray)),
            Span::styled("[Enter] Search  ", Style::default().fg(Color::Gray)),
            Span::styled("[j/k] Navigate  ", Style::default().fg(Color::Gray)),
            if is_elevated {
                Span::styled("[K] Kill  ", Style::default().fg(Color::Red))
            } else {
                Span::styled("[K] Kill (admin)  ", Style::default().fg(Color::DarkGray))
            },
            Span::styled("[Esc] Close", Style::default().fg(Color::Gray)),
        ]
    };
    lines.push(Line::from(hints));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Handle Search ")
            .title_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn render_process_details_modal(
    f: &mut Frame,
    details: &crate::app::ProcessDetails,
    is_elevated: bool,
) {
    let area = centered_rect(80, 25, f.area());
    let height = 25usize;

    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(Span::styled(
        "Process Details",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // Basic info
    lines.push(Line::from(vec![
        Span::styled("Name:     ", Style::default().fg(Color::Yellow)),
        Span::styled(&details.name, Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("PID:      ", Style::default().fg(Color::Yellow)),
        Span::styled(details.pid.to_string(), Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Parent:   ", Style::default().fg(Color::Yellow)),
        Span::styled(
            details.parent_pid.to_string(),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("CPU:      ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{:.1}%", details.cpu_usage),
            Style::default().fg(Color::White),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Memory:   ", Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{:.1} MB", details.memory_mb),
            Style::default().fg(Color::White),
        ),
    ]));

    if let Some(path) = &details.path {
        lines.push(Line::from(vec![
            Span::styled("Path:     ", Style::default().fg(Color::Yellow)),
            Span::styled(path, Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));

    // Fixed lines before module section
    let fixed_line_count = lines.len();

    // Reserved lines at the bottom: blank + error (if present) + help
    let has_error = details.error.is_some();
    let bottom_reserved = if has_error { 4 } else { 2 }; // blank, error, blank, help vs blank, help
    let max_module_lines = height.saturating_sub(fixed_line_count + bottom_reserved);

    if !details.modules.is_empty() {
        lines.push(Line::from(Span::styled(
            "Loaded Modules:",
            Style::default().fg(Color::Yellow),
        )));

        let total = details.modules.len();
        let selected = details.module_selected;
        let available = max_module_lines.saturating_sub(1); // -1 for the header

        // Calculate scroll offset to keep selected visible
        let scroll = if available == 0 || total <= available {
            0
        } else if selected < available / 2 {
            0
        } else if selected + available / 2 >= total {
            total.saturating_sub(available)
        } else {
            selected.saturating_sub(available / 2)
        };

        for (i, module) in details
            .modules
            .iter()
            .enumerate()
            .skip(scroll)
            .take(available)
        {
            let is_selected = i == selected;
            let prefix = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}", prefix, module),
                style,
            )));
        }

        // Show "more" indicator if there are items below the visible window
        if scroll + available < total {
            lines.push(Line::from(Span::styled(
                format!("  ↓ {} more", total - (scroll + available)),
                Style::default().fg(Color::DarkGray),
            )));
        }
    } else if has_error {
        lines.push(Line::from(Span::styled(
            "Modules: (access denied)",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Error section
    if has_error {
        lines.push(Line::from(""));
        if let Some(err) = &details.error {
            lines.push(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::styled(err, Style::default().fg(Color::Red)),
            ]));
        }
    }

    // Help text
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "[j/k] Navigate  ",
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            "[K] Kill  ",
            if is_elevated {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        Span::styled("[Esc] Close", Style::default().fg(Color::Gray)),
    ]));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} (PID: {}) ", details.name, details.pid))
            .title_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn render_export_format_modal(f: &mut Frame) {
    let area = centered_rect(50, 12, f.area());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Export Data",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[j]", Style::default().fg(Color::Green)),
            Span::styled(" Export to JSON", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("[c]", Style::default().fg(Color::Green)),
            Span::styled(" Export to CSV", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Esc]", Style::default().fg(Color::Gray)),
            Span::styled(" Cancel", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Export ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn render_env_var_edit_modal(
    f: &mut Frame,
    name: &str,
    value: &str,
    scope: &crate::app::EnvScopeEdit,
    is_new: bool,
    field: u8,
    is_elevated: bool,
) {
    let area = centered_rect(70, 17, f.area());

    let cursor_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(Color::Yellow);
    let value_style = Style::default().fg(Color::White);
    let selected_style = Style::default()
        .bg(Color::DarkGray)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let dim_style = Style::default().fg(Color::DarkGray);

    let field_label = if field == 0 { "▸ " } else { "  " };
    let value_label = if field == 1 { "▸ " } else { "  " };
    let scope_label = if field == 2 { "▸ " } else { "  " };

    let scope_user_mark = if matches!(scope, crate::app::EnvScopeEdit::User) {
        "●"
    } else {
        "○"
    };
    let scope_system_mark = if matches!(scope, crate::app::EnvScopeEdit::System) {
        "●"
    } else {
        "○"
    };

    let title_text = if is_new { "Add" } else { "Edit" };

    let system_warning = !is_elevated && matches!(scope, crate::app::EnvScopeEdit::System);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Environment Variable", title_text),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(field_label, if field == 0 { cursor_style } else { dim_style }),
            Span::styled("Name:  ", label_style),
            Span::styled(
                if name.is_empty() { "(empty)" } else { name },
                if field == 0 { selected_style } else { value_style },
            ),
        ]),
        Line::from(vec![
            Span::styled(value_label, if field == 1 { cursor_style } else { dim_style }),
            Span::styled("Value: ", label_style),
            Span::styled(
                if value.is_empty() { "(empty)" } else { value },
                if field == 1 { selected_style } else { value_style },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(scope_label, if field == 2 { cursor_style } else { dim_style }),
            Span::styled("Scope: ", label_style),
            Span::styled(scope_user_mark, if field == 2 { cursor_style } else { value_style }),
            Span::styled(" User  ", value_style),
            Span::styled(scope_system_mark, if field == 2 { cursor_style } else { value_style }),
            Span::styled(" System", value_style),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            if system_warning {
                "[!] System scope requires admin elevation"
            } else {
                "Note: System scope requires admin elevation"
            },
            if system_warning {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Tab] Next  ", Style::default().fg(Color::Gray)),
            Span::styled("[Space] Toggle Scope  ", Style::default().fg(Color::Gray)),
            Span::styled("[Enter] Save  ", Style::default().fg(Color::Green)),
            Span::styled("[Esc] Cancel", Style::default().fg(Color::Gray)),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} Environment Variable ", title_text))
            .title_style(Style::default().fg(Color::Cyan)),
    );

    f.render_widget(Clear, area);
    f.render_widget(paragraph, area);
}

fn render_env_var_delete_modal(
    f: &mut Frame,
    name: &str,
    scope: &crate::state::env::EnvScope,
) {
    let area = centered_rect(60, 11, f.area());

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Delete Environment Variable",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Delete \"", Style::default().fg(Color::White)),
            Span::styled(name, Style::default().fg(Color::Red)),
            Span::styled(
                format!("\" ({} scope)?", scope.as_str()),
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("       [Y] Yes  ", Style::default().fg(Color::Green)),
            Span::styled("[N] No", Style::default().fg(Color::Red)),
        ]),
        Line::from(""),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirmation ")
                .title_style(Style::default().fg(Color::Red)),
        )
        .alignment(Alignment::Center);

    f.render_widget(Clear, area);
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
