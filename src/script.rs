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
    // Get the script metadata
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
    let (shebang_interpreter, shebang_argument) = match get_interpreter(shebang_raw) {
        Some((interpreter, argument)) => (Some(interpreter), argument),
        None => (None, None),
    };

    // Find file association from config based on matching exec_runtime,
    // shebang_interpreter, or extension in that order.
    let assoc = shebang_interpreter.as_ref().and_then(|name| {
        associations.iter().find(|assoc| assoc.exec_runtime == *name)
    }).or_else(|| {
        shebang_interpreter.as_ref().and_then(|name| {
            associations
                .iter()
                .find(|assoc| assoc.shebang_interpreter.as_deref() == Some(name))
        })
    }).or_else(|| {
        extension.as_ref().and_then(|ext| {
            associations
                .iter()
                .find(|assoc| assoc.extension.as_deref() == Some(ext))
        })
    });

    let metadata = ScriptMetadata {
        shebang,
        shebang_exe: shebang_interpreter,
        shebang_arg: shebang_argument,
        extension,
        association: assoc.cloned(),
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
pub(crate) fn get_interpreter(shebang: &str) -> Option<(String, Option<String>)> {
    let mut parts = shebang.trim_start_matches("#!").trim().split_whitespace();

    let interpreter = parts.next()?;
    let arg = parts.next();

    if parts.next().is_some() {
        log_debug!("Error: Too many parts in interpreter");
        return None;
    }

    let path = Path::new(interpreter);

    if path.exists() {
        let name = path.file_name()?.to_string_lossy();

        if name == "env" {
            let arg_val = arg.map(|s| s.to_string());
            log_debug!(&format!("Found env interpreter: {:?}, arg: {:?}", interpreter, arg_val));
            return Some(("env".to_string(), arg_val));
        }

        log_debug!(&format!("Found interpreter: {:?}, arg: {:?}", name, arg));
        return Some((name.into_owned(), arg.map(|s| s.to_string())));
    }

    let basename = path.file_name()?.to_string_lossy().into_owned();

    if resolve_executable(&basename).is_some() {
        log_debug!(&format!("Found interpreter in PATH: {:?}, arg: {:?}", basename, arg));
        return Some((basename, arg.map(|s| s.to_string())));
    }

    if basename == "env" {
        if let Some(arg) = arg {
            if resolve_executable(arg).is_some() {
                log_debug!(&format!("Found env interpreter in PATH: {:?}, arg: {:?}", arg, arg));
                return Some((arg.to_string(), None));
            }
        }
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
}
