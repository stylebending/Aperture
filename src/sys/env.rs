use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::System::Environment::SetEnvironmentVariableW;
use windows::Win32::System::Registry::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const USER_ENV_KEY: &str = "Environment";
const SYSTEM_ENV_KEY: &str = "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";

fn read_registry_values(hkey: HKEY, subkey: &str) -> Vec<(String, String)> {
    unsafe {
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let mut phk: HKEY = HKEY::default();
        let result = RegOpenKeyExW(hkey, PCWSTR(subkey_w.as_ptr()), 0, KEY_READ, &mut phk);
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
                    let bytes = &data_buf[..data_len as usize];
                    let (prefix, shorts, suffix) = unsafe { bytes.align_to::<u16>() };
                    let value_utf16 = if prefix.is_empty() && suffix.is_empty() {
                        shorts
                    } else {
                        // Unlikely for registry data — fallback by copying
                        &[]
                    };
                    let trimmed = if value_utf16.last() == Some(&0) {
                        &value_utf16[..value_utf16.len() - 1]
                    } else {
                        value_utf16
                    };
                    let value = String::from_utf16_lossy(trimmed);
                    entries.push((name, value));
                } else if value_type == REG_MULTI_SZ.0 {
                    let bytes = &data_buf[..data_len as usize];
                    let (prefix, shorts, suffix) = unsafe { bytes.align_to::<u16>() };
                    let value_utf16 = if prefix.is_empty() && suffix.is_empty() {
                        shorts
                    } else {
                        &[]
                    };
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

fn set_registry_value(hkey: HKEY, subkey: &str, name: &str, value: &str) -> Result<(), String> {
    unsafe {
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let mut phk: HKEY = HKEY::default();
        let result =
            RegOpenKeyExW(hkey, PCWSTR(subkey_w.as_ptr()), 0, KEY_SET_VALUE, &mut phk);
        if result.is_err() {
            return Err("Failed to open registry key".to_string());
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let value_data: Vec<u16> = format!("{}\0", value).encode_utf16().collect();

        let data_bytes = value_data.as_ptr() as *const u8;
        let data_byte_len = value_data.len() * 2;
        let data_slice = std::slice::from_raw_parts(data_bytes, data_byte_len);

        let set_result = RegSetValueExW(
            phk,
            PCWSTR(name_w.as_ptr()),
            0u32,
            REG_SZ,
            Some(data_slice),
        );

        let _ = RegCloseKey(phk);

        if set_result.is_err() {
            Err("Failed to write registry value".to_string())
        } else {
            Ok(())
        }
    }
}

fn delete_registry_value(hkey: HKEY, subkey: &str, name: &str) -> Result<(), String> {
    unsafe {
        let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
        let mut phk: HKEY = HKEY::default();
        let result =
            RegOpenKeyExW(hkey, PCWSTR(subkey_w.as_ptr()), 0, KEY_SET_VALUE, &mut phk);
        if result.is_err() {
            return Err("Failed to open registry key".to_string());
        }

        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let del_result = RegDeleteValueW(phk, PCWSTR(name_w.as_ptr()));

        let _ = RegCloseKey(phk);

        if del_result.is_err() {
            Err("Failed to delete registry value".to_string())
        } else {
            Ok(())
        }
    }
}

pub fn set_user_env_var(name: &str, value: &str) -> Result<(), String> {
    set_registry_value(HKEY_CURRENT_USER, USER_ENV_KEY, name, value)
}

pub fn delete_user_env_var(name: &str) -> Result<(), String> {
    delete_registry_value(HKEY_CURRENT_USER, USER_ENV_KEY, name)
}

pub fn set_system_env_var(name: &str, value: &str) -> Result<(), String> {
    set_registry_value(HKEY_LOCAL_MACHINE, SYSTEM_ENV_KEY, name, value)
}

pub fn delete_system_env_var(name: &str) -> Result<(), String> {
    delete_registry_value(HKEY_LOCAL_MACHINE, SYSTEM_ENV_KEY, name)
}

/// Removes a variable from the current process's environment block (PEB).
/// This prevents ghost Process entries from persisting after registry deletion.
/// Only used for *deletion* (not setting) — lower risk than SetEnvironmentVariableW with a value.
pub fn delete_process_env_var(name: &str) -> Result<(), String> {
    unsafe {
        let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let result = SetEnvironmentVariableW(PCWSTR(name_w.as_ptr()), PCWSTR::null());
        if result.is_err() {
            Err("Failed to delete process environment variable".to_string())
        } else {
            Ok(())
        }
    }
}

pub fn broadcast_env_change() -> Result<(), String> {
    unsafe {
        let msg = "Environment\0";
        let msg_w: Vec<u16> = msg.encode_utf16().collect();

        let result = SendMessageTimeoutW(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            WPARAM(0),
            LPARAM(msg_w.as_ptr() as isize),
            SMTO_ABORTIFHUNG,
            200,
            None,
        );

        if result.0 != 0 {
            Ok(())
        } else {
            Err("Failed to broadcast environment change".to_string())
        }
    }
}
