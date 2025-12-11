use crate::config::FileAssociation;
use crate::log_debug;
use crate::platform::resolve_executable;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::{fs, io};

#[derive(Debug)]
pub struct ScriptMetadata {
    /// Shebang line minus the prefix
    pub shebang: Option<String>,
    /// Interpreter name
    pub shebang_exe: Option<String>,
    /// Optional argument to the interpreter
    pub shebang_arg: Option<String>,
    /// Extension of the file
    pub extension: Option<String>,
    /// File association from the config
    pub association: Option<FileAssociation>,
    /// File path
    pub file_path: PathBuf,
    /// File size in bytes
    pub file_size: u64,
}

/// Get the script metadata from the file.
///
/// # Arguments
///
/// * `script_path`:
/// * `associations`:
///
/// returns: ScriptMetadata
///
/// # Examples
///
/// ```
/// let script_path = "path/to/script.sh".to_string();
/// let metadata = get_script_metadata(&script_path, &associations);
/// ```
pub(crate) fn get_script_metadata(
    script_path: &String,
    associations: &[FileAssociation],
) -> ScriptMetadata {
    let script_pbuf = PathBuf::from(script_path);
    let file_size = fs::metadata(script_path)
        .map(|m| m.len())
        .unwrap_or_default();
    let shebang = read_shebang(&*script_pbuf);

    let extension = script_pbuf
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string());

    let shebang_raw = shebang.as_deref().unwrap_or("");
    let (shebang_interpreter, shebang_argument) =
        match get_interpreter(shebang_raw) {
            Some((interpreter, argument)) => (Some(interpreter), argument),
            None => (None, None),
        };

    // Own the association value instead of borrowing
    let mut assoc: Option<FileAssociation> = shebang_interpreter
        .as_ref()
        .and_then(|name| {
            associations
                .iter()
                .find(|assoc| assoc.exec_runtime == *name)
                .cloned()
        })
        .or_else(|| {
            shebang_interpreter.as_ref().and_then(|name| {
                associations
                    .iter()
                    .find(|assoc| {
                        assoc.shebang_interpreter.as_deref() == Some(name)
                    })
                    .cloned()
            })
        })
        .or_else(|| {
            extension.as_ref().and_then(|ext| {
                associations
                    .iter()
                    .find(|assoc| assoc.extension.as_deref() == Some(ext))
                    .cloned()
            })
        });

    if assoc.is_none() && shebang_interpreter.is_some() {
        log_debug!(
            "No association found for shebang interpreter, creating new association"
        );
        assoc = Some(FileAssociation {
            shebang_interpreter: shebang_interpreter.clone(),
            exec_runtime: shebang_interpreter.clone().unwrap_or_default(),
            exec_argv_override: None,
            view_runtime: None,
            extension: None,
            default_operation: None,
        });
    }

    let metadata = ScriptMetadata {
        shebang,
        shebang_exe: shebang_interpreter,
        shebang_arg: shebang_argument,
        extension,
        association: assoc,
        file_path: script_pbuf,
        file_size,
    };

    log_debug!(&format!("Script metadata: {:?}", metadata));
    metadata
}

/// Read the shebang line from a file.
///
/// # Arguments
///
/// * `path`:
///
/// returns: Option<String>
///
/// # Examples
///
/// ```
/// let path = Path::new("path/to/script.sh");
/// let shebang = read_shebang(&path);
/// ```
pub(crate) fn read_shebang(path: &Path) -> Option<String> {
    // Read the first line of the file to get the shebang
    let file = fs::File::open(path).ok()?;
    let mut reader = io::BufReader::new(file);
    let mut first_line = String::new();

    reader.read_line(&mut first_line).unwrap_or_default();

    let line = first_line.trim();

    log_debug!(&format!("Shebang line: {:?}", line));

    const ALLOWED_PREFIXES: [&str; 2] = ["#!", "//!"];
    let prefix = ALLOWED_PREFIXES.iter().find(|p| line.starts_with(*p))?;

    log_debug!(&format!("Found prefix: {:?}", prefix));

    let line = line.strip_prefix(prefix)?.trim();

    log_debug!(&format!("Shebang line after prefix: {:?}", line));

    if line.is_empty() {
        log_debug!("Error: Shebang line is empty after prefix");
        None // Empty shebang line
    } else {
        Some(line.to_string())
    }
}

