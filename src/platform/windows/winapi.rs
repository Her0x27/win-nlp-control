#![allow(non_snake_case, unused_unsafe)]

use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM, BOOL, RECT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowTextW, GetWindowTextLengthW, SendMessageW, ShowWindow, SetWindowPos,
    SW_MAXIMIZE, SW_MINIMIZE, SW_SHOWNORMAL, WM_CLOSE, WM_GETTEXT, WM_GETTEXTLENGTH,
    WM_SETTEXT, EnumWindows, IsWindowVisible, EnumChildWindows, GetClassNameW,
    GetClientRect
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{INPUT, SendInput, INPUT_0, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY};
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, PROCESS_TERMINATE, GetWindowThreadProcessId
};
use windows_sys::Win32::System::Memory::{
     GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE
};
use windows_sys::Win32::Graphics::Gdi::{
    GetDC, CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, BitBlt, DeleteDC, DeleteObject,
    SRCCOPY, GetDeviceCaps, HORZRES, VERTRES
};
use windows_sys::Win32::System::Clipboard::{
    OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard, CF_UNICODETEXT
};
use windows_sys::core::{PCWSTR, w, PSTR};
use std::ffi::OsString;
use std::os::windows::prelude::*;
use std::mem;

use log::{warn};

// --- Helper functions to reduce boilerplate and improve safety ---

/// Converts a Rust String to a wide string (UTF-16) suitable for WinAPI.
pub fn to_wide(s: &str) -> Vec<u16> {
    OsString::from(s).encode_wide().chain(Some(0)).collect()
}

// --- Window Management Functions ---

/// Finds a window by class name and window name (title).  Returns `HWND(0)` on failure.
pub unsafe fn find_window(class_name: Option<&str>, window_name: Option<&str>) -> HWND {
    let class_name_wide = class_name.map(|s| to_wide(s));
    let window_name_wide = window_name.map(|s| to_wide(s));

    let class_name_ptr = class_name_wide.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null());
    let window_name_ptr = window_name_wide.as_ref().map(|s| s.as_ptr()).unwrap_or(std::ptr::null());
    FindWindowW(class_name_ptr as PCWSTR, window_name_ptr as PCWSTR)
}

/// Gets the text of a window.
pub unsafe fn get_window_text(hwnd: HWND) -> Option<String> {
    let len = GetWindowTextLengthW(hwnd) as usize;
    if len == 0 {
        return None;
    }

    let mut buffer: Vec<u16> = vec![0; len + 1]; // +1 for null terminator

    let result = GetWindowTextW(hwnd, buffer.as_mut_ptr(), (len + 1) as i32);

    if result == 0 {
        return None; // Or handle error appropriately
    }

    // Convert from UTF-16 to String, stopping at the first null
    String::from_utf16(&buffer[..len]).ok() // Stop at null terminator, ignore errors
}

/// Sets the text of a window.
pub unsafe fn set_window_text(hwnd: HWND, text: &str) -> bool {
    let wide_text = to_wide(text);
    let result = SendMessageW(hwnd, WM_SETTEXT, WPARAM(0), LPARAM(wide_text.as_ptr() as isize));
    result.0 != 0
}

