#![cfg(target_os = "windows")]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::{Config, FileAssociation};
use crate::install::self_progid;
use crate::log_debug;
use crate::registry::{Root, read_string};

/// Result of dispatching a verb. The numeric value is propagated as the
/// process exit code so callers (e.g. AHK's ShellExecuteEx fallback chain)
/// can react meaningfully.
pub enum DispatchOutcome {
    /// The verb's command was resolved and the child process ran. Carries the
    /// child's exit code.
    Ran(i32),
    /// The verb could not be resolved; the caller should fall back to its own
    /// default. Carries a suggested exit code (always non-zero).
    Unresolved,
}

impl DispatchOutcome {
    pub fn exit_code(&self) -> i32 {
        match self {
            DispatchOutcome::Ran(code) => *code,
            DispatchOutcome::Unresolved => 1,
        }
    }
}

/// Dispatch a non-Open shell verb by resolving the underlying ProgID's
/// command (or a config override) and spawning it.
pub fn dispatch_verb(
    verb: &str,
    file_path: &Path,
    extra_args: &[String],
    config: &Config,
) -> DispatchOutcome {
    log_debug!(&format!(
        "dispatch_verb: verb={:?}, file={:?}, extra={:?}",
        verb, file_path, extra_args
    ));

    let extension = file_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());

    // Step 2: config override per-(extension, verb).
    if let Some(ref ext) = extension {
        if let Some(assoc) = find_association(config, ext) {
            if let Some(template) = override_for_verb(&assoc, verb) {
                log_debug!(&format!(
                    "dispatch_verb: using config override for .{}/{} -> {}",
                    ext, verb, template
                ));
                return spawn_template(&template, file_path, extra_args);
            }
        }
    }

    // Step 3: resolve the underlying ProgID.
    let Some(ext) = extension else {
        log_debug!("dispatch_verb: no extension on file path, cannot resolve");
        return DispatchOutcome::Unresolved;
    };

    let progid = match resolve_underlying_progid(&ext) {
        Some(p) => p,
        None => {
            log_debug!(&format!(
                "dispatch_verb: no underlying ProgID for .{}",
                ext
            ));
            return DispatchOutcome::Unresolved;
        }
    };

    if let Some(self_id) = self_progid() {
        if progid.eq_ignore_ascii_case(&self_id) {
            log_debug!(&format!(
                "dispatch_verb: underlying ProgID for .{} is Winbang itself \
                ({}); refusing to self-loop. Use UserChoice to set Winbang \
                as the handler instead of overwriting HKCR\\.{}",
                ext, progid, ext
            ));
            return DispatchOutcome::Unresolved;
        }
    } else {
        log_debug!(
            "dispatch_verb: could not determine self ProgID; skipping self-loop guard"
        );
    }

    // Step 4: read the verb's command, with open-verb fallback.
    let template = match read_verb_command(&progid, verb) {
        Some(t) => t,
        None => {
            log_debug!(&format!(
                "dispatch_verb: no shell\\{}\\command on {}; trying open",
                verb, progid
            ));
            match read_verb_command(&progid, "open") {
                Some(t) => t,
                None => {
                    log_debug!(&format!(
                        "dispatch_verb: no open-verb fallback on {} either",
                        progid
                    ));
                    return DispatchOutcome::Unresolved;
                }
            }
        }
    };

    spawn_template(&template, file_path, extra_args)
}

fn find_association<'a>(
    config: &'a Config,
    extension: &str,
) -> Option<&'a FileAssociation> {
    config
        .file_associations
        .as_deref()?
        .iter()
        .find(|a| a.extension.as_deref().map(|e| e.eq_ignore_ascii_case(extension)).unwrap_or(false))
}

fn override_for_verb(assoc: &FileAssociation, verb: &str) -> Option<String> {
    match verb.to_ascii_lowercase().as_str() {
        "edit" => assoc.verb_edit.clone(),
        "print" => assoc.verb_print.clone(),
        "printto" => assoc.verb_printto.clone(),
        "runas" => assoc.verb_runas.clone(),
        "uiaccess" => assoc.verb_uiaccess.clone(),
        _ => None,
    }
}

