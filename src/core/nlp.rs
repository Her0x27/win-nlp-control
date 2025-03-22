use regex::Regex;
use lazy_static::lazy_static;
use std::collections::HashMap;
use rust_stemmers::{Algorithm, Stemmer};
use log::{debug, error}; // Import logging macros

// Import language-specific regex patterns and messages.
use crate::core::language::PATTERNS;

/// The result of natural language processing analysis.
#[derive(Debug, Clone)]
pub struct NLPResult {
    pub intent: String,
    pub parameters: HashMap<String, String>,
}

/// Analyze and normalize natural language commands using stemming and language-specific regex patterns.
pub fn parse_command(command: &str) -> NLPResult {
    debug!("Parsing command: {}", command);
    let normalized_command = morphological_analyze(command);
    let lower_command = normalized_command.to_lowercase();

    let mut result = NLPResult {
        intent: "unknown".to_string(),
        parameters: HashMap::new(),
    };

    // Check commands using regex patterns loaded from the language file.
    if let Some(caps) = PATTERNS.universal_open_re.captures(&lower_command) {
        result.intent = "launch_object".to_string();
        let object = caps.get(1).map_or("", |m| m.as_str()).to_string(); // Corrected index
        result.parameters.insert("object".to_string(), object);
        debug!("Intent: launch_object, Object: {}", object);
        return result;
    }
    if let Some(caps) = PATTERNS.universal_focus_re.captures(&lower_command) {
        result.intent = "focus_object".to_string();
        let object = caps.get(1).map_or("", |m| m.as_str()).to_string(); // Corrected index
        result.parameters.insert("object".to_string(), object);
        debug!("Intent: focus_object, Object: {}", object);
        return result;
    }
    if let Some(caps) = PATTERNS.group_windows_re.captures(&lower_command) {
        result.intent = "group_windows".to_string();
        let group = caps.get(1).map_or("", |m| m.as_str()).to_string(); // Corrected index
        result.parameters.insert("group".to_string(), group);
        result.parameters.insert("windows".to_string(), "".to_string());
        debug!("Intent: group_windows, Group: {}", group);
        return result;
    }
    if let Some(caps) = PATTERNS.select_text_re.captures(&lower_command) {
        result.intent = "edit_select_text".to_string();
        if let (Some(start), Some(end)) = (caps.get(1), caps.get(2)) { // Corrected indices
            result.parameters.insert("start".to_string(), start.as_str().to_string());
            result.parameters.insert("end".to_string(), end.as_str().to_string());
        }
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
        debug!("Intent: edit_select_text, Start: {:?}, End: {:?}", caps.get(1), caps.get(2));
        return result;
    }
    if PATTERNS.copy_text_re.is_match(&lower_command) {
        result.intent = "edit_copy_text".to_string();
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
        debug!("Intent: edit_copy_text");
        return result;
    }
    if PATTERNS.cut_text_re.is_match(&lower_command) {
        result.intent = "edit_cut_text".to_string();
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
         debug!("Intent: edit_cut_text");
        return result;
    }
    if PATTERNS.delete_text_re.is_match(&lower_command) {
        result.intent = "edit_delete_text".to_string();
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
         debug!("Intent: edit_delete_text");
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
         debug!("Intent: edit_paste_text, Text: {:?}", extract_quoted_text(&lower_command));
        return result;
    }
    if let Some(caps) = PATTERNS.enter_text_re.captures(&lower_command) {
        result.intent = "edit_enter_text".to_string();
        if let Some(text) = caps.get(1) {
            result.parameters.insert("text".to_string(), text.as_str().to_string());
        }
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
         debug!("Intent: edit_enter_text, Text: {:?}, Label: {:?}", caps.get(1), extract_label(&lower_command));
        return result;
    }

    if PATTERNS.get_text_re.is_match(&lower_command) {
        result.intent = "static_get_text".to_string();
        let label = extract_label(&lower_command).unwrap_or_else(|| "default".to_string());
        result.parameters.insert("label".to_string(), label);
        debug!("Intent: static_get_text, Label: {}", label);
        return result;
    }
    if let Some(caps) = PATTERNS.set_text_re.captures(&lower_command) {
        result.intent = "set_text".to_string();
        if let Some(text) = caps.get(1) {
            result.parameters.insert("text".to_string(), text.as_str().to_string());
        }
        if let Some(label) = extract_label(&lower_command) {
            result.parameters.insert("label".to_string(), label);
        }
         debug!("Intent: set_text, Text: {:?}, Label: {:?}", caps.get(1), extract_label(&lower_command));
        return result;
    }

    if let Some(caps) = PATTERNS.window_resize_re.captures(&lower_command) {
        result.intent = "window_resize".to_string();
        if let (Some(width), Some(height)) = (caps.get(1), caps.get(2)) {
            result.parameters.insert("width".to_string(), width.as_str().to_string());
            result.parameters.insert("height".to_string(), height.as_str().to_string());
        }
        debug!("Intent: window_resize, Width: {:?}, Height: {:?}", caps.get(1), caps.get(2));
        return result;
    }

    if let Some(caps) = PATTERNS.window_minimize_re.captures(&lower_command) {
        result.intent = "window_minimize".to_string();
         if let Some(label) = caps.get(1) {
            result.parameters.insert("label".to_string(), label.as_str().to_string());
        }
        debug!("Intent: window_minimize, Label: {:?}", caps.get(1));
        return result;
    }
    if let Some(caps) = PATTERNS.window_maximize_re.captures(&lower_command) {
        result.intent = "window_maximize".to_string();
         if let Some(label) = caps.get(1) {
            result.parameters.insert("label".to_string(), label.as_str().to_string());
        }
         debug!("Intent: window_maximize, Label: {:?}", caps.get(1));
        return result;
    }
     if let Some(caps) = PATTERNS.window_close_re.captures(&lower_command) {
        result.intent = "window_close".to_string();
         if let Some(label) = caps.get(1) {
            result.parameters.insert("label".to_string(), label.as_str().to_string());
        }
        debug!("Intent: window_close, Label: {:?}", caps.get(1));
        return result;
    }

    if let Some(caps) = PATTERNS.window_move_re.captures(&lower_command) {
        result.intent = "window_move".to_string();
        if let (Some(x), Some(y)) = (caps.get(2), caps.get(3)) {
            result.parameters.insert("x".to_string(), x.as_str().to_string());
            result.parameters.insert("y".to_string(), y.as_str().to_string());
        }
         if let Some(label) = caps.get(1) {
            result.parameters.insert("label".to_string(), label.as_str().to_string());
        }
        debug!("Intent: window_move, X: {:?}, Y: {:?}, Label: {:?}", caps.get(2), caps.get(3), caps.get(1));
        return result;
    }

    if let Some(caps) = PATTERNS.file_open_re.captures(&lower_command) {
        result.intent = "open_file".to_string();
        if let Some(file) = caps.get(1) {
            result.parameters.insert("file".to_string(), file.as_str().to_string());
        }
        debug!("Intent: open_file, File: {:?}", caps.get(1));
        return result;
    }

    // Fallback: no known command detected.
    result.intent = "unknown".to_string();
    result.parameters.insert("hint".to_string(), PATTERNS.msg_hint.clone());
    debug!("Intent: unknown, Hint: {}", PATTERNS.msg_hint.clone());
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
    let result = words.join(" ");
    debug!("Morphological analysis result: {}", result);
    result
}

/// Extracts a label from the command using a simple inline regex.
fn extract_label(command: &str) -> Option<String> {
    let re = Regex::new(r#""([^"]+)""#).ok()?;
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
