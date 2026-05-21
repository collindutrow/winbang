#![cfg(target_os = "windows")]

use std::ffi::OsString;
use std::iter::once;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::slice;

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE,
    REG_EXPAND_SZ, REG_OPTION_NON_VOLATILE, REG_SZ, REG_VALUE_TYPE,
    RegCloseKey, RegCreateKeyExW, RegOpenKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
use windows::core::PCWSTR;

#[derive(Copy, Clone, Debug)]
pub enum Root {
    ClassesRoot,
    CurrentUser,
}

impl Root {
    fn hkey(self) -> HKEY {
        match self {
            Root::ClassesRoot => HKEY_CLASSES_ROOT,
            Root::CurrentUser => HKEY_CURRENT_USER,
        }
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(once(0)).collect()
}

fn to_wide_os(s: &str) -> Vec<u16> {
    std::ffi::OsStr::new(s).encode_wide().chain(once(0)).collect()
}

struct OwnedKey(HKEY);

impl Drop for OwnedKey {
    fn drop(&mut self) {
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

fn open_read(root: Root, path: &str) -> Option<OwnedKey> {
    let wpath = to_wide_os(path);
    let mut hkey = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            root.hkey(),
            PCWSTR(wpath.as_ptr()),
            Some(0),
            KEY_READ,
            &mut hkey,
        )
    };
    if status == ERROR_SUCCESS {
        Some(OwnedKey(hkey))
    } else {
        None
    }
}

fn create_or_open_write(root: Root, path: &str) -> Option<OwnedKey> {
    let wpath = to_wide_os(path);
    let mut hkey = HKEY::default();
    let status = unsafe {
        RegCreateKeyExW(
            root.hkey(),
            PCWSTR(wpath.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            None,
            &mut hkey,
            None,
        )
    };
    if status == ERROR_SUCCESS {
        Some(OwnedKey(hkey))
    } else {
        None
    }
}

/// Read a REG_SZ or REG_EXPAND_SZ value from the given key path.
///
/// `value_name` may be empty (`""`) to read the key's default value.
/// Returns `None` if the key is absent, the value is absent, or the value type
/// is not a string. Errors that are not "not-found" are also collapsed to
/// `None`, with the rationale that callers want a clean fallback path.
pub fn read_string(root: Root, key_path: &str, value_name: &str) -> Option<String> {
    let key = open_read(root, key_path)?;
    let wname = to_wide(value_name);
    let mut value_type = REG_VALUE_TYPE::default();
    let mut data_len: u32 = 0;

    // First call: discover required buffer size.
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            PCWSTR(wname.as_ptr()),
            None,
            Some(&mut value_type),
            None,
            Some(&mut data_len),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    if value_type != REG_SZ && value_type != REG_EXPAND_SZ {
        return None;
    }
    if data_len == 0 {
        return Some(String::new());
    }

    let mut buf = vec![0u8; data_len as usize];
    let mut data_len2 = data_len;
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            PCWSTR(wname.as_ptr()),
            None,
            Some(&mut value_type),
            Some(buf.as_mut_ptr()),
            Some(&mut data_len2),
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }

    // Reinterpret bytes as UTF-16 code units.
    let used = data_len2 as usize;
    let code_units = used / 2;
    let wide = unsafe {
        slice::from_raw_parts(buf.as_ptr() as *const u16, code_units)
    };
    // Strip trailing NULs (the registry includes them in the byte count).
    let trimmed: &[u16] = match wide.iter().rposition(|&c| c != 0) {
        Some(idx) => &wide[..=idx],
        None => &[],
    };
    Some(OsString::from_wide(trimmed).to_string_lossy().into_owned())
}

/// Check whether the given (key, value_name) exists. `value_name` may be `""`
/// for the default value.
pub fn value_exists(root: Root, key_path: &str, value_name: &str) -> bool {
    let Some(key) = open_read(root, key_path) else {
        return false;
    };
    let wname = to_wide(value_name);
    let mut value_type = REG_VALUE_TYPE::default();
    let mut data_len: u32 = 0;
    let status = unsafe {
        RegQueryValueExW(
            key.0,
            PCWSTR(wname.as_ptr()),
            None,
            Some(&mut value_type),
            None,
            Some(&mut data_len),
        )
    };
    status == ERROR_SUCCESS
}

/// Write a REG_SZ string value at (key_path, value_name), creating the key
/// chain if needed. Returns true on success. Does NOT check first — callers
/// that want fill-in-the-blanks semantics must combine with `value_exists`.
pub fn write_string(
    root: Root,
    key_path: &str,
    value_name: &str,
    data: &str,
) -> bool {
    let Some(key) = create_or_open_write(root, key_path) else {
        return false;
    };
    let wname = to_wide(value_name);
    let wdata: Vec<u16> = data.encode_utf16().chain(once(0)).collect();
    let byte_len = wdata.len() * 2;
    let bytes = unsafe {
        slice::from_raw_parts(wdata.as_ptr() as *const u8, byte_len)
    };
    let status = unsafe {
        RegSetValueExW(
            key.0,
            PCWSTR(wname.as_ptr()),
            None,
            REG_SZ,
            Some(bytes),
        )
    };
    status == ERROR_SUCCESS
}

/// Read the first value name under `key_path` (used for the
/// `.<ext>\OpenWithProgids` fallback, whose entries are stored as value names).
pub fn first_value_name(root: Root, key_path: &str) -> Option<String> {
    use windows::Win32::System::Registry::RegEnumValueW;
    let key = open_read(root, key_path)?;
    let mut name_buf = vec![0u16; 256];
    let mut name_len: u32 = name_buf.len() as u32;
    let status = unsafe {
        RegEnumValueW(
            key.0,
            0,
            Some(windows::core::PWSTR(name_buf.as_mut_ptr())),
            &mut name_len,
            None,
            None,
            None,
            None,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let name: Vec<u16> = name_buf.into_iter().take(name_len as usize).collect();
    Some(OsString::from_wide(&name).to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_then_read_default_value() {
        let key = "Software\\Classes\\winbang-test-registry\\sub";
        assert!(write_string(Root::CurrentUser, key, "", "hello"));
        let got = read_string(Root::CurrentUser, key, "");
        assert_eq!(got.as_deref(), Some("hello"));
        // Clean up: delete the test key tree.
        let _ = unsafe {
            use windows::Win32::System::Registry::RegDeleteTreeW;
            let wpath = to_wide_os("Software\\Classes\\winbang-test-registry");
            RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(wpath.as_ptr()))
        };
    }

    #[test]
    fn value_exists_round_trip() {
        let key = "Software\\Classes\\winbang-test-registry-exists";
        assert!(!value_exists(Root::CurrentUser, key, ""));
        assert!(write_string(Root::CurrentUser, key, "", "x"));
        assert!(value_exists(Root::CurrentUser, key, ""));
        let _ = unsafe {
            use windows::Win32::System::Registry::RegDeleteTreeW;
            let wpath = to_wide_os(key);
            RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(wpath.as_ptr()))
        };
    }
}
