use crate::nlp::NLPResult;
use crate::config::SharedConfig;
use crate::config::AppConfig;
use std::collections::HashMap;

/// Represents an action derived from the natural language input.
#[derive(Debug)]
pub enum Action {
    ButtonClick { label: String },
    ButtonDoubleClick { label: String },
    EditEnterText { label: String, text: String },
    EditSelectText { label: String, start: Option<u32>, end: Option<u32> },
    EditCopyText { label: String },
    EditCutText { label: String },
    EditClearField { label: String },
    EditDeleteText { label: String },
    EditPasteText { label: String, text: Option<String> },
    StaticGetText { label: String },
    SetText { label: String, text: String },
    SetFocus { label: String },
    CheckboxSetState { label: String, state: bool },
    RadioSelect { label: String, variant: Option<String> },
    TreeViewSelect { label: String, node: Option<String> },
    TreeViewExpand { label: String, node: Option<String> },
    ListViewSelectItem { label: String, item: String },
    TabControlSelectTab { label: String, tab: String },
    WindowResize { width: u32, height: u32 },
    WindowMinimize { label: String },
    WindowMaximize { label: String },
    WindowClose { label: String },
    WindowMove { label: String, x: u32, y: u32 },
    LaunchApplication { app: String },
    FocusApplication { app: String },
    GroupWindows { group: String, windows: String },
    LaunchObject { object: String },
    FocusObject { object: String },
    WindowMinimizeAll,
    WindowMaximizeAll,
    WindowCloseAll,
    OpenFileProperties { file: String },
    ListSelect { label: String, item: String },
    KeyPress { key: String },
    Scroll { direction: String, amount: Option<u32> },
    Screenshot,
    SpinnerAdjust { label: String, operation: String, value: u32 },
    SelectFiles { criteria: String },
    FileOperation { operation: String },
    PasteFiles { destination: String },
    CreateDirectory { name: String },
    DeleteDirectory { name: String },
    CreateFile { name: String },
    DeleteFile { name: String },
    MultiStep { steps: Vec<Action> },
    Unknown { hint: String },
}

/// Attempts to apply an alias to the NLP result using the current configuration.
/// If an alias is found matching the NLP intent, it replaces the intent and parameters accordingly.
fn try_apply_alias(nlp_result: &NLPResult, shared_config: &SharedConfig) -> Option<Action> {
    let config_lock = shared_config.lock().ok()?;
    let config = config_lock.as_ref()?;
    for alias in config.aliases.iter() {
        if alias.alias.to_lowercase() == nlp_result.intent.to_lowercase() {
            let mut new_result = nlp_result.clone();
            new_result.intent = alias.intent.clone();
            if let Some(ref alias_params) = alias.parameters {
                for (k, v) in alias_params {
                    new_result.parameters.entry(k.clone()).or_insert(v.clone());
                }
            }
            if let Some(cmd_type) = &alias.command_type {
                if cmd_type.to_lowercase() == "multi" {
                    if let Some(steps) = &alias.steps {
                        let mapped_steps = steps
                            .iter()
                            .map(|step_alias| {
                                let mut step_result = nlp_result.clone();
                                step_result.intent = step_alias.intent.clone();
                                if let Some(ref step_params) = step_alias.parameters {
                                    for (k, v) in step_params {
                                        step_result.parameters.entry(k.clone()).or_insert(v.clone());
                                    }
                                }
                                map_intent_impl(&step_result)
                            })
                            .collect();
                        return Some(Action::MultiStep { steps: mapped_steps });
                    }
                }
            }
            return Some(map_intent_impl(&new_result));
        }
    }
    None
}

/// Public API for mapping an NLP result to an Action, potentially utilizing alias configuration.
pub fn map_intent(nlp_result: &NLPResult, shared_config: &SharedConfig) -> Action {
    if let Some(alias_action) = try_apply_alias(nlp_result, shared_config) {
        return alias_action;
    }
    map_intent_impl(nlp_result)
}

