#![allow(non_snake_case, unused_unsafe)]

use crate::platform::windows::winapi::*;
use log::{info, warn, error, debug};
use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BM_CLICK, BM_GETCHECK, BM_SETCHECK, BST_CHECKED, BST_UNCHECKED, EM_SETSEL,
    SB_LINEUP, SB_LINEDOWN, SW_MAXIMIZE, SW_MINIMIZE, SW_SHOWNORMAL,
    TCM_SETCURSEL, TVM_EXPAND, TVM_SELECTITEM, WM_VSCROLL, WM_CLOSE, LVM_SETITEMSTATE,
    MoveWindow, SetWindowPos, SWP_NOZORDER, SWP_NOACTIVATE, FindWindowW, GetWindowTextW,
    GetWindowTextLengthW, SendMessageW, ShowWindow, SetWindowTextW, EnumWindows, IsWindowVisible,
    GetForegroundWindow, SetFocus, EnumChildWindows, GetClassNameW, WM_COPY, WM_CUT, WM_CLEAR,
    WM_PASTE, GetClientRect, CB_SETCURSEL, CB_GETCOUNT, CBS_DROPDOWNLIST, IsWindowEnabled,
    GWL_STYLE, GetWindowLongW, SHELLEXECUTEINFOW, ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS,
    SEE_MASK_FLAG_DDE, SEE_MASK_INVOKEIDLIST, SEE_MASK_IDLIST, SEE_MASK_CLASSNAME, SW_SHOW
};
use windows_sys::Win32::Graphics::Gdi::{HORZRES, VERTRES, SRCCOPY};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::core::{PCWSTR, w, PSTR};
use std::ffi::OsString;
use std::os::windows::prelude::*;
use std::mem;
use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

// Generic Result type for platform-specific operations
pub type PlatformResult<T> = Result<T, String>;

pub struct WinUiController {}

impl WinUiController {
    pub fn new() -> Self {
        WinUiController {}
    }

    /// Clicks a button with the given label.
    pub fn click_button(&self, label: &str) -> PlatformResult<()> {
        info!("Clicking button with label: {}", label);
        unsafe {
            let hwnd = find_window(Some("Button"), Some(label));
            if hwnd.0 == 0 {
                error!("Button with label '{}' not found", label);
                return Err(format!("Button with label '{}' not found", label));
            }

            let result = send_message(hwnd, BM_CLICK, WPARAM(0), LPARAM(0));
            if result == 0 {
                 warn!("Click failed for button with label '{}'", label);
                return Err(format!("Click failed for button with label '{}'", label));
            }
            Ok(())
        }
    }

     /// Double-clicks a button with the given label.
    pub fn double_click_button(&self, label: &str) -> PlatformResult<()> {
        info!("Double-clicking button with label: {}", label);
        self.click_button(label)?;
        std::thread::sleep(std::time::Duration::from_millis(100)); // Small delay
        self.click_button(label)
    }

    /// Enters text into an edit control with the given label.
    pub fn enter_text(&self, label: &str, text: &str) -> PlatformResult<()> {
        info!("Entering text '{}' into edit control with label: {}", text, label);
        unsafe {
            let hwnd = find_window(Some("Edit"), Some(label));
            if hwnd.0 == 0 {
                error!("Edit control with label '{}' not found", label);
                return Err(format!("Edit control with label '{}' not found", label));
            }
            if !set_window_text(hwnd, text) {
                error!("Failed to set text for edit control with label '{}'", label);
                return Err(format!("Failed to set text for edit control with label '{}'", label));
            }
            Ok(())
        }
    }

    /// Selects text in an edit control
    pub fn select_text(&self, label: &str, start: Option<u32>, end: Option<u32>) -> PlatformResult<()> {
        info!("Selecting text in edit control '{}' from {:?} to {:?}", label, start, end);
        unsafe {
            let hwnd = find_window(Some("Edit"), Some(label));
            if hwnd.0 == 0 {
                error!("Edit control with label '{}' not found", label);
                return Err(format!("Edit control with label '{}' not found", label));
            }

            let sel_start = start.unwrap_or(0) as usize;
            let sel_end = end.map(|e| e as i32).unwrap_or(-1) as isize; // -1 selects all text

            let result = send_message(hwnd, EM_SETSEL, WPARAM(sel_start), LPARAM(sel_end));
            if result == 0 {
                warn!("Failed to select text in edit control '{}'", label);
                return Err(format!("Failed to select text in edit control '{}'", label));
            }

            Ok(())
        }
    }

