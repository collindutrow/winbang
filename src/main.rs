mod cli;
mod config;
mod dispatch;
mod gui;
mod install;
mod logging;
mod platform;
mod registry;
mod script;
mod verb;

use crate::config::{Config, find_config_path, load_config};
use crate::dispatch::{
    build_command, handle_fallback_dispatch, handle_interactive_dispatch,
};
use crate::platform::is_interactive_parent;
use crate::script::get_script_metadata;
use clap::Parser;
use std::path::PathBuf;
use std::{env, io, process};

fn main() -> io::Result<()> {
    #[cfg(debug_assertions)]
    if let Ok(cwd) = env::current_dir() {
        log_debug!(&format!("Current working directory: {:?}", cwd));
    }

    let cli = cli::Cli::parse();

    install::ensure_verbs_registered();

    if cli.reinstall_verbs {
        install::reinstall_verbs();
        return Ok(());
    }

    match cli.command {
        Some(cli::Command::DispatchVerb { verb, file, extras }) => {
            let config = load_active_config();
            let outcome =
                verb::dispatch_verb(&verb, &file, &extras, &config);
            process::exit(outcome.exit_code());
        }
        Some(cli::Command::Script(argv)) => run_script(&argv),
        None => {
            eprintln!("Usage: winbang <script> [args...]");
            Ok(())
        }
    }
}

fn load_active_config() -> Config {
    let config_path =
        find_config_path().unwrap_or_else(|| PathBuf::from("config.toml"));
    load_config(&config_path)
}

fn run_script(argv: &[String]) -> io::Result<()> {
    let script_arg = &argv[0];
    let config = load_active_config();

    let script = get_script_metadata(
        script_arg,
        config.file_associations.as_deref().unwrap_or(&[]),
    );

    let extra_args: Option<Vec<String>> = if argv.len() > 1 {
        Some(argv[1..].to_vec())
    } else {
        None
    };

    log_debug!(&format!("Extra args passed to runtime: {:?}", extra_args));

    if script.association.is_some() {
        let mut command = build_command(&script, extra_args, &config);
        log_debug!("command = {:?}", command);

        if is_interactive_parent(&config.gui_shells.clone().unwrap_or_default())
        {
            log_debug!(&format!("Script executed (interactive): {:?}", script));
            handle_interactive_dispatch(&script, &mut command, &config)?;
        } else {
            log_debug!(&format!("Script executed: {:?}", script));
            command.spawn()?.wait()?;
        }
    } else {
        log_debug!(&format!(
            "No interpreter found for script: {:?}, using fallback handler",
            script
        ));

        handle_fallback_dispatch(&script, &config)?;
    }

    Ok(())
}
