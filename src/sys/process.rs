use std::collections::HashMap;
use std::mem;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{
    EnumProcessModules, EnumProcesses, GetModuleBaseNameW, GetModuleFileNameExW,
    GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
    PROCESS_NAME_FORMAT, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: String,
    pub path: Option<String>,
    pub cpu_usage: f32,
    pub memory_mb: f64,
    // Cache for displaying stable values when metrics temporarily unavailable
    pub last_cpu_usage: f32,
    pub last_memory_mb: f64,
}

static PREV_CPU_TIMES: OnceLock<Mutex<HashMap<u32, (u64, Instant)>>> = OnceLock::new();
static NUM_CPUS: OnceLock<u32> = OnceLock::new();

fn get_num_cpus() -> u32 {
    *NUM_CPUS.get_or_init(|| unsafe {
        let mut sys_info: SYSTEM_INFO = SYSTEM_INFO::default();
        GetSystemInfo(&mut sys_info);
        sys_info.dwNumberOfProcessors
    })
}

pub fn is_elevated() -> bool {
    unsafe {
        let mut token = Default::default();

        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut size = mem::size_of::<TOKEN_ELEVATION>() as u32;

        let result = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            size,
            &mut size,
        );

        let _ = CloseHandle(token);

        result.is_ok() && elevation.TokenIsElevated != 0
    }
}

pub fn kill_process(pid: u32) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, false, pid)?;
        windows::Win32::System::Threading::TerminateProcess(handle, 1)?;
        let _ = CloseHandle(handle);
    }
    Ok(())
}

pub fn enumerate_processes() -> Result<Vec<ProcessInfo>, Box<dyn std::error::Error>> {
    let mut processes = Vec::new();
    let mut parent_map: HashMap<u32, u32> = HashMap::new();

    unsafe {
        // First, get parent PIDs using ToolHelp API
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let pid = entry.th32ProcessID;
                let parent_pid = entry.th32ParentProcessID;
                parent_map.insert(pid, parent_pid);

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);

        // Now enumerate processes to get full details
        let mut pids = vec![0u32; 1024];
        let mut bytes_returned = 0u32;

        EnumProcesses(
            pids.as_mut_ptr(),
            (pids.len() * 4) as u32,
            &mut bytes_returned,
        )?;

        let count = bytes_returned as usize / 4;
        pids.truncate(count);

        for pid in pids {
            if pid == 0 {
                continue;
            }

            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);

            if let Ok(handle) = handle {
                let mut path_buffer = [0u16; 260];
                let mut path_len = path_buffer.len() as u32;

                let path = if QueryFullProcessImageNameW(
                    handle,
                    PROCESS_NAME_FORMAT(0),
                    PWSTR(path_buffer.as_mut_ptr()),
                    &mut path_len,
                )
                .is_ok()
                {
                    let path = String::from_utf16_lossy(&path_buffer[..path_len as usize]);
                    let name = path.rsplit('\\').next().unwrap_or(&path).to_string();
                    Some((name, Some(path)))
                } else {
                    // Try to get name from ToolHelp data
                    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
                    if let Ok(snap) = snapshot {
                        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
                        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

                        let mut found_name = None;
                        if Process32FirstW(snap, &mut entry).is_ok() {
                            loop {
                                if entry.th32ProcessID == pid {
                                    let name = String::from_utf16_lossy(&entry.szExeFile)
                                        .trim_end_matches('\0')
                                        .to_string();
                                    found_name = Some((name, None));
                                    break;
                                }
                                if Process32NextW(snap, &mut entry).is_err() {
                                    break;
                                }
                            }
                        }
                        let _ = CloseHandle(snap);
                        found_name
                    } else {
                        None
                    }
                };

                let _ = CloseHandle(handle);

                if let Some((name, path)) = path {
                    let parent_pid = parent_map.get(&pid).copied().unwrap_or(0);
                    processes.push(ProcessInfo {
                        pid,
                        parent_pid,
                        name,
                        path,
                        cpu_usage: 0.0,
                        memory_mb: 0.0,
                        last_cpu_usage: 0.0,
                        last_memory_mb: 0.0,
                    });
                }
            }
        }
    }

    processes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(processes)
}

fn filetime_to_u64(ft: FILETIME) -> u64 {
    ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64)
}

