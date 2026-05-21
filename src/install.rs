#![cfg(target_os = "windows")]

use std::env;
use std::path::PathBuf;

use crate::log_debug;
use crate::registry::{Root, value_exists, write_string};

/// File-name component of current exe.
pub fn self_exe_basename() -> Option<String> {
    env::current_exe()
        .ok()
        .as_ref()
        .and_then(|p: &PathBuf| p.file_name())
        .map(|s| s.to_string_lossy().to_string())
}

/// `Applications\<exe-name>` ProgID. Used by verb dispatcher.
pub fn self_progid() -> Option<String> {
    self_exe_basename().map(|name| format!("Applications\\{}", name))
}

fn shell_key_root() -> Option<String> {
    self_progid().map(|p| format!("Software\\Classes\\{}\\shell", p))
}

struct VerbSpec {
    /// Verb subkey name (e.g. "edit", "runas"). Used verbatim under the
    /// shell\<verb> subkey.
    name: &'static str,
    /// Friendly context-menu label written to shell\<verb>\(Default).
    label: &'static str,
    /// Argument template appended after the exe path in the command.
    /// `%E` is replaced with the verb name here before writing.
    args_template: &'static str,
}

const VERBS: &[VerbSpec] = &[
    VerbSpec {
        name: "open",
        label: "Open",
        args_template: "\"%1\" %*",
    },
    VerbSpec {
        name: "edit",
        label: "Edit",
        args_template: "dispatch-verb --verb edit --file \"%1\" %*",
    },
    VerbSpec {
        name: "print",
        label: "Print",
        args_template: "dispatch-verb --verb print --file \"%1\" %*",
    },
    VerbSpec {
        name: "printto",
        label: "PrintTo",
        args_template: "dispatch-verb --verb printto --file \"%1\" \"%2\"",
    },
    VerbSpec {
        name: "runas",
        label: "Run as administrator",
        args_template: "dispatch-verb --verb runas --file \"%1\" %*",
    },
    VerbSpec {
        name: "UIAccess",
        label: "UIAccess",
        args_template: "dispatch-verb --verb UIAccess --file \"%1\" %*",
    },
];

/// Idempotently fill in any missing verb subkeys under
/// `HKCU\Software\Classes\Applications\<exe-name>\shell`.
pub fn ensure_verbs_registered() {
    write_verbs(false);
}

/// Unconditionally overwrite every verb subkey under
/// `HKCU\Software\Classes\Applications\<exe-name>\shell`.
pub fn reinstall_verbs() {
    write_verbs(true);
}

fn write_verbs(force: bool) {
    let exe = match env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            log_debug!(&format!("write_verbs: current_exe() failed: {}", e));
            return;
        }
    };
    let exe_str = exe.to_string_lossy().to_string();
    let shell_root = match shell_key_root() {
        Some(s) => s,
        None => {
            log_debug!(
                "write_verbs: could not determine self exe basename; skipping"
            );
            return;
        }
    };

    for spec in VERBS {
        let verb_key = format!("{}\\{}", shell_root, spec.name);
        let command_key = format!("{}\\command", verb_key);

        if force || !value_exists(Root::CurrentUser, &verb_key, "") {
            let ok = write_string(Root::CurrentUser, &verb_key, "", spec.label);
            log_debug!(&format!(
                "write_verbs: wrote label for {} -> {}",
                spec.name, ok
            ));
        } else {
            log_debug!(&format!(
                "write_verbs: label for {} already present, leaving alone",
                spec.name
            ));
        }

        if force || !value_exists(Root::CurrentUser, &command_key, "") {
            let command = format!("\"{}\" {}", exe_str, spec.args_template);
            let ok =
                write_string(Root::CurrentUser, &command_key, "", &command);
            log_debug!(&format!(
                "write_verbs: wrote command for {} -> {} ({})",
                spec.name, ok, command
            ));
        } else {
            log_debug!(&format!(
                "write_verbs: command for {} already present, leaving alone",
                spec.name
            ));
        }
    }
}
