use rdev::{Event, EventType, Key};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{Emitter, Manager};

use crate::audio::{
    cancel_action, cancel_action_silently, start_recording, stop_recording, AudioState,
};

static CURRENT_GENERATION: AtomicU64 = AtomicU64::new(0);

lazy_static::lazy_static! {
    static ref PRESSED_KEYS: Arc<Mutex<HashMap<String, Instant>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref CURRENT_HOTKEY: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    static ref CANCEL_HOTKEY: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    static ref IS_RECORDING: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref PUSH_TO_TALK: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref RECORD_START_TIME: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    static ref HOOK_HEARTBEAT: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));
    static ref HOOK_EVENT_COUNT: Arc<Mutex<u64>> = Arc::new(Mutex::new(0));
}

fn key_to_string(key: Key) -> Option<&'static str> {
    match key {
        Key::ControlLeft => Some("ControlLeft"),
        Key::ControlRight => Some("ControlRight"),
        Key::ShiftLeft => Some("ShiftLeft"),
        Key::ShiftRight => Some("ShiftRight"),
        Key::Alt => Some("AltLeft"),
        Key::AltGr => Some("AltRight"),
        Key::MetaLeft => Some("MetaLeft"),
        Key::MetaRight => Some("MetaRight"),
        Key::Space => Some("Space"),
        Key::Escape => Some("Escape"),
        Key::Return => Some("Enter"),
        Key::Backspace => Some("Backspace"),
        Key::Tab => Some("Tab"),
        Key::F1 => Some("F1"),
        Key::F2 => Some("F2"),
        Key::F3 => Some("F3"),
        Key::F4 => Some("F4"),
        Key::F5 => Some("F5"),
        Key::F6 => Some("F6"),
        Key::F7 => Some("F7"),
        Key::F8 => Some("F8"),
        Key::F9 => Some("F9"),
        Key::F10 => Some("F10"),
        Key::F11 => Some("F11"),
        Key::F12 => Some("F12"),
        Key::KeyA => Some("A"),
        Key::KeyB => Some("B"),
        Key::KeyC => Some("C"),
        Key::KeyD => Some("D"),
        Key::KeyE => Some("E"),
        Key::KeyF => Some("F"),
        Key::KeyG => Some("G"),
        Key::KeyH => Some("H"),
        Key::KeyI => Some("I"),
        Key::KeyJ => Some("J"),
        Key::KeyK => Some("K"),
        Key::KeyL => Some("L"),
        Key::KeyM => Some("M"),
        Key::KeyN => Some("N"),
        Key::KeyO => Some("O"),
        Key::KeyP => Some("P"),
        Key::KeyQ => Some("Q"),
        Key::KeyR => Some("R"),
        Key::KeyS => Some("S"),
        Key::KeyT => Some("T"),
        Key::KeyU => Some("U"),
        Key::KeyV => Some("V"),
        Key::KeyW => Some("W"),
        Key::KeyX => Some("X"),
        Key::KeyY => Some("Y"),
        Key::KeyZ => Some("Z"),
        Key::Num0 => Some("0"),
        Key::Num1 => Some("1"),
        Key::Num2 => Some("2"),
        Key::Num3 => Some("3"),
        Key::Num4 => Some("4"),
        Key::Num5 => Some("5"),
        Key::Num6 => Some("6"),
        Key::Num7 => Some("7"),
        Key::Num8 => Some("8"),
        Key::Num9 => Some("9"),
        _ => None,
    }
}

fn is_modifier_key(key_str: &str) -> bool {
    matches!(
        key_str,
        "ControlLeft"
            | "ControlRight"
            | "ShiftLeft"
            | "ShiftRight"
            | "AltLeft"
            | "AltRight"
            | "MetaLeft"
            | "MetaRight"
    )
}

