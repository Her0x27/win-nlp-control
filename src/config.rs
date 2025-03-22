use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use notify::{RecommendedWatcher, DebouncedEvent, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

/// Application configuration structure that integrates alias definitions and language settings.
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub aliases: Vec<AliasConfig>,
    pub language: String,
}

/// Alias configuration definition.
#[derive(Debug, Deserialize, Clone)]
pub struct AliasConfig {
    pub alias: String,
    pub intent: String,
    pub parameters: Option<HashMap<String, String>>,
    /// Command type: "single" or "multi".
    pub command_type: Option<String>,
    pub steps: Option<Vec<AliasConfig>>,
}

impl AppConfig {
    /// Securely loads the configuration from a JSON file.
    /// This method checks file permissions and ensures that the configuration file is not world-writable.
    /// It also uses canonical paths and restricts file access to a trusted base directory.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        // Define the trusted base directory (for this example, the current working directory)
        let base_dir = std::env::current_dir()
            .map_err(|e| format!("Failed to get current directory: {}", e))?
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize current directory: {}", e))?;
        
        // Canonicalize the provided config path
        let config_path = path.as_ref().canonicalize()
            .map_err(|e| format!("Failed to canonicalize config file path '{}': {}", path.as_ref().display(), e))?;
        
        // Ensure the config_path is within the trusted base directory to avoid path injection
        if !config_path.starts_with(&base_dir) {
            return Err(format!(
                "Path injection vulnerability detected: '{}' is not in '{}'",
                config_path.display(),
                base_dir.display()
            ));
        }
        
        // Check file existence and permissions.
        if !config_path.exists() {
            return Err(format!("Configuration file '{}' does not exist", config_path.display()));
        }
        let metadata = fs::metadata(&config_path)
            .map_err(|e| format!("Failed to retrieve metadata for '{}': {}", config_path.display(), e))?;
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        // Mode 0o644 is typical for secure files; warn if file is writable by group or others.
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
}

/// Reads the language setting from a plain-text configuration file (config.conf).
/// The file should contain a line such as "language=ru". This method is designed with careful file I/O.
pub fn load_language() -> String {
    let conf_path = Path::new("config.conf").canonicalize();
    if let Ok(conf_path) = conf_path {
        if conf_path.exists() {
            if let Ok(file) = File::open(&conf_path) {
                let reader = BufReader::new(file);
                for line in reader.lines() {
                    if let Ok(l) = line {
                        let trimmed = l.trim();
                        if trimmed.starts_with("language=") {
                            let parts: Vec<&str> = trimmed.split('=').collect();
                            if parts.len() == 2 {
                                return parts[1].trim().to_string();
                            }
                        }
                    }
                }
            }
        }
    }
    "ru".to_string()
}

/// Shared configuration type used application-wide.
pub type SharedConfig = Arc<Mutex<Option<AppConfig>>>;

/// Initializes the shared configuration by loading the configuration file and language setting.
/// This function also starts a secure file watcher to automatically update the configuration 
/// when changes are detected. All file changes are validated for integrity.
pub fn init_shared_config<P: AsRef<Path>>(config_path: P) -> SharedConfig {
    let mut config = AppConfig::load_from_file(&config_path).ok();
    // Update configuration with language from config.conf.
    let language = load_language();
    if let Some(ref mut cfg) = config {
        cfg.language = language;
    }
    let shared_config: SharedConfig = Arc::new(Mutex::new(config));

    let shared_config_clone = Arc::clone(&shared_config);
    let config_path_str = config_path.as_ref().to_string_lossy().into_owned();
    let (tx, rx) = channel();

    // Create the watcher with a secure timeout.
    let mut watcher: RecommendedWatcher = RecommendedWatcher::new(tx, Duration::from_secs(2))
        .expect("Failed to create file watcher");
    watcher.watch(Path::new(&config_path_str), RecursiveMode::NonRecursive)
        .expect("Failed to watch config file");

    std::thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(DebouncedEvent::Write(_)) | Ok(DebouncedEvent::Create(_)) => {
                    // Reload configuration securely.
                    match AppConfig::load_from_file(&config_path_str) {
                        Ok(new_config) => {
                            let mut config_lock = shared_config_clone.lock().unwrap();
                            *config_lock = Some(new_config);
                            println!("[CONFIG] Secure configuration updated.");
                        },
                        Err(e) => {
                            eprintln!("[CONFIG] Secure configuration update failed: {}", e);
                        }
                    }
                },
                Ok(_) => {},
                Err(e) => {
                    eprintln!("[CONFIG] Watcher error: {:?}", e);
                    break;
                }
            }
        }
    });
    shared_config
}