use serde::{Deserialize, Serialize};
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;

static PASTE_LOCK: Mutex<()> = Mutex::new(());
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AppInfo {
    pub title: String,
    pub icon_base64: String,
    pub hwnd: isize,
}

pub(crate) fn command(
    program: &str,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<Vec<u8>, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{program}: {error}"))?;
    let write_result = if let Some(input) = input {
        child
            .stdin
            .take()
            .ok_or("Missing command stdin".to_string())?
            .write_all(input)
            .map_err(|error| format!("{program}: {error}"))
    } else {
        Ok(())
    };
    let output = child
        .wait_with_output()
        .map_err(|error| format!("{program}: {error}"))?;
    write_result?;
    if !output.status.success() {
        return Err(format!("{program} failed ({})", output.status));
    }
    Ok(output.stdout)
}

pub fn get_active_app_info() -> Option<AppInfo> {
    if crate::linux::is_wayland() {
        return None;
    }
    let id = String::from_utf8(command("xdotool", &["getwindowfocus"], None).ok()?).ok()?;
    let hwnd = id.trim().parse().ok()?;
    let title = command("xdotool", &["getwindowname", id.trim()], None).ok()?;
    Some(AppInfo {
        title: String::from_utf8_lossy(&title).trim().into(),
        icon_base64: String::new(),
        hwnd,
    })
}

#[derive(Clone)]
struct ClipboardContent {
    mime: String,
    bytes: Vec<u8>,
}

fn clipboard_snapshot() -> Result<Option<ClipboardContent>, String> {
    let wayland = crate::linux::is_wayland();
    let (program, args): (&str, &[&str]) = if wayland {
        ("wl-paste", &["--list-types"])
    } else {
        (
            "xclip",
            &["-selection", "clipboard", "-out", "-target", "TARGETS"],
        )
    };
    let output = Command::new(program)
        .args(args)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("{program}: {error}"))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        // Do not confuse permission or connection failures with an empty selection.
        if error.contains("Nothing is copied") || error.contains("target TARGETS not available") {
            return Ok(None);
        }
        return Err(format!(
            "{program} cannot read clipboard targets ({})",
            output.status
        ));
    }
    let targets = output.stdout;
    let targets = String::from_utf8_lossy(&targets);
    let mime = [
        "text/plain;charset=utf-8",
        "UTF8_STRING",
        "text/plain",
        "image/png",
    ]
    .into_iter()
    .find(|target| targets.lines().any(|line| line.trim() == *target))
    .ok_or("Clipboard format cannot be preserved; choose Keep clipboard or Copy only")?;
    let bytes = if wayland {
        command("wl-paste", &["--no-newline", "--type", mime], None)?
    } else {
        command(
            "xclip",
            &["-selection", "clipboard", "-out", "-target", mime],
            None,
        )?
    };
    Ok(Some(ClipboardContent {
        mime: mime.into(),
        bytes,
    }))
}

fn write_selection(program: &str, args: &[&str], bytes: &[u8]) -> Result<(), String> {
    // The selection owner forks into the background. Do not give it captured
    // output pipes: inherited pipes would prevent wait_with_output from finishing.
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let result = child
        .stdin
        .take()
        .ok_or("Missing clipboard stdin")?
        .write_all(bytes)
        .map_err(|e| e.to_string());
    let status = child.wait().map_err(|e| e.to_string())?;
    result?;
    if !status.success() {
        return Err(format!("{program} failed ({status})"));
    }
    Ok(())
}

fn write_clipboard(content: &ClipboardContent) -> Result<(), String> {
    if crate::linux::is_wayland() {
        write_selection("wl-copy", &["--type", &content.mime], &content.bytes)
    } else {
        let mime = if content.mime.starts_with("text/plain") {
            "UTF8_STRING"
        } else {
            &content.mime
        };
        write_selection(
            "xclip",
            &["-selection", "clipboard", "-in", "-target", mime],
            &content.bytes,
        )
    }
}

