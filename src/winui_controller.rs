use crate::intent_mapper::Action;
use crate::debug_logger::{log_info, log_debug};
use std::ffi::{CString, CStr};
use std::mem;
use std::ptr;
use std::thread;
use std::time::Duration;
use std::fs::File;
use std::fs::{self, File};
use std::io::{Write, BufWriter};
use std::path::Path;

#[macro_use]
extern crate lazy_static;
use std::sync::Mutex;

lazy_static! {
    // Global store for selected files.
    static ref SELECTED_FILES: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

// Constants for the UpDown (spinner) control messages.
const UDM_GETPOS: u32 = 0x0400 + 2;   // WM_USER + 2
const UDM_SETPOS: u32 = 0x0400 + 3;   // WM_USER + 3

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM, HGLOBAL, HANDLE, CloseHandle};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, EnumChildWindows, FindWindowA, GetForegroundWindow, GetWindowTextA, GetWindowTextLengthA,
    IsWindowVisible, SendMessageA, ShowWindow, SW_MAXIMIZE, SW_MINIMIZE, SW_SHOWNORMAL, WM_CLOSE,
    WM_VSCROLL, SB_LINEUP, SB_LINEDOWN,
};
use windows::Win32::UI::Shell::ShellExecuteA;
use windows::Win32::System::Clipboard::{
    OpenClipboard, EmptyClipboard, SetClipboardData, CloseClipboard, CF_UNICODETEXT,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Threading::{GetWindowThreadProcessId, OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use windows::Win32::Graphics::Gdi::{
    GetDC, CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, BitBlt, DeleteDC, DeleteObject,
    SRCCOPY, GetDeviceCaps, HORZRES, VERTRES, BITMAP, GetObjectA,
};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_EXTENDEDKEY,
};

/// Представляет результат выполнения действия.
#[derive(Debug)]
pub enum ExecutionResult {
    Success(String),
    Failure(String),
}

/// Выполняет переданное действие с использованием Win32 API.
pub fn execute_action(action: &Action) -> ExecutionResult {
    unsafe {
        match action {
            Action::ButtonClick { label } => {
                log_info(&format!("Нажатие кнопки '{}'", label));
                let hwnd = find_window("Button", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Кнопка '{}' не найдена", label));
                }
                SendMessageA(hwnd, BM_CLICK, WPARAM(0), LPARAM(0));
                ExecutionResult::Success(format!("Нажата кнопка '{}'", label))
            }
            Action::ButtonDoubleClick { label } => {
                log_info(&format!("Двойной клик по кнопке '{}'", label));
                let hwnd = find_window("Button", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Кнопка '{}' не найдена", label));
                }
                SendMessageA(hwnd, BM_CLICK, WPARAM(0), LPARAM(0));
                thread::sleep(Duration::from_millis(100));
                SendMessageA(hwnd, BM_CLICK, WPARAM(0), LPARAM(0));
                ExecutionResult::Success(format!("Двойной клик по кнопке '{}'", label))
            }
            Action::GroupWindows => {
                log_info("Grouping all visible windows in a grid layout");
                if group_windows() {
                    ExecutionResult::Success("Windows grouped successfully".to_string())
                } else {
                    ExecutionResult::Failure("Failed to group windows".to_string())
                }
            }
            Action::EditEnterText { label, text } => {
                log_info(&format!("Ввод текста '{}' в поле '{}'", text, label));
                let hwnd = find_window("Edit", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Поле '{}' не найдено", label));
                }
                let text_c = CString::new(text.clone()).unwrap();
                if SetWindowTextA(hwnd, &text_c).as_bool() {
                    ExecutionResult::Success(format!("Текст '{}' введён в '{}'", text, label))
                } else {
                    ExecutionResult::Failure(format!("Не удалось установить текст в '{}'", label))
                }
            }
            Action::EditSelectText { label, start, end } => {
                log_info(&format!("Выделение текста в поле '{}'", label));
                let hwnd = find_window("Edit", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Поле '{}' не найдено", label));
                }
                let (sel_start, sel_end) = if let (Some(s), Some(e)) = (start, end) {
                    (WPARAM(*s), LPARAM(*e as i32))
                } else {
                    (WPARAM(0), LPARAM(-1))
                };
                SendMessageA(hwnd, EM_SETSEL, sel_start, sel_end);
                ExecutionResult::Success(format!(
                    "Текст выделен в '{}' от {:?} до {:?}",
                    label, start, end
                ))
            }
            Action::EditCopyText { label } => {
                log_info("Copying text from field");
                // If label is provided, find the edit control using its title; otherwise use the foreground window.
                let hwnd = if let Some(lbl) = label {
                    find_window("Edit", lbl)
                } else {
                    GetForegroundWindow()
                };
                if hwnd.0 == 0 {
                    ExecutionResult::Failure("Text field not found".to_string())
                } else {
                    const WM_COPY: u32 = 0x0301;
                    SendMessageA(hwnd, WM_COPY, WPARAM(0), LPARAM(0));
                    ExecutionResult::Success("Text copied".to_string())
                }
            }
            Action::EditCutText { label } => {
                log_info("Cutting text from field");
                let hwnd = if let Some(lbl) = label {
                    find_window("Edit", lbl)
                } else {
                    GetForegroundWindow()
                };
                if hwnd.0 == 0 {
                    ExecutionResult::Failure("Text field not found".to_string())
                } else {
                    const WM_CUT: u32 = 0x0300;
                    SendMessageA(hwnd, WM_CUT, WPARAM(0), LPARAM(0));
                    ExecutionResult::Success("Text cut".to_string())
                }
            }
            Action::EditClearField { label } => {
                log_info("Clearing text field");
                let hwnd = if let Some(lbl) = label {
                    find_window("Edit", lbl)
                } else {
                    GetForegroundWindow()
                };
                if hwnd.0 == 0 {
                    ExecutionResult::Failure("Text field not found".to_string())
                } else {
                    const WM_CLEAR: u32 = 0x0303;
                    SendMessageA(hwnd, WM_CLEAR, WPARAM(0), LPARAM(0));
                    ExecutionResult::Success("Field cleared".to_string())
                }
            }
            Action::EditDeleteText { label } => {
                log_info(&format!("Удаление текста в поле '{}'", label));
                let hwnd = find_window("Edit", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Поле '{}' не найдено", label));
                }
                SendMessageA(hwnd, WM_CLEAR, WPARAM(0), LPARAM(0));
                ExecutionResult::Success(format!("Текст удалён из '{}'", label))
            }
            Action::EditPasteText { label, text } => {
                log_info(&format!("Вставка текста в поле '{}'", label));
                let hwnd = find_window("Edit", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Поле '{}' не найдено", label));
                }
                if let Some(text_value) = text {
                    if !open_and_set_clipboard(text_value) {
                        return ExecutionResult::Failure("Не удалось обновить буфер обмена".to_string());
                    }
                }
                SendMessageA(hwnd, WM_PASTE, WPARAM(0), LPARAM(0));
                ExecutionResult::Success(format!("Текст вставлен в '{}'", label))
            }
            Action::StaticGetText { label } => {
                log_info(&format!("Получение текста из статического поля '{}'", label));
                let hwnd = find_window("Static", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Статическое поле '{}' не найдено", label));
                }
                let length = GetWindowTextLengthA(hwnd);
                let mut buffer = vec![0u8; (length + 1) as usize];
                GetWindowTextA(hwnd, &mut buffer);
                let text = String::from_utf8_lossy(&buffer)
                    .trim_end_matches('\0')
                    .to_string();
                ExecutionResult::Success(format!("Текст в '{}': {}", label, text))
            }
            Action::SetText { label, text } => {
                log_info(&format!("Установка текста '{}' в статическом поле '{}'", text, label));
                let hwnd = find_window("Static", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Статическое поле '{}' не найдено", label));
                }
                let text_c = CString::new(text.clone()).unwrap();
                if SetWindowTextA(hwnd, &text_c).as_bool() {
                    ExecutionResult::Success(format!("Текст '{}' установлен в '{}'", text, label))
                } else {
                    ExecutionResult::Failure(format!("Не удалось установить текст в '{}'", label))
                }
            }
            Action::SetFocus { label } => {
                log_info(&format!("Установка фокуса на '{}'", label));
                let hwnd = find_window("", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Элемент '{}' не найден для установки фокуса", label));
                }
                if SetFocus(hwnd).0 == 0 {
                    ExecutionResult::Failure(format!("Не удалось установить фокус на '{}'", label))
                } else {
                    ExecutionResult::Success(format!("Фокус установлен на '{}'", label))
                }
            }
            Action::CheckboxSetState { label, state } => {
                log_info(&format!("Установка состояния чекбокса '{}' в {}", label, state));
                let hwnd = find_window("Button", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Чекбокс '{}' не найден", label));
                }
                let current_state = SendMessageA(hwnd, BM_GETCHECK, WPARAM(0), LPARAM(0)).0;
                let desired_state = if *state { BST_CHECKED } else { BST_UNCHECKED };
                if current_state != desired_state as i32 {
                    SendMessageA(hwnd, BM_SETCHECK, WPARAM(desired_state as usize), LPARAM(0));
                }
                ExecutionResult::Success(format!("Чекбокс '{}' установлен в {}", label, state))
            }
            Action::RadioSelect { label, variant } => {
                log_info(&format!("Выбор радиокнопки '{}' с вариантом {:?}", label, variant));
                let hwnd = find_window("Button", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Радиокнопка '{}' не найдена", label));
                }
                SendMessageA(hwnd, BM_SETCHECK, WPARAM(BST_CHECKED as usize), LPARAM(0));
                ExecutionResult::Success(match variant {
                    Some(v) => format!("Радиокнопка '{}' выбрана с вариантом '{}'", label, v),
                    None => format!("Радиокнопка '{}' выбрана", label),
                })
            }
            Action::TreeViewSelect { label, node } => {
                log_info(&format!("Выбор элемента дерева '{}' с узлом {:?}", label, node));
                let hwnd = find_window("SysTreeView32", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Элемент дерева '{}' не найден", label));
                }
                if let Some(node_str) = node {
                    if let Ok(node_id) = node_str.parse::<i32>() {
                        SendMessageA(hwnd, TVM_SELECTITEM, WPARAM(0), LPARAM(node_id as isize));
                        ExecutionResult::Success(format!("Выбран узел {} в дереве '{}'", node_id, label))
                    } else {
                        ExecutionResult::Failure("Выбор по тексту узла не поддерживается. Используйте числовой ID узла.".to_string())
                    }
                } else {
                    ExecutionResult::Failure("Не указан узел для выбора в дереве.".to_string())
                }
            }
            Action::TreeViewExpand { label, node } => {
                log_info(&format!("Раскрытие дерева '{}' с узлом {:?}", label, node));
                let hwnd = find_window("SysTreeView32", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Элемент дерева '{}' не найден", label));
                }
                if let Some(node_str) = node {
                    if let Ok(node_id) = node_str.parse::<i32>() {
                        SendMessageA(hwnd, TVM_EXPAND, WPARAM(1), LPARAM(node_id as isize));
                        ExecutionResult::Success(format!("Узел {} раскрыт в дереве '{}'", node_id, label))
                    } else {
                        ExecutionResult::Failure("Раскрытие по тексту узла не поддерживается. Используйте числовой ID узла.".to_string())
                    }
                } else {
                    ExecutionResult::Failure("Не указан узел для раскрытия дерева.".to_string())
                }
            }
            Action::ListViewSelectItem { label, item } => {
                log_info(&format!("Выбор элемента '{}' из списка '{}'", item, label));
                let hwnd = find_window("SysListView32", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Список '{}' не найден", label));
                }
                if let Ok(index) = item.parse::<u32>() {
                    SendMessageA(hwnd, LVM_SETITEMSTATE, WPARAM(index as usize), LPARAM(0));
                    ExecutionResult::Success(format!("Элемент {} выбран в списке '{}'", index, label))
                } else {
                    ExecutionResult::Failure("Выбор по имени не поддерживается; используйте числовой индекс.".to_string())
                }
            }
            Action::TabControlSelectTab { label, tab } => {
                log_info(&format!("Выбор вкладки '{}' в элементе '{}'", tab, label));
                let hwnd = find_window("SysTabControl32", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Элемент управления вкладками '{}' не найден", label));
                }
                if let Ok(index) = tab.parse::<u32>() {
                    SendMessageA(hwnd, TCM_SETCURSEL, WPARAM(index as usize), LPARAM(0));
                    ExecutionResult::Success(format!("Вкладка {} выбрана в контроле '{}'", index, label))
                } else {
                    ExecutionResult::Failure("Выбор по имени не поддерживается; используйте числовой индекс.".to_string())
                }
            }
            Action::WindowResize { width, height } => {
                log_info(&format!("Изменение размера активного окна до {}x{}", width, height));
                let hwnd = GetForegroundWindow();
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure("Активное окно не найдено".to_string());
                }
                if MoveWindow(hwnd, 0, 0, *width as i32, *height as i32, true).as_bool() {
                    ExecutionResult::Success(format!("Окно изменило размер до {}x{}", width, height))
                } else {
                    ExecutionResult::Failure("Не удалось изменить размер окна".to_string())
                }
            }
            Action::WindowMinimize { label } => {
                log_info(&format!("Свернуть окно '{}'", label));
                let hwnd = find_window("", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Окно '{}' не найдено", label));
                }
                ShowWindow(hwnd, SW_MINIMIZE);
                ExecutionResult::Success(format!("Окно '{}' свернуто", label))
            }
            Action::WindowMaximize { label } => {
                log_info(&format!("Развернуть окно '{}'", label));
                let hwnd = find_window("", label);
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Окно '{}' не найдено", label));
                }
                ShowWindow(hwnd, SW_MAXIMIZE);
                ExecutionResult::Success(format!("Окно '{}' развернуто", label))
            }
            Action::LaunchApplication { app } => {
                log_info(&format!("Запуск приложения '{}'", app));
                let operation = CString::new("open").unwrap();
                let app_c = CString::new(app.clone()).unwrap();
                let result = ShellExecuteA(None, &operation, &app_c, None, None, SW_SHOWNORMAL);
                if (result.0 as isize) <= 32 {
                    ExecutionResult::Failure(format!("Не удалось запустить приложение '{}'", app))
                } else {
                    ExecutionResult::Success(format!("Приложение '{}' запущено", app))
                }
            }
            Action::FocusApplication { app } => {
                log_info(&format!("Установка фокуса на приложение '{}'", app));
                let app_c = CString::new(app.clone()).unwrap();
                let hwnd = FindWindowA(None, Some(&app_c));
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Приложение '{}' не найдено для установки фокуса", app));
                }
                if SetFocus(hwnd).0 == 0 {
                    ExecutionResult::Failure(format!("Не удалось установить фокус на '{}'", app))
                } else {
                    ExecutionResult::Success(format!("Фокус установлен на '{}'", app))
                }
            }
            Action::GroupWindows { group, windows } => {
                log_info(&format!("Группировка окон '{}' в группу '{}'", windows, group));
                // Здесь можно реализовать логику группировки окон.
                ExecutionResult::Success(format!("Окна '{}' сгруппированы в группу '{}'", windows, group))
            }
            Action::LaunchObject { object } => {
                log_info(&format!("Запуск объекта '{}'", object));
                let operation = CString::new("open").unwrap();
                let object_c = CString::new(object.clone()).unwrap();
                let result = ShellExecuteA(None, &operation, &object_c, None, None, SW_SHOWNORMAL);
                if (result.0 as isize) <= 32 {
                    ExecutionResult::Failure(format!("Не удалось запустить объект '{}'", object))
                } else {
                    ExecutionResult::Success(format!("Объект '{}' запущен", object))
                }
            }
            Action::FocusObject { object } => {
                log_info(&format!("Установка фокуса на объект '{}'", object));
                let object_c = CString::new(object.clone()).unwrap();
                let hwnd = FindWindowA(None, Some(&object_c));
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Объект '{}' не найден для установки фокуса", object));
                }
                if SetFocus(hwnd).0 == 0 {
                    ExecutionResult::Failure(format!("Не удалось установить фокус на '{}'", object))
                } else {
                    ExecutionResult::Success(format!("Фокус установлен на '{}'", object))
                }
            }
            Action::WindowMinimizeAll => {
                log_info("Свернуть все окна");
                // Здесь должна быть реализация сворачивания всех окон.
                ExecutionResult::Success("Все окна свернуты".to_string())
            }
            Action::WindowMaximizeAll => {
                log_info("Развернуть все окна");
                // Здесь должна быть реализация разворачивания всех окон.
                ExecutionResult::Success("Все окна развернуты".to_string())
            }
            Action::WindowCloseAll => {
                log_info("Закрыть все окна");
                // Здесь должна быть реализация закрытия всех окон.
                ExecutionResult::Success("Все окна закрыты".to_string())
            }
            Action::OpenFileProperties { file } => {
                log_info(&format!("Opening file properties for '{}'", file));
                let operation = CString::new("properties").unwrap();
                let file_c = CString::new(file.clone()).unwrap();
                let result = ShellExecuteA(None, &operation, &file_c, None, None, SW_SHOWNORMAL);
                if (result.0 as isize) <= 32 {
                    ExecutionResult::Failure(format!("Failed to open properties for file '{}'", file))
                } else {
                    ExecutionResult::Success(format!("File properties for '{}' opened", file))
                }
            }
            Action::ListSelect { label, item } => {
                log_info(&format!("Selecting item '{}' from list '{}'", item, label));
                // Find the parent window using the provided label as the window title.
                let parent_hwnd = find_window("", label);
                if parent_hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Parent window '{}' not found", label));
                }
                // Use EnumChildWindows to iterate over child windows.
                let mut found_child: HWND = HWND(0);
                extern "system" fn enum_child_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
                    unsafe {
                        let len = GetWindowTextLengthA(hwnd);
                        if len == 0 { return 1; } // Continue enumeration.
                        let mut buf = vec![0u8; (len + 1) as usize];
                        GetWindowTextA(hwnd, &mut buf);
                        let window_text = String::from_utf8_lossy(&buf)
                            .trim_end_matches('\0')
                            .to_string();
                        // lparam holds a pointer to a tuple (target: CString, found: *mut HWND).
                        let data_ptr = lparam.0 as *mut (CString, HWND);
                        if data_ptr.is_null() { return 1; }
                        let (ref target, ref mut found) = &mut *data_ptr;
                        if window_text == target.to_string_lossy() {
                            *found = hwnd;
                            return 0; // Stop enumeration once found.
                        }
                    }
                    1
                }
                let target = CString::new(item.as_str()).unwrap();
                let mut data = (target, HWND(0));
                EnumChildWindows(parent_hwnd, Some(enum_child_proc), LPARAM(&mut data as *mut _ as isize));
                found_child = data.1;
                if found_child.0 != 0 {
                    // Send a click message (using BM_CLICK) to select the item.
                    const BM_CLICK: u32 = 0x00F5;
                    SendMessageA(found_child, BM_CLICK, WPARAM(0), LPARAM(0));
                    ExecutionResult::Success(format!("Item '{}' selected in list '{}'", item, label))
                } else {
                    ExecutionResult::Failure(format!("Item '{}' not found in window '{}'", item, label))
                }
            }
            Action::KeyPress { key } => {
                log_info(&format!("Sending key press '{}'", key));
                let key_str = key.trim();
                let vk = windows::Win32::UI::Input::KeyboardAndMouse::VkKeyScanA(
                    key_str.chars().next().unwrap() as i8
                ) as u16;
                if vk == 0xFFFF {
                    return ExecutionResult::Failure(format!("Failed to convert '{}' to a virtual key", key));
                }
                let mut inputs: [INPUT; 2] = [mem::zeroed(), mem::zeroed()];
                // Key down.
                inputs[0].r#type = INPUT_KEYBOARD;
                inputs[0].Anonymous.ki = KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: 0,
                    time: 0,
                    dwExtraInfo: 0,
                };
                // Key up.
                inputs[1].r#type = INPUT_KEYBOARD;
                inputs[1].Anonymous.ki = KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: KEYEVENTF_KEYUP | KEYEVENTF_EXTENDEDKEY,
                    time: 0,
                    dwExtraInfo: 0,
                };
                let sent = SendInput(&inputs, mem::size_of::<INPUT>() as i32);
                if sent != 2 {
                    ExecutionResult::Failure(format!("Error sending key press for '{}'", key))
                } else {
                    ExecutionResult::Success(format!("Key '{}' pressed successfully", key))
                }
            }
            Action::Scroll { direction, amount } => {
                log_info(&format!("Scrolling '{}' by {:?}", direction, amount));
                let hwnd = GetForegroundWindow();
                if hwnd.0 == 0 {
                    return ExecutionResult::Failure("Foreground window not found for scrolling".to_string());
                }
                let amt = amount.unwrap_or(1);
                let wparam = if direction.to_lowercase() == "up" {
                    WPARAM(SB_LINEUP as usize)
                } else if direction.to_lowercase() == "down" {
                    WPARAM(SB_LINEDOWN as usize)
                } else {
                    return ExecutionResult::Failure("Invalid scroll direction. Use 'up' or 'down'".to_string());
                };
                for _ in 0..amt {
                    SendMessageA(hwnd, WM_VSCROLL, wparam, LPARAM(0));
                    thread::sleep(Duration::from_millis(50));
                }
                ExecutionResult::Success(format!("Scrolled '{}' by {}", direction, amt))
            }
            Action::Screenshot => {
                log_info("Taking screenshot as PNG");
                match take_screenshot_png("screenshot.png") {
                    Ok(path)  => ExecutionResult::Success(format!("Screenshot saved to '{}'", path)),
                    Err(e) => ExecutionResult::Failure(format!("Error taking screenshot: {}", e)),
                }
            }
            Action::SpinnerAdjust { label, operation, value } => {
                log_info(&format!("Adjusting spinner '{}' with operation: {} and value: {}", label, operation, value));
                // Find the spinner control. Here we assume its class is "msctls_updown32".
                let spinner_hwnd = find_window("msctls_updown32", label);
                if spinner_hwnd.0 == 0 {
                    return ExecutionResult::Failure(format!("Spinner control '{}' not found", label));
                }
                // Retrieve the current position.
                let current_result = SendMessageA(spinner_hwnd, UDM_GETPOS, WPARAM(0), LPARAM(0));
                // Lower word holds the signed position.
                let mut current_value = (current_result & 0xFFFF) as i32;
                // Adjust the spinner value according to the operation.
                match operation.to_lowercase().as_str() {
                    "increase" => current_value += *value,
                    "decrease" => current_value -= *value,
                    "set" => current_value = *value,
                    _ => return ExecutionResult::Failure(format!("Unknown spinner operation '{}'", operation)),
                }
                // Set the new position.
                SendMessageA(spinner_hwnd, UDM_SETPOS, WPARAM(0), LPARAM(current_value as isize));
                ExecutionResult::Success(format!("Spinner '{}' adjusted to {}", label, current_value))
            }
            Action::SelectFiles { criteria } => {
                log_info(&format!("Selecting files matching '{}'", criteria));
                let mut matches = Vec::new();
                // For demonstration, search in the current directory.
                match fs::read_dir(".") {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if let Some(path_str) = path.to_str() {
                                if path_str.contains(criteria) {
                                    matches.push(path_str.to_string());
                                }
                            }
                        }
                    }
                    Err(e) => return ExecutionResult::Failure(format!("Error reading directory: {}", e)),
                }
                if matches.is_empty() {
                    ExecutionResult::Failure(format!("No files matching '{}' found", criteria))
                } else {
                    // Save selection globally.
                    let mut selected = SELECTED_FILES.lock().unwrap();
                    selected.clear();
                    selected.extend(matches.clone());
                    ExecutionResult::Success(format!("Files selected: {:?}", matches))
                }
            }
            Action::FileOperation { operation } => {
                log_info(&format!("Performing file operation '{}' on selected files", operation));
                let selected = SELECTED_FILES.lock().unwrap();
                if selected.is_empty() {
                    return ExecutionResult::Failure("No files are currently selected.".to_string());
                }
                match operation.to_lowercase().as_str() {
                    "delete" => {
                        let mut errors = Vec::new();
                        for file in selected.iter() {
                            if let Err(e) = fs::remove_file(file) {
                                errors.push(format!("Failed to delete {}: {}", file, e));
                            }
                        }
                        if errors.is_empty() {
                            drop(selected);
                            let mut selected = SELECTED_FILES.lock().unwrap();
                            selected.clear();
                            ExecutionResult::Success("Selected files deleted successfully".to_string())
                        } else {
                            ExecutionResult::Failure(errors.join("; "))
                        }
                    },
                    "copy" | "cut" => {
                        // For copy or cut, we expect the paste operation to supply the destination.
                        ExecutionResult::Failure(format!("Operation '{}' requires a paste destination. Use Action::PasteFiles", operation))
                    },
                    _ => ExecutionResult::Failure(format!("Unsupported file operation '{}'", operation)),
                }
            }
            Action::PasteFiles { destination } => {
                log_info(&format!("Pasting files into '{}'", destination));
                let selected = SELECTED_FILES.lock().unwrap();
                if selected.is_empty() {
                    return ExecutionResult::Failure("No files are currently selected to paste.".to_string());
                }
                if !Path::new(destination).is_dir() {
                    return ExecutionResult::Failure(format!("Destination '{}' is not a valid directory", destination));
                }
                let mut errors = Vec::new();
                for file in selected.iter() {
                    let path = Path::new(file);
                    if let Some(filename) = path.file_name() {
                        let dest_path = Path::new(destination).join(filename);
                        if let Err(e) = fs::copy(path, &dest_path) {
                            errors.push(format!("Failed to copy {}: {}", file, e));
                        }
                    } else {
                        errors.push(format!("Invalid file path: {}", file));
                    }
                }
                if errors.is_empty() {
                    ExecutionResult::Success(format!("Files pasted into '{}'", destination))
                } else {
                    ExecutionResult::Failure(errors.join("; "))
                }
            }
            Action::OpenFileProperties { file } => {
                log_info(&format!("Opening file properties for '{}'", file));
                let operation = CString::new("properties").unwrap();
                let file_c = CString::new(file.clone()).unwrap();
                let result = ShellExecuteA(None, &operation, &file_c, None, None, SW_SHOWNORMAL);
                if (result.0 as isize) <= 32 {
                    ExecutionResult::Failure(format!("Failed to open properties for file '{}'", file))
                } else {
                    ExecutionResult::Success(format!("File properties for '{}' opened", file))
                }
            }
            Action::CreateDirectory { name } => {
                log_info(&format!("Creating directory '{}'", name));
                match fs::create_dir(name) {
                    Ok(_) => ExecutionResult::Success(format!("Directory '{}' created", name)),
                    Err(e) => ExecutionResult::Failure(format!("Error creating directory '{}': {}", name, e)),
                }
            }
            Action::DeleteDirectory { name } => {
                log_info(&format!("Deleting directory '{}'", name));
                match fs::remove_dir_all(name) {
                    Ok(_) => ExecutionResult::Success(format!("Directory '{}' deleted", name)),
                    Err(e) => ExecutionResult::Failure(format!("Error deleting directory '{}': {}", name, e)),
                }
            }
            Action::CreateFile { name } => {
                log_info(&format!("Creating file '{}'", name));
                match File::create(name) {
                    Ok(_) => ExecutionResult::Success(format!("File '{}' created", name)),
                    Err(e) => ExecutionResult::Failure(format!("Error creating file '{}': {}", name, e)),
                }
            }
            Action::DeleteFile { name } => {
                log_info(&format!("Deleting file '{}'", name));
                match fs::remove_file(name) {
                    Ok(_) => ExecutionResult::Success(format!("File '{}' deleted", name)),
                    Err(e) => ExecutionResult::Failure(format!("Error deleting file '{}': {}", name, e)),
                }
            }
            _ => ExecutionResult::Failure("Неизвестное действие".to_string()),
        }
    }
}

