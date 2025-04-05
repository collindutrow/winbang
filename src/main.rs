use serde::Deserialize;
use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use toml;
use windows::{
    Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
    Win32::Foundation::{HINSTANCE, HWND},
    Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    },
    Win32::System::ProcessStatus::K32GetModuleBaseNameW,
    Win32::System::Threading::GetCurrentProcessId,
    Win32::System::Threading::OpenProcess,
    Win32::System::Threading::{PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    Win32::UI::Controls::{
        TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDF_ALLOW_DIALOG_CANCELLATION, TaskDialogIndirect,
    },
    core::PCWSTR,
};

// Enum to represent user choices in the interactive prompt
enum UserChoice {
    Run,
    Edit,
    Exit,
}

#[derive(Deserialize)]
struct DefaultHandler {
    util: String,
    args: Option<String>,
}

#[derive(Deserialize)]
struct DefaultLargeHandler {
    size_mb_threshold: u64,
    util: String,
    args: Option<String>,
}

// Structure to define file associations
#[derive(Deserialize)]
struct FileAssociation {
    extension: String,
    interpreter: String,
    editor: Option<String>,
}

#[derive(Deserialize)]
struct DispatchOverride {
    interpreter: String,
    path_override: Option<String>,
    args_override: Option<String>,
}

#[derive(Deserialize)]
struct Config {
    gui_shells: Option<Vec<String>>,
    file_associations: Option<Vec<FileAssociation>>,
    dispatch_overrides: Option<Vec<DispatchOverride>>,
    default: Option<DefaultHandler>,
    default_large: Option<DefaultLargeHandler>,
}

// Determine if the parent process is a known GUI shell
fn is_interactive_parent(gui_shells: &[String]) -> bool {
    let parent_pid = get_parent_pid().unwrap_or(0);
    if let Some(parent_name) = get_process_name(parent_pid) {
        gui_shells
            .iter()
            .any(|shell| shell.eq_ignore_ascii_case(&parent_name))
    } else {
        false
    }
}

// Get the parent process ID
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
                    CloseHandle(snapshot);
                    return Some(entry.th32ParentProcessID);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        None
    }
}

// Get the process name given a process ID
fn get_process_name(pid: u32) -> Option<String> {
    unsafe {
        let h_process =
            OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;
        if h_process.is_invalid() {
            return None;
        }

        let mut buffer = [0u16; 260];
        let len = K32GetModuleBaseNameW(h_process, None, &mut buffer);
        CloseHandle(h_process);

        if len == 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..len as usize]))
    }
}

// Resolve the full path of an executable in the system's PATH
fn resolve_executable(executable: &str) -> Option<PathBuf> {
    which::which(executable).ok()
}

// Read the first line of a file to determine the interpreter or fallback based on extension
fn read_dispatch_line_or_fallback(
    script_path: &Path,
    associations: &[FileAssociation],
) -> Option<(String, Option<String>)> {
    if let Ok(file) = fs::File::open(script_path) {
        let mut reader = io::BufReader::new(file);
        let mut first_line = String::new();

        if reader.read_line(&mut first_line).is_ok() {
            if first_line.starts_with("#!") || first_line.starts_with("//!") {
                let parts: Vec<&str> = first_line.trim().split_whitespace().collect();
                if parts.len() > 1 {
                    let full_command = parts[1]; // e.g. "/usr/bin/env" or "/usr/bin/deno"
                    let first_arg = parts.get(2).copied(); // for env-based dispatch

                    let command = if full_command.ends_with("env") && first_arg.is_some() {
                        Path::new(first_arg.unwrap())
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or(first_arg.unwrap())
                            .to_string()
                    } else {
                        Path::new(full_command)
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or(full_command)
                            .to_string()
                    };

                    return Some((command, None)); // return just the command, e.g. "deno"
                }
            }
        }
    }
    fallback_by_extension(script_path, associations)
}

// Fallback to determine interpreter based on file extension
fn fallback_by_extension(
    script_path: &Path,
    associations: &[FileAssociation],
) -> Option<(String, Option<String>)> {
    if let Some(ext) = script_path.extension().and_then(|e| e.to_str()) {
        for assoc in associations {
            if assoc.extension.eq_ignore_ascii_case(ext) {
                return Some((assoc.interpreter.clone(), None));
            }
        }
    }
    None
}

