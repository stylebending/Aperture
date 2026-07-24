use std::mem;
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::System::Registry::*;

const USER_ENV_KEY: &str = "Environment";
const SYSTEM_ENV_KEY: &str = "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";

fn read_registry_values(hkey: HKEY, subkey: &str) -> Vec<(String, String)> {
    unsafe {
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let mut phk: HKEY = HKEY::default();
        let result =
            RegOpenKeyExW(hkey, PCWSTR(subkey_w.as_ptr()), 0, KEY_READ, &mut phk);
        if result.is_err() {
            return Vec::new();
        }

        let mut value_count: u32 = 0;
        let mut max_value_name_len: u32 = 0;
        let mut max_value_data_len: u32 = 0;

        let info_result = RegQueryInfoKeyW(
            phk,
            PWSTR::null(),
            None,
            None,
            None,
            None,
            None,
            Some(&mut value_count),
            Some(&mut max_value_name_len),
            Some(&mut max_value_data_len),
            None,
            None,
        );

        if info_result.is_err() || value_count == 0 {
            let _ = RegCloseKey(phk);
            return Vec::new();
        }

        max_value_name_len += 1;
        let mut entries = Vec::with_capacity(value_count as usize);

        for i in 0..value_count {
            let mut name_buf = vec![0u16; max_value_name_len as usize];
            let mut name_len = max_value_name_len;
            let mut value_type: u32 = 0;
            let mut data_buf = vec![0u8; max_value_data_len as usize];
            let mut data_len = max_value_data_len;

            let enum_result = RegEnumValueW(
                phk,
                i,
                PWSTR(name_buf.as_mut_ptr()),
                &mut name_len,
                None,
                Some(&mut value_type),
                Some(data_buf.as_mut_ptr()),
                Some(&mut data_len),
            );

            if enum_result.is_ok() {
                let name = String::from_utf16_lossy(&name_buf[..name_len as usize]);

                if value_type == REG_SZ.0 || value_type == REG_EXPAND_SZ.0 {
                    let value_utf16: &[u16] =
                        mem::transmute(&data_buf[..data_len as usize]);
                    let trimmed: &[u16] = if value_utf16.last() == Some(&0) {
                        &value_utf16[..value_utf16.len() - 1]
                    } else {
                        value_utf16
                    };
                    let value = String::from_utf16_lossy(trimmed);
                    entries.push((name, value));
                } else if value_type == REG_MULTI_SZ.0 {
                    let value_utf16: &[u16] =
                        mem::transmute(&data_buf[..data_len as usize]);
                    let parts: Vec<String> = value_utf16
                        .split(|&c| c == 0)
                        .filter(|s| !s.is_empty())
                        .map(|s| String::from_utf16_lossy(s))
                        .collect();
                    entries.push((name, parts.join("; ")));
                }
            }
        }

        let _ = RegCloseKey(phk);
        entries
    }
}

pub fn get_user_env_vars() -> Vec<(String, String)> {
    read_registry_values(HKEY_CURRENT_USER, USER_ENV_KEY)
}

pub fn get_system_env_vars() -> Vec<(String, String)> {
    read_registry_values(HKEY_LOCAL_MACHINE, SYSTEM_ENV_KEY)
}

pub fn get_process_env_vars() -> Vec<(String, String)> {
    std::env::vars().collect()
}