     /// Copies text from edit control
    pub fn copy_text(&self, label: &str) -> PlatformResult<()> {
        info!("Copying text from edit control: {}", label);
         unsafe {
             let hwnd = find_window(Some("Edit"), Some(label));
            if hwnd.0 == 0 {
                error!("Edit control with label '{}' not found", label);
                return Err(format!("Edit control with label '{}' not found", label));
            }

            send_message(hwnd, WM_COPY, WPARAM(0), LPARAM(0));

            Ok(())
         }
    }
    /// Cuts text from edit control
    pub fn cut_text(&self, label: &str) -> PlatformResult<()> {
        info!("Cutting text from edit control: {}", label);
         unsafe {
              let hwnd = find_window(Some("Edit"), Some(label));
            if hwnd.0 == 0 {
                error!("Edit control with label '{}' not found", label);
                return Err(format!("Edit control with label '{}' not found", label));
            }

            send_message(hwnd, WM_CUT, WPARAM(0), LPARAM(0));
            Ok(())
         }
    }

    /// Clears text from edit control
     pub fn clear_field(&self, label: &str) -> PlatformResult<()> {
        info!("Clearing text from edit control: {}", label);
         unsafe {
               let hwnd = find_window(Some("Edit"), Some(label));
            if hwnd.0 == 0 {
                error!("Edit control with label '{}' not found", label);
                return Err(format!("Edit control with label '{}' not found", label));
            }
            send_message(hwnd, WM_CLEAR, WPARAM(0), LPARAM(0));
            Ok(())
         }
    }

    /// Pastes text to edit control
     pub fn paste_text(&self, label: &str) -> PlatformResult<()> {
        info!("Pasting text to edit control: {}", label);
         unsafe {
              let hwnd = find_window(Some("Edit"), Some(label));
            if hwnd.0 == 0 {
                error!("Edit control with label '{}' not found", label);
                return Err(format!("Edit control with label '{}' not found", label));
            }
              send_message(hwnd, WM_PASTE, WPARAM(0), LPARAM(0));
              Ok(())
         }
    }

     /// Gets text from static control
    pub fn get_static_text(&self, label: &str) -> PlatformResult<String> {
         info!("Getting text from static control: {}", label);
         unsafe {
             let hwnd = find_window(Some("Static"), Some(label));
             if hwnd.0 == 0 {
                 error!("Static control with label '{}' not found", label);
                 return Err(format!("Static control with label '{}' not found", label));
             }
             let len = GetWindowTextLengthW(hwnd) as usize;
            if len == 0 {
                return Ok("".to_string());
            }

            let mut buffer: Vec<u16> = vec![0; len + 1];

            let result = GetWindowTextW(hwnd, buffer.as_mut_ptr(), (len + 1) as i32);

             if result == 0 {
                 return Err("GetWindowTextW return 0".to_string());
             }

             String::from_utf16(&buffer[..len]).map_err(|e| format!("Failed to convert from UTF-16: {}", e))
         }
    }

    /// Sets focus
    pub fn set_focus(&self, label: &str) -> PlatformResult<()> {
         info!("Setting focus on {}", label);
         unsafe {
             let hwnd = find_window(None, Some(label));
            if hwnd.0 == 0 {
                error!("Window with label '{}' not found", label);
                return Err(format!("Window with label '{}' not found", label));
            }
           if SetFocus(hwnd).0 == 0 {
                error!("Failed to set focus on window with label '{}'", label);
                return Err(format!("Failed to set focus on window with label '{}'", label));
            }
            Ok(())
         }
    }

    /// Sets the checked state of a checkbox
    pub fn set_checkbox_state(&self, label: &str, checked: bool) -> PlatformResult<()> {
        info!("Setting checkbox '{}' to state: {}", label, checked);
        unsafe {
            let hwnd = find_window(Some("Button"), Some(label));
            if hwnd.0 == 0 {
                error!("Checkbox with label '{}' not found", label);
                return Err(format!("Checkbox with label '{}' not found", label));
            }
            let check_state = if checked { BST_CHECKED } else { BST_UNCHECKED };
            send_message(hwnd, BM_SETCHECK, WPARAM(check_state as usize), LPARAM(0));
             Ok(())
        }
    }

    /// Selects a radio button
    pub fn select_radio_button(&self, label: &str) -> PlatformResult<()> {
        info!("Selecting radio button: {}", label);
        unsafe {
            let hwnd = find_window(Some("Button"), Some(label));
            if hwnd.0 == 0 {
                error!("Radio button with label '{}' not found", label);
                return Err(format!("Radio button with label '{}' not found", label));
            }
             send_message(hwnd, BM_SETCHECK, WPARAM(BST_CHECKED as usize), LPARAM(0));
             Ok(())
        }
    }

    /// Selects a TreeView item
    pub fn select_treeview_item(&self, label: &str, node_id: i32) -> PlatformResult<()> {
        info!("Selecting TreeView item with node_id: {}", node_id);
        unsafe {
            let hwnd = find_window(Some("SysTreeView32"), Some(label));
            if hwnd.0 == 0 {
                error!("TreeView with label '{}' not found", label);
                return Err(format!("TreeView with label '{}' not found", label));
            }
             send_message(hwnd, TVM_SELECTITEM, WPARAM(0), LPARAM(node_id as isize));
            Ok(())
        }
    }

