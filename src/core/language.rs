use regex::Regex;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use serde::Deserialize;
use log::error;

/// Represents both regular expression patterns and various messages loaded from a language file.
#[derive(Debug, Deserialize, Clone)]
pub struct LanguageData {
    // Regex patterns
    pub click_re: String,
    pub double_click_re: String,
    pub menu_re: String,
    pub navigation_re: String,
    pub window_resize_re: String,
    pub window_minimize_re: String,
    pub window_maximize_re: String,
    pub window_close_re: String,
    pub window_move_re: String,
    pub group_windows_re: String,
    pub tabcontrol_re: String,
    pub listview_re: String,
    pub radio_re: String,
    pub checkbox_re: String,
    pub file_open_re: String,
    pub file_copy_re: String,
    pub file_move_re: String,
    pub file_rename_re: String,
    pub file_delete_re: String,
    pub enter_text_re: String,
    pub get_text_re: String,
    pub set_text_re: String,
    pub select_text_re: String,
    pub copy_text_re: String,
    pub cut_text_re: String,
    pub delete_text_re: String,
    pub paste_text_re: String,
    pub universal_open_re: String,
    pub universal_focus_re: String,
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

impl LanguageData {
    /// Securely loads language data from a specified JSON file.
    pub fn load_from_file(lang_file: &str) -> Result<Self, String> {
        // Define the trusted base directory for language files.
        let base_dir = std::env::current_dir()
            .map_err(|e| format!("Failed to retrieve current directory: {}", e))?
            .join("assets")
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

        // Deserialize the JSON data into the LanguageData struct.
        let data: LanguageData = serde_json::from_str(&contents)
            .map_err(|e| format!("Error parsing language file '{}': {}", input_path.display(), e))?;

        Ok(data)
    }
}

/// Структура для хранения скомпилированных Regex и данных языка.
pub struct Patterns {
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
    pub fn new(language_data: LanguageData) -> Result<Self, String> {
        macro_rules! compile_regex {
            ($field:ident) => {
                Regex::new(&language_data.$field)
                    .map_err(|e| format!("Failed to compile regex for {}: {}", stringify!($field), e))?
            };
        }

        Ok(Patterns {
            click_re: compile_regex!(click_re),
            double_click_re: compile_regex!(double_click_re),
            menu_re: compile_regex!(menu_re),
            navigation_re: compile_regex!(navigation_re),
            window_resize_re: compile_regex!(window_resize_re),
            window_minimize_re: compile_regex!(window_minimize_re),
            window_maximize_re: compile_regex!(window_maximize_re),
            window_close_re: compile_regex!(window_close_re),
            window_move_re: compile_regex!(window_move_re),
            group_windows_re: compile_regex!(group_windows_re),
            tabcontrol_re: compile_regex!(tabcontrol_re),
            listview_re: compile_regex!(listview_re),
            radio_re: compile_regex!(radio_re),
            checkbox_re: compile_regex!(checkbox_re),
            file_open_re: compile_regex!(file_open_re),
            file_copy_re: compile_regex!(file_copy_re),
            file_move_re: compile_regex!(file_move_re),
            file_rename_re: compile_regex!(file_rename_re),
            file_delete_re: compile_regex!(file_delete_re),
            enter_text_re: compile_regex!(enter_text_re),
            get_text_re: compile_regex!(get_text_re),
            set_text_re: compile_regex!(set_text_re),
            select_text_re: compile_regex!(select_text_re),
            copy_text_re: compile_regex!(copy_text_re),
            cut_text_re: compile_regex!(cut_text_re),
            delete_text_re: compile_regex!(delete_text_re),
            paste_text_re: compile_regex!(paste_text_re),
            universal_open_re: compile_regex!(universal_open_re),
            universal_focus_re: compile_regex!(universal_focus_re),
            msg_hint: language_data.msg_hint,
            msg_action_executed: language_data.msg_action_executed,
            msg_task_queued: language_data.msg_task_queued,
            msg_task_processing: language_data.msg_task_processing,
            msg_task_success: language_data.msg_task_success,
            msg_task_failure: language_data.msg_task_failure,
            msg_execution_result: language_data.msg_execution_result,
            msg_error: language_data.msg_error,
        })
    }
}

lazy_static::lazy_static! {
    pub static ref PATTERNS: Patterns = {
        let lang = "ru";
        let lang_file = format!("lang/{}.json", lang);
        let language_data = LanguageData::load_from_file(&lang_file)
            .unwrap_or_else(|e| {
                error!("Failed to load language data: {}", e);
                // Provide a default or fallback LanguageData here
                LanguageData {
                    click_re: "".to_string(),
                    double_click_re: "".to_string(),
                     menu_re: "".to_string(),
                    navigation_re: "".to_string(),
                    window_resize_re: "".to_string(),
                    window_minimize_re: "".to_string(),
                    window_maximize_re: "".to_string(),
                    window_close_re: "".to_string(),
                    window_move_re: "".to_string(),
                    group_windows_re: "".to_string(),
                    tabcontrol_re: "".to_string(),
                    listview_re: "".to_string(),
                    radio_re: "".to_string(),
                    checkbox_re: "".to_string(),
                    file_open_re: "".to_string(),
                    file_copy_re: "".to_string(),
                    file_move_re: "".to_string(),
                    file_rename_re: "".to_string(),
                    file_delete_re: "".to_string(),
                    enter_text_re: "".to_string(),
                    get_text_re: "".to_string(),
                    set_text_re: "".to_string(),
                    select_text_re: "".to_string(),
                    copy_text_re: "".to_string(),
                    cut_text_re: "".to_string(),
                    delete_text_re: "".to_string(),
                    paste_text_re: "".to_string(),
                    universal_open_re: "".to_string(),
                    universal_focus_re: "".to_string(),
                    msg_hint: "Command not recognized. Please try again.".to_string(),
                    msg_action_executed: "Action executed: {}".to_string(),
                    msg_task_queued: "Task queued".to_string(),
                    msg_task_processing: "Task processing".to_string(),
                    msg_task_success: "Task succeeded".to_string(),
                    msg_task_failure: "Task failed".to_string(),
                    msg_execution_result: "Execution result: {}".to_string(),
                    msg_error: "Error: {}".to_string(),
                }
            });
        Patterns::new(language_data)
            .expect("Failed to create Patterns")
    };
}
