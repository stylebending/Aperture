use crate::state;
use crate::sys;

pub use crate::sys::handle::LockingProcess;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppEvent {
    Tick,
    PollData,
    MetricsTick,
    ServiceUpdate(Vec<sys::service::ServiceInfo>),
    ProcessUpdate(Vec<sys::process::ProcessInfo>),
    NetworkUpdate(Vec<sys::network::ConnectionInfo>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Locker,
    Controller,
    Nexus,
}

impl Tab {
    pub fn as_str(&self) -> &'static str {
        match self {
            Tab::Locker => "Locker",
            Tab::Controller => "Controller",
            Tab::Nexus => "Nexus",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[Tab::Locker, Tab::Controller, Tab::Nexus]
    }
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub enum Modal {
    KillConfirmation {
        pid: u32,
        name: String,
    },
    HandleSearch {
        input: String,
        results: Vec<LockingProcess>,
        selected: usize,
        loading: bool,
        error: Option<String>,
        is_directory: bool,
        files_scanned: Option<usize>,
    },
}

pub struct AppState {
    pub locker: state::locker::LockerState,
    pub controller: state::controller::ControllerState,
    pub nexus: state::nexus::NexusState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            locker: state::locker::LockerState::new(),
            controller: state::controller::ControllerState::new(),
            nexus: state::nexus::NexusState::new(),
        }
    }
}

