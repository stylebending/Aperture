use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem},
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
        })
        .collect();

    // Build title with filter and sort info
    let total = state.processes.len();
    let showing = filtered.len();
    let sort_info = format!("{} {}", state.sort_key.as_str(), state.sort_order.as_str());
    let title = if showing != total {
        format!(
            " Processes (Locker) [{}/{} | {}] ",
            showing, total, sort_info
        )
    } else {
        format!(" Processes (Locker) [{}] ", sort_info)
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    // Pass mutable reference directly (not cloned) so selection is preserved
    f.render_stateful_widget(list, area, &mut state.list_state);
}
