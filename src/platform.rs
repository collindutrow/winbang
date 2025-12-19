use crate::log_debug;
use std::path::PathBuf;
use windows::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
    TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::K32GetModuleBaseNameW;
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, PROCESS_QUERY_INFORMATION,
    PROCESS_VM_READ,
};

/// Check if the parent process is a GUI shell.
///
/// # Arguments
///
/// * `gui_shells`: List of GUI shell process names.
///
/// returns: bool
///
/// # Examples
///
/// ```
/// let gui_shells = vec!["explorer.exe".to_string()];
/// let is_gui_shell = is_interactive_parent(&gui_shells);
/// ```
pub fn is_interactive_parent(gui_shells: &[String]) -> bool {
    let parent_pid = get_parent_pid().unwrap_or(0);
    let parent_name = get_process_name(parent_pid);
    let is_gui_shell = parent_name
        .as_ref()
        .map(|name| {
            gui_shells
                .iter()
                .any(|shell| shell.eq_ignore_ascii_case(name))
        })
        .unwrap_or(false);

    log_debug!(&format!("GUI Shells: {:?}", gui_shells));

    log_debug!(&format!(
        "Parent PID: {}, Parent Name: {:?}, Is GUI Shell: {}",
        parent_pid, parent_name, is_gui_shell
    ));

    is_gui_shell
}

/// Get the parent process ID of the current process.
///
/// # Arguments
///
/// * None
///
/// returns: Option<u32>
///
/// # Examples
///
/// ```
/// let parent_pid = get_parent_pid();
/// ```
fn get_parent_pid() -> Option<u32> {
    unsafe {
        let current_pid = GetCurrentProcessId();
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;

        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == current_pid {
                    let _ = CloseHandle(snapshot);
                    return Some(entry.th32ParentProcessID);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
        None
    }
}

/// Get the name of a process by its PID.
///
/// # Arguments
///
/// * `pid`: Process ID of the target process.
///
/// returns: Option<String>
///
/// # Examples
///
/// ```
/// let pid = 1234;
/// let process_name = get_process_name(pid);
/// ```
fn get_process_name(pid: u32) -> Option<String> {
    unsafe {
        let h_process = OpenProcess(
            PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        )
        .ok()?;
        if h_process.is_invalid() {
            return None;
        }

        let mut buffer = [0u16; 260];
        let len = K32GetModuleBaseNameW(h_process, None, &mut buffer);
        let _ = CloseHandle(h_process);

        if len == 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

/// Resolve the executable path using the `which` command.
///
/// # Arguments
///
/// * `executable`: Name of the executable to resolve.
///
/// returns: Option<PathBuf>
///
/// # Examples
///
/// ```
/// let executable = "python";
/// let resolved_path = resolve_executable(executable);
/// ```
pub(crate) fn resolve_executable(executable: &str) -> Option<PathBuf> {
    which::which(executable).ok()
}
