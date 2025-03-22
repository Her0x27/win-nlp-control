use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use notify::{RecommendedWatcher, DebouncedEvent, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::Duration;
use serde_json;
use log::{info, error, debug}; // Import logging macros

/// Application configuration structure.
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub aliases: Vec<AliasConfig>,
    pub language: String,
    pub notification_enable: bool,
    pub antiflood: bool,
    pub notification_delay: u32, // Задержка для уведомлений
}

/// Alias configuration definition.
#[derive(Debug, Deserialize, Clone)]
pub struct AliasConfig {
    pub alias: String,
    pub intent: String,
    pub parameters: Option<HashMap<String, String>>,
    pub command_type: Option<String>,
    pub steps: Option<Vec<AliasConfig>>,
}

impl AppConfig {
    /// Securely loads the configuration from a JSON file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let base_dir = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?;
        let base_dir = base_dir.canonicalize()
            .map_err(|e| format!("Failed to canonicalize current directory: {}", e))?;

        let config_path = path.as_ref().canonicalize()
            .map_err(|e| format!("Failed to canonicalize config file path '{}': {}", path.as_ref().display(), e))?;

        if !config_path.starts_with(&base_dir) {
            return Err(format!(
                "Path injection vulnerability detected: '{}' is not in '{}'",
                config_path.display(),
                base_dir.display()
            ));
        }

        if !config_path.exists() {
            return Err(format!("Configuration file '{}' does not exist", config_path.display()));
        }

        let metadata = fs::metadata(&config_path)
            .map_err(|e| format!("Failed to retrieve metadata for '{}': {}", config_path.display(), e))?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();

        if mode & 0o022 != 0 {
            return Err(format!(
                "Configuration file '{}' is writable by group or others (mode {:o}). Please secure the file.",
                config_path.display(), mode
            ));
        }

        let json_str = fs::read_to_string(&config_path)
            .map_err(|e| format!("Error reading config file '{}': {}", config_path.display(), e))?;

        serde_json::from_str(&json_str)
            .map_err(|e| format!("Error parsing config file '{}': {}", config_path.display(), e))
    }

    // Getters for config values
    pub fn get_language(&self) -> String {
        self.language.clone()
    }

    pub fn get_notification_delay(&self) -> u32 {
        self.notification_delay
    }

    pub fn get_notification_enable(&self) -> bool {
        self.notification_enable
    }

    pub fn get_antiflood(&self) -> bool {
        self.antiflood
    }
}

/// Shared configuration type used application-wide.
pub type SharedConfig = Arc<Mutex<Option<AppConfig>>>;

/// Initializes the shared configuration, loads settings, and sets up file watching.
pub fn init_shared_config<P: AsRef<Path>>(config_path: P, on_config_change: Option<Box<dyn Fn() + Send + Sync + 'static>>) -> SharedConfig {
    let initial_config = AppConfig::load_from_file(&config_path);
    let mut config = match initial_config {
        Ok(mut cfg) => {
             // provide default values if not present in file
            cfg.notification_enable = cfg.notification_enable;
            cfg.antiflood = cfg.antiflood;
            Some(cfg)
        },
        Err(e) => {
            error!("Failed to load initial config: {}, use default values", e);
             Some(AppConfig {
                aliases: Vec::new(),
                language: "en".to_string(),
                notification_enable: true, // default value
                antiflood: false, // default value
                notifications_delay: 500,
             })
        }
    };

    let shared_config: SharedConfig = Arc::new(Mutex::new(config));
    let shared_config_clone = Arc::clone(&shared_config);
    let config_path_str = config_path.as_ref().to_string_lossy().into_owned();
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = RecommendedWatcher::new(tx, Duration::from_secs(2))
        .expect("Failed to create file watcher");
    watcher.watch(Path::new(&config_path_str), RecursiveMode::NonRecursive)
        .expect("Failed to watch config file");

    std::thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(DebouncedEvent::Write(_)) | Ok(DebouncedEvent::Create(_)) => {
                    match AppConfig::load_from_file(&config_path_str) {
                        Ok(new_config) => {
                            let mut config_lock = shared_config_clone.lock().unwrap();
                            *config_lock = Some(new_config);
                            info!("[CONFIG] Secure configuration updated.");
                            if let Some(ref callback) = on_config_change {
                                callback();
                            }
                        },
                        Err(e) => {
                            error!("[CONFIG] Secure configuration update failed: {}", e);
                        }
                    }
                },
                Ok(_) => {},
                Err(e) => {
                    error!("[CONFIG] Watcher error: {}", e);
                    break;
                }
            }
        }
    });
    shared_config
}