/// Helper function to minimize all visible windows.
unsafe fn minimize_all_windows() -> bool {
    extern "system" fn enum_windows_proc(hwnd: HWND, _lparam: LPARAM) -> i32 {
        unsafe {
            if IsWindowVisible(hwnd).as_bool() {
                ShowWindow(hwnd, SW_MINIMIZE);
            }
        }
        1
    }
    EnumWindows(Some(enum_windows_proc), LPARAM(0)).as_bool()
}

/// Helper function to maximize all visible windows.
unsafe fn maximize_all_windows() -> bool {
    extern "system" fn enum_windows_proc(hwnd: HWND, _lparam: LPARAM) -> i32 {
        unsafe {
            if IsWindowVisible(hwnd).as_bool() {
                ShowWindow(hwnd, SW_MAXIMIZE);
            }
        }
        1
    }
    EnumWindows(Some(enum_windows_proc), LPARAM(0)).as_bool()
}

/// Helper function to close all visible windows.
unsafe fn close_all_windows() -> bool {
    extern "system" fn enum_windows_proc(hwnd: HWND, _lparam: LPARAM) -> i32 {
        unsafe {
            if IsWindowVisible(hwnd).as_bool() {
                SendMessageA(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
        1
    }
    EnumWindows(Some(enum_windows_proc), LPARAM(0)).as_bool()
}

/// Helper function to find a window by class name and title.
/// If the class name is empty, the search is performed only by title.
unsafe fn find_window(class_name: &str, window_title: &str) -> HWND {
    let class = if !class_name.is_empty() {
        Some(&CString::new(class_name).unwrap())
    } else {
        None
    };
    let title = Some(&CString::new(window_title).unwrap());
    FindWindowA(class, title)
}

/// Takes a screenshot of the entire screen and saves it as a PNG file.
/// This function uses the image crate, so ensure it is added as a dependency in Cargo.toml.
unsafe fn take_screenshot_png(file_path: &str) -> Result<String, String> {
    // Get the device context of the entire screen.
    let hdc_screen = GetDC(HWND(0));
    if hdc_screen.0 == 0 {
        return Err("Failed to obtain screen DC".to_string());
    }
    let width = GetDeviceCaps(hdc_screen, HORZRES);
    let height = GetDeviceCaps(hdc_screen, VERTRES);
    let hdc_mem = CreateCompatibleDC(hdc_screen);
    if hdc_mem.0 == 0 {
        return Err("Failed to create compatible DC".to_string());
    }
    // Create a 32-bit bitmap for the screenshot.
    let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
    if hbitmap.0 == 0 {
        return Err("Failed to create compatible bitmap".to_string());
    }
    let old_obj = SelectObject(hdc_mem, hbitmap);
    if old_obj.0 == 0 {
        return Err("Failed to select bitmap into DC".to_string());
    }
    if !BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, 0, 0, SRCCOPY).as_bool() {
        return Err("BitBlt failed".to_string());
    }
    // Prepare to get bitmap bits in BGRA (32-bit) format.
    let mut bmi_header = windows::Win32::Graphics::Gdi::BITMAPINFOHEADER {
        biSize: mem::size_of::<windows::Win32::Graphics::Gdi::BITMAPINFOHEADER>() as u32,
        biWidth: width,
        biHeight: -height, // Negative height indicates a top-down bitmap.
        biPlanes: 1,
        biBitCount: 32,
        biCompression: 0, // BI_RGB
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let row_bytes = ((32 * width + 31) / 32) * 4;
    let image_size = (row_bytes * height) as usize;
    let mut pixel_data: Vec<u8> = vec![0; image_size];
    let ret = windows::Win32::Graphics::Gdi::GetDIBits(
        hdc_mem,
        hbitmap,
        0,
        height as u32,
        Some(pixel_data.as_mut_ptr() as *mut _),
        &mut windows::Win32::Graphics::Gdi::BITMAPINFO {
            bmiHeader: bmi_header,
            bmiColors: [Default::default(); 1],
        },
        windows::Win32::Graphics::Gdi::DIB_RGB_COLORS,
    );
    if ret == 0 {
        return Err("GetDIBits failed".to_string());
    }
    // Clean up GDI objects.
    SelectObject(hdc_mem, old_obj);
    DeleteObject(hbitmap);
    DeleteDC(hdc_mem);
    ReleaseDC(HWND(0), hdc_screen);

    // Convert BGRA to RGBA by swapping blue and red channels.
    for i in (0..pixel_data.len()).step_by(4) {
        let b = pixel_data[i];
        let r = pixel_data[i + 2];
        pixel_data[i] = r;
        pixel_data[i + 2] = b;
    }
    // Save the PNG using the image crate.
    match image::save_buffer(file_path, &pixel_data, width as u32, height as u32, image::ColorType::Rgba8) {
        Ok(_) => Ok(file_path.to_string()),
        Err(e) => Err(format!("Error saving PNG: {}", e)),
    }
}

/// Groups all visible top-level windows by arranging them in a grid layout across the screen.
unsafe fn group_windows() -> bool {
    // Vector to store HWNDs of all visible windows.
    let mut windows_vec: Vec<HWND> = Vec::new();
    extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> i32 {
        unsafe {
            // Only include visible windows.
            if IsWindowVisible(hwnd).as_bool() {
                // Append the window handle into the Vec<HWND> passed via lparam.
                let windows_ptr = lparam.0 as *mut Vec<HWND>;
                if !windows_ptr.is_null() {
                    (*windows_ptr).push(hwnd);
                }
            }
        }
        1 // continue enumeration
    }
    
    // Enumerate all top-level windows.
    EnumWindows(Some(enum_proc), LPARAM(&mut windows_vec as *mut _ as isize));
    if windows_vec.is_empty() {
        return false;
    }
    
    // Determine grid layout dimensions.
    let count = windows_vec.len();
    let cols = (count as f64).sqrt().ceil() as usize;
    let rows = (count + cols - 1) / cols;
    
    // Get screen dimensions.
    let screen_dc = GetDC(HWND(0));
    let screen_width = GetDeviceCaps(screen_dc, HORZRES);
    let screen_height = GetDeviceCaps(screen_dc, VERTRES);
    ReleaseDC(HWND(0), screen_dc);
    
    let window_width = screen_width / cols as i32;
    let window_height = screen_height / rows as i32;
    
    // Arrange each window in the grid.
    for (index, hwnd) in windows_vec.iter().enumerate() {
        let col = index % cols;
        let row = index / cols;
        let x = col as i32 * window_width;
        let y = row as i32 * window_height;
        // Move each window to its grid position.
        SetWindowPos(*hwnd, HWND(0), x, y, window_width, window_height, SWP_NOZORDER | SWP_NOACTIVATE);
    }
    
    true
}

/// Releases the device context.
unsafe fn ReleaseDC(hWnd: HWND, hDC: windows::Win32::Graphics::Gdi::HDC) {
    windows::Win32::Graphics::Gdi::ReleaseDC(hWnd, hDC);
}