pub fn update_process_metrics(
    processes: &mut [ProcessInfo],
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let now = Instant::now();
        let prev_times = PREV_CPU_TIMES.get_or_init(|| Mutex::new(HashMap::new()));
        let mut prev_times_guard = prev_times.lock().unwrap();
        let mut new_times: HashMap<u32, (u64, Instant)> = HashMap::new();

        for process in processes.iter_mut() {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, process.pid);

            if let Ok(handle) = handle {
                let mut creation_time = FILETIME::default();
                let mut exit_time = FILETIME::default();
                let mut kernel_time = FILETIME::default();
                let mut user_time = FILETIME::default();

                let times_ok = GetProcessTimes(
                    handle,
                    &mut creation_time,
                    &mut exit_time,
                    &mut kernel_time,
                    &mut user_time,
                )
                .is_ok();

                let mut mem_counters = PROCESS_MEMORY_COUNTERS::default();
                let mem_ok = GetProcessMemoryInfo(
                    handle,
                    &mut mem_counters as *mut _ as *mut _,
                    mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
                )
                .is_ok();

                let _ = CloseHandle(handle);

                if times_ok {
                    let total_time = filetime_to_u64(kernel_time) + filetime_to_u64(user_time);
                    new_times.insert(process.pid, (total_time, now));

                    if let Some(&(prev_time, prev_instant)) = prev_times_guard.get(&process.pid) {
                        let elapsed = now.duration_since(prev_instant).as_millis() as u64;
                        if elapsed > 0 {
                            let delta = total_time.saturating_sub(prev_time);
                            let num_cpus = get_num_cpus() as f64;
                            let cpu_percent =
                                ((delta as f64 / 10_000_000.0) / (elapsed as f64 / 1000.0) * 100.0)
                                    / num_cpus;
                            process.cpu_usage = cpu_percent.clamp(0.0, 100.0) as f32;
                            process.last_cpu_usage = process.cpu_usage;
                        }
                    } else {
                        process.last_cpu_usage = 0.0;
                    }
                }

                if mem_ok {
                    process.memory_mb = mem_counters.WorkingSetSize as f64 / (1024.0 * 1024.0);
                    // Cache the value for stable display
                    process.last_memory_mb = process.memory_mb;
                }
            }
        }

        // Merge new times into existing history instead of replacing
        // This preserves CPU history for processes that couldn't be accessed temporarily
        for (pid, time_data) in new_times {
            prev_times_guard.insert(pid, time_data);
        }
    }

    Ok(())
}

pub fn get_process_details(
    pid: u32,
) -> (
    Option<String>,
    Vec<(String, String)>,
    Vec<String>,
    Option<String>,
) {
    let mut command_line = None;
    let environment = Vec::new();
    let mut modules = Vec::new();
    let mut error = None;

    unsafe {
        // Try to open process with VM read and query access
        let handle = OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | windows::Win32::System::Threading::PROCESS_VM_READ,
            false,
            pid,
        );

        if let Ok(handle) = handle {
            // Get loaded modules
            let mut module_handles: [windows::Win32::Foundation::HMODULE; 1024] =
                std::mem::zeroed();
            let mut cb_needed = 0u32;

            if EnumProcessModules(
                handle,
                module_handles.as_mut_ptr(),
                (module_handles.len() * std::mem::size_of::<windows::Win32::Foundation::HMODULE>())
                    as u32,
                &mut cb_needed,
            )
            .is_ok()
            {
                let module_count =
                    cb_needed as usize / std::mem::size_of::<windows::Win32::Foundation::HMODULE>();

                for i in 0..module_count.min(module_handles.len()) {
                    let mut name_buffer = [0u16; 260];
                    let name_len = GetModuleBaseNameW(handle, module_handles[i], &mut name_buffer);

                    if name_len > 0 {
                        let name = String::from_utf16_lossy(&name_buffer[..name_len as usize]);
                        modules.push(name);
                    }
                }
            }

            // Try to get full path of main module
            let mut path_buffer = [0u16; 260];
            let path_len = GetModuleFileNameExW(handle, module_handles[0], &mut path_buffer);

            if path_len > 0 {
                let _path = String::from_utf16_lossy(&path_buffer[..path_len as usize]);
                // Could use this for command line reconstruction
            }

            let _ = CloseHandle(handle);
        } else {
            error = Some("Could not access process - try running as administrator".to_string());
        }
    }

    (command_line, environment, modules, error)
}
