use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "winbang",
    version,
    about = "Unix-like shebang support for Windows."
)]
pub struct Cli {
    /// Force-rewrite Winbang's shell verb registry entries and exit.
    #[arg(long)]
    pub reinstall_verbs: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// (Internal) Handle a non-Open shell verb dispatched from the registry.
    /// Not intended to be invoked directly by users.
    #[command(hide = true)]
    DispatchVerb {
        #[arg(long)]
        verb: String,
        #[arg(long)]
        file: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        extras: Vec<String>,
    },

    /// Any non-subcommand first argument is treated as a script path; trailing
    /// arguments are passed to the resolved interpreter.
    #[command(external_subcommand)]
    Script(Vec<String>),
}