fn resolve_underlying_progid(ext_lower: &str) -> Option<String> {
    let ext_key = format!(".{}", ext_lower);
    let default = read_string(Root::ClassesRoot, &ext_key, "");
    if let Some(v) = default.as_deref() {
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    // OpenWithProgids fallback: take any one entry.
    let openwith_key = format!(".{}\\OpenWithProgids", ext_lower);
    crate::registry::first_value_name(Root::ClassesRoot, &openwith_key)
}

fn read_verb_command(progid: &str, verb: &str) -> Option<String> {
    let key = format!("{}\\shell\\{}\\command", progid, verb);
    let val = read_string(Root::ClassesRoot, &key, "")?;
    if val.is_empty() { None } else { Some(val) }
}

/// Expand Windows %-tokens and spawn the resulting argv.
fn spawn_template(
    template: &str,
    file_path: &Path,
    extra_args: &[String],
) -> DispatchOutcome {
    let expanded = expand_tokens(template, file_path, extra_args);
    log_debug!(&format!("spawn_template: expanded={:?}", expanded));

    let argv = match shell_words::split(&expanded) {
        Ok(v) => v,
        Err(e) => {
            log_debug!(&format!(
                "spawn_template: shell_words::split failed: {}",
                e
            ));
            return DispatchOutcome::Unresolved;
        }
    };

    if argv.is_empty() {
        log_debug!("spawn_template: empty argv after split");
        return DispatchOutcome::Unresolved;
    }

    let exe = PathBuf::from(&argv[0]);
    let mut cmd = Command::new(&exe);
    cmd.args(&argv[1..]);
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    match cmd.spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status) => DispatchOutcome::Ran(status.code().unwrap_or(0)),
            Err(e) => {
                log_debug!(&format!("spawn_template: wait() failed: {}", e));
                DispatchOutcome::Unresolved
            }
        },
        Err(e) => {
            log_debug!(&format!(
                "spawn_template: spawn failed for {:?}: {}",
                exe, e
            ));
            DispatchOutcome::Unresolved
        }
    }
}

/// Expand Windows shell %-tokens in a command template.
///
/// Handles `%1`, `%L`, `%V` → file path; `%2` → first extra arg; `%*` →
/// space-joined extra args. Tokens are quoted when needed. Unrecognized
/// `%` sequences are left as-is, matching Windows' tolerant behavior.
fn expand_tokens(
    template: &str,
    file_path: &Path,
    extra_args: &[String],
) -> String {
    let file_str = file_path.to_string_lossy().to_string();
    let extra_joined =
        extra_args.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
    let extra_first = extra_args.first().map(|s| s.as_str()).unwrap_or("");

    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            Some('1') | Some('L') | Some('V') => {
                chars.next();
                out.push_str(&file_str);
            }
            Some('2') => {
                chars.next();
                out.push_str(extra_first);
            }
            Some('*') => {
                chars.next();
                out.push_str(&extra_joined);
            }
            Some('%') => {
                // %% → literal %
                chars.next();
                out.push('%');
            }
            _ => {
                out.push('%');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_percent_one() {
        let out = expand_tokens(
            "\"%1\" /flag",
            Path::new("C:\\foo bar\\baz.ahk"),
            &[],
        );
        assert_eq!(out, "\"C:\\foo bar\\baz.ahk\" /flag");
    }

    #[test]
    fn expand_percent_star() {
        let out = expand_tokens(
            "\"%1\" %*",
            Path::new("C:\\x.ahk"),
            &["a".to_string(), "b".to_string()],
        );
        assert_eq!(out, "\"C:\\x.ahk\" a b");
    }

    #[test]
    fn expand_printto_percent_two() {
        let out = expand_tokens(
            "\"%1\" \"%2\"",
            Path::new("C:\\x.doc"),
            &["My Printer".to_string()],
        );
        assert_eq!(out, "\"C:\\x.doc\" \"My Printer\"");
    }

    #[test]
    fn expand_double_percent_literal() {
        let out = expand_tokens("100%% done %1", Path::new("a"), &[]);
        assert_eq!(out, "100% done a");
    }

    #[test]
    fn expand_unrecognized_passthrough() {
        let out = expand_tokens("%Z stays", Path::new("a"), &[]);
        assert_eq!(out, "%Z stays");
    }
}