pub struct App {
    pub current_tab: Tab,
    pub state: AppState,
    pub is_elevated: bool,
    pub search_mode: bool,
    pub search_query: String,
    pub status_message: Option<String>,
    pub modal: Option<Modal>,
    pub handle_search_input_mode: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            current_tab: Tab::Locker,
            state: AppState::new(),
            is_elevated: false,
            search_mode: false,
            search_query: String::new(),
            status_message: None,
            modal: None,
            handle_search_input_mode: false,
        }
    }

    pub fn check_elevation(&mut self) {
        self.is_elevated = sys::process::is_elevated();
        if !self.is_elevated {
            self.status_message =
                Some("Running without admin - some actions unavailable".to_string());
        }
    }

    pub fn next_tab(&mut self) {
        let tabs = Tab::all();
        let idx = tabs.iter().position(|&t| t == self.current_tab).unwrap();
        self.current_tab = tabs[(idx + 1) % tabs.len()];
    }

    pub fn prev_tab(&mut self) {
        let tabs = Tab::all();
        let idx = tabs.iter().position(|&t| t == self.current_tab).unwrap();
        self.current_tab = tabs[(idx + tabs.len() - 1) % tabs.len()];
    }

    pub fn select_next(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_next(&self.search_query),
            Tab::Controller => self.state.controller.select_next(&self.search_query),
            Tab::Nexus => self.state.nexus.select_next(&self.search_query),
        }
    }

    pub fn select_prev(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_prev(&self.search_query),
            Tab::Controller => self.state.controller.select_prev(&self.search_query),
            Tab::Nexus => self.state.nexus.select_prev(&self.search_query),
        }
    }

    pub fn select_page_up(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_page_up(&self.search_query),
            Tab::Controller => self.state.controller.select_page_up(&self.search_query),
            Tab::Nexus => self.state.nexus.select_page_up(&self.search_query),
        }
    }

    pub fn select_page_down(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_page_down(&self.search_query),
            Tab::Controller => self.state.controller.select_page_down(&self.search_query),
            Tab::Nexus => self.state.nexus.select_page_down(&self.search_query),
        }
    }

    pub fn on_enter(&mut self) {
        match self.current_tab {
            Tab::Controller => {
                if self.is_elevated {
                    self.state
                        .controller
                        .toggle_selected_service(&self.search_query);
                }
            }
            _ => {}
        }
    }

    pub fn enter_search_mode(&mut self) {
        self.search_mode = true;
        self.search_query.clear();
    }

    pub fn exit_search_mode(&mut self) {
        // Store the search query as the active filter before exiting
        let query = self.search_query.clone();
        match self.current_tab {
            Tab::Locker => self.state.locker.set_filter(query),
            Tab::Controller => self.state.controller.set_filter(query),
            Tab::Nexus => self.state.nexus.set_filter(query),
        }
        self.search_mode = false;
        self.search_query.clear();
    }

    pub fn clear_current_filter(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.clear_filter(),
            Tab::Controller => self.state.controller.clear_filter(),
            Tab::Nexus => self.state.nexus.clear_filter(),
        }
    }

    pub fn has_active_filter(&self) -> bool {
        match self.current_tab {
            Tab::Locker => self.state.locker.active_filter.is_some(),
            Tab::Controller => self.state.controller.active_filter.is_some(),
            Tab::Nexus => self.state.nexus.active_filter.is_some(),
        }
    }

    pub fn handle_search_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    pub fn handle_search_backspace(&mut self) {
        self.search_query.pop();
    }

    pub fn show_kill_confirmation(&mut self) {
        if self.current_tab == Tab::Locker {
            if let Some(process) = self.state.locker.get_selected_process(&self.search_query) {
                self.modal = Some(Modal::KillConfirmation {
                    pid: process.pid,
                    name: process.name.clone(),
                });
            }
        }
    }

    pub fn confirm_kill(&mut self) {
        if let Some(Modal::KillConfirmation { pid, .. }) = &self.modal {
            let pid = *pid;
            if let Err(e) = sys::process::kill_process(pid) {
                self.status_message = Some(format!("Failed to kill process: {}", e));
            } else {
                self.status_message = Some(format!("Process {} killed", pid));
                self.refresh_current_tab();
            }
        }
        self.modal = None;
    }

    pub fn cancel_modal(&mut self) {
        self.modal = None;
    }

    pub fn open_handle_search(&mut self) {
        self.modal = Some(Modal::HandleSearch {
            input: String::new(),
            results: Vec::new(),
            selected: 0,
            loading: false,
            error: None,
            is_directory: false,
            files_scanned: None,
        });
        self.handle_search_input_mode = false;
    }

    pub fn enter_handle_search_input_mode(&mut self) {
        self.handle_search_input_mode = true;
    }

    pub fn exit_handle_search_input_mode(&mut self) {
        self.handle_search_input_mode = false;
    }

    pub fn handle_search_modal_char(&mut self, c: char) {
        if let Some(Modal::HandleSearch { input, .. }) = &mut self.modal {
            input.push(c);
        }
    }

    pub fn handle_search_modal_backspace(&mut self) {
        if let Some(Modal::HandleSearch { input, .. }) = &mut self.modal {
            input.pop();
        }
    }

    pub fn execute_handle_search(&mut self) {
        let file_paths: Vec<String> = match &self.modal {
            Some(Modal::HandleSearch { input, .. }) => input
                .lines()
                .filter(|l| !l.is_empty())
                .map(|s| s.to_string())
                .collect(),
            _ => return,
        };

        if file_paths.is_empty() {
            if let Some(Modal::HandleSearch { error, .. }) = &mut self.modal {
                *error = Some("Enter file path(s)".to_string());
            }
            return;
        }

        let input_str = file_paths.join("\n");
        let first_path = file_paths.first().map(|p| p.as_str()).unwrap_or("");
        let path = std::path::Path::new(first_path);

        let is_directory = path.is_dir();

        self.modal = Some(Modal::HandleSearch {
            input: input_str.clone(),
            results: Vec::new(),
            selected: 0,
            loading: true,
            error: None,
            is_directory,
            files_scanned: None,
        });

        if is_directory {
            let result = sys::handle::find_locking_processes_in_directory(first_path);
            self.modal = Some(match result {
                Ok((locking_procs, scanned_count)) => Modal::HandleSearch {
                    input: input_str,
                    results: locking_procs,
                    selected: 0,
                    loading: false,
                    error: None,
                    is_directory,
                    files_scanned: Some(scanned_count),
                },
                Err(e) => Modal::HandleSearch {
                    input: input_str,
                    results: Vec::new(),
                    selected: 0,
                    loading: false,
                    error: Some(e.to_string()),
                    is_directory: false,
                    files_scanned: None,
                },
            });
        } else {
            let file_refs: Vec<&str> = file_paths.iter().map(|s| s.as_str()).collect();
            let result = sys::handle::find_locking_processes(&file_refs);
            self.modal = Some(match result {
                Ok(locking_procs) => Modal::HandleSearch {
                    input: input_str,
                    results: locking_procs,
                    selected: 0,
                    loading: false,
                    error: None,
                    is_directory,
                    files_scanned: None,
                },
                Err(e) => Modal::HandleSearch {
                    input: input_str,
                    results: Vec::new(),
                    selected: 0,
                    loading: false,
                    error: Some(e.to_string()),
                    is_directory: false,
                    files_scanned: None,
                },
            });
        }
    }

    pub fn handle_search_modal_select_next(&mut self) {
        if let Some(Modal::HandleSearch {
            results, selected, ..
        }) = &mut self.modal
        {
            if !results.is_empty() {
                *selected = (*selected + 1) % results.len();
            }
        }
    }

    pub fn handle_search_modal_select_prev(&mut self) {
        if let Some(Modal::HandleSearch {
            results, selected, ..
        }) = &mut self.modal
        {
            if !results.is_empty() {
                *selected = (*selected + results.len() - 1) % results.len();
            }
        }
    }

    pub fn kill_selected_locking_process(&mut self) {
        if let Some(Modal::HandleSearch {
            results, selected, ..
        }) = &self.modal
        {
            if let Some(proc) = results.get(*selected) {
                let pid = proc.pid;
                let name = proc.name.clone();
                self.modal = Some(Modal::KillConfirmation { pid, name });
            }
        }
    }

    pub fn refresh_current_tab(&mut self) {
        match self.current_tab {
            Tab::Locker => {
                if let Ok(processes) = sys::process::enumerate_processes() {
                    self.state.locker.update_processes(processes);
                }
            }
            Tab::Controller => {
                if let Ok(services) = sys::service::enumerate_services() {
                    self.state.controller.update_services(services);
                }
            }
            Tab::Nexus => {
                if let Ok(connections) = sys::network::enumerate_connections() {
                    self.state.nexus.update_connections(connections);
                }
            }
        }
    }

    pub fn refresh_all_tabs(&mut self) {
        // Load data for all tabs so switching is instant
        if let Ok(processes) = sys::process::enumerate_processes() {
            self.state.locker.update_processes(processes);
        }
        if let Ok(services) = sys::service::enumerate_services() {
            self.state.controller.update_services(services);
        }
        if let Ok(connections) = sys::network::enumerate_connections() {
            self.state.nexus.update_connections(connections);
        }
    }

    pub fn update_metrics(&mut self) {
        // Update metrics for all processes, not just current tab
        let _ = sys::process::update_process_metrics(&mut self.state.locker.processes);
    }

    pub fn cycle_sort_key(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.cycle_sort_key(),
            Tab::Controller => self.state.controller.cycle_sort_key(),
            Tab::Nexus => self.state.nexus.cycle_sort_key(),
        }
    }

    pub fn toggle_sort_order(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.toggle_sort_order(),
            Tab::Controller => self.state.controller.toggle_sort_order(),
            Tab::Nexus => self.state.nexus.toggle_sort_order(),
        }
    }
}
