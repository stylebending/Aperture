use std::net::Ipv4Addr;
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCPTABLE_OWNER_PID, MIB_UDPTABLE_OWNER_PID,
    TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};
use windows::Win32::Networking::WinSock::{ntohl, ntohs};

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub protocol: String,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: String,
    pub pid: u32,
    pub process_name: Option<String>,
}

fn tcp_state_to_string(state: u32) -> String {
    match state {
        1 => "CLOSED".to_string(),
        2 => "LISTENING".to_string(),
        3 => "SYN_SENT".to_string(),
        4 => "SYN_RCVD".to_string(),
        5 => "ESTABLISHED".to_string(),
        6 => "FIN_WAIT1".to_string(),
        7 => "FIN_WAIT2".to_string(),
        8 => "CLOSE_WAIT".to_string(),
        9 => "CLOSING".to_string(),
        10 => "LAST_ACK".to_string(),
        11 => "TIME_WAIT".to_string(),
        12 => "DELETE_TCB".to_string(),
        _ => format!("UNKNOWN({})", state),
    }
}

fn ip_to_string(ip: u32) -> String {
    let bytes = ip.to_be_bytes();
    Ipv4Addr::from(bytes).to_string()
}

pub fn enumerate_connections() -> Result<Vec<ConnectionInfo>, Box<dyn std::error::Error>> {
    let mut connections = Vec::new();

    unsafe {
        let mut size = 0u32;
        let _ = GetExtendedTcpTable(None, &mut size, false, 2, TCP_TABLE_OWNER_PID_ALL, 0);

        let mut buffer = vec![0u8; size as usize];

        let result = GetExtendedTcpTable(
            Some(buffer.as_mut_ptr() as *mut _),
            &mut size,
            false,
            2,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );

        if result == 0 {
            let table = buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID;
            let num_entries = (*table).dwNumEntries;
            let rows = (*table).table.as_ptr();

            for i in 0..num_entries {
                let row = &*rows.add(i as usize);

                let local_addr = ip_to_string(ntohl(row.dwLocalAddr));
                let local_port = ntohs(row.dwLocalPort as u16);
                let remote_addr = ip_to_string(ntohl(row.dwRemoteAddr));
                let remote_port = ntohs(row.dwRemotePort as u16);

                connections.push(ConnectionInfo {
                    protocol: "TCP".to_string(),
                    local_addr,
                    local_port,
                    remote_addr,
                    remote_port,
                    state: tcp_state_to_string(row.dwState),
                    pid: row.dwOwningPid,
                    process_name: None,
                });
            }
        }

        let mut size = 0u32;
        let _ = GetExtendedUdpTable(None, &mut size, false, 2, UDP_TABLE_OWNER_PID, 0);

        let mut buffer = vec![0u8; size as usize];

        let result = GetExtendedUdpTable(
            Some(buffer.as_mut_ptr() as *mut _),
            &mut size,
            false,
            2,
            UDP_TABLE_OWNER_PID,
            0,
        );

        if result == 0 {
            let table = buffer.as_ptr() as *const MIB_UDPTABLE_OWNER_PID;
            let num_entries = (*table).dwNumEntries;
            let rows = (*table).table.as_ptr();

            for i in 0..num_entries {
                let row = &*rows.add(i as usize);

                let local_addr = ip_to_string(ntohl(row.dwLocalAddr));
                let local_port = ntohs(row.dwLocalPort as u16);

                connections.push(ConnectionInfo {
                    protocol: "UDP".to_string(),
                    local_addr,
                    local_port,
                    remote_addr: "0.0.0.0".to_string(),
                    remote_port: 0,
                    state: "N/A".to_string(),
                    pid: row.dwOwningPid,
                    process_name: None,
                });
            }
        }
    }

    connections.sort_by(|a, b| a.pid.cmp(&b.pid));
    Ok(connections)
}
