use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

use crate::state::locker::LockerState;

pub fn render(f: &mut Frame, state: &mut LockerState, search_query: &str, area: Rect) {
    let filtered = state.filtered_processes(search_query);

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|(_, p)| {
            // Use cached values if current is 0, for stable display
            let cpu_val = if p.cpu_usage > 0.0 {
                p.cpu_usage
            } else {
                p.last_cpu_usage
            };
            let mem_val = if p.memory_mb > 0.0 {
                p.memory_mb
            } else {
                p.last_memory_mb
            };

            let cpu_str = if cpu_val > 0.0 {
                format!("{:5.1}%", cpu_val)
            } else {
                "     -".to_string()
            };
            let mem_str = if mem_val > 0.0 {
                format!("{:5.1}MB", mem_val)
            } else {
                "     -".to_string()
            };
            ListItem::new(format!(
                "{:6} {:20} {} {} {}",
                p.pid,
                if p.name.len() > 20 {
                    &p.name[..20]
                } else {
                    &p.name
                },
                cpu_str,
                mem_str,
                p.path.as_deref().unwrap_or("-")
            ))
            .style(Style::default().fg(Color::White))
        })
        .collect();

    // Build title with filter and sort info
    let total = state.processes.len();
    let showing = filtered.len();
    let sort_info = format!("{} {}", state.sort_key.as_str(), state.sort_order.as_str());
    let title = format!(
        " Processes (Locker) [{}/{} | {}] ",
        showing, total, sort_info
    );

    // Create inner area inside the border for the header
    let inner_area = area.inner(Margin::new(1, 1));

    // Split inner area into header (1 line) and list (remaining space)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner_area);

    // Render header as non-selectable text in the first line of inner area
    let header_text = format!(
        "{:6} {:20} {:>6} {:>6} {}",
        "PID", "Name", "CPU%", "Mem", "Path"
    );
    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )]));
    f.render_widget(header, chunks[0]);

    // Render list block with border (full area)
    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan));
    f.render_widget(list_block.clone(), area);

    // Render list items in the remaining space (below header, inside border)
    let list = List::new(items).highlight_style(Style::default().bg(Color::DarkGray));

    // Pass mutable reference directly (not cloned) so selection is preserved
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);
}
