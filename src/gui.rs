use crate::log_debug;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TDF_ALLOW_DIALOG_CANCELLATION, TaskDialogIndirect,
};
use windows::core::PCWSTR;
use crate::script::ScriptMetadata;

pub(crate) enum UserChoice {
    Run,
    Edit,
    Exit,
}

/// Prompt the user for action using a Windows Task Dialog.
///
/// # Arguments
///
/// * `script`: Path to the script.
/// * `editor`: Path to the editor.
///
/// returns: Result<UserChoice, Error>
///
/// # Examples
///
/// ```
/// let script_path = Path::new("example_script.sh");
/// let editor = "notepad";
/// let user_choice = interactive_prompt(script_path, editor)?;
/// ```
pub(crate) fn interactive_prompt(script: &ScriptMetadata, editor: &str) -> io::Result<UserChoice> {
    const ID_RUN: i32 = 1001;
    const ID_EDIT: i32 = 1002;
    const ID_CANCEL: i32 = 1003;

    // UTF-16 strings for buttons and dialog
    let run_text: Vec<u16> = "Run\0".encode_utf16().collect();
    let edit_text: Vec<u16> = "Open\0".encode_utf16().collect();
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
        nDefaultButton: ID_CANCEL,
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

            log_debug!(&format!(
                "User chose to edit the script: {:?} with editor: {:?}",
                script, editor_path
            ));

            match Command::new(editor_path).arg::<&PathBuf>(&script.file_path).spawn() {
                Ok(mut child) => {
                    if let Err(e) = child.wait() {
                        log_debug!(&format!("Editor wait() failed: {}", e));
                    }
                }
                Err(e) => {
                    log_debug!(&format!("Editor spawn() failed: {}", e));
                }
            }

            Ok(UserChoice::Edit)
        }
        _ => Ok(UserChoice::Exit),
    }
}