fn is_hotkey_pressed(hotkey: &[String], pressed: &HashMap<String, Instant>) -> bool {
    if hotkey.is_empty() {
        return false;
    }

    for part in hotkey {
        match part.as_str() {
            "Control" => {
                if !pressed.contains_key("ControlLeft") && !pressed.contains_key("ControlRight") {
                    return false;
                }
            }
            "Shift" => {
                if !pressed.contains_key("ShiftLeft") && !pressed.contains_key("ShiftRight") {
                    return false;
                }
            }
            "Alt" => {
                if !pressed.contains_key("AltLeft") && !pressed.contains_key("AltRight") {
                    return false;
                }
            }
            "Meta" => {
                if !pressed.contains_key("MetaLeft") && !pressed.contains_key("MetaRight") {
                    return false;
                }
            }
            _ => {
                if !pressed.contains_key(part) {
                    return false;
                }
            }
        }
    }

    // Check that no extra MODIFIERS are pressed
    for k in pressed.keys() {
        if is_modifier_key(k) && !is_key_in_hotkey(k, hotkey) {
            return false;
        }
    }

    true
}

fn is_key_in_hotkey(key_str: &str, hotkey: &[String]) -> bool {
    for part in hotkey {
        if part == key_str {
            return true;
        }
        match part.as_str() {
            "Control" => {
                if key_str == "ControlLeft" || key_str == "ControlRight" {
                    return true;
                }
            }
            "Shift" => {
                if key_str == "ShiftLeft" || key_str == "ShiftRight" {
                    return true;
                }
            }
            "Alt" => {
                if key_str == "AltLeft" || key_str == "AltRight" {
                    return true;
                }
            }
            "Meta" => {
                if key_str == "MetaLeft" || key_str == "MetaRight" {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

#[derive(Debug)]
enum HotkeyAction {
    StartRecording,
    StopRecording,
    CancelRecording,
    CancelRecordingSilently,
    RawEvent(Event),
}

fn is_key_physically_pressed(key_str: &str) -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let vk = match key_str {
        "ControlLeft" => VK_LCONTROL.0 as i32,
        "ControlRight" => VK_RCONTROL.0 as i32,
        "ShiftLeft" => VK_LSHIFT.0 as i32,
        "ShiftRight" => VK_RSHIFT.0 as i32,
        "AltLeft" => VK_LMENU.0 as i32,
        "AltRight" => VK_RMENU.0 as i32,
        "MetaLeft" => VK_LWIN.0 as i32,
        "MetaRight" => VK_RWIN.0 as i32,
        "Space" => VK_SPACE.0 as i32,
        "Escape" => VK_ESCAPE.0 as i32,
        "Enter" => VK_RETURN.0 as i32,
        "Backspace" => VK_BACK.0 as i32,
        "Tab" => VK_TAB.0 as i32,
        "F1" => VK_F1.0 as i32,
        "F2" => VK_F2.0 as i32,
        "F3" => VK_F3.0 as i32,
        "F4" => VK_F4.0 as i32,
        "F5" => VK_F5.0 as i32,
        "F6" => VK_F6.0 as i32,
        "F7" => VK_F7.0 as i32,
        "F8" => VK_F8.0 as i32,
        "F9" => VK_F9.0 as i32,
        "F10" => VK_F10.0 as i32,
        "F11" => VK_F11.0 as i32,
        "F12" => VK_F12.0 as i32,
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase() as i32
            } else {
                return true;
            }
        }
        _ => return true,
    };
    unsafe { (GetAsyncKeyState(vk) as i16) < 0 }
}

fn prune_stuck_keys(current_event_key: Option<&str>) {
    let mut pressed = PRESSED_KEYS.lock().unwrap();
    let mut to_remove = Vec::new();
    let now = Instant::now();

    for (key_str, pressed_time) in pressed.iter() {
        if Some(key_str.as_str()) == current_event_key {
            continue;
        }

        // Only prune keys that have been pressed for > 2 seconds
        if now.duration_since(*pressed_time).as_secs() > 2 {
            if !is_key_physically_pressed(key_str) {
                to_remove.push(key_str.clone());
            }
        }
    }
    for k in to_remove {
        #[cfg(debug_assertions)]
        eprintln!("[debug][hotkeys] Self-correcting stuck key: {}", k);
        pressed.remove(&k);
    }
}

static LIVE_LISTENERS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn start_rdev_listener(tx: std::sync::mpsc::Sender<HotkeyAction>) {
    let my_gen = CURRENT_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;

    let live_count = LIVE_LISTENERS.fetch_add(1, Ordering::SeqCst) + 1;
    eprintln!(
        "[debug][hotkeys] Starting rdev listener generation #{}. Total live listeners: {}",
        my_gen, live_count
    );

    std::thread::spawn(move || {
        let callback = move |event: Event| {
            let hook_start = Instant::now();
            let event_type_dbg = format!("{:?}", event.event_type);

            if CURRENT_GENERATION.load(Ordering::Relaxed) != my_gen {
                let elapsed = hook_start.elapsed();
                if elapsed > std::time::Duration::from_millis(5) {
                    eprintln!("[HOOK SLOW] ZOMBIE gen {} took {}ms for {}", my_gen, elapsed.as_millis(), event_type_dbg);
                }
                return;
            }

            *HOOK_HEARTBEAT.lock().unwrap() = hook_start;

            match event.event_type {
                EventType::KeyPress(_) | EventType::KeyRelease(_) => {
                    let _ = tx.send(HotkeyAction::RawEvent(event));
                }
                _ => {}
            }

            let elapsed = hook_start.elapsed();
            if elapsed > std::time::Duration::from_millis(5) {
                eprintln!("[HOOK SLOW] ACTIVE gen {} took {}ms for {}", my_gen, elapsed.as_millis(), event_type_dbg);
            }
        };

        if let Err(error) = rdev::listen(callback) {
            eprintln!(
                "Error starting rdev listener (gen #{}): {:?}",
                my_gen, error
            );
        }
        LIVE_LISTENERS.fetch_sub(1, Ordering::SeqCst);
        eprintln!(
            "[warn][hotkeys] rdev listener thread exited (gen #{})",
            my_gen
        );
    });
}

pub fn setup_hotkeys(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let settings = crate::settings::load_settings();

    {
        let mut hk = CURRENT_HOTKEY.lock().unwrap();
        *hk = settings.hotkey.split('+').map(|s| s.to_string()).collect();

        let mut ck = CANCEL_HOTKEY.lock().unwrap();
        *ck = settings
            .cancel_hotkey
            .split('+')
            .map(|s| s.to_string())
            .collect();

        let mut ptt = PUSH_TO_TALK.lock().unwrap();
        *ptt = settings.push_to_talk;
    }

    let (tx, rx) = std::sync::mpsc::channel::<HotkeyAction>();
    let app_handle = app.handle().clone();
    let tx_processor = tx.clone();

    // Event processor thread (to interact with Tauri safely outside of rdev callback)
    std::thread::spawn(move || {
        while let Ok(action) = rx.recv() {
            let app_clone = app_handle.clone();
            match action {
                HotkeyAction::RawEvent(event) => {
                    match event.event_type {
                EventType::KeyPress(key) => {
                                        *HOOK_EVENT_COUNT.lock().unwrap() += 1;

                    if let Some(key_str) = key_to_string(key) {
                        prune_stuck_keys(Some(key_str));
                        let mut pressed = PRESSED_KEYS.lock().unwrap();
                        let was_not_pressed = pressed
                            .insert(key_str.to_string(), Instant::now())
                            .is_none();

                        #[cfg(debug_assertions)]
                        {
                            let log_all = std::env::var("VISPEAK_LOG_ALL_KEYS").is_ok();
                            let is_mod = is_modifier_key(key_str);
                            let is_esc = key_str == "Escape";
                            let in_hk = is_key_in_hotkey(key_str, &CURRENT_HOTKEY.lock().unwrap())
                                || is_key_in_hotkey(key_str, &CANCEL_HOTKEY.lock().unwrap());
                            if log_all || is_mod || is_esc || in_hk {
                                eprintln!(
                                    "[debug][hotkeys] KeyPress: {} | current pressed={:?}",
                                    key_str,
                                    pressed.keys().collect::<Vec<_>>()
                                );
                            }
                        }

                        if was_not_pressed {
                            let hotkey = CURRENT_HOTKEY.lock().unwrap();
                            let cancel_hk = CANCEL_HOTKEY.lock().unwrap();
                            let ptt = *PUSH_TO_TALK.lock().unwrap();
                            let mut is_rec = IS_RECORDING.lock().unwrap();

                            // Check cancel hotkey first
                            if *is_rec && is_hotkey_pressed(&cancel_hk, &pressed) {
                                *is_rec = false;
                                let _ = tx_processor.send(HotkeyAction::CancelRecording);
                            } else if is_hotkey_pressed(&hotkey, &pressed) {
                                if !*is_rec {
                                    *is_rec = true;
                                    *RECORD_START_TIME.lock().unwrap() = Some(Instant::now());
                                    let _ = tx_processor.send(HotkeyAction::StartRecording);
                                } else if !ptt {
                                    // In Toggle mode, pressing again stops
                                    *is_rec = false;
                                    let _ = tx_processor.send(HotkeyAction::StopRecording);
                                }
                            }
                        }
                    }
                }
                EventType::KeyRelease(key) => {
                                        *HOOK_EVENT_COUNT.lock().unwrap() += 1;

                    if let Some(key_str) = key_to_string(key) {
                        prune_stuck_keys(Some(key_str));
                        let mut pressed = PRESSED_KEYS.lock().unwrap();
                        pressed.remove(key_str);

                        #[cfg(debug_assertions)]
                        {
                            let log_all = std::env::var("VISPEAK_LOG_ALL_KEYS").is_ok();
                            let is_mod = is_modifier_key(key_str);
                            let is_esc = key_str == "Escape";
                            let in_hk = is_key_in_hotkey(key_str, &CURRENT_HOTKEY.lock().unwrap())
                                || is_key_in_hotkey(key_str, &CANCEL_HOTKEY.lock().unwrap());
                            if log_all || is_mod || is_esc || in_hk {
                                eprintln!(
                                    "[debug][hotkeys] KeyRelease: {} | current pressed={:?}",
                                    key_str,
                                    pressed.keys().collect::<Vec<_>>()
                                );
                            }
                        }

                        let hotkey = CURRENT_HOTKEY.lock().unwrap();
                        let mut is_rec = IS_RECORDING.lock().unwrap();
                        let ptt = *PUSH_TO_TALK.lock().unwrap();

                        if *is_rec && ptt {
                            if is_key_in_hotkey(key_str, &hotkey) {
                                *is_rec = false;
                                let start_time = *RECORD_START_TIME.lock().unwrap();
                                let mut do_silent = false;
                                if let Some(st) = start_time {
                                    if st.elapsed().as_millis() < 300 {
                                        do_silent = true;
                                    }
                                }
                                if do_silent {
                                    let _ = tx_processor.send(HotkeyAction::CancelRecordingSilently);
                                } else {
                                    let _ = tx_processor.send(HotkeyAction::StopRecording);
                                }
                            }
                        }
                    }
                }

                        _ => {}
                    }
                }
                HotkeyAction::StartRecording => {
                    let active_model = crate::settings::load_settings().active_model;
                    let mut can_start = false;
                    if let Some(ref id) = active_model {
                        let path = crate::models::get_model_path(id);
                        if path.exists() {
                            can_start = true;
                        }
                    }

                    if can_start {
                        if let Some(app_info) = crate::paste::get_active_app_info() {
                            {
                                let state_arc = app_clone.state::<Arc<Mutex<AudioState>>>();
                                let mut state = state_arc.inner().lock().unwrap();
                                state.target_hwnd = Some(app_info.hwnd);
                                state.app_info = Some(app_info.clone());
                            }
                            let _ = app_clone.emit("target-app", app_info);
                        }
                        let _ = start_recording(app_clone);
                    } else {
                        let _ = app_clone.emit("show-error", "err_no_model_selected".to_string());
                        crate::show_overlay(app_clone.clone());
                    }
                }
                HotkeyAction::StopRecording => {
                    let _ = stop_recording(app_clone);
                }
                HotkeyAction::CancelRecording => {
                    let _ = cancel_action(app_clone);
                }
                HotkeyAction::CancelRecordingSilently => {
                    let _ = cancel_action_silently(app_clone);
                }
            }
        }
    });

    // Low level keyboard hook thread with watchdog & self-correction
    start_rdev_listener(tx.clone());

    let tx_watchdog = tx.clone();
    std::thread::spawn(move || {
        use windows::Win32::System::SystemInformation::GetTickCount;
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

        loop {
            std::thread::sleep(std::time::Duration::from_millis(2500));
            prune_stuck_keys(None);

            let heartbeat_elapsed = HOOK_HEARTBEAT.lock().unwrap().elapsed();
            if heartbeat_elapsed > std::time::Duration::from_secs(10) {
                let mut lii = LASTINPUTINFO {
                    cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                    dwTime: 0,
                };
                let sys_input_active = unsafe {
                    if GetLastInputInfo(&mut lii).as_bool() {
                        let now_ticks = GetTickCount();
                        let diff = now_ticks.wrapping_sub(lii.dwTime);
                        diff < 3000
                    } else {
                        false
                    }
                };

                if sys_input_active {
                    // Grace confirmation: wait 1000ms while user is actively using the system.
                    // If events start arriving (rdev was merely waking up from idle), heartbeat resets.
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    let elapsed_after_grace = HOOK_HEARTBEAT.lock().unwrap().elapsed();
                    if elapsed_after_grace > std::time::Duration::from_secs(10) {
                        let still_active = unsafe {
                            if GetLastInputInfo(&mut lii).as_bool() {
                                let now_ticks = GetTickCount();
                                now_ticks.wrapping_sub(lii.dwTime) < 3000
                            } else {
                                false
                            }
                        };
                        if still_active {
                            eprintln!("[warn][hotkeys] Watchdog recovery triggered! Heartbeat stale ({}s) during confirmed active system input. Restarting rdev listener...", elapsed_after_grace.as_secs());
                                                        start_rdev_listener(tx_watchdog.clone());
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn update_hotkey(_app: tauri::AppHandle, new_key: String) -> Result<(), String> {
    let mut settings = crate::settings::load_settings();

    settings.hotkey = new_key.clone();
    crate::settings::save_settings(&settings).map_err(|e| e.to_string())?;

    {
        let mut hk = CURRENT_HOTKEY.lock().unwrap();
        *hk = new_key.split('+').map(|s| s.to_string()).collect();
    }

    Ok(())
}

#[tauri::command]
pub fn update_cancel_hotkey(_app: tauri::AppHandle, new_key: String) -> Result<(), String> {
    let mut settings = crate::settings::load_settings();

    settings.cancel_hotkey = new_key.clone();
    crate::settings::save_settings(&settings).map_err(|e| e.to_string())?;

    {
        let mut hk = CANCEL_HOTKEY.lock().unwrap();
        *hk = new_key.split('+').map(|s| s.to_string()).collect();
    }

    Ok(())
}

#[tauri::command]
pub fn update_push_to_talk(_app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let mut settings = crate::settings::load_settings();

    settings.push_to_talk = enabled;
    crate::settings::save_settings(&settings).map_err(|e| e.to_string())?;

    {
        let mut ptt = PUSH_TO_TALK.lock().unwrap();
        *ptt = enabled;
    }

    Ok(())
}