fn shortcut_keycodes(symbols: &[i32]) -> Result<Vec<i32>, String> {
    symbols
        .iter()
        .map(|symbol| match *symbol {
            0xffe3 => Ok(29),  // KEY_LEFTCTRL
            0xffe1 => Ok(42),  // KEY_LEFTSHIFT
            0x76 => Ok(47),    // KEY_V, independent of the active keyboard layout
            0xff63 => Ok(110), // KEY_INSERT
            0xff0d => Ok(28),  // KEY_ENTER
            _ => Err("Unsupported physical shortcut key".into()),
        })
        .collect()
}

fn chord(symbols: &[i32], x11_keys: &str) -> Result<(), String> {
    if crate::linux::is_wayland() {
        let keys = shortcut_keycodes(symbols)?;
        let mut sequence: Vec<_> = keys.iter().map(|key| (*key, true)).collect();
        sequence.extend(keys.iter().rev().map(|key| (*key, false)));
        crate::linux::keycode_sequence(&sequence)
    } else {
        command("xdotool", &["key", "--clearmodifiers", x11_keys], None).map(|_| ())
    }
}

fn type_text(text: &str) -> Result<(), String> {
    if crate::linux::is_wayland() {
        let sequence: Vec<_> = text
            .chars()
            .flat_map(|character| {
                let symbol = match character {
                    '\n' => 0xff0d,
                    '\t' => 0xff09,
                    ch if ch as u32 <= 0xff => ch as i32,
                    ch => (0x01000000 | ch as u32) as i32,
                };
                [(symbol, true), (symbol, false)]
            })
            .collect();
        crate::linux::key_sequence(&sequence)
    } else {
        command(
            "xdotool",
            &["type", "--clearmodifiers", "--file", "-"],
            Some(text.as_bytes()),
        )
        .map(|_| ())
    }
}

fn target_is_focused(target: Option<isize>) -> bool {
    if crate::linux::vispeak_is_focused() {
        return false;
    }
    if crate::linux::is_wayland() {
        return true;
    }
    get_active_app_info().is_some_and(|info| target.is_none_or(|id| id == info.hwnd))
}

fn paste(text: &str, target: Option<isize>) -> Result<bool, String> {
    let _guard = PASTE_LOCK.lock().map_err(|_| "Paste lock failed")?;
    let settings = crate::settings::load_settings();
    crate::log_debug(&format!(
        "[paste] begin: method={}, keyboard_ready={}",
        settings.text_input_method,
        crate::linux::input_ready()
    ));
    let copy_only = settings.text_input_method == "copy_only";
    let typing = settings.text_input_method == "type_chars";
    let text = if settings.trailing_space && !copy_only {
        format!("{text} ")
    } else {
        text.into()
    };
    let content = ClipboardContent {
        mime: "text/plain;charset=utf-8".into(),
        bytes: text.into_bytes(),
    };
    if copy_only {
        write_clipboard(&content)?;
        return Ok(true);
    }
    if !crate::linux::input_ready() {
        return Err("err_linux_input_permission".into());
    }
    // Never activate a captured window: the user may have switched applications.
    if !target_is_focused(target) {
        return Err("The dictation target is no longer focused".into());
    }
    let modifies_clipboard = !typing || settings.clipboard_after == "keep";
    let previous = if modifies_clipboard && settings.clipboard_after == "restore" {
        clipboard_snapshot()?
    } else {
        None
    };
    if modifies_clipboard {
        write_clipboard(&content)?;
    }
    crate::log_debug("[paste] clipboard prepared; checking target before keyboard input");
    std::thread::sleep(Duration::from_millis(50));
    let result = if !target_is_focused(target) {
        Err("Focus changed before input".into())
    } else if typing {
        type_text(&String::from_utf8_lossy(&content.bytes))
    } else {
        match settings.text_input_method.as_str() {
            "paste_raw" => chord(&[0xffe3, 0xffe1, 0x76], "ctrl+shift+v"),
            "paste_shift_ins" => chord(&[0xffe1, 0xff63], "shift+Insert"),
            _ => chord(&[0xffe3, 0x76], "ctrl+v"),
        }
    };
    if modifies_clipboard && settings.clipboard_after == "restore" {
        std::thread::sleep(Duration::from_millis(300));
        // Do not overwrite content copied by the user during this delay.
        if clipboard_snapshot()?.is_some_and(|current| current.bytes == content.bytes) {
            if let Some(previous) = previous {
                write_clipboard(&previous)?;
            } else if crate::linux::is_wayland() {
                command("wl-copy", &["--clear"], None)?;
            } else {
                write_clipboard(&ClipboardContent {
                    mime: content.mime,
                    bytes: Vec::new(),
                })?;
            }
        }
    }
    result?;
    crate::log_debug("[paste] keyboard input dispatched successfully");
    if target_is_focused(target) {
        match settings.send_after.as_str() {
            "enter" => chord(&[0xff0d], "Return")?,
            "ctrl_enter" => chord(&[0xffe3, 0xff0d], "ctrl+Return")?,
            _ => (),
        }
    }
    Ok(false)
}