// Load configuration from a TOML file
fn load_config(config_path: &Path) -> Config {
    let default_config = Config {
        gui_shells: Some(vec!["explorer.exe".to_string()]),
        default: Some(DefaultHandler {
            util: "notepad".to_string(),
            args: Some("$script".to_string()),
        }),
        default_large: Some(DefaultLargeHandler {
            size_mb_threshold: 50,
            util: "notepad".to_string(),
            args: Some("$script".to_string()),
        }),
        dispatch_overrides: Some(vec![]),
        file_associations: Some(vec![
            FileAssociation {
                extension: "rb".to_string(),
                interpreter: "ruby".to_string(),
                editor: None,
            },
            FileAssociation {
                extension: "py".to_string(),
                interpreter: "python".to_string(),
                editor: None,
            },
            FileAssociation {
                extension: "js".to_string(),
                interpreter: "node".to_string(),
                editor: None,
            },
            FileAssociation {
                extension: "ts".to_string(),
                interpreter: "deno".to_string(),
                editor: None,
            },
            FileAssociation {
                extension: "pl".to_string(),
                interpreter: "perl".to_string(),
                editor: None,
            },
            FileAssociation {
                extension: "sh".to_string(),
                interpreter: "bash".to_string(),
                editor: None,
            },
        ]),
    };

    if let Ok(config_str) = fs::read_to_string(config_path) {
        toml::from_str(&config_str).unwrap_or(default_config)
    } else {
        default_config
    }
}

// Display an interactive message box with options to Run, Edit, or Exit
fn interactive_prompt(script: &Path, editor: &str) -> io::Result<UserChoice> {
    const ID_RUN: i32 = 1001;
    const ID_EDIT: i32 = 1002;
    const ID_CANCEL: i32 = 1003;

    // UTF-16 strings for buttons and dialog
    let run_text: Vec<u16> = "Run\0".encode_utf16().collect();
    let edit_text: Vec<u16> = "Edit Script\0".encode_utf16().collect();
    let cancel_text: Vec<u16> = "Cancel\0".encode_utf16().collect();
    let title: Vec<u16> = "Script Execution\0".encode_utf16().collect();
    let content: Vec<u16> = "Do you want to run the script?\0".encode_utf16().collect();

    let buttons = [
        TASKDIALOG_BUTTON {
            nButtonID: ID_RUN,
            pszButtonText: PCWSTR(run_text.as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: ID_EDIT,
            pszButtonText: PCWSTR(edit_text.as_ptr()),
        },
        TASKDIALOG_BUTTON {
            nButtonID: ID_CANCEL,
            pszButtonText: PCWSTR(cancel_text.as_ptr()),
        },
    ];

    let mut selected_button: i32 = 0;

    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        hwndParent: HWND(std::ptr::null_mut()),
        hInstance: HINSTANCE(std::ptr::null_mut()),
        pszWindowTitle: PCWSTR(title.as_ptr()),
        pszContent: PCWSTR(content.as_ptr()),
        cButtons: buttons.len() as u32,
        pButtons: buttons.as_ptr(),
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
        ..Default::default()
    };

    unsafe {
        // ComCtl32 v6 is required and is enabled via app.manifest
        TaskDialogIndirect(&config, Some(&mut selected_button), None, None)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
    }

    match selected_button {
        ID_RUN => Ok(UserChoice::Run),
        ID_EDIT => {
            let editor_path = which::which(editor).unwrap_or_else(|_| PathBuf::from("notepad")); // fallback

            if cfg!(debug_assertions) {
                log_debug(&format!(
                    "User chose to edit the script: {:?} with editor: {:?}",
                    script, editor_path
                ));
            }

            match Command::new(editor_path).arg(script).spawn() {
                Ok(mut child) => {
                    if let Err(e) = child.wait() {
                        log_debug(&format!("Editor wait() failed: {}", e));
                    }
                }
                Err(e) => {
                    log_debug(&format!("Editor spawn() failed: {}", e));
                }
            }

            Ok(UserChoice::Edit)
        }
        _ => Ok(UserChoice::Exit),
    }
}

// If debugging is enabled, log the message to a file
#[cfg(debug_assertions)]
fn log_debug(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug.log")
    {
        let _ = writeln!(file, "[DEBUG] {}", msg);
        let _ = file.flush();
    }
}