/// Get the interpreter and its arguments from the shebang line.
/// Does not validate that the shebang line is a valid format,
/// only that it does not exceed the expected number of parts.
///
/// Supports `env -S` flag which allows multiple arguments to be passed.
///
/// # Arguments
///
/// * `shebang`: The shebang line to parse.
///
/// returns: Option<(String, Option<String>)>
///
/// # Examples
///
/// ```
/// let shebang_line = "#!/usr/bin/env python3";
/// let result = get_interpreter(shebang_line);
/// ```
pub(crate) fn get_interpreter(
    shebang: &str,
) -> Option<(String, Option<String>)> {
    let mut parts = shebang.trim_start_matches("#!").trim().split_whitespace();

    let interpreter = parts.next()?;
    let arg = parts.next();

    let path = Path::new(interpreter);
    let basename = path.file_name()?.to_string_lossy().into_owned();

    // Handle env -S flag (allows multiple arguments)
    if basename == "env" && arg == Some("-S") {
        let remaining: Vec<&str> = parts.collect();
        if remaining.is_empty() {
            log_debug!("Error: env -S requires an interpreter");
            return None;
        }

        let env_interpreter = remaining[0];
        let env_args = if remaining.len() > 1 {
            Some(remaining[1..].join(" "))
        } else {
            None
        };

        if resolve_executable(env_interpreter).is_some() {
            log_debug!(&format!(
                "Found env -S interpreter in PATH: {:?}, args: {:?}",
                env_interpreter, env_args
            ));
            return Some((env_interpreter.to_string(), env_args));
        }

        log_debug!(&format!(
            "Error: env -S interpreter not found in PATH: {:?}",
            env_interpreter
        ));
        return None;
    }

    // Handle env with interpreter name (e.g., #!/usr/bin/env node)
    if basename == "env" {
        if let Some(arg) = arg {
            // Check for extra arguments (not allowed without -S flag)
            if parts.next().is_some() {
                log_debug!("Error: Too many parts in env interpreter (use -S flag for multiple args)");
                return None;
            }

            if resolve_executable(arg).is_some() {
                log_debug!(&format!(
                    "Found env interpreter in PATH: {:?}",
                    arg
                ));
                return Some((arg.to_string(), None));
            }
        }
        // env without a valid interpreter argument
        return None;
    }

    if parts.next().is_some() {
        log_debug!("Error: Too many parts in interpreter");
        return None;
    }

    // If the interpreter is an absolute path, check if it exists (it probably won't)
    if path.exists() {
        let name = path.file_name()?.to_string_lossy();
        log_debug!(&format!("Found interpreter: {:?}, arg: {:?}", name, arg));
        return Some((name.into_owned(), arg.map(|s| s.to_string())));
    }

    if resolve_executable(&basename).is_some() {
        log_debug!(&format!(
            "Found interpreter in PATH: {:?}, arg: {:?}",
            basename, arg
        ));
        return Some((basename, arg.map(|s| s.to_string())));
    }

    log_debug!(&format!(
        "Error: Interpreter not found in PATH, returning basename: {:?} with arg: {:?}",
        basename, arg
    ));

    None
}

#[cfg(test)]
mod tests {
    use super::get_interpreter;

    #[test]
    fn test_valid_absolute_interpreter() {
        let line = "#!/usr/bin/python3";
        let result = get_interpreter(line);
        assert_eq!(result, Some(("python3".to_string(), None)));
    }

    #[test]
    fn test_env_interpreter() {
        let line = "#!/usr/bin/env node";
        let result = get_interpreter(line);
        assert_eq!(result, Some(("node".to_string(), None)));
    }

    #[test]
    fn test_env_spaced_interpreter() {
        let line = "#! /usr/bin/env node";
        let result = get_interpreter(line);
        assert_eq!(result, Some(("node".to_string(), None)));
    }

    #[test]
    fn test_invalid_prefix() {
        // We don't validate, so this should still return python3.
        let line = "//usr/bin/python3";
        let result = get_interpreter(line);
        assert_eq!(result, Some(("python3".to_string(), None)));
    }

    #[test]
    fn test_too_many_parts() {
        let line = "#!/usr/bin/env python3 extra";
        let result = get_interpreter(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_only_prefix() {
        let line = "#!";
        let result = get_interpreter(line);
        assert_eq!(result, None);
    }

    #[test]
    fn test_env_s_flag_with_single_arg() {
        let line = "#!/usr/bin/env -S node --experimental-modules";
        let result = get_interpreter(line);
        assert_eq!(
            result,
            Some(("node".to_string(), Some("--experimental-modules".to_string())))
        );
    }

    #[test]
    fn test_env_s_flag_with_multiple_args() {
        let line = "#!/usr/bin/env -S python3 -u -W ignore";
        let result = get_interpreter(line);
        assert_eq!(
            result,
            Some(("python3".to_string(), Some("-u -W ignore".to_string())))
        );
    }

    #[test]
    fn test_env_s_flag_no_args() {
        let line = "#!/usr/bin/env -S node";
        let result = get_interpreter(line);
        assert_eq!(result, Some(("node".to_string(), None)));
    }

    #[test]
    fn test_env_s_flag_missing_interpreter() {
        let line = "#!/usr/bin/env -S";
        let result = get_interpreter(line);
        assert_eq!(result, None);
    }
}
