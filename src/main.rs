mod app;
mod export;
mod state;
mod sys;
mod ui;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use app::{App, AppEvent};

const TICK_RATE_MS: u64 = 100;
const DATA_POLL_INTERVAL_MS: u64 = 2000;
const SERVICE_POLL_INTERVAL_MS: u64 = 500; // Faster polling for services
const METRICS_INTERVAL_MS: u64 = 1000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::channel::<AppEvent>(32);

    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(TICK_RATE_MS));
        loop {
            interval.tick().await;
            if tick_tx.send(AppEvent::Tick).await.is_err() {
                break;
            }
        }
    });

    let poll_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(DATA_POLL_INTERVAL_MS));
        loop {
            interval.tick().await;
            if poll_tx.send(AppEvent::PollData).await.is_err() {
                break;
            }
        }
    });

    // Separate service polling for near real-time updates
    let service_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(SERVICE_POLL_INTERVAL_MS));
        loop {
            interval.tick().await;
            // Only poll services if we're on the Controller tab to save resources
            if service_tx.send(AppEvent::PollServices).await.is_err() {
                break;
            }
        }
    });

    let metrics_tx = tx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(METRICS_INTERVAL_MS));
        loop {
            interval.tick().await;
            if metrics_tx.send(AppEvent::MetricsTick).await.is_err() {
                break;
            }
        }
    });

    let mut app = App::new();
    app.check_elevation();

    // Load all data at startup so all tabs have data immediately
    app.refresh_all_tabs();

    let res = run_app(&mut terminal, &mut app, &mut rx).await;

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {err}");
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    rx: &mut mpsc::Receiver<AppEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        tokio::select! {
            Some(event) = rx.recv() => {
                match event {
                    AppEvent::Tick => {}
                    AppEvent::PollData => {
                        // Refresh all tabs so data is always current when switching
                        app.refresh_all_tabs();
                    }
                    AppEvent::PollServices => {
                        // Fast polling for services - only update if on Controller tab
                        if app.current_tab == app::Tab::Controller {
                            if let Ok(services) = sys::service::enumerate_services() {
                                app.state.controller.update_services(services);
                            }
                        }
                    }
                    AppEvent::MetricsTick => {
                        app.update_metrics();
                    }
                    AppEvent::ServiceUpdate(services) => {
                        app.state.controller.update_services(services);
                    }
                    AppEvent::ProcessUpdate(processes) => {
                        app.state.locker.update_processes(processes);
                    }
                    AppEvent::NetworkUpdate(connections) => {
                        app.state.nexus.update_connections(connections);
                    }
                }
            }
            _ = async {
                event::poll(Duration::from_millis(TICK_RATE_MS)).ok();
            } => {
                if event::poll(Duration::from_millis(0))?
                    && let Event::Key(key) = event::read()?
                        && key.kind == KeyEventKind::Press
                            && handle_key_event(app, key)? {
                                return Ok(());
                            }
            }
        }
    }
}