fn main() -> io::Result<()> {
    // Collect command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <script>", args[0]);
        return Ok(());
    }

    let script = Path::new(&args[1]);

    // Load configuration from config.toml or defaults
    let config_path = Path::new("config.toml");
    let config = load_config(config_path);

    // Load configured GUI shell list or default to ["explorer.exe"]
    let gui_shells = config
        .gui_shells
        .unwrap_or_else(|| vec!["explorer.exe".to_string()]);

    // Load file extension interpreter/editor associations
    let associations = config.file_associations.unwrap_or_default();

    // Try to detect interpreter from dispatcher line or file extension fallback
    if let Some((interpreter, extra_arg)) = read_dispatch_line_or_fallback(script, &associations) {
        // Apply dispatch_overrides if interpreter matches
        let mut final_interpreter = interpreter.clone();
        let mut final_args = extra_arg;

        if let Some(overrides) = &config.dispatch_overrides {
            if let Some(dispatch_override) = overrides
                .iter()
                .find(|o| o.interpreter.eq_ignore_ascii_case(&interpreter))
            {
                if let Some(path_override) = &dispatch_override.path_override {
                    final_interpreter = path_override.clone(); // override the executable
                }

                if let Some(args_override) = &dispatch_override.args_override {
                    final_args = Some(args_override.clone()); // override args completely
                }
            }
        }

        // Resolve interpreter to full path, using PATH or literal path
        let resolved_interpreter =
            which::which(&final_interpreter).unwrap_or_else(|_| PathBuf::from(&final_interpreter));

        let mut command = Command::new(resolved_interpreter);

        // Parse final_args and handle `$script` substitution
        if let Some(arg_string) = final_args {
            if arg_string.contains("$script") {
                for part in shell_words::split(&arg_string).unwrap_or_default() {
                    if part == "$script" {
                        command.arg(script);
                    } else {
                        command.arg(part);
                    }
                }
            } else {
                for part in shell_words::split(&arg_string).unwrap_or_default() {
                    command.arg(part);
                }
                command.arg(script);
            }
        } else {
            // No args specified, just pass the script as the sole argument
            command.arg(script);
        }

        command
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        // Determine if we were launched from a GUI (e.g., Explorer)
        if is_interactive_parent(&gui_shells) {
            // Resolve editor for this file type (or fallback to code/notepad)
            let editor = associations
                .iter()
                .find(|assoc| script.extension().and_then(|e| e.to_str()) == Some(&assoc.extension))
                .and_then(|assoc| assoc.editor.clone())
                .unwrap_or_else(|| {
                    if resolve_executable("code").is_some() {
                        "code".to_string()
                    } else {
                        "notepad".to_string()
                    }
                });

            // Show interactive dialog: Run, Edit, or Exit
            match interactive_prompt(script, &editor)? {
                UserChoice::Run => {
                    let mut child = command.spawn()?;
                    child.wait()?;

                    if cfg!(debug_assertions) {
                        log_debug(&format!("Script executed: {:?}", script));
                    }
                }
                UserChoice::Edit => {
                    // Already handled inside prompt
                }
                UserChoice::Exit => {
                    // Will have already exited
                }
            }
        } else {
            // Launched from terminal, run directly
            let mut child = command.spawn()?;
            child.wait()?;
        }
    } else {
        // Fallback to [default] or [default_large]
        let metadata = fs::metadata(script)?;
        let size_mb = metadata.len() / 1_048_576;

        let (fallback_util, fallback_args) = if let Some(default_large) = &config.default_large {
            if size_mb >= default_large.size_mb_threshold {
                (
                    default_large.util.clone(),
                    default_large.args.clone().unwrap_or("$script".to_string()),
                )
            } else if let Some(default) = &config.default {
                (
                    default.util.clone(),
                    default.args.clone().unwrap_or("$script".to_string()),
                )
            } else {
                ("notepad".to_string(), "$script".to_string())
            }
        } else if let Some(default) = &config.default {
            (
                default.util.clone(),
                default.args.clone().unwrap_or("$script".to_string()),
            )
        } else {
            ("notepad".to_string(), "$script".to_string())
        };

        // Resolve fallback executable
        let resolved_fallback =
            which::which(&fallback_util).unwrap_or_else(|_| PathBuf::from(&fallback_util));
        let mut fallback_cmd = Command::new(resolved_fallback);

        // Inject script
        if fallback_args.contains("$script") {
            for part in shell_words::split(&fallback_args).unwrap_or_default() {
                if part == "$script" {
                    fallback_cmd.arg(script);
                } else {
                    fallback_cmd.arg(part);
                }
            }
        } else {
            for part in shell_words::split(&fallback_args).unwrap_or_default() {
                fallback_cmd.arg(part);
            }
            fallback_cmd.arg(script);
        }

        fallback_cmd
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let mut child = fallback_cmd.spawn()?;
        child.wait()?;
    }

    Ok(())
}