pub fn paste_text(text: &str, target: Option<isize>) -> bool {
    match paste(text, target) {
        Ok(copied) => copied,
        Err(error) => {
            let code = if error == "err_linux_input_permission" {
                "err_linux_input_permission"
            } else {
                "err_linux_paste"
            };
            crate::linux::report_error(code, &error);
            false
        }
    }
}

// Wayland does not expose the focused foreign window, so live typing cannot
// safely guarantee that every delta reaches the original target.
pub fn supports_live_typing() -> bool {
    !crate::linux::is_wayland()
}
pub fn type_text_delta(text: &str, target: Option<isize>) -> bool {
    if !supports_live_typing() || !target_is_focused(target) {
        return false;
    }
    type_text(text).is_ok()
}

pub fn try_paste_text(text: &str, target: Option<isize>) -> Result<bool, String> {
    paste(text, target)
}

#[cfg(test)]
mod tests {
    #[test]
    fn paste_shortcuts_use_evdev_keycodes_independent_of_layout() {
        assert_eq!(
            super::shortcut_keycodes(&[0xffe3, 0x76]).unwrap(),
            vec![29, 47]
        );
        assert_eq!(
            super::shortcut_keycodes(&[0xffe3, 0xffe1, 0x76]).unwrap(),
            vec![29, 42, 47]
        );
        assert_eq!(
            super::shortcut_keycodes(&[0xffe1, 0xff63]).unwrap(),
            vec![42, 110]
        );
        assert_eq!(super::shortcut_keycodes(&[0xff0d]).unwrap(), vec![28]);
        assert!(super::shortcut_keycodes(&[0xdead]).is_err());
    }
    use super::*;
    #[test]
    #[ignore = "requires an isolated X11 display and xclip (run with xvfb-run)"]
    fn x11_clipboard_preserves_unicode_and_image() {
        assert!(!crate::linux::is_wayland());
        let text = ClipboardContent {
            mime: "UTF8_STRING".into(),
            bytes: "Привет, Linux 🐧\n$(not a command)".as_bytes().to_vec(),
        };
        write_clipboard(&text).unwrap();
        assert_eq!(clipboard_snapshot().unwrap().unwrap().bytes, text.bytes);
        let image = ClipboardContent {
            mime: "image/png".into(),
            bytes: include_bytes!("../../icons/32x32.png").to_vec(),
        };
        write_clipboard(&image).unwrap();
        let saved = clipboard_snapshot().unwrap().unwrap();
        write_clipboard(&text).unwrap();
        write_clipboard(&saved).unwrap();
        let restored = clipboard_snapshot().unwrap().unwrap();
        assert_eq!(restored.mime, image.mime);
        assert_eq!(restored.bytes, image.bytes);
    }
}