/// Sends a message to a window.
pub unsafe fn send_message(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> isize {
    SendMessageW(hwnd, msg, wparam, lparam).0
}

/// Shows or hides a window.
pub unsafe fn show_window(hwnd: HWND, command: i32) -> bool {
    ShowWindow(hwnd, command).as_bool()
}

/// Sets the position and size of a window.
pub unsafe fn set_window_pos(hwnd: HWND, hwnd_insert_after: HWND, x: i32, y: i32, cx: i32, cy: i32, flags: u32) -> bool {
    SetWindowPos(hwnd, hwnd_insert_after, x, y, cx, cy, flags).as_bool()
}

// --- Clipboard Functions ---
pub unsafe fn open_and_set_clipboard(text: &str) -> bool {
    if OpenClipboard(HWND(0)).as_bool() {
        EmptyClipboard();

        let wide_text = to_wide(text);
        let len_in_bytes = wide_text.len() * 2;  // UTF-16: 2 bytes per character

        let hglobal = GlobalAlloc(GMEM_MOVEABLE, len_in_bytes as usize);
        if hglobal.0 == 0 {
             warn!("GlobalAlloc failed");
            CloseClipboard();
            return false;
        }

        let global_ptr = GlobalLock(hglobal) as *mut u16;
        if global_ptr.is_null() {
            warn!("GlobalLock failed");
            GlobalUnlock(hglobal);
            CloseClipboard();
            return false;
        }

        // Copy the UTF-16 string into the global memory
        std::ptr::copy_nonoverlapping(wide_text.as_ptr(), global_ptr, wide_text.len());

        GlobalUnlock(hglobal);

        // Set the clipboard data
        let result = SetClipboardData(CF_UNICODETEXT, hglobal);
        CloseClipboard(); // Always close the clipboard

        result.0 != 0
    } else {
         warn!("OpenClipboard failed");
        false
    }
}

// --- Window Enumeration Functions ---

// Define a more Rust-friendly callback type
pub type EnumWindowsCallback = Box<dyn FnMut(HWND) -> bool + Send + Sync>;

/// Enumerate all top-level windows.
pub unsafe fn enum_windows(callback: EnumWindowsCallback) -> bool {
    // We need to bridge the Rust-style closure to the C-style callback.
    // We use a raw pointer to the closure and transmute it to the expected type.
    // This requires careful management to avoid memory unsafety.
    let mut closure = callback; // Move the closure into this scope
    let closure_ptr: *mut dyn FnMut(HWND) -> bool = &mut *closure; // Create a raw pointer

    // Transmute the raw pointer to the correct C-style callback type.
    let enum_windows_proc: unsafe extern "system" fn(HWND, LPARAM) -> BOOL = transmute_closure(closure_ptr);

    // Call EnumWindows with the transmuted callback function and the LPARAM.
    EnumWindows(Some(enum_windows_proc), LPARAM(0)).as_bool()
}

unsafe fn transmute_closure(closure_ptr: *mut dyn FnMut(HWND) -> bool) -> unsafe extern "system" fn(HWND, LPARAM) -> BOOL {
    unsafe extern "system" fn enum_windows_proc_wrapper(hwnd: HWND, lparam: LPARAM) -> BOOL {
        // Reinterpret the LPARAM back into a pointer to the Rust closure.
        let closure_ptr = lparam.0 as *mut (dyn FnMut(HWND) -> bool);
        let closure = &mut *(closure_ptr);
        // Call the Rust closure.
        if closure(hwnd) {
            BOOL(1) // Continue enumeration
        } else {
            BOOL(0) // Stop enumeration
        }
    }
    mem::transmute(enum_windows_proc_wrapper::<dyn FnMut(HWND) -> bool>)
}

/// Enumerate all child windows.
pub unsafe fn enum_child_windows(hwnd: HWND, callback: EnumWindowsCallback) -> bool {
    let mut closure = callback; // Move the closure into this scope
    let closure_ptr: *mut dyn FnMut(HWND) -> bool = &mut *closure; // Create a raw pointer
    let enum_child_proc: unsafe extern "system" fn(HWND, LPARAM) -> BOOL = transmute_closure(closure_ptr);

    EnumChildWindows(hwnd, Some(enum_child_proc), LPARAM(0)).as_bool()
}

// --- Process Management Functions ---

/// Opens a process by its ID.  Requires PROCESS_TERMINATE rights for TerminateProcess.
pub unsafe fn open_process(process_id: u32) -> windows_sys::Win32::Foundation::HANDLE {
    OpenProcess(PROCESS_TERMINATE, 0, process_id)
}

/// Terminates a process.
pub unsafe fn terminate_process(process_handle: windows_sys::Win32::Foundation::HANDLE, exit_code: u32) -> bool {
    TerminateProcess(process_handle, exit_code).as_bool()
}

/// Gets the process ID for a window.
pub unsafe fn get_window_thread_process_id(hwnd: HWND) -> u32 {
    let mut process_id: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut process_id);
    process_id
}

// --- GDI Functions (Basic, for Screenshot) ---

/// Gets the device context for a window (or the entire screen if hWnd is NULL).
pub unsafe fn get_dc(hwnd: HWND) -> windows_sys::Win32::Graphics::Gdi::HDC {
    GetDC(hwnd)
}

/// Creates a compatible DC.
pub unsafe fn create_compatible_dc(hdc: windows_sys::Win32::Graphics::Gdi::HDC) -> windows_sys::Win32::Graphics::Gdi::HDC {
    CreateCompatibleDC(hdc)
}

/// Creates a compatible bitmap.
pub unsafe fn create_compatible_bitmap(hdc: windows_sys::Graphics::Gdi::HDC, width: i32, height: i32) -> windows_sys::Win32::Graphics::Gdi::HBITMAP {
    CreateCompatibleBitmap(hdc, width, height)
}

/// Selects an object into the specified device context.
pub unsafe fn select_object(hdc: windows_sys::Graphics::Gdi::HDC, hgdiobj: isize) -> isize {
    SelectObject(hdc, hgdiobj)
}

/// Performs a bit-block transfer.
pub unsafe fn bit_blt(
    hdc_dest: windows_sys::Graphics::Gdi::HDC,
    x_dest: i32,
    y_dest: i32,
    width: i32,
    height: i32,
    hdc_src: windows_sys::Graphics::Gdi::HDC,
    x_src: i32,
    y_src: i32,
    rop: u32,
) -> bool {
    BitBlt(hdc_dest, x_dest, y_dest, width, height, hdc_src, x_src, y_src, rop).as_bool()
}

/// Deletes a device context.
pub unsafe fn delete_dc(hdc: windows_sys::Graphics::Gdi::HDC) -> bool {
    DeleteDC(hdc).as_bool()
}

/// Deletes a GDI object.
pub unsafe fn delete_object(hgdiobj: isize) -> bool {
    DeleteObject(hgdiobj).as_bool()
}

/// Gets device capabilities.
pub unsafe fn get_device_caps(hdc: windows_sys::Graphics::Gdi::HDC, index: i32) -> i32 {
    GetDeviceCaps(hdc, index)
}