    /// Expands a TreeView item
    pub fn expand_treeview_item(&self, label: &str, node_id: i32) -> PlatformResult<()> {
        info!("Expanding TreeView item with node_id: {}", node_id);
        unsafe {
            let hwnd = find_window(Some("SysTreeView32"), Some(label));
            if hwnd.0 == 0 {
                error!("TreeView with label '{}' not found", label);
                return Err(format!("TreeView with label '{}' not found", label));
            }
            send_message(hwnd, TVM_EXPAND, WPARAM(1), LPARAM(node_id as isize));
            Ok(())
        }
    }

     /// Selects an item from a ListView
    pub fn select_listview_item(&self, label: &str, index: usize) -> PlatformResult<()> {
        info!("Selecting ListView item at index: {}", index);
        unsafe {
            let hwnd = find_window(Some("SysListView32"), Some(label));
            if hwnd.0 == 0 {
                error!("ListView with label '{}' not found", label);
                return Err(format!("ListView with label '{}' not found", label));
            }
           send_message(hwnd, LVM_SETITEMSTATE, WPARAM(index), LPARAM(0));
            Ok(())
        }
    }

    /// Selects a tab in a TabControl
    pub fn select_tabcontrol_tab(&self, label: &str, index: usize) -> PlatformResult<()> {
        info!("Selecting TabControl tab at index: {}", index);
        unsafe {
            let hwnd = find_window(Some("SysTabControl32"), Some(label));
            if hwnd.0 == 0 {
                error!("TabControl with label '{}' not found", label);
                return Err(format!("TabControl with label '{}' not found", label));
            }
            send_message(hwnd, TCM_SETCURSEL, WPARAM(index), LPARAM(0));
            Ok(())
        }
    }

    /// Resizes a window
    pub fn resize_window(&self, label: &str, width: i32, height: i32) -> PlatformResult<()> {
         info!("Resizing window '{}' to {}x{}", label, width, height);

        unsafe {
            let hwnd = find_window(None, Some(label));
            if hwnd.0 == 0 {
                error!("Window with label '{}' not found", label);
                return Err(format!("Window with label '{}' not found", label));
            }
            if !SetWindowPos(hwnd, HWND(0), 0, 0, width, height, SWP_NOZORDER | SWP_NOACTIVATE).as_bool() {
               error!("Failed to resize window with label '{}'", label);
               return Err(format!("Failed to resize window with label '{}'", label));
            }
             Ok(())
        }
    }

    /// Moves a window
    pub fn move_window(&self, label: &str, x: i32, y: i32) -> PlatformResult<()> {
        info!("Moving window '{}' to {}, {}", label, x, y);

        unsafe {
           let hwnd = find_window(None, Some(label));
           if hwnd.0 == 0 {
                error!("Window with label '{}' not found", label);
                return Err(format!("Window with label '{}' not found", label));
            }
            if !SetWindowPos(hwnd, HWND(0), x, y, 0, 0, SWP_NOZORDER | SWP_NOACTIVATE | windows_sys::Win32::UI::WindowsAndMessaging::SWP_NOSIZE).as_bool() {
                error!("Failed to move window with label '{}'", label);
                return Err(format!("Failed to move window with label '{}'", label));
            }
             Ok(())
        }
    }

    /// Sends a KeyPress
     pub fn key_press(&self, key: &str) -> PlatformResult<()> {
        info!("Sending key press: {}", key);
         unsafe {
                let wide_key: Vec<u16> = key.encode_utf16().collect();
                for &code_point in &wide_key {
                        let mut input: INPUT = mem::zeroed();
                        input.r#type = windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT_KEYBOARD as u32;
                        input.Anonymous.ki.wVk = 0;
                        input.Anonymous.ki.wScan = code_point; // Unicode code point
                        input.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE;
                        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);

                        input.Anonymous.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
                        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
                }
              Ok(())
         }
    }

    /// Launches an application using ShellExecuteW
    pub fn launch_application(&self, app: &str) -> PlatformResult<()> {
        info!("Launching application: {}", app);
        unsafe {
             let wide_app = to_wide(app);
             let operation = to_wide("open");  // Operation is hardcoded
            let result = ShellExecuteW(
                HWND(0),
                operation.as_ptr(), // L"open"
                wide_app.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL as i32, // Show the application normally
            );
           if result.0 <= 32 {
               error!("Failed to launch application: {}", app);
                return Err(format!("Failed to launch application: {} with error code {}", app, result.0));
           }
            Ok(())
        }
    }
}
