use regex::Regex;
use lazy_static::lazy_static;
use std::collections::HashMap;
use rust_stemmers::{Algorithm, Stemmer};

// Import language-specific regex patterns and messages.
use crate::language::PATTERNS;

/// The result of natural language processing analysis.
#[derive(Debug, Clone)]
pub struct NLPResult {
    pub intent: String,
    pub parameters: HashMap<String, String>,
}

/// Analyze and normalize natural language commands using stemming and language-specific regex patterns.
pub fn parse_command(command: &str) -> NLPResult {
    let normalized_command = morphological_analyze(command);
    let lower_command = normalized_command.to_lowercase();

    let mut result = NLPResult {
        intent: "unknown".to_string(),
        parameters: HashMap::new(),
    };

    // Check commands using regex patterns loaded from the language file.
    if let Some(caps) = PATTERNS.universal_open_re.captures(&lower_command) {
        result.intent = "launch_object".to_string();
        let object = caps.get(2).map_or("default_object", |m| m.as_str()).to_string();
        result.parameters.insert("object".to_string(), object);
        return result;
    }
    if let Some(caps) = PATTERNS.universal_focus_re.captures(&lower_command) {
        result.intent = "focus_object".to_string();
        let object = caps.get(2).map_or("default_object", |m| m.as_str()).to_string();
        result.parameters.insert("object".to_string(), object);
        return result;
    }
    if let Some(caps) = PATTERNS.group_windows_re.captures(&lower_command) {
        result.intent = "group_windows".to_string();
        let group = caps.get(2).map_or("default_group", |m| m.as_str()).to_string();
        result.parameters.insert("group".to_string(), group);
        result.parameters.insert("windows".to_string(), "".to_string());
        return result;
    }
    if let Some(caps) = PATTERNS.select_text_re.captures(&lower_command) {
        result.intent = "edit_select_text".to_string();
        if let (Some(start), Some(end)) = (caps.get(2), caps.get(3)) {
            result.parameters.insert("start".to_string(), start.as_str().to_string());
            result.parameters.insert("end".to_string(), end.as_str().to_string());
        }
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
        return result;
    }
    if PATTERNS.copy_text_re.is_match(&lower_command) {
        result.intent = "edit_copy_text".to_string();
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
        return result;
    }
    if PATTERNS.cut_text_re.is_match(&lower_command) {
        result.intent = "edit_cut_text".to_string();
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
        return result;
    }
    if PATTERNS.delete_text_re.is_match(&lower_command) {
        result.intent = "edit_delete_text".to_string();
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
        return result;
    }
    if PATTERNS.paste_text_re.is_match(&lower_command) {
        result.intent = "edit_paste_text".to_string();
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
        if let Some(text) = extract_quoted_text(&lower_command) {
            result.parameters.insert("text".to_string(), text);
        }
        return result;
    }
    if PATTERNS.enter_text_re.is_match(&lower_command) {
        result.intent = "edit_enter_text".to_string();
        let label = extract_label(&lower_command).unwrap_or_else(|| "default".to_string());
        result.parameters.insert("label".to_string(), label);
        if let Some(text) = extract_quoted_text(&lower_command) {
            result.parameters.insert("text".to_string(), text);
        } else {
            result.parameters.insert("text".to_string(), "example".to_string());
        }
        return result;
    }
    if PATTERNS.get_text_re.is_match(&lower_command) {
        result.intent = "static_get_text".to_string();
        let label = extract_label(&lower_command).unwrap_or_else(|| "default".to_string());
        result.parameters.insert("label".to_string(), label);
        return result;
    }
    if PATTERNS.set_text_re.is_match(&lower_command) {
        result.intent = "set_text".to_string();
        let label = extract_label(&lower_command).unwrap_or_else(|| "default".to_string());
        result.parameters.insert("label".to_string(), label);
        if let Some(text) = extract_quoted_text(&lower_command) {
            result.parameters.insert("text".to_string(), text);
        } else {
            result.parameters.insert("text".to_string(), "new_text".to_string());
        }
        return result;
    }
    if PATTERNS.window_resize_re.is_match(&lower_command) {
        result.intent = "window_resize".to_string();
        let nums = extract_numbers(&lower_command);
        if nums.len() >= 2 {
            result.parameters.insert("width".to_string(), nums[0].clone());
            result.parameters.insert("height".to_string(), nums[1].clone());
        } else {
            result.parameters.insert("width".to_string(), "800".to_string());
            result.parameters.insert("height".to_string(), "600".to_string());
        }
        return result;
    }
    if PATTERNS.window_minimize_re.is_match(&lower_command) {
        result.intent = "window_minimize".to_string();
        let label = extract_label(&lower_command).unwrap_or_else(|| "default".to_string());
        result.parameters.insert("label".to_string(), label);
        return result;
    }
    if PATTERNS.window_maximize_re.is_match(&lower_command) {
        result.intent = "window_maximize".to_string();
        let label = extract_label(&lower_command).unwrap_or_else(|| "default".to_string());
        result.parameters.insert("label".to_string(), label);
        return result;
    }
    if PATTERNS.window_close_re.is_match(&lower_command) {
        result.intent = "window_close".to_string();
        let label = extract_label(&lower_command).unwrap_or_else(|| "default".to_string());
        result.parameters.insert("label".to_string(), label);
        return result;
    }
    if PATTERNS.window_move_re.is_match(&lower_command) {
        result.intent = "window_move".to_string();
        let nums = extract_numbers(&lower_command);
        if nums.len() >= 2 {
            result.parameters.insert("x".to_string(), nums[0].clone());
            result.parameters.insert("y".to_string(), nums[1].clone());
        }
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
        return result;
    }
    if PATTERNS.file_open_re.is_match(&lower_command) {
        result.intent = "open_file".to_string();
        if let Some(file) = extract_quoted_text(&lower_command) {
            result.parameters.insert("file".to_string(), file);
        }
        return result;
    }
    if PATTERNS.file_copy_re.is_match(&lower_command) {
        result.intent = "copy_file".to_string();
        if let Some(file) = extract_quoted_text(&lower_command) {
            result.parameters.insert("file".to_string(), file);
        }
        return result;
    }
    if PATTERNS.file_move_re.is_match(&lower_command) {
        result.intent = "move_file".to_string();
        if let Some(file) = extract_quoted_text(&lower_command) {
            result.parameters.insert("file".to_string(), file);
        }
        return result;
    }
    if PATTERNS.file_rename_re.is_match(&lower_command) {
        result.intent = "rename_file".to_string();
        if let Some(file) = extract_quoted_text(&lower_command) {
            result.parameters.insert("file".to_string(), file);
        }
        return result;
    }
    if PATTERNS.file_delete_re.is_match(&lower_command) {
        result.intent = "delete_file".to_string();
        if let Some(file) = extract_quoted_text(&lower_command) {
            result.parameters.insert("file".to_string(), file);
        }
        return result;
    }
    // Fallback: no known command detected.
    result.intent = "unknown".to_string();
    result.parameters.insert("hint".to_string(), PATTERNS.msg_hint.clone());
    result
}

