use std::collections::HashSet;
use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::System::RestartManager::{
    RmEndSession, RmGetList, RmRegisterResources, RmStartSession, RM_APP_STATUS,
    RM_INVALID_PROCESS, RM_PROCESS_INFO,
};

#[derive(Debug, Clone)]
pub struct LockingProcess {
    pub pid: u32,
    pub name: String,
}

/// Canonicalizes a path for Windows Restart Manager.
/// Converts to absolute path with proper Windows formatting.
fn canonicalize_path(path: &str) -> Option<String> {
    let path_obj = Path::new(path);

    // Try to get absolute path
    let absolute = if path_obj.is_absolute() {
        path_obj.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path_obj)
    };

    // Clean up the path: normalize . and .. components
    match absolute.canonicalize() {
        Ok(canon) => {
            // canonicalize() adds \\?\ prefix on Windows, which is good for long paths
            Some(canon.to_string_lossy().to_string())
        }
        Err(_) => {
            // If file doesn't exist, still try to get absolute path
            Some(absolute.to_string_lossy().to_string())
        }
    }
}

/// Finds processes that are locking the specified files using Windows Restart Manager API.
/// This is the official, reliable way to detect file locks on Windows Vista and later.
pub fn find_locking_processes(
    file_paths: &[&str],
) -> Result<Vec<LockingProcess>, Box<dyn std::error::Error>> {
    if file_paths.is_empty() {
        return Ok(Vec::new());
    }

    // Canonicalize all paths
    let canonical_paths: Vec<String> = file_paths
        .iter()
        .filter_map(|&p| canonicalize_path(p))
        .collect();

    if canonical_paths.is_empty() {
        return Ok(Vec::new());
    }

    unsafe {
        // Start a Restart Manager session
        let mut session_handle: u32 = 0;
        let mut session_key = [0u16; 256];

        let result = RmStartSession(
            &mut session_handle,
            0,
            windows::core::PWSTR(session_key.as_mut_ptr()),
        );

        if result.0 != 0 {
            return Err(format!("RmStartSession failed with error {}", result.0).into());
        }

        // Prepare file paths as wide strings
        let wide_paths: Vec<Vec<u16>> = canonical_paths
            .iter()
            .map(|p| {
                let mut wide: Vec<u16> = p.encode_utf16().collect();
                wide.push(0); // null terminator
                wide
            })
            .collect();

        // Create slice of PCWSTR pointers
        let path_refs: Vec<PCWSTR> = wide_paths.iter().map(|p| PCWSTR(p.as_ptr())).collect();

        // Register the file resources we want to check
        let result = if path_refs.is_empty() {
            RmRegisterResources(session_handle, None, None, None)
        } else {
            RmRegisterResources(session_handle, Some(&path_refs), None, None)
        };

        if result.0 != 0 {
            let _ = RmEndSession(session_handle);
            return Err(format!("RmRegisterResources failed with error {}", result.0).into());
        }

        // Get the list of processes that are using these resources
        let mut proc_info_needed: u32 = 0;
        let mut proc_info_count: u32 = 0;
        let mut reboot_reasons: u32 = 0;

        // First call to get required buffer size
        let result = RmGetList(
            session_handle,
            &mut proc_info_needed,
            &mut proc_info_count,
            None,
            &mut reboot_reasons,
        );

        // Check for specific error codes
        if result.0 != 0 && result.0 != 234 {
            // 234 = ERROR_MORE_DATA, expected on first call
            let _ = RmEndSession(session_handle);
            return Err(format!("RmGetList (first call) failed with error {}", result.0).into());
        }

        if proc_info_needed == 0 {
            // No processes are locking these files
            let _ = RmEndSession(session_handle);
            return Ok(Vec::new());
        }

        // Allocate buffer for process info
        let mut process_info: Vec<RM_PROCESS_INFO> =
            vec![RM_PROCESS_INFO::default(); proc_info_needed as usize];
        proc_info_count = proc_info_needed;

        // Second call to get actual data
        let result = RmGetList(
            session_handle,
            &mut proc_info_needed,
            &mut proc_info_count,
            Some(process_info.as_mut_ptr()),
            &mut reboot_reasons,
        );

        if result.0 != 0 {
            let _ = RmEndSession(session_handle);
            return Err(format!("RmGetList (second call) failed with error {}", result.0).into());
        }

        // Collect unique processes
        let mut seen_pids: HashSet<u32> = HashSet::new();
        let mut locking_processes: Vec<LockingProcess> = Vec::new();

        for i in 0..proc_info_count as usize {
            let info = &process_info[i];
            let pid = info.Process.dwProcessId;

            // Skip invalid PIDs and duplicates
            let invalid_pid = RM_INVALID_PROCESS as u32;
            if pid == 0 || pid == invalid_pid || !seen_pids.insert(pid) {
                continue;
            }

            // Only include running processes (check if app status indicates running)
            let app_status = RM_APP_STATUS(info.AppStatus as i32);
            if app_status.0 & 1 != 0 {
                // RM_APP_STATUS_RUNNING = 1
                // Extract process name from the wide string array
                let name = if info.strAppName[0] != 0 {
                    let len = info.strAppName.iter().position(|&c| c == 0).unwrap_or(256);
                    String::from_utf16_lossy(&info.strAppName[..len])
                } else {
                    format!("PID {}", pid)
                };

                locking_processes.push(LockingProcess { pid, name });
            }
        }

        // Clean up the session
        let _ = RmEndSession(session_handle);

        // Sort and deduplicate by PID (already deduped by HashSet, but sort for consistency)
        locking_processes.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(locking_processes)
    }
}

/// Finds processes locking files in a directory.
/// Returns the list of locking processes and the count of files scanned.
pub fn find_locking_processes_in_directory(
    directory: &str,
) -> Result<(Vec<LockingProcess>, usize), Box<dyn std::error::Error>> {
    use std::fs;

    let path = Path::new(directory);

    // Collect all files to check
    let mut all_files: Vec<String> = Vec::new();

    if path.is_dir() {
        // Read all entries in the directory
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Some(path_str) = entry_path.to_str() {
                        all_files.push(path_str.to_string());
                    }
                }
            }
        }
    } else if path.is_file() {
        // Single file
        if let Some(path_str) = path.to_str() {
            all_files.push(path_str.to_string());
        }
    }

    let file_count = all_files.len();

    if all_files.is_empty() {
        return Ok((Vec::new(), 0));
    }

    // Convert to slice of string references
    let file_refs: Vec<&str> = all_files.iter().map(|s| s.as_str()).collect();

    // Find locking processes
    let locking_processes = find_locking_processes(&file_refs)?;

    Ok((locking_processes, file_count))
}
