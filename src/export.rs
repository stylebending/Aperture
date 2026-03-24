use serde::Serialize;
use std::io::Write;
use std::time::SystemTime;

use crate::state::controller::ControllerState;
use crate::state::locker::LockerState;
use crate::state::nexus::NexusState;
use crate::sys::network::ConnectionInfo;
use crate::sys::process::ProcessInfo;
use crate::sys::service::ServiceInfo;

#[derive(Serialize)]
pub struct ExportData {
    pub timestamp: String,
    pub processes: Vec<ProcessInfo>,
    pub services: Vec<ServiceInfo>,
    pub connections: Vec<ConnectionInfo>,
}

pub fn export_to_json(
    locker_state: &LockerState,
    controller_state: &ControllerState,
    nexus_state: &NexusState,
) -> Result<String, Box<dyn std::error::Error>> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let data = ExportData {
        timestamp: format!("{}", timestamp),
        processes: locker_state.processes.clone(),
        services: controller_state.services.clone(),
        connections: nexus_state.connections.clone(),
    };

    let json = serde_json::to_string_pretty(&data)?;

    let filename = format!("aperture_export_{}.json", timestamp);
    let path = get_export_path(&filename)?;

    let mut file = std::fs::File::create(&path)?;
    file.write_all(json.as_bytes())?;

    Ok(path.to_string_lossy().to_string())
}

pub fn export_to_csv(
    locker_state: &LockerState,
    controller_state: &ControllerState,
    nexus_state: &NexusState,
) -> Result<String, Box<dyn std::error::Error>> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_secs();

    let filename = format!("aperture_export_{}.csv", timestamp);
    let path = get_export_path(&filename)?;

    let mut writer = csv::Writer::from_path(&path)?;

    // Write header
    writer.write_record(&["Type", "ID", "Name", "Status", "Details"])?;

    // Write processes
    for process in &locker_state.processes {
        writer.write_record(&[
            "Process",
            &process.pid.to_string(),
            &process.name,
            &format!(
                "CPU: {:.1}%, Mem: {:.1} MB",
                process.cpu_usage, process.memory_mb
            ),
            &process.path.as_deref().unwrap_or("-"),
        ])?;
    }

    // Write services
    for service in &controller_state.services {
        writer.write_record(&[
            "Service",
            &service.pid.to_string(),
            &service.display_name,
            &service.status,
            &format!(
                "Start: {}, Type: {}",
                service.start_type, service.service_type
            ),
        ])?;
    }

    // Write connections
    for conn in &nexus_state.connections {
        writer.write_record(&[
            "Connection",
            &conn.pid.to_string(),
            &conn.process_name.as_deref().unwrap_or("-"),
            &conn.state,
            &format!(
                "{}:{} -> {}:{}",
                conn.local_addr, conn.local_port, conn.remote_addr, conn.remote_port
            ),
        ])?;
    }

    writer.flush()?;

    Ok(path.to_string_lossy().to_string())
}

fn get_export_path(filename: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    // Try to get the Documents folder
    if let Some(home) = dirs::home_dir() {
        let documents = home.join("Documents");
        if documents.exists() {
            return Ok(documents.join(filename));
        }
    }

    // Fallback to temp directory
    let temp = std::env::temp_dir();
    Ok(temp.join(filename))
}

// Add dirs crate support for home directory detection
use dirs;
