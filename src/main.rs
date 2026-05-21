mod config;
mod dispatch;
mod gui;
mod install;
mod logging;
mod platform;
mod registry;
mod script;
mod verb;

use crate::config::{find_config_path, load_config};
use crate::dispatch::{
    build_command, handle_fallback_dispatch, handle_interactive_dispatch,
};
use crate::platform::is_interactive_parent;
use crate::script::get_script_metadata;
use std::path::{Path, PathBuf};
use std::{env, io, process};

fn main() -> io::Result<()> {
    // Print the current working directory
    #[cfg(debug_assertions)]
    if let Ok(cwd) = env::current_dir() {
        log_debug!(&format!("Current working directory: {:?}", cwd));
    }

    // Ensure Winbang owns every common verb under its own ProgID.
    // Fill-in-the-blanks: never overwrites pre-existing values.
    install::ensure_verbs_registered();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <script>", args[0]);
        return Ok(());
    }

    // dispatch-verb subcommand: invoked from Winbang's own registered
    // shell\<verb>\command keys when Windows routes a non-Open verb to us.
    if args[1] == "dispatch-verb" {
        let code = run_dispatch_verb(&args[2..]);
        process::exit(code);
    }

    let config_path =
        find_config_path().unwrap_or_else(|| PathBuf::from("config.toml"));
    let config = load_config(&config_path);

    let script = get_script_metadata(
        &args[1],
        config.file_associations.as_deref().unwrap_or(&[]),
    );

    let extra_args: Option<Vec<String>> = if args.len() > 2 {
        Some(args[2..].to_vec())
    } else {
        None
    };

    log_debug!(&format!("Extra args passed to runtime: {:?}", extra_args));

    if script.association.is_some() {
        let mut command = build_command(&script, extra_args, &config);
        log_debug!("command = {:?}", command);

        // Check if the parent process is a recognized GUI shell
        if is_interactive_parent(&config.gui_shells.clone().unwrap_or_default())
        {
            log_debug!(&format!("Script executed (interactive): {:?}", script));
            handle_interactive_dispatch(&script, &mut command, &config)?;
        } else {
            log_debug!(&format!("Script executed: {:?}", script));
            command.spawn()?.wait()?;
        }
    } else {
        // No interpreter found, fallback to default handler
        log_debug!(&format!(
            "No interpreter found for script: {:?}, using fallback handler",
            script
        ));

        handle_fallback_dispatch(&script, &config)?;
    }

    Ok(())
}

/// Parse and run a `dispatch-verb` invocation. Returns the process exit code.
///
/// Usage shape:
///   <exe-name> dispatch-verb --verb <name> --file <path> [extra-args...]
///
/// Anything after the recognized flag pairs is treated as extra args to pass
/// to the underlying command (substituted into `%*` / `%2`).
fn run_dispatch_verb(args: &[String]) -> i32 {
    let mut verb: Option<String> = None;
    let mut file: Option<String> = None;
    let mut extras: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--verb" => {
                if i + 1 >= args.len() {
                    eprintln!("--verb requires a value");
                    return 2;
                }
                verb = Some(args[i + 1].clone());
                i += 2;
            }
            "--file" => {
                if i + 1 >= args.len() {
                    eprintln!("--file requires a value");
                    return 2;
                }
                file = Some(args[i + 1].clone());
                i += 2;
            }
            "--" => {
                extras.extend_from_slice(&args[i + 1..]);
                break;
            }
            other => {
                extras.push(other.to_string());
                i += 1;
            }
        }
    }

    let Some(verb) = verb else {
        eprintln!("dispatch-verb: --verb is required");
        return 2;
    };
    let Some(file) = file else {
        eprintln!("dispatch-verb: --file is required");
        return 2;
    };

    let config_path =
        find_config_path().unwrap_or_else(|| PathBuf::from("config.toml"));
    let config = load_config(&config_path);

    let outcome =
        verb::dispatch_verb(&verb, Path::new(&file), &extras, &config);
    outcome.exit_code()
}
