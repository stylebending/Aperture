use windows::core::PWSTR;
use windows::Win32::Foundation::WIN32_ERROR;
use windows::Win32::System::RestartManager::{
    RmEndSession, RmGetList, RmRegisterResources, RmStartSession, RM_PROCESS_INFO,
};

#[derive(Debug, Clone)]
pub struct LockingProcess {
    pub pid: u32,
    pub name: String,
}

pub fn find_locking_processes(
    file_paths: &[&str],
) -> Result<Vec<LockingProcess>, Box<dyn std::error::Error>> {
    unsafe {
        let mut session_handle: u32 = 0;
        let mut session_key = [0u16; 64];

        let result = RmStartSession(&mut session_handle, 0, PWSTR(session_key.as_mut_ptr()));

        if result != WIN32_ERROR(0) {
            return Err(format!("RmStartSession failed with error: {:?}", result).into());
        }

        let wide_paths: Vec<Vec<u16>> = file_paths
            .iter()
            .map(|p| p.encode_utf16().chain(std::iter::once(0)).collect())
            .collect();

        let path_ptrs: Vec<windows::core::PCWSTR> = wide_paths
            .iter()
            .map(|p| windows::core::PCWSTR(p.as_ptr()))
            .collect();

        let result = RmRegisterResources(session_handle, Some(&path_ptrs), None, None);

        if result != WIN32_ERROR(0) {
            let _ = RmEndSession(session_handle);
            return Err(format!("RmRegisterResources failed with error: {:?}", result).into());
        }

        let mut num_processes: u32 = 0;
        let mut bytes_needed: u32 = 0;

        let _ = RmGetList(
            session_handle,
            &mut num_processes,
            &mut bytes_needed,
            None,
            std::ptr::null_mut(),
        );

        if num_processes == 0 {
            let _ = RmEndSession(session_handle);
            return Ok(Vec::new());
        }

        let buffer_size = bytes_needed as usize;
        let mut buffer: Vec<u8> = vec![0; buffer_size];
        let process_info_ptr = buffer.as_mut_ptr() as *mut RM_PROCESS_INFO;

        let result = RmGetList(
            session_handle,
            &mut num_processes,
            &mut bytes_needed,
            Some(process_info_ptr),
            std::ptr::null_mut(),
        );

        if result != WIN32_ERROR(0) {
            let _ = RmEndSession(session_handle);
            return Err(format!("RmGetList failed with error: {:?}", result).into());
        }

        let mut locking_processes = Vec::new();

        for i in 0..num_processes as usize {
            let info = &*process_info_ptr.add(i);
            let pid = info.Process.dwProcessId;

            let name_len = info
                .strAppName
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(info.strAppName.len());

            let name = String::from_utf16_lossy(&info.strAppName[..name_len]);

            locking_processes.push(LockingProcess { pid, name });
        }

        let _ = RmEndSession(session_handle);

        locking_processes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(locking_processes)
    }
}

pub fn find_locking_processes_in_directory(
    directory: &str,
) -> Result<Vec<LockingProcess>, Box<dyn std::error::Error>> {
    use std::fs;
    use std::path::Path;

    let mut all_files: Vec<String> = Vec::new();
    let path = Path::new(directory);

    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if entry.path().is_file() {
                    if let Some(path_str) = entry.path().to_str() {
                        all_files.push(path_str.to_string());
                    }
                }
            }
        }
    } else if path.is_file() {
        if let Some(path_str) = path.to_str() {
            all_files.push(path_str.to_string());
        }
    }

    if all_files.is_empty() {
        return Ok(Vec::new());
    }

    let file_refs: Vec<&str> = all_files.iter().map(|s| s.as_str()).collect();
    find_locking_processes(&file_refs)
}
