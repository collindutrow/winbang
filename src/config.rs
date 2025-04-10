use crate::gui::UserChoice;
use crate::log_debug;
use crate::platform::resolve_executable;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::{env, fs};

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub(crate) gui_shells: Option<Vec<String>>,
    pub(crate) default_operation: Option<DefaultOperation>,
    pub(crate) default: Option<DefaultHandler>,
    pub(crate) default_large: Option<DefaultLargeHandler>,
    pub(crate) file_associations: Option<Vec<FileAssociation>>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DefaultOperation {
    Prompt,
    Open,
    Execute,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DefaultHandler {
    pub(crate) view_runtime: String,
    pub(crate) args: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DefaultLargeHandler {
    pub(crate) size_mb_threshold: u64,
    pub(crate) view_runtime: String,
    pub(crate) args: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct FileAssociation {
    pub(crate) shebang_interpreter: Option<String>,
    pub(crate) exec_runtime: String,
    pub(crate) exec_argv_override: Option<String>,
    pub(crate) view_runtime: Option<String>,
    pub(crate) extension: Option<String>,
    pub(crate) default_operation: Option<DefaultOperation>,
}

/// Find the configuration file in the current directory, PROGRAMDATA, or APPDATA.
///
/// # Arguments
///
/// * None
///
/// returns: Result<(), Error>
///
/// # Examples
///
/// ```
/// let config_path = find_config_path().unwrap_or_else(|| PathBuf::from("config.toml"));
/// ```
pub(crate) fn find_config_path() -> Option<PathBuf> {
    let current = Path::new("config.toml").to_path_buf();
    let mut selected: Option<PathBuf> = None;

    if current.exists() {
        selected = Some(current.clone());

        log_debug!(&format!(
            "Found config in current directory: {:?}",
            current
        ));
    }

    if selected.is_none() {
        if let Ok(programdata) = env::var("PROGRAMDATA") {
            let pd_config =
                Path::new(&programdata).join("Winbang").join("config.toml");
            if pd_config.exists() {
                selected = Some(pd_config.clone());

                log_debug!(&format!(
                    "Found config in PROGRAMDATA: {:?}",
                    pd_config
                ));
            }
        }
    }

    // Regardless of earlier matches, APPDATA may override if explicitly allowed
    if let Ok(appdata) = env::var("APPDATA") {
        let ad_config = Path::new(&appdata).join("Winbang").join("config.toml");
        if ad_config.exists() {
            if let Ok(programdata) = env::var("PROGRAMDATA") {
                let pd_config =
                    Path::new(&programdata).join("Winbang").join("config.toml");
                if let Ok(cfg_str) = fs::read_to_string(&pd_config) {
                    if let Ok(cfg) = toml::from_str::<toml::Value>(&cfg_str) {
                        let allow_user = cfg
                            .get("allow_user_config")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        if allow_user {
                            log_debug!(&format!(
                                "Overriding with APPDATA config: {:?}",
                                ad_config
                            ));

                            return Some(ad_config);
                        } else if cfg!(debug_assertions) {
                            log_debug!(
                                "APPDATA config found but disallowed by PROGRAMDATA setting"
                            );
                        }
                    }
                }
            }
        }
    }

    selected
}

/// Load the configuration from a file.
///
/// # Arguments
///
/// * `config_path`: Path to the configuration file.
///
/// returns: Config
///
/// # Examples
///
/// ```
/// let config_path = Path::new("config.toml");
/// let config = load_config(config_path);
/// ```
pub(crate) fn load_config(config_path: &Path) -> Config {
    log_debug!(&format!("Loading config from: {:?}", config_path));
    let default_config = Config {
        gui_shells: Some(vec!["explorer.exe".to_string()]),
        default_operation: Some(DefaultOperation::Prompt),
        default: Some(DefaultHandler {
            view_runtime: "notepad".to_string(),
            args: Some("$script".to_string()),
        }),
        default_large: Some(DefaultLargeHandler {
            size_mb_threshold: 50,
            view_runtime: "notepad".to_string(),
            args: Some("$script".to_string()),
        }),
        file_associations: Some(vec![
            FileAssociation {
                shebang_interpreter: Option::from("ruby".to_string()),
                exec_runtime: "ruby".to_string(),
                extension: Option::from("rb".to_string()),
                view_runtime: None,
                default_operation: Option::from(DefaultOperation::Prompt),
                exec_argv_override: None,
            },
            FileAssociation {
                shebang_interpreter: Option::from("python".to_string()),
                exec_runtime: "python".to_string(),
                extension: Option::from("py".to_string()),
                view_runtime: None,
                default_operation: Option::from(DefaultOperation::Prompt),
                exec_argv_override: None,
            },
            FileAssociation {
                shebang_interpreter: if resolve_executable("deno").is_some() {
                    Some("deno".to_string())
                } else if resolve_executable("bun").is_some() {
                    Some("bun".to_string())
                } else {
                    Some("node".to_string())
                },
                exec_runtime: if resolve_executable("deno").is_some() {
                    "deno".to_string()
                } else if resolve_executable("bun").is_some() {
                    "bun".to_string()
                } else {
                    "node".to_string()
                },
                extension: Option::from("js".to_string()),
                view_runtime: None,
                default_operation: Option::from(DefaultOperation::Prompt),
                exec_argv_override: None,
            },
            FileAssociation {
                shebang_interpreter: if resolve_executable("deno").is_some() {
                    Some("deno".to_string())
                } else {
                    Some("ts-node".to_string())
                },
                exec_runtime: if resolve_executable("deno").is_some() {
                    "deno".to_string()
                } else {
                    "ts-node".to_string()
                },
                extension: Option::from("ts".to_string()),
                view_runtime: None,
                default_operation: Option::from(DefaultOperation::Prompt),
                exec_argv_override: None,
            },
            FileAssociation {
                shebang_interpreter: Option::from("perl".to_string()),
                exec_runtime: "perl".to_string(),
                extension: Option::from("pl".to_string()),
                view_runtime: None,
                default_operation: Option::from(DefaultOperation::Prompt),
                exec_argv_override: None,
            },
            FileAssociation {
                shebang_interpreter: Option::from("bash".to_string()),
                exec_runtime: "bash".to_string(),
                extension: Option::from("sh".to_string()),
                view_runtime: None,
                default_operation: Option::from(DefaultOperation::Prompt),
                exec_argv_override: None,
            },
        ]),
    };

    if let Ok(config_str) = fs::read_to_string(config_path) {
        toml::from_str(&config_str).unwrap_or(default_config)
    } else {
        log_debug!(&format!(
            "Error: Failed to read config file: {:?}",
            config_path
        ));
        default_config
    }
}