/// Internal implementation of intent mapping based on the NLP result.
/// If the intent is not recognized, returns an Unknown action with a hint message based on language settings.
fn map_intent_impl(nlp_result: &NLPResult) -> Action {
    match nlp_result.intent.as_str() {
        "button_click" => Action::ButtonClick {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "button_double_click" => Action::ButtonDoubleClick {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "edit_enter_text" => Action::EditEnterText {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            text: nlp_result.parameters.get("text").cloned().unwrap_or_default(),
        },
        "edit_select_text" => Action::EditSelectText {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            start: nlp_result.parameters.get("start").and_then(|s| s.parse::<u32>().ok()),
            end: nlp_result.parameters.get("end").and_then(|s| s.parse::<u32>().ok()),
        },
        "edit_copy_text" => Action::EditCopyText {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "edit_cut_text" => Action::EditCutText {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "edit_clear_field" => Action::EditClearField {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "edit_delete_text" => Action::EditDeleteText {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "edit_paste_text" => Action::EditPasteText {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            text: nlp_result.parameters.get("text").cloned(),
        },
        "static_get_text" => Action::StaticGetText {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "set_text" => Action::SetText {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            text: nlp_result.parameters.get("text").cloned().unwrap_or_default(),
        },
        "set_focus" => Action::SetFocus {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "checkbox_set_state" => {
            let state_str = nlp_result.parameters.get("state").cloned().unwrap_or_else(|| "false".to_string());
            Action::CheckboxSetState {
                label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
                state: state_str == "true",
            }
        },
        "radio_select" => Action::RadioSelect {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            variant: nlp_result.parameters.get("variant").cloned(),
        },
        "treeview_select" => Action::TreeViewSelect {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            node: nlp_result.parameters.get("node").cloned(),
        },
        "treeview_expand" => Action::TreeViewExpand {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            node: nlp_result.parameters.get("node").cloned(),
        },
        "listview_select_item" => Action::ListViewSelectItem {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            item: nlp_result.parameters.get("item").cloned().unwrap_or_default(),
        },
        "tabcontrol_select_tab" => Action::TabControlSelectTab {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            tab: nlp_result.parameters.get("tab").cloned().unwrap_or_default(),
        },
        "window_resize" => Action::WindowResize {
            width: nlp_result.parameters.get("width").and_then(|s| s.parse::<u32>().ok()).unwrap_or(800),
            height: nlp_result.parameters.get("height").and_then(|s| s.parse::<u32>().ok()).unwrap_or(600),
        },
        "window_minimize" => Action::WindowMinimize {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "window_maximize" => Action::WindowMaximize {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "window_close" => Action::WindowClose {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
        },
        "window_move" => Action::WindowMove {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            x: nlp_result.parameters.get("x").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0),
            y: nlp_result.parameters.get("y").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0),
        },
        "launch_object" | "launch_application" => Action::LaunchApplication {
            app: nlp_result.parameters.get("object")
                .or_else(|| nlp_result.parameters.get("app"))
                .cloned()
                .unwrap_or_default(),
        },
        "focus_object" | "focus_application" => Action::FocusApplication {
            app: nlp_result.parameters.get("object")
                .or_else(|| nlp_result.parameters.get("app"))
                .cloned()
                .unwrap_or_default(),
        },
        "group_windows" => Action::GroupWindows {
            group: nlp_result.parameters.get("group").cloned().unwrap_or_default(),
            windows: nlp_result.parameters.get("windows").cloned().unwrap_or_default(),
        },
        "window_minimize_all" => Action::WindowMinimizeAll,
        "window_maximize_all" => Action::WindowMaximizeAll,
        "window_close_all" => Action::WindowCloseAll,
        "open_file" => Action::OpenFileProperties {
            file: nlp_result.parameters.get("file").cloned().unwrap_or_default(),
        },
        "list_select" => Action::ListSelect {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            item: nlp_result.parameters.get("item").cloned().unwrap_or_default(),
        },
        "key_press" => Action::KeyPress {
            key: nlp_result.parameters.get("key").cloned().unwrap_or_default(),
        },
        "scroll" => Action::Scroll {
            direction: nlp_result.parameters.get("direction").cloned().unwrap_or_else(|| "up".to_string()),
            amount: nlp_result.parameters.get("amount").and_then(|s| s.parse::<u32>().ok()),
        },
        "screenshot" => Action::Screenshot,
        "spinner_adjust" => Action::SpinnerAdjust {
            label: nlp_result.parameters.get("label").cloned().unwrap_or_default(),
            operation: nlp_result.parameters.get("operation").cloned().unwrap_or_default(),
            value: nlp_result.parameters.get("value").and_then(|s| s.parse::<u32>().ok()).unwrap_or(0),
        },
        "select_files" => Action::SelectFiles {
            criteria: nlp_result.parameters.get("criteria").cloned().unwrap_or_default(),
        },
        "copy_file" | "cut_file" | "delete_file" | "move_file" | "rename_file" => {
            Action::FileOperation {
                operation: nlp_result.intent.clone(),
            }
        },
        "paste_files" => Action::PasteFiles {
            destination: nlp_result.parameters.get("destination").cloned().unwrap_or_default(),
        },
        "create_directory" => Action::CreateDirectory {
            name: nlp_result.parameters.get("name").cloned().unwrap_or_default(),
        },
        "delete_directory" => Action::DeleteDirectory {
            name: nlp_result.parameters.get("name").cloned().unwrap_or_default(),
        },
        "create_file" => Action::CreateFile {
            name: nlp_result.parameters.get("name").cloned().unwrap_or_default(),
        },
        "delete_file" => Action::DeleteFile {
            name: nlp_result.parameters.get("name").cloned().unwrap_or_default(),
        },
        "multi_step" => {
            // This should be handled by an alias.
            Action::MultiStep { steps: vec![] }
        }
        // Fallback for unknown intent.
        _ => Action::Unknown {
            hint: nlp_result.parameters.get("hint").cloned().unwrap_or_else(|| {
                // Default to a hint message from language messages.
                // Note: This usage assumes that the language module has already provided a message hint.
                // You can integrate more dynamic behavior here if needed.
                "Команда не распознана. Попробуйте уточнить запрос.".to_string()
            }),
        },
    }
}