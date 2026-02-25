use std::collections::HashMap;
use std::mem;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, FILETIME};
use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
use windows::Win32::System::ProcessStatus::{
    EnumProcesses, GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
};
use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
use windows::Win32::System::Threading::{
    GetCurrentProcess, GetProcessTimes, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
    PROCESS_NAME_FORMAT, PROCESS_QUERY_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_TERMINATE, PROCESS_VM_READ,
};

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
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

    unsafe {
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
                    None
                };

                let _ = CloseHandle(handle);

                if let Some((name, path)) = path {
                    processes.push(ProcessInfo {
                        pid,
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
            let handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                false,
                process.pid,
            );

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

        *prev_times_guard = new_times;
    }

    Ok(())
}
