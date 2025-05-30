mod config;
mod dispatch;
mod gui;
mod logging;
mod platform;
mod script;

use crate::config::{find_config_path, load_config};
use crate::dispatch::{
    build_command, handle_fallback_dispatch, handle_interactive_dispatch,
};
use crate::platform::is_interactive_parent;
use crate::script::get_script_metadata;
use std::path::{Path, PathBuf};
use std::{env, io};

fn main() -> io::Result<()> {
    // Print the current working directory
    #[cfg(debug_assertions)]
    if let Ok(cwd) = env::current_dir() {
        log_debug!(&format!("Current working directory: {:?}", cwd));
    }
    
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <script>", args[0]);
        return Ok(());
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
