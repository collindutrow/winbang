use crate::config::{Config, DefaultOperation};
use crate::gui::{interactive_prompt, UserChoice};
use crate::log_debug;
use crate::platform::resolve_executable;
use crate::script::ScriptMetadata;
use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, io};

/// Build a command to execute the script.
///
/// Constructs a command to execute the script using the specified interpreter.
///
/// # Arguments
///
/// * `script`: ScriptMetadata containing the script details.
/// * `extra_args`: Optional additional arguments to pass to the command.
/// * `config`: Configuration object containing file associations and defaults.
///
/// returns: Command
///
/// # Examples
///
/// ```
/// let interpreter = "python";
/// ```
pub(crate) fn build_command(
    script: &ScriptMetadata,
    extra_args: Option<Vec<String>>,
    config: &Config,
) -> Command {
    log_debug!("build_command({:?}, {:?})", script, &config);

    let mut command =
        Command::new(&script.association.as_ref().unwrap().exec_runtime);

    // If exec_argv_override was found, use it.
    if let Some(arg_string) =
        &script.association.as_ref().unwrap().exec_argv_override
    {
        let mut vars = HashMap::new();
        let file_path = script.file_path.to_str().unwrap();

        vars.insert("script", file_path.replace("\\", "\\\\"));
        vars.insert("script_unix", file_path.replace("\\", "/"));

        expand_and_push_args(&mut command, arg_string, &vars, extra_args.as_ref());
    } else {
        // No override found, use the default behavior and optional argument
        log_debug!("No exec argv override found, using default behavior");

        // If a shebang interpreter argument is specified, use it
        // Split on whitespace to handle env -S flag with multiple args
        if let Some(arg) = &script.shebang_arg {
            // Use shell_words to properly handle quoted strings
            for part in shell_words::split(arg).unwrap_or_else(|_| vec![arg.clone()]) {
                command.arg(part);
            }
        }

        // Append the script file path
        command.arg(&script.file_path);

        // Append extra arguments if provided
        if let Some(extra_args) = extra_args {
            for arg in extra_args {
                command.arg(arg);
            }
        }
    }

    // Set command's standard input/output/error to inherit from parent
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command
}

/// Handle interactive dispatch for script execution.
/// This function is called when the parent process is a GUI shell.
///
/// # Arguments
///
/// * `script`: Path to the script.
/// * `command`: Command object to execute the script.
/// * `config`: Configuration object.
///
/// returns: Result<(), Error>
///
/// # Examples
///
/// ```
/// let script_path = Path::new("example_script.sh");
/// let mut command = Command::new("bash");
/// handle_interactive_dispatch(script_path, &mut command, &config)?;
/// ```
pub(crate) fn handle_interactive_dispatch(
    script: &ScriptMetadata,
    command: &mut Command,
    config: &Config,
) -> io::Result<()> {
    log_debug!("Interactive dispatch for script: {:?}", script);
    let editor = resolve_view_runtime(script, config);
    let operation = resolve_operation(script, config);

    log_debug!("Editor resolved: {:?}", editor);
    log_debug!("Operation resolved: {:?}", operation);

    match operation {
        DefaultOperation::Prompt => {
            match interactive_prompt(script, &editor)? {
                UserChoice::Run => {
                    let mut child = command.spawn()?;
                    child.wait()?;
                    log_debug!(&format!("Script executed: {:?}", script));
                }
                UserChoice::Edit => { /* already handled */ }
                UserChoice::Exit => { /* do nothing */ }
            }
        }
        DefaultOperation::Execute => {
            let mut child = command.spawn()?;
            child.wait()?;
            log_debug!(&format!("Script auto-executed: {:?}", script));
        }
        DefaultOperation::Open => {
            let editor_path = which::which(&editor)
                .unwrap_or_else(|_| PathBuf::from("notepad"));
            Command::new(editor_path)
                .arg::<&PathBuf>(&script.file_path)
                .spawn()?
                .wait()?;
            log_debug!(&format!(
                "Script opened in editor: {:?} -> {:?}",
                editor, script
            ));
        }
    }

    Ok(())
}

/// Handle dispatch when no interpreter is found.
///
/// # Arguments
///
/// * `script`: Path to the script.
/// * `config`: Configuration object.
///
/// returns: Result<(), Error>
///
/// # Examples
///
/// ```
/// let script_path = Path::new("example_script.sh");
/// handle_fallback_dispatch(script_path, &config)?;
/// ```
pub(crate) fn handle_fallback_dispatch(
    script: &ScriptMetadata,
    config: &Config,
) -> io::Result<()> {
    let metadata = fs::metadata(&script.file_path)?;
    let size_mb = metadata.len() / 1_048_576;

    let (fallback_util, fallback_args) =
        if let Some(default_large) = &config.default_large {
            if size_mb >= default_large.size_mb_threshold {
                (
                    &default_large.view_runtime,
                    default_large.args.as_deref().unwrap_or("$script"),
                )
            } else if let Some(default) = &config.default {
                (
                    &default.view_runtime,
                    default.args.as_deref().unwrap_or("$script"),
                )
            } else {
                (&"notepad".to_string(), "$script")
            }
        } else if let Some(default) = &config.default {
            (
                &default.view_runtime,
                default.args.as_deref().unwrap_or("$script"),
            )
        } else {
            (&"notepad".to_string(), "$script")
        };

    let resolved = which::which(fallback_util)
        .unwrap_or_else(|_| PathBuf::from(fallback_util));
    let mut fallback_cmd = Command::new(resolved);

    if fallback_args.contains("$script") {
        for part in shell_words::split(fallback_args).unwrap_or_default() {
            if part == "$script" {
                fallback_cmd.arg(&script.file_path);
            } else {
                fallback_cmd.arg(part);
            }
        }
    } else {
        for part in shell_words::split(fallback_args).unwrap_or_default() {
            fallback_cmd.arg(part);
        }
        fallback_cmd.arg(&script.file_path);
    }

    fallback_cmd
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = fallback_cmd.spawn()?;
    child.wait()?;

    Ok(())
}

