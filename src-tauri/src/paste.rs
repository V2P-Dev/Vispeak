use clipboard_win::{formats, get_clipboard, set_clipboard};
use enigo::{Enigo, Key, KeyboardControllable};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowTextW,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppInfo {
    pub title: String,
    pub icon_base64: String,
    pub hwnd: isize,
}

pub fn get_active_app_info() -> Option<AppInfo> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0 == std::ptr::null_mut() {
            return None;
        }

        let mut title_buf = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title_buf);
        let title = String::from_utf16_lossy(&title_buf[..len as usize]);

        let icon_base64 = crate::icon::get_window_icon_base64(hwnd);

        Some(AppInfo {
            title,
            icon_base64,
            hwnd: hwnd.0 as isize,
        })
    }
}

fn finalize_clipboard(policy: &str, previous_text: &str, text: &str, is_typing: bool) {
    println!("[DEBUG PASTE] finalize_clipboard START: policy='{}', previous_text='{}', text='{}', is_typing={}", policy, previous_text, text, is_typing);
    if policy == "restore" {
        if is_typing {
            println!("[DEBUG PASTE] finalize_clipboard(policy=restore, is_typing=true): clipboard untouched, nothing to restore.");
            return;
        }
        std::thread::sleep(Duration::from_millis(300));
        if !previous_text.is_empty() {
            let mut restored = false;
            for i in 0..15 {
                match set_clipboard(formats::Unicode, previous_text) {
                    Ok(_) => {
                        let current: String = get_clipboard(formats::Unicode).unwrap_or_default();
                        println!("[DEBUG PASTE] finalize_clipboard(policy=restore): restored previous text on attempt {}. Current right after restore: '{}'", i + 1, current);
                        restored = true;
                        break;
                    }
                    Err(e) => {
                        println!("[DEBUG PASTE] finalize_clipboard(policy=restore): attempt {} failed to set clipboard: {:?}", i + 1, e);
                    }
                }
                std::thread::sleep(Duration::from_millis(30));
            }
            if !restored {
                println!("[DEBUG PASTE] finalize_clipboard(policy=restore): FAILED to restore after 15 attempts!");
            }
        } else {
            println!("[DEBUG PASTE] finalize_clipboard(policy=restore): previous_text is empty, skipping restore.");
        }
    } else if policy == "keep" {
        std::thread::sleep(Duration::from_millis(50));
        for i in 0..10 {
            let current: String = get_clipboard(formats::Unicode).unwrap_or_default();
            if current == text {
                println!("[DEBUG PASTE] finalize_clipboard(policy=keep): verified clipboard contains target text on attempt {}.", i + 1);
                break;
            }
            println!("[DEBUG PASTE] finalize_clipboard(policy=keep): buffer mismatch (got '{:?}'), rewriting target text on attempt {}", current, i + 1);
            let _ = set_clipboard(formats::Unicode, text);
            std::thread::sleep(Duration::from_millis(50));
        }
    } else {
        println!(
            "[DEBUG PASTE] finalize_clipboard: policy '{}', no action taken.",
            policy
        );
    }
}

