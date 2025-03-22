use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Represents both regular expression patterns and various messages loaded from a language file.
pub struct Patterns {
    // Regex patterns
    pub click_re: Regex,
    pub double_click_re: Regex,
    pub menu_re: Regex,
    pub navigation_re: Regex,
    pub window_resize_re: Regex,
    pub window_minimize_re: Regex,
    pub window_maximize_re: Regex,
    pub window_close_re: Regex,
    pub window_move_re: Regex,
    pub group_windows_re: Regex,
    pub tabcontrol_re: Regex,
    pub listview_re: Regex,
    pub radio_re: Regex,
    pub checkbox_re: Regex,
    pub file_open_re: Regex,
    pub file_copy_re: Regex,
    pub file_move_re: Regex,
    pub file_rename_re: Regex,
    pub file_delete_re: Regex,
    pub enter_text_re: Regex,
    pub get_text_re: Regex,
    pub set_text_re: Regex,
    pub select_text_re: Regex,
    pub copy_text_re: Regex,
    pub cut_text_re: Regex,
    pub delete_text_re: Regex,
    pub paste_text_re: Regex,
    pub universal_open_re: Regex,
    pub universal_focus_re: Regex,
    // Message strings
    pub msg_hint: String,
    pub msg_action_executed: String,
    pub msg_task_queued: String,
    pub msg_task_processing: String,
    pub msg_task_success: String,
    pub msg_task_failure: String,
    pub msg_execution_result: String,
    pub msg_error: String,
}

impl Patterns {
    /// Loads regex patterns and messages from a specified language file.
    ///
    /// The function ensures that the language file is located within the trusted "lang"
    /// directory in the project root and checks file permissions to mitigate path injection vulnerabilities.
    pub fn new(lang_file: &str) -> Result<Self, String> {
        // Define the trusted base directory for language files.
        let base_dir = std::env::current_dir()
            .map_err(|e| format!("Failed to retrieve current directory: {}", e))?
            .join("lang")
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize language base directory: {}", e))?;
        
        // Canonicalize the provided language file path.
        let input_path = Path::new(lang_file)
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize language file path '{}': {}", lang_file, e))?;
        
        // Ensure that the language file is within the trusted base directory.
        if !input_path.starts_with(&base_dir) {
            return Err(format!(
                "Path injection detected: '{}' is not within trusted base '{}'",
                input_path.display(),
                base_dir.display()
            ));
        }
        
        // Check file permissions: ensure that the file is not writable by group or others.
        let metadata = fs::metadata(&input_path)
            .map_err(|e| format!("Unable to read metadata for '{}': {}", input_path.display(), e))?;
        let mode = metadata.permissions().mode();
        if mode & 0o022 != 0 {
            return Err(format!(
                "Language file '{}' is writable by group or others (mode {:o}). Please secure the file.",
                input_path.display(), mode
            ));
        }
        
        // Read the file contents.
        let contents = fs::read_to_string(&input_path)
            .map_err(|e| format!("Error reading language file '{}': {}", input_path.display(), e))?;
        
        // Parse the file lines into a map.
        let mut map = HashMap::new();
        for line in contents.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(idx) = trimmed.find('=') {
                let key = trimmed[..idx].trim();
                let value = trimmed[idx + 1..].trim();
                map.insert(key.to_string(), value.to_string());
            }
        }
        
        // Helper macro to compile a regex for a given key.
        macro_rules! get_regex {
            ($key:expr) => {
                Regex::new(
                    map.get($key)
                        .ok_or_else(|| format!("Missing regex for key '{}'", $key))?
                )
                .map_err(|e| format!("Error compiling regex for key '{}': {}", $key, e))?
            };
        }

        // Helper macro to fetch a message string for a given key.
        macro_rules! get_msg {
            ($key:expr) => {
                map.get($key)
                    .ok_or_else(|| format!("Missing message for key '{}'", $key))?
                    .to_string()
            };
        }
        
        Ok(Patterns {
            click_re: get_regex!("CLICK_RE"),
            double_click_re: get_regex!("DOUBLE_CLICK_RE"),
            menu_re: get_regex!("MENU_RE"),
            navigation_re: get_regex!("NAVIGATION_RE"),
            window_resize_re: get_regex!("WINDOW_RESIZE_RE"),
            window_minimize_re: get_regex!("WINDOW_MINIMIZE_RE"),
            window_maximize_re: get_regex!("WINDOW_MAXIMIZE_RE"),
            window_close_re: get_regex!("WINDOW_CLOSE_RE"),
            window_move_re: get_regex!("WINDOW_MOVE_RE"),
            group_windows_re: get_regex!("GROUP_WINDOWS_RE"),
            tabcontrol_re: get_regex!("TABCONTROL_RE"),
            listview_re: get_regex!("LISTVIEW_RE"),
            radio_re: get_regex!("RADIO_RE"),
            checkbox_re: get_regex!("CHECKBOX_RE"),
            file_open_re: get_regex!("FILE_OPEN_RE"),
            file_copy_re: get_regex!("FILE_COPY_RE"),
            file_move_re: get_regex!("FILE_MOVE_RE"),
            file_rename_re: get_regex!("FILE_RENAME_RE"),
            file_delete_re: get_regex!("FILE_DELETE_RE"),
            enter_text_re: get_regex!("ENTER_TEXT_RE"),
            get_text_re: get_regex!("GET_TEXT_RE"),
            set_text_re: get_regex!("SET_TEXT_RE"),
            select_text_re: get_regex!("SELECT_TEXT_RE"),
            copy_text_re: get_regex!("COPY_TEXT_RE"),
            cut_text_re: get_regex!("CUT_TEXT_RE"),
            delete_text_re: get_regex!("DELETE_TEXT_RE"),
            paste_text_re: get_regex!("PASTE_TEXT_RE"),
            universal_open_re: get_regex!("UNIVERSAL_OPEN_RE"),
            universal_focus_re: get_regex!("UNIVERSAL_FOCUS_RE"),
            // Messages
            msg_hint: get_msg!("MSG_HINT"),
            msg_action_executed: get_msg!("MSG_ACTION_EXECUTED"),
            msg_task_queued: get_msg!("MSG_TASK_QUEUED"),
            msg_task_processing: get_msg!("MSG_TASK_PROCESSING"),
            msg_task_success: get_msg!("MSG_TASK_SUCCESS"),
            msg_task_failure: get_msg!("MSG_TASK_FAILURE"),
            msg_execution_result: get_msg!("MSG_EXECUTION_RESULT"),
            msg_error: get_msg!("MSG_ERROR"),
        })
    }
}

lazy_static::lazy_static! {
    // Load the patterns and messages using the language specified by configuration.
    // For demonstration, default to Russian ("ru") with language file "lang/ru.lng".
    pub static ref PATTERNS: Patterns = {
        let lang = "ru";
        let lang_file = format!("lang/{}.lng", lang);
        Patterns::new(&lang_file).expect("Failed to load language regex patterns and messages")
    };
}