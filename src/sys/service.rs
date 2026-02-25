use windows::core::PCWSTR;
use windows::Win32::System::Services::{
    CloseServiceHandle, ControlService, EnumServicesStatusExW, OpenSCManagerW, OpenServiceW,
    QueryServiceConfigW, StartServiceW, ENUM_SERVICE_STATUS_PROCESSW, QUERY_SERVICE_CONFIGW,
    SC_ENUM_PROCESS_INFO, SERVICE_CONTROL_STOP, SERVICE_QUERY_CONFIG, SERVICE_STATE_ALL,
    SERVICE_STATUS, SERVICE_STATUS_CURRENT_STATE, SERVICE_WIN32,
};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ServiceInfo {
    pub service_name: String,
    pub display_name: String,
    pub status: String,
    pub start_type: String,
    pub service_type: String,
    pub pid: u32,
}

fn status_to_string(current_state: SERVICE_STATUS_CURRENT_STATE) -> String {
    match current_state.0 {
        0x00000001 => "Stopped".to_string(),
        0x00000002 => "Start Pending".to_string(),
        0x00000003 => "Stop Pending".to_string(),
        0x00000004 => "Running".to_string(),
        0x00000005 => "Continue Pending".to_string(),
        0x00000006 => "Pause Pending".to_string(),
        0x00000007 => "Paused".to_string(),
        _ => format!("Unknown ({:#x})", current_state.0),
    }
}

fn service_type_to_string(service_type: u32) -> String {
    match service_type {
        0x00000010 => "Own Process",
        0x00000020 => "Share Process",
        0x00000110 => "Own Process (Interactive)",
        0x00000120 => "Share Process (Interactive)",
        _ => "Unknown",
    }
    .to_string()
}

fn start_type_to_string(start_type: u32) -> String {
    match start_type {
        0x00000000 => "Boot",
        0x00000001 => "System",
        0x00000002 => "Auto",
        0x00000003 => "Manual",
        0x00000004 => "Disabled",
        _ => "Unknown",
    }
    .to_string()
}

unsafe fn pwstr_to_string(ptr: windows::core::PWSTR) -> String {
    unsafe {
        if ptr.0.is_null() {
            return String::new();
        }
        let mut len = 0usize;
        while *ptr.0.add(len) != 0 {
            len += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(ptr.0, len))
    }
}

pub fn enumerate_services() -> Result<Vec<ServiceInfo>, Box<dyn std::error::Error>> {
    unsafe {
        let sc_manager = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), 0x0004)?;

        let mut bytes_needed = 0u32;
        let mut services_returned = 0u32;

        let _ = EnumServicesStatusExW(
            sc_manager,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            None,
            &mut bytes_needed,
            &mut services_returned,
            None,
            PCWSTR::null(),
        );

        if bytes_needed == 0 {
            let _ = CloseServiceHandle(sc_manager);
            return Ok(Vec::new());
        }

        let buffer_size = bytes_needed as usize;
        let mut buffer: Vec<u8> = vec![0; buffer_size];

        EnumServicesStatusExW(
            sc_manager,
            SC_ENUM_PROCESS_INFO,
            SERVICE_WIN32,
            SERVICE_STATE_ALL,
            Some(buffer.as_mut_slice()),
            &mut bytes_needed,
            &mut services_returned,
            None,
            PCWSTR::null(),
        )?;

        let _ = CloseServiceHandle(sc_manager);

        let mut services = Vec::new();

        let ptr = buffer.as_ptr() as *const ENUM_SERVICE_STATUS_PROCESSW;

        for i in 0..services_returned as usize {
            let service = &*ptr.add(i);

            let service_name = pwstr_to_string(service.lpServiceName);
            let display_name = pwstr_to_string(service.lpDisplayName);
            let status = status_to_string(service.ServiceStatusProcess.dwCurrentState);
            let service_type = service_type_to_string(service.ServiceStatusProcess.dwServiceType.0);

            // Query service config to get start type
            let start_type = if let Ok(sc_manager) =
                OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), 0x0001)
            {
                let wide_name: Vec<u16> = service_name
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let service_handle =
                    OpenServiceW(sc_manager, PCWSTR(wide_name.as_ptr()), SERVICE_QUERY_CONFIG);

                let start_type_str = if let Ok(handle) = service_handle {
                    let mut config_buffer_size = 0u32;
                    let _ = QueryServiceConfigW(handle, None, 0, &mut config_buffer_size);

                    let mut start = "Unknown".to_string();
                    if config_buffer_size > 0 {
                        let mut config_buffer: Vec<u8> = vec![0; config_buffer_size as usize];
                        if QueryServiceConfigW(
                            handle,
                            Some(config_buffer.as_mut_ptr() as *mut _),
                            config_buffer_size,
                            &mut config_buffer_size,
                        )
                        .is_ok()
                        {
                            let config = &*(config_buffer.as_ptr() as *const QUERY_SERVICE_CONFIGW);
                            start = start_type_to_string(config.dwStartType.0);
                        }
                    }
                    let _ = CloseServiceHandle(handle);
                    start
                } else {
                    "Unknown".to_string()
                };

                let _ = CloseServiceHandle(sc_manager);
                start_type_str
            } else {
                "Unknown".to_string()
            };

            services.push(ServiceInfo {
                service_name,
                display_name,
                status,
                start_type,
                service_type,
                pid: service.ServiceStatusProcess.dwProcessId,
            });
        }

        services.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        Ok(services)
    }
}

pub fn toggle_service(
    service_name: &str,
    current_status: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let sc_manager = OpenSCManagerW(PCWSTR::null(), PCWSTR::null(), 0x0001)?;

        let wide_name: Vec<u16> = service_name
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let service = OpenServiceW(sc_manager, PCWSTR(wide_name.as_ptr()), 0x0001 | 0x0020)?;

        let mut status = SERVICE_STATUS::default();

        match current_status {
            "Running" => {
                ControlService(service, SERVICE_CONTROL_STOP, &mut status)?;
            }
            "Stopped" => {
                StartServiceW(service, None)?;
            }
            _ => {}
        }

        let _ = CloseServiceHandle(service);
        let _ = CloseServiceHandle(sc_manager);
    }

    Ok(())
}
