use crate::state;
use crate::sys;

pub use crate::sys::handle::LockingProcess;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AppEvent {
    Tick,
    PollData,
    PollServices,
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
    Env,
}

impl Tab {
    pub fn as_str(&self) -> &'static str {
        match self {
            Tab::Locker => "Locker",
            Tab::Controller => "Controller",
            Tab::Nexus => "Nexus",
            Tab::Env => "Env",
        }
    }

    pub fn all() -> &'static [Tab] {
        &[Tab::Locker, Tab::Controller, Tab::Nexus, Tab::Env]
    }
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct ProcessDetails {
    pub pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub command_line: Option<String>,
    pub environment: Vec<(String, String)>,
    pub modules: Vec<String>,
    pub parent_pid: u32,
    pub cpu_usage: f32,
    pub memory_mb: f64,
    pub error: Option<String>,
    pub module_selected: usize,
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
    ProcessDetails(ProcessDetails),
    ExportFormat,
    EnvVarEdit {
        name: String,
        original_name: String,
        value: String,
        scope: EnvScopeEdit,
        is_new: bool,
        field: u8,
    },
    EnvVarConfirmDelete {
        name: String,
        scope: state::env::EnvScope,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvScopeEdit {
    User,
    System,
}

impl EnvScopeEdit {
    pub fn toggle(self) -> Self {
        match self {
            EnvScopeEdit::User => EnvScopeEdit::System,
            EnvScopeEdit::System => EnvScopeEdit::User,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            EnvScopeEdit::User => "User",
            EnvScopeEdit::System => "System",
        }
    }
}

pub struct AppState {
    pub locker: state::locker::LockerState,
    pub controller: state::controller::ControllerState,
    pub nexus: state::nexus::NexusState,
    pub env: state::env::EnvState,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            locker: state::locker::LockerState::new(),
            controller: state::controller::ControllerState::new(),
            nexus: state::nexus::NexusState::new(),
            env: state::env::EnvState::new(),
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
    pub status_is_error: bool,
    pub modal: Option<Modal>,
    pub handle_search_input_mode: bool,
    pub pending_gg: bool,
    pub total_cpu: f32,
    pub total_memory_mb: f64,
    pub total_system_memory_mb: f64,
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
            status_is_error: false,
            modal: None,
            handle_search_input_mode: false,
            pending_gg: false,
            total_cpu: 0.0,
            total_memory_mb: 0.0,
            total_system_memory_mb: 0.0,
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
            Tab::Env => self.state.env.select_next(),
        }
    }

    pub fn select_prev(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_prev(&self.search_query),
            Tab::Controller => self.state.controller.select_prev(&self.search_query),
            Tab::Nexus => self.state.nexus.select_prev(&self.search_query),
            Tab::Env => self.state.env.select_prev(),
        }
    }

    pub fn select_page_up(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_page_up(&self.search_query),
            Tab::Controller => self.state.controller.select_page_up(&self.search_query),
            Tab::Nexus => self.state.nexus.select_page_up(&self.search_query),
            Tab::Env => self.state.env.select_page_up(),
        }
    }

    pub fn select_page_down(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_page_down(&self.search_query),
            Tab::Controller => self.state.controller.select_page_down(&self.search_query),
            Tab::Nexus => self.state.nexus.select_page_down(&self.search_query),
            Tab::Env => self.state.env.select_page_down(),
        }
    }

    pub fn select_first(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_first(&self.search_query),
            Tab::Controller => self.state.controller.select_first(&self.search_query),
            Tab::Nexus => self.state.nexus.select_first(&self.search_query),
            Tab::Env => self.state.env.select_first(),
        }
    }

    pub fn select_last(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.select_last(&self.search_query),
            Tab::Controller => self.state.controller.select_last(&self.search_query),
            Tab::Nexus => self.state.nexus.select_last(&self.search_query),
            Tab::Env => self.state.env.select_last(),
        }
    }

    pub fn on_enter(&mut self) {
        if self.current_tab == Tab::Controller
            && self.is_elevated {
                self.state
                    .controller
                    .toggle_selected_service(&self.search_query);
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
            Tab::Env => self.state.env.set_filter(query),
        }
        self.search_mode = false;
        self.search_query.clear();
    }

    pub fn clear_current_filter(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.clear_filter(),
            Tab::Controller => self.state.controller.clear_filter(),
            Tab::Nexus => self.state.nexus.clear_filter(),
            Tab::Env => self.state.env.clear_filter(),
        }
    }

    pub fn has_active_filter(&self) -> bool {
        match self.current_tab {
            Tab::Locker => self.state.locker.active_filter.is_some(),
            Tab::Controller => self.state.controller.active_filter.is_some(),
            Tab::Nexus => self.state.nexus.active_filter.is_some(),
            Tab::Env => self.state.env.active_filter.is_some(),
        }
    }

    pub fn handle_search_char(&mut self, c: char) {
        self.search_query.push(c);
    }

    pub fn handle_search_backspace(&mut self) {
        self.search_query.pop();
    }

    pub fn show_kill_confirmation(&mut self) {
        if self.current_tab == Tab::Locker
            && let Some(process) = self.state.locker.get_selected_process(&self.search_query) {
                self.modal = Some(Modal::KillConfirmation {
                    pid: process.pid,
                    name: process.name.clone(),
                });
            }
    }

    pub fn confirm_kill(&mut self) {
        if let Some(Modal::KillConfirmation { pid, .. }) = &self.modal {
            let pid = *pid;
            if let Err(e) = sys::process::kill_process(pid) {
                self.status_message = Some(format!("Failed to kill process: {}", e));
                self.status_is_error = true;
            } else {
                self.status_message = Some(format!("Process {} killed", pid));
                self.status_is_error = false;
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
            && !results.is_empty() {
                *selected = (*selected + 1) % results.len();
            }
    }

    pub fn handle_search_modal_select_prev(&mut self) {
        if let Some(Modal::HandleSearch {
            results, selected, ..
        }) = &mut self.modal
            && !results.is_empty() {
                *selected = (*selected + results.len() - 1) % results.len();
            }
    }

    pub fn handle_search_modal_select_first(&mut self) {
        if let Some(Modal::HandleSearch {
            results, selected, ..
        }) = &mut self.modal
            && !results.is_empty() {
                *selected = 0;
            }
    }

    pub fn handle_search_modal_select_last(&mut self) {
        if let Some(Modal::HandleSearch {
            results, selected, ..
        }) = &mut self.modal
            && !results.is_empty() {
                *selected = results.len() - 1;
            }
    }

    pub fn kill_selected_locking_process(&mut self) {
        if let Some(Modal::HandleSearch {
            results, selected, ..
        }) = &self.modal
            && let Some(proc) = results.get(*selected) {
                let pid = proc.pid;
                let name = proc.name.clone();
                self.modal = Some(Modal::KillConfirmation { pid, name });
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
            Tab::Env => {
                self.refresh_env();
            }
        }
    }

    pub fn refresh_env(&mut self) {
        let process_vars = sys::env::get_process_env_vars();
        let user_vars = sys::env::get_user_env_vars();
        let system_vars = sys::env::get_system_env_vars();
        self.state
            .env
            .update_env_vars(process_vars, user_vars, system_vars);
    }

    pub fn open_env_add(&mut self) {
        self.modal = Some(Modal::EnvVarEdit {
            name: String::new(),
            original_name: String::new(),
            value: String::new(),
            scope: EnvScopeEdit::User,
            is_new: true,
            field: 0,
        });
    }

    pub fn open_env_edit(&mut self) {
        let entry = match self.state.env.get_selected_entry() {
            Some(e) => e,
            None => return,
        };
        match entry.scope {
            state::env::EnvScope::Process => {
                self.status_message = Some("Session-only var — press `a` to add a persisted copy".to_string());
                self.status_is_error = false;
            }
            state::env::EnvScope::User => {
                self.modal = Some(Modal::EnvVarEdit {
                    name: entry.name.clone(),
                    original_name: entry.name.clone(),
                    value: entry.value.clone(),
                    scope: EnvScopeEdit::User,
                    is_new: false,
                    field: 0,
                });
            }
            state::env::EnvScope::System => {
                self.modal = Some(Modal::EnvVarEdit {
                    name: entry.name.clone(),
                    original_name: entry.name.clone(),
                    value: entry.value.clone(),
                    scope: EnvScopeEdit::System,
                    is_new: false,
                    field: 0,
                });
            }
        }
    }

    pub fn open_env_delete(&mut self) {
        let entry = match self.state.env.get_selected_entry() {
            Some(e) => e,
            None => return,
        };
        match entry.scope {
            state::env::EnvScope::Process => {
                self.status_message = Some("Session-only var cannot be deleted — restart without it".to_string());
                self.status_is_error = false;
            }
            state::env::EnvScope::User | state::env::EnvScope::System => {
                self.modal = Some(Modal::EnvVarConfirmDelete {
                    name: entry.name.clone(),
                    scope: entry.scope,
                });
            }
        }
    }

    pub fn confirm_env_delete(&mut self) {
        let (name, scope) = match &self.modal {
            Some(Modal::EnvVarConfirmDelete { name, scope }) => (name.clone(), *scope),
            _ => return,
        };
        self.modal = None;

        if scope == state::env::EnvScope::System && !self.is_elevated {
            self.status_message = Some("Cannot delete System env var without admin elevation".to_string());
            self.status_is_error = true;
            self.refresh_env();
            return;
        }

        let result = match scope {
            state::env::EnvScope::User => sys::env::delete_user_env_var(&name),
            state::env::EnvScope::System => sys::env::delete_system_env_var(&name),
            state::env::EnvScope::Process => return,
        };

        match result {
            Ok(()) => {
                let _ = sys::env::broadcast_env_change();
                // Purge from current process's env block to prevent ghost Process entries
                let _ = sys::env::delete_process_env_var(&name);
                self.status_message = Some(format!("Deleted \"{}\" ({})", name, scope.as_str()));
                self.status_is_error = false;
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to delete \"{}\": {}", name, e));
                self.status_is_error = true;
            }
        }
        self.refresh_env();
    }

    pub fn confirm_env_save(&mut self) {
        let (is_new, name, original_name, value, scope) = match &self.modal {
            Some(Modal::EnvVarEdit {
                name,
                original_name,
                value,
                scope,
                is_new,
                field: _,
            }) if !name.is_empty() => (*is_new, name.clone(), original_name.clone(), value.clone(), *scope),
            _ => return,
        };
        self.modal = None;

        if scope == EnvScopeEdit::System && !self.is_elevated {
            self.status_message = Some("Cannot save System env var without admin elevation".to_string());
            self.status_is_error = true;
            self.refresh_env();
            return;
        }

        let result = match scope {
            EnvScopeEdit::User => {
                if !is_new && name != original_name {
                    let _ = sys::env::delete_user_env_var(&original_name);
                }
                sys::env::set_user_env_var(&name, &value)
            }
            EnvScopeEdit::System => {
                if !is_new && name != original_name {
                    let _ = sys::env::delete_system_env_var(&original_name);
                }
                sys::env::set_system_env_var(&name, &value)
            }
        };

        match result {
            Ok(()) => {
                let _ = sys::env::broadcast_env_change();
                // If renamed, purge the old name from the current process's env block
                if !is_new && name != original_name {
                    let _ = sys::env::delete_process_env_var(&original_name);
                }
                let action = if is_new { "Added" } else { "Saved" };
                self.status_message = Some(format!("{} \"{}\" ({})", action, name, scope.as_str()));
                self.status_is_error = false;
            }
            Err(e) => {
                self.status_message = Some(format!("Failed to save \"{}\": {}", name, e));
                self.status_is_error = true;
            }
        }
        self.refresh_env();
    }

    pub fn env_edit_char(&mut self, c: char) {
        let modal = match &mut self.modal {
            Some(Modal::EnvVarEdit {
                name, value, field, ..
            }) => {
                match field {
                    0 => name.push(c),
                    1 => value.push(c),
                    _ => {}
                }
            }
            _ => return,
        };
        let _ = modal;
    }

    pub fn env_edit_backspace(&mut self) {
        let modal = match &mut self.modal {
            Some(Modal::EnvVarEdit {
                name, value, field, ..
            }) => {
                match field {
                    0 => { name.pop(); }
                    1 => { value.pop(); }
                    _ => {}
                }
            }
            _ => return,
        };
        let _ = modal;
    }

    pub fn env_edit_toggle_scope(&mut self) {
        let modal = match &mut self.modal {
            Some(Modal::EnvVarEdit { scope, field, .. }) if *field == 2 => {
                *scope = scope.toggle();
            }
            _ => return,
        };
        let _ = modal;
    }

    pub fn env_edit_next_field(&mut self) {
        let modal = match &mut self.modal {
            Some(Modal::EnvVarEdit { field, .. }) => {
                *field = (*field + 1) % 3;
            }
            _ => return,
        };
        let _ = modal;
    }

    pub fn env_edit_prev_field(&mut self) {
        let modal = match &mut self.modal {
            Some(Modal::EnvVarEdit { field, .. }) => {
                *field = (*field + 2) % 3;
            }
            _ => return,
        };
        let _ = modal;
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
        self.refresh_env();
    }

    pub fn update_metrics(&mut self) {
        // Update metrics for all processes, not just current tab
        let _ = sys::process::update_process_metrics(&mut self.state.locker.processes);
        // Re-sort if sorted by metrics that change dynamically
        if matches!(
            self.state.locker.sort_key,
            state::locker::SortKey::Memory | state::locker::SortKey::Cpu
        ) {
            self.state.locker.sort_processes();
        }

        // Query system CPU usage via GetSystemTimes (matches Task Manager)
        self.total_cpu = sys::process::get_total_cpu_usage();

        // Query system memory info for accurate total usage (matches Task Manager)
        let (total_mb, avail_mb) = sys::process::get_system_memory_info();
        self.total_system_memory_mb = total_mb;
        self.total_memory_mb = total_mb - avail_mb;
    }

    pub fn cycle_sort_key(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.cycle_sort_key(),
            Tab::Controller => self.state.controller.cycle_sort_key(),
            Tab::Nexus => self.state.nexus.cycle_sort_key(),
            Tab::Env => self.state.env.cycle_sort_key(),
        }
    }

    pub fn toggle_sort_order(&mut self) {
        match self.current_tab {
            Tab::Locker => self.state.locker.toggle_sort_order(),
            Tab::Controller => self.state.controller.toggle_sort_order(),
            Tab::Nexus => self.state.nexus.toggle_sort_order(),
            Tab::Env => self.state.env.toggle_sort_order(),
        }
    }

    pub fn toggle_tree_mode(&mut self) {
        if self.current_tab == Tab::Locker {
            self.state.locker.toggle_tree_mode();
        }
    }

    pub fn toggle_expand(&mut self) {
        if self.current_tab == Tab::Locker {
            self.state.locker.toggle_expand();
        }
    }

    pub fn show_process_details(&mut self) {
        if self.current_tab == Tab::Locker {
            if let Some(process) = self.state.locker.get_selected_process(&self.search_query) {
                let pid = process.pid;
                let name = process.name.clone();
                let path = process.path.clone();
                let parent_pid = process.parent_pid;
                let cpu_usage = if process.cpu_usage > 0.0 {
                    process.cpu_usage
                } else {
                    process.last_cpu_usage
                };
                let memory_mb = if process.memory_mb > 0.0 {
                    process.memory_mb
                } else {
                    process.last_memory_mb
                };
                
                // Get detailed info
                let (command_line, environment, modules, error) = 
                    sys::process::get_process_details(pid);
                
                self.modal = Some(Modal::ProcessDetails(ProcessDetails {
                    pid,
                    name,
                    path,
                    command_line,
                    environment,
                    modules,
                    parent_pid,
                    cpu_usage,
                    memory_mb,
                    error,
                    module_selected: 0,
                }));
            }
        }
    }

    pub fn select_next_module(&mut self) {
        if let Some(Modal::ProcessDetails(details)) = &mut self.modal {
            if !details.modules.is_empty() {
                details.module_selected =
                    (details.module_selected + 1) % details.modules.len();
            }
        }
    }

    pub fn select_prev_module(&mut self) {
        if let Some(Modal::ProcessDetails(details)) = &mut self.modal {
            if !details.modules.is_empty() {
                details.module_selected = (details.module_selected
                    + details.modules.len()
                    - 1)
                    % details.modules.len();
            }
        }
    }

    pub fn select_prev_page_modules(&mut self) {
        if let Some(Modal::ProcessDetails(details)) = &mut self.modal {
            if !details.modules.is_empty() {
                details.module_selected =
                    details.module_selected.saturating_sub(10);
            }
        }
    }

    pub fn select_next_page_modules(&mut self) {
        if let Some(Modal::ProcessDetails(details)) = &mut self.modal {
            if !details.modules.is_empty() {
                let last = details.modules.len() - 1;
                details.module_selected =
                    std::cmp::min(details.module_selected + 10, last);
            }
        }
    }

    pub fn select_first_module(&mut self) {
        if let Some(Modal::ProcessDetails(details)) = &mut self.modal {
            if !details.modules.is_empty() {
                details.module_selected = 0;
            }
        }
    }

    pub fn select_last_module(&mut self) {
        if let Some(Modal::ProcessDetails(details)) = &mut self.modal {
            if !details.modules.is_empty() {
                details.module_selected = details.modules.len() - 1;
            }
        }
    }

    pub fn export_to_json(&mut self) {
        match crate::export::export_to_json(
            &self.state.locker,
            &self.state.controller,
            &self.state.nexus,
        ) {
            Ok(path) => {
                self.status_message = Some(format!("Exported to {}", path));
                self.status_is_error = false;
            }
            Err(e) => {
                self.status_message = Some(format!("Export failed: {}", e));
                self.status_is_error = true;
            }
        }
    }

    pub fn export_to_csv(&mut self) {
        match crate::export::export_to_csv(
            &self.state.locker,
            &self.state.controller,
            &self.state.nexus,
        ) {
            Ok(path) => {
                self.status_message = Some(format!("Exported to {}", path));
                self.status_is_error = false;
            }
            Err(e) => {
                self.status_message = Some(format!("Export failed: {}", e));
                self.status_is_error = true;
            }
        }
    }

    pub fn open_export_modal(&mut self) {
        self.modal = Some(Modal::ExportFormat);
    }
}