/// Applies stemming to the input command while removing punctuation and stop words.
fn morphological_analyze(command: &str) -> String {
    let stop_words = vec!["и", "в", "на", "с", "к", "по", "за", "для", "также", "не", "но", "а", "то", "же"];
    let stemmer = Stemmer::create(Algorithm::Russian);
    let cleaned = command.replace(|c: char| !c.is_alphanumeric() && !c.is_whitespace(), " ");
    let words: Vec<String> = cleaned
        .split_whitespace()
        .filter(|w| !stop_words.contains(&w.to_lowercase().as_str()))
        .map(|w| stemmer.stem(w).to_string())
        .collect();
    words.join(" ")
}

/// Extracts a label from the command using a simple inline regex.
fn extract_label(command: &str) -> Option<String> {
    let re = Regex::new(r"(?:название|лейбл)\s+([а-яa-z0-9_]+)").ok()?;
    re.captures(command)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extracts text enclosed in double quotes.
fn extract_quoted_text(command: &str) -> Option<String> {
    let re = Regex::new(r#""([^"]+)""#).ok()?;
    re.captures(command)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

/// Extracts all numbers present in the command.
fn extract_numbers(command: &str) -> Vec<String> {
    let re = Regex::new(r"\b(\d+)\b").unwrap();
    re.captures_iter(command)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .collect()
}