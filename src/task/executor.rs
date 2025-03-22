use crate::core::intent::Action;
use crate::platform::windows::controller::{WinUiController, PlatformResult};
use log::{info, error};

/// Executes a given action using the provided WinUiController.
pub fn execute_action_on_platform(
    action: &Action,
    controller: &WinUiController,
) -> PlatformResult<()> {
    match action {
        Action::ButtonClick { label } => {
            info!("Executing ButtonClick action for label: {}", label);
            controller.click_button(label)
        }
        Action::ButtonDoubleClick { label } => {
            info!("Executing ButtonDoubleClick action for label: {}", label);
            controller.double_click_button(label)
        }
        Action::EditEnterText { label, text } => {
            info!("Executing EditEnterText action for label: {}, text: {}", label, text);
            controller.enter_text(label, text)
        }
        Action::EditSelectText { label, start, end } => {
            info!("Executing EditSelectText action for label: {}, start: {:?}, end: {:?}", label, start, end);
            controller.select_text(label, *start, *end)
        }
        Action::CheckboxSetState { label, state } => {
            info!("Executing CheckboxSetState action for label: {}, state: {}", label, state);
            controller.set_checkbox_state(label, *state)
        }
        Action::RadioSelect { label, variant } => {
            info!("Executing RadioSelect action for label: {}, variant: {:?}", label, variant);
            controller.select_radio_button(label)
        }
        Action::TreeViewSelect { label, node } => {
            info!("Executing TreeViewSelect action for label: {}, node: {:?}", label, node);
            if let Some(node_str) = node {
                if let Ok(node_id) = node_str.parse::<i32>() {
                     controller.select_treeview_item(label, node_id)
                } else {
                     error!("Invalid node ID format: {}", node_str);
                     Err(format!("Invalid node ID format: {}", node_str))
                }

            } else {
                Err("Node ID is required".to_string())
            }
        }
        Action::TreeViewExpand { label, node } => {
            info!("Executing TreeViewExpand action for label: {}, node: {:?}", label, node);
            if let Some(node_str) = node {
                if let Ok(node_id) = node_str.parse::<i32>() {
                    controller.expand_treeview_item(label, node_id)
                } else {
                     error!("Invalid node ID format: {}", node_str);
                     Err(format!("Invalid node ID format: {}", node_str))
                }
            } else {
                Err("Node ID is required".to_string())
            }
        }
        Action::ListViewSelectItem { label, item } => {
            info!("Executing ListViewSelectItem action for label: {}, item: {}", label, item);
             if let Ok(index) = item.parse::<usize>() {
                 controller.select_listview_item(label, index)
             } else {
                 error!("Invalid list index format: {}", item);
                Err(format!("Invalid list index format: {}", item))
             }
        }
        Action::TabControlSelectTab { label, tab } => {
            info!("Executing TabControlSelectTab action for label: {}, tab: {}", label, tab);
             if let Ok(index) = tab.parse::<usize>() {
                 controller.select_tabcontrol_tab(label, index)
             } else {
                 error!("Invalid tab index format: {}", tab);
                Err(format!("Invalid tab index format: {}", tab))
             }
        }
        Action::WindowResize { width, height } => {
            info!("Executing WindowResize action to {}x{}", width, height);
             controller.resize_window("Main", *width as i32, *height as i32) // Assuming main window
        }
        Action::WindowMinimize { label } => {
            info!("Executing WindowMinimize action for label: {}", label);
            controller.minimize_window(label)
        }
        Action::WindowMaximize { label } => {
            info!("Executing WindowMaximize action for label: {}", label);
            controller.maximize_window(label)
        }
        Action::WindowClose { label } => {
            info!("Executing WindowClose action for label: {}", label);
            controller.close_window(label)
        }
        Action::KeyPress { key } => {
             info!("Executing KeyPress action for key: {}", key);
             controller.key_press(key)
        }
        Action::Scroll { direction, amount } => {
             info!("Executing Scroll action: {} by {:?}", direction, amount);
             controller.scroll_window(direction, *amount)
        }
       Action::LaunchApplication { app } => {
           info!("Executing LaunchApplication action for app: {}", app);
           controller.launch_application(app)
       }
        Action::StaticGetText { label } => {
            // Implement this to get text from static UI element, if possible
            info!("Executing StaticGetText action for label: {}", label);
            match controller.get_static_text(label) {
                Ok(text) => {
                    info!("Static text: {}", text);
                    Ok(())
                }
                Err(e) => {
                    error!("Error getting static text: {}", e);
                    Err(e)
                }
            }
        }
        Action::MultiStep { steps } => {
            info!("Executing MultiStep action with {} steps", steps.len());
            for step in steps {
                execute_action_on_platform(step, controller)?;
            }
            Ok(())
        }
        Action::SetFocus { label } => {
            info!("Executing SetFocus action for label: {}", label);
            controller.set_focus(label)
        }
        _ => {
            error!("Unsupported action: {:?}", action);
            Err(format!("Unsupported action: {:?}", action))
        }
    }
}