fn handle_key_event(app: &mut App, key: event::KeyEvent) -> Result<bool, Box<dyn std::error::Error>> {
    let code = key.code;
    let modifiers = key.modifiers;

    if let Some(modal) = &app.modal {
        match modal {
            app::Modal::KillConfirmation { .. } => {
                match code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        app.confirm_kill();
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                        app.cancel_modal();
                    }
                    _ => {}
                }
            }
            app::Modal::HandleSearch { .. } => {
                if app.handle_search_input_mode {
                    match code {
                        KeyCode::Esc => {
                            app.exit_handle_search_input_mode();
                        }
                        KeyCode::Enter => {
                            app.exit_handle_search_input_mode();
                            app.execute_handle_search();
                        }
                        KeyCode::Char(c) => {
                            app.handle_search_modal_char(c);
                        }
                        KeyCode::Backspace => {
                            app.handle_search_modal_backspace();
                        }
                        _ => {}
                    }
                } else {
                    match code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.pending_gg = false;
                            app.cancel_modal();
                        }
                        KeyCode::Char('/') => {
                            app.pending_gg = false;
                            app.enter_handle_search_input_mode();
                        }
                        KeyCode::Enter => {
                            app.pending_gg = false;
                            app.execute_handle_search();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.pending_gg = false;
                            app.handle_search_modal_select_next();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.pending_gg = false;
                            app.handle_search_modal_select_prev();
                        }
                        KeyCode::Char('g') => {
                            if app.pending_gg {
                                // Second 'g' - jump to first
                                app.handle_search_modal_select_first();
                                app.pending_gg = false;
                            } else {
                                // First 'g' - set flag
                                app.pending_gg = true;
                            }
                        }
                        KeyCode::Char('G') => {
                            app.pending_gg = false;
                            app.handle_search_modal_select_last();
                        }
                        KeyCode::Char('K') => {
                            app.pending_gg = false;
                            if app.is_elevated {
                                app.kill_selected_locking_process();
                            }
                        }
                        KeyCode::Backspace => {
                            app.pending_gg = false;
                            app.handle_search_modal_backspace();
                        }
                        _ => {
                            app.pending_gg = false;
                        }
                    }
                }
            }
            app::Modal::ProcessDetails(details) => {
                match code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.cancel_modal();
                    }
                    KeyCode::Char('K') => {
                        if app.is_elevated {
                            app.modal = Some(app::Modal::KillConfirmation {
                                pid: details.pid,
                                name: details.name.clone(),
                            });
                        }
                    }
                    _ => {}
                }
            }
            app::Modal::ExportFormat => {
                match code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.cancel_modal();
                    }
                    KeyCode::Char('j') => {
                        app.export_to_json();
                        app.cancel_modal();
                    }
                    KeyCode::Char('c') => {
                        app.export_to_csv();
                        app.cancel_modal();
                    }
                    _ => {}
                }
            }
        }
        return Ok(false);
    }

    if app.search_mode {
        match code {
            KeyCode::Esc => {
                app.exit_search_mode();
            }
            KeyCode::Char(c) => {
                app.handle_search_char(c);
            }
            KeyCode::Backspace => {
                app.handle_search_backspace();
            }
            KeyCode::Enter => {
                // Exit search mode and persist the filter
                app.exit_search_mode();
            }
            _ => {}
        }
        return Ok(false);
    }

    // Handle Ctrl+D and Ctrl+U for page navigation
    if modifiers.contains(KeyModifiers::CONTROL) {
        match code {
            KeyCode::Char('d') => {
                app.select_page_down();
                return Ok(false);
            }
            KeyCode::Char('u') => {
                app.select_page_up();
                return Ok(false);
            }
            _ => {}
        }
    }

    match code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Tab => app.next_tab(),
        KeyCode::BackTab => app.prev_tab(),
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
        KeyCode::Enter => app.on_enter(),
        KeyCode::Char('r') => app.refresh_current_tab(),
        KeyCode::Char('/') => app.enter_search_mode(),
        KeyCode::Char('f') => {
            app.open_handle_search();
        }
        KeyCode::Char('d') => {
            if app.current_tab == app::Tab::Locker {
                app.show_process_details();
            }
        }
        KeyCode::Char('e') => {
            app.open_export_modal();
        }
        KeyCode::Char('K') => {
            if app.current_tab == app::Tab::Locker && app.is_elevated {
                app.show_kill_confirmation();
            }
        }
        KeyCode::Char('s') => {
            // Check if Shift is held (uppercase S)
            if modifiers.contains(KeyModifiers::SHIFT) {
                app.toggle_sort_order();
            } else {
                app.cycle_sort_key();
            }
        }
        KeyCode::Char('S') => {
            // Shift+S - toggle sort order
            app.toggle_sort_order();
        }
        KeyCode::Char('t') => {
            if app.current_tab == app::Tab::Locker {
                app.toggle_tree_mode();
            }
        }
        KeyCode::Char(' ') => {
            if app.current_tab == app::Tab::Locker && app.state.locker.tree_mode {
                app.toggle_expand();
            }
        }
        KeyCode::Char('g') => {
            if app.pending_gg {
                // Second 'g' - jump to first
                app.select_first();
                app.pending_gg = false;
            } else {
                // First 'g' - set flag
                app.pending_gg = true;
            }
        }
        KeyCode::Char('G') => {
            app.pending_gg = false;
            app.select_last();
        }
        KeyCode::Esc => {
            app.pending_gg = false;
            if app.has_active_filter() {
                app.clear_current_filter();
            }
        }
        _ => {
            app.pending_gg = false;
        }
    }

    Ok(false)
}