pub fn paste_text(text: &str, target_hwnd: Option<isize>) -> bool {
    let settings = crate::settings::load_settings();
    let is_copy_only = settings.text_input_method == "copy_only";
    let is_typing = settings.text_input_method == "type_chars";

    let text = if settings.trailing_space && !is_copy_only {
        format!("{} ", text)
    } else {
        text.to_string()
    };

    println!("[DEBUG PASTE] paste_text START: method='{}', clipboard_after='{}', text='{}' (len: {}, ends_with_space: {})", settings.text_input_method, settings.clipboard_after, text, text.len(), text.ends_with(' '));

    if let Some(hwnd_val) = target_hwnd {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;
            let _ = SetForegroundWindow(HWND(hwnd_val as _));
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    let mut previous_text = String::new();
    let should_write_initial_clipboard =
        !is_typing || settings.clipboard_after == "keep" || is_copy_only;

    if !is_typing || settings.clipboard_after == "restore" {
        for i in 0..5 {
            match get_clipboard(formats::Unicode) {
                Ok(t) => {
                    previous_text = t;
                    println!(
                        "[DEBUG PASTE] paste_text: read previous_text on attempt {}: '{}'",
                        i + 1,
                        previous_text
                    );
                    break;
                }
                Err(e) => {
                    println!("[DEBUG PASTE] paste_text: failed to read previous_text on attempt {}: {:?}", i + 1, e);
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    if should_write_initial_clipboard {
        for i in 0..5 {
            match set_clipboard(formats::Unicode, text.as_str()) {
                Ok(_) => {
                    println!("[DEBUG PASTE] paste_text: set initial clipboard to '{}' succeeded on attempt {}", text, i + 1);
                    break;
                }
                Err(e) => {
                    println!(
                        "[DEBUG PASTE] paste_text: set initial clipboard attempt {} failed: {:?}",
                        i + 1,
                        e
                    );
                }
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if is_copy_only {
        return true;
    }

    let mut enigo = Enigo::new();

    if is_typing {
        enigo.key_sequence(&text);
    } else {
        match settings.text_input_method.as_str() {
            "paste_raw" => {
                enigo.key_down(Key::Control);
                enigo.key_down(Key::Shift);
                std::thread::sleep(Duration::from_millis(10));
                enigo.key_click(Key::Raw(0x56)); // V
                std::thread::sleep(Duration::from_millis(10));
                enigo.key_up(Key::Shift);
                enigo.key_up(Key::Control);
            }
            "paste_shift_ins" => {
                enigo.key_down(Key::Shift);
                std::thread::sleep(Duration::from_millis(10));
                enigo.key_click(Key::Insert);
                std::thread::sleep(Duration::from_millis(10));
                enigo.key_up(Key::Shift);
            }
            _ => {
                // "paste"
                enigo.key_down(Key::Control);
                std::thread::sleep(Duration::from_millis(10));
                enigo.key_click(Key::Raw(0x56)); // V
                std::thread::sleep(Duration::from_millis(10));
                enigo.key_up(Key::Control);
            }
        }
    }

    std::thread::sleep(Duration::from_millis(50));

    finalize_clipboard(&settings.clipboard_after, &previous_text, &text, is_typing);

    // Auto-send STRICTLY AFTER finalize_clipboard
    match settings.send_after.as_str() {
        "enter" => {
            std::thread::sleep(Duration::from_millis(50));
            enigo.key_click(Key::Return);
        }
        "ctrl_enter" => {
            std::thread::sleep(Duration::from_millis(50));
            enigo.key_down(Key::Control);
            std::thread::sleep(Duration::from_millis(10));
            enigo.key_click(Key::Return);
            std::thread::sleep(Duration::from_millis(10));
            enigo.key_up(Key::Control);
        }
        _ => {}
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use clipboard_win::{formats, get_clipboard, set_clipboard};
    use std::sync::Mutex;

    static CLIPBOARD_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_finalize_clipboard_keep() {
        let _guard = CLIPBOARD_MUTEX.lock().unwrap();
        let _ = set_clipboard(formats::Unicode, "СТАРОЕ");
        finalize_clipboard("keep", "СТАРОЕ", "НОВОЕ", false);
        std::thread::sleep(Duration::from_millis(1500));
        let current: String = get_clipboard(formats::Unicode).unwrap_or_default();
        assert_eq!(current, "НОВОЕ");
    }

    #[test]
    fn test_finalize_clipboard_restore() {
        let _guard = CLIPBOARD_MUTEX.lock().unwrap();
        let _ = set_clipboard(formats::Unicode, "СТАРОЕ");
        let _ = set_clipboard(formats::Unicode, "НОВОЕ");
        finalize_clipboard("restore", "СТАРОЕ", "НОВОЕ", false);
        std::thread::sleep(Duration::from_millis(1500));
        let current: String = get_clipboard(formats::Unicode).unwrap_or_default();
        assert_eq!(current, "СТАРОЕ");
    }

    #[test]
    fn test_finalize_clipboard_restore_typing() {
        let _guard = CLIPBOARD_MUTEX.lock().unwrap();
        let _ = set_clipboard(formats::Unicode, "СТАРОЕ");
        finalize_clipboard("restore", "СТАРОЕ", "НОВОЕ", true);
        std::thread::sleep(Duration::from_millis(1500));
        let current: String = get_clipboard(formats::Unicode).unwrap_or_default();
        assert_eq!(current, "СТАРОЕ");
    }
}