/// Resolve the view runtime for the script.
///
/// # Arguments
///
/// * `script`:
/// * `config`:
///
/// returns: String
///
/// # Examples
///
/// ```
/// let runtime = resolve_view_runtime(&script, &config);
/// ```
fn resolve_view_runtime(script: &ScriptMetadata, config: &Config) -> String {
    // Priority order: shebang interpreter > file extension > default
    if let Some(runtime) = script
        .association
        .as_ref()
        .and_then(|a| a.view_runtime.clone())
    {
        return runtime;
    }

    if let Some(default_large) = &config.default_large {
        if script.file_size / 1_048_576 >= default_large.size_mb_threshold {
            log_debug!(&format!(
                "File size exceeds threshold: {} MB",
                script.file_size / 1_048_576
            ));

            return default_large.view_runtime.clone();
        } else {
            log_debug!(&format!(
                "File size is within threshold: {} MB",
                script.file_size / 1_048_576
            ));
        }
    }

    // Check if config.default.view_runtime is set
    if let Some(default) = &config.default {
        return default.view_runtime.clone();
    }

    // Hardcoded fallback to "code" or "notepad"
    resolve_executable("code")
        .map(|_| "code".to_string())
        .unwrap_or_else(|| "notepad".to_string())
}

/// Resolve the default operation for the script.
///
/// # Arguments
///
/// * `script`:
/// * `config`:
///
/// returns: DefaultOperation
///
/// # Examples
///
/// ```
/// let operation = resolve_operation(&script, &config);
/// ```
fn resolve_operation(
    script: &ScriptMetadata,
    config: &Config,
) -> DefaultOperation {
    if let Some(op) = script
        .association
        .as_ref()
        .and_then(|a| a.default_operation.clone())
    {
        return op;
    }

    if let Some(op) = config.default_operation {
        return op;
    }

    DefaultOperation::Prompt
}

/// Expand variable strings inside command arguments and push them to the command.
/// Modifies the command object directly.
///
/// # Arguments
///
/// * `command`: Command object to modify.
/// * `arg_string`: String containing arguments with placeholders.
/// * `vars`: HashMap of variables to expand.
///
/// returns: ()
///
/// # Examples
///
/// ```
/// let mut command = Command::new("python");
/// let arg_string = "arg1 @{{script}} arg2";
/// let vars = HashMap::new();
/// vars.insert("script", "example.py".to_string());
/// expand_and_push_args(&mut command, arg_string, &vars);
/// ```
fn expand_and_push_args(
    command: &mut Command,
    arg_str: &str,
    vars: &HashMap<&str, String>,
    passed_args: Option<&Vec<String>>,
) {
    log_debug!(&format!("Expanding arguments with vars: {:?}", vars));

    // Split the argument string into parts and expand each part
    for part in shell_words::split(arg_str).unwrap_or_default() {
        log_debug!(&format!("Expanding part: '{}'", part));

        // Special handling for @{passed_args} - expand to multiple separate args
        if part == "@{passed_args}" {
            if let Some(args) = passed_args {
                for arg in args {
                    log_debug!(&format!("Adding passed argument: '{}'", arg));
                    command.arg(arg);
                }
            }
            // If no passed_args, don't add anything (no empty args)
            continue;
        }

        let expanded = expand_placeholders(&part, vars);

        // Skip empty arguments after expansion to avoid passing spurious empty args
        if expanded.is_empty() {
            log_debug!("Skipping empty expanded argument");
            continue;
        }

        // Push the expanded argument directly without re-splitting.
        // The initial shell_words::split already handled quoting,
        // so re-splitting would break paths with spaces.
        log_debug!(&format!("Expanded argument: '{}'", expanded));
        command.arg(expanded);
    }
}

/// Expand placeholders in a string using a HashMap.
///
/// # Arguments
///
/// * `s`: String containing placeholders.
/// * `vars`: HashMap of variables to expand.
///
/// returns: String
///
/// # Examples
///
/// ```
/// let s = "Hello @{{name}}!";
/// let mut vars = HashMap::new();
/// vars.insert("name", "World".to_string());
/// let result = expand_placeholders(s, &vars);
/// ```
fn expand_placeholders(s: &str, vars: &HashMap<&str, String>) -> String {
    let mut result = s.to_owned();
    for (key, val) in vars {
        let placeholder = format!("@{{{}}}", key);
        result = result.replace(&placeholder, val);
    }
    result
}
