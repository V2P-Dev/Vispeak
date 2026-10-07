//! Desktop integration for Linux. Wayland input is authorized by desktop portals.
pub(crate) mod kwin;
pub mod overlay;
mod portal_parent;
use ashpd::desktop::{
    global_shortcuts::{GlobalShortcuts, NewShortcut},
    remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions},
    Session,
};
use futures_util::{stream, StreamExt};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    mpsc::Sender,
    Arc, Mutex, OnceLock,
};
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager};

static PORTAL_DIALOGS: AtomicUsize = AtomicUsize::new(0);

struct PortalDialogGuard;
impl PortalDialogGuard {
    fn new() -> Self {
        PORTAL_DIALOGS.fetch_add(1, Ordering::SeqCst);
        Self
    }
}
impl Drop for PortalDialogGuard {
    fn drop(&mut self) {
        PORTAL_DIALOGS.fetch_sub(1, Ordering::SeqCst);
    }
}

static APP: OnceLock<tauri::AppHandle> = OnceLock::new();
static ERRORS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static RECORD_SHORTCUT: Mutex<Option<String>> = Mutex::new(None);
static RECORD_PORTAL: Mutex<Option<(Arc<GlobalShortcuts>, Arc<Session<GlobalShortcuts>>)>> =
    Mutex::new(None);

pub fn record_shortcut() -> Option<String> {
    RECORD_SHORTCUT.lock().ok().and_then(|value| value.clone())
}

pub fn input_ready() -> bool {
    !is_wayland() || INPUT.lock().is_ok_and(|input| input.is_some())
}

fn update_record_shortcut(shortcuts: &[ashpd::desktop::global_shortcuts::Shortcut]) {
    let description = shortcuts
        .iter()
        .find(|shortcut| shortcut.id() == "record")
        .map(|shortcut| shortcut.trigger_description().to_owned());
    eprintln!(
        "[linux] record shortcut assigned by portal: {}",
        description
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or("unassigned")
    );
    if let Ok(mut value) = RECORD_SHORTCUT.lock() {
        *value = Some(description.unwrap_or_default());
    }
}

pub async fn configure_record_shortcut() -> Result<(), String> {
    let app = APP.get().ok_or("err_linux_shortcuts")?;
    validate_hotkey_update(app, &crate::settings::load_settings().hotkey)?;
    let (proxy, session) = RECORD_PORTAL
        .lock()
        .map_err(|_| "err_linux_shortcuts")?
        .as_ref()
        .map(|(proxy, session)| (proxy.clone(), session.clone()))
        .ok_or("err_linux_shortcuts")?;
    let _dialog = PortalDialogGuard::new();
    let parent = portal_parent::get(app).await;
    proxy
        .configure_shortcuts(&session, parent, Default::default())
        .await
        .map_err(|error| {
            eprintln!("[linux] configure shortcuts: {error}");
            "err_linux_shortcuts".to_string()
        })
}
static PORTAL_ID: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();

async fn register_portal_app() {
    PORTAL_ID
        .get_or_init(|| async {
            // Register the shared ashpd connection before either portal uses it.
            // Older portals may lack Registry and use desktop-launch detection.
            if let Ok(id) = ashpd::AppID::try_from("app.vispeak") {
                if let Err(error) = ashpd::register_host_app(id).await {
                    eprintln!("[linux] portal application registration: {error}");
                }
            }
        })
        .await;
}

pub fn integration_errors() -> Vec<String> {
    ERRORS
        .lock()
        .map(|errors| errors.clone())
        .unwrap_or_default()
}

static INPUT: Mutex<Option<(RemoteDesktop, Session<RemoteDesktop>)>> = Mutex::new(None);
static CANCEL_SESSION: Mutex<Option<String>> = Mutex::new(None);
static HOTKEY_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn is_wayland() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var("XDG_SESSION_TYPE").is_ok_and(|value| value == "wayland")
}

pub fn report_error(code: &str, detail: &str) {
    crate::log_debug(&format!("[linux] {code}: {detail}"));
    if let Ok(mut errors) = ERRORS.lock() {
        if !errors.iter().any(|error| error == code) {
            errors.push(code.to_string());
        }
    }
    if let Some(app) = APP.get() {
        let _ = app.emit("show-error", code);
        show_overlay(app.clone());
    }
}

fn blocks_dictation(visible: bool, focused: bool) -> bool {
    visible && focused
}

pub fn vispeak_is_focused() -> bool {
    // Portal authorization dialogs can own focus without belonging to a Vispeak
    // window. Never inject dictation into those dialogs.
    let dialogs = PORTAL_DIALOGS.load(Ordering::SeqCst);
    if dialogs > 0 {
        crate::log_debug(&format!(
            "[paste] focus blocked: {dialogs} pending portal request(s)"
        ));
        return true;
    }
    APP.get().is_none_or(|app| {
        ["main", "overlay"].iter().any(|label| {
            app.get_webview_window(label).is_some_and(|window| {
                // GTK/Tao may retain focus on a hidden window. That window
                // cannot receive the dictation and must not block the target.
                match window.is_visible() {
                    Ok(false) => false,
                    Ok(visible) => match window.is_focused() {
                        Ok(focused) => {
                            let blocked = blocks_dictation(visible, focused);
                            if blocked {
                                crate::log_debug(&format!(
                                    "[paste] focus blocked: visible {label} window active"
                                ));
                            }
                            blocked
                        }
                        Err(error) => {
                            crate::log_debug(&format!(
                                "[paste] {label} focus check failed: {error}"
                            ));
                            true
                        }
                    },
                    Err(error) => {
                        crate::log_debug(&format!(
                            "[paste] {label} visibility check failed: {error}"
                        ));
                        true
                    }
                }
            })
        })
    })
}

pub fn setup(app: tauri::AppHandle) {
    use gtk::prelude::*;
    if let Some(window) = app.get_webview_window("main") {
        if let Ok(native) = window.gtk_window() {
            native.set_app_paintable(false);
            native.set_opacity(1.0);
        }
        let _ = window.with_webview(|webview| {
            use webkit2gtk::WebViewExt;
            webview
                .inner()
                .set_background_color(&gtk::gdk::RGBA::new(1.0, 1.0, 1.0, 1.0));
        });
    }
    overlay::setup(&app);
    if let Some(window) = app.get_webview_window("overlay") {
        let _ = window.set_focusable(false);
    }
    let _ = APP.set(app.clone());
    if is_wayland() {
        tauri::async_runtime::spawn(async move {
            register_portal_app().await;
            let parent = portal_parent::get(&app).await;
            let result = async {
                let _dialog = PortalDialogGuard::new();
                let proxy = RemoteDesktop::new().await?;
                let session = proxy.create_session(Default::default()).await?;
                proxy
                    .select_devices(
                        &session,
                        SelectDevicesOptions::default()
                            .set_devices(Some(DeviceType::Keyboard.into())),
                    )
                    .await?
                    .response()?;
                let devices = proxy
                    .start(&session, parent, Default::default())
                    .await?
                    .response()?;
                if !devices.devices().contains(DeviceType::Keyboard) {
                    session.close().await?;
                    return Err(ashpd::Error::NoResponse);
                }
                Ok::<_, ashpd::Error>((proxy, session))
            }
            .await;
            match result {
                Ok(input) => {
                    if let Ok(mut slot) = INPUT.lock() {
                        *slot = Some(input);
                        crate::log_debug("[linux] keyboard portal access granted");
                    }
                }
                Err(error) => report_error("err_linux_input_permission", &error.to_string()),
            }
        });
    }
}

/// Called from transcription worker threads, never the UI or async executor.
pub fn key_sequence(symbols: &[(i32, bool)]) -> Result<(), String> {
    send_key_sequence(symbols, false)
}

pub fn keycode_sequence(keys: &[(i32, bool)]) -> Result<(), String> {
    send_key_sequence(keys, true)
}

async fn notify_key(
    proxy: &RemoteDesktop,
    session: &Session<RemoteDesktop>,
    key: i32,
    pressed: bool,
    keycodes: bool,
) -> Result<(), ashpd::Error> {
    let state = if pressed {
        KeyState::Pressed
    } else {
        KeyState::Released
    };
    if keycodes {
        proxy
            .notify_keyboard_keycode(session, key, state, Default::default())
            .await
    } else {
        proxy
            .notify_keyboard_keysym(session, key, state, Default::default())
            .await
    }
}

fn send_key_sequence(symbols: &[(i32, bool)], keycodes: bool) -> Result<(), String> {
    let input = INPUT
        .lock()
        .map_err(|_| "Input session lock failed".to_string())?;
    let (proxy, session) = input
        .as_ref()
        .ok_or("Wayland keyboard access has not been granted")?;
    tauri::async_runtime::block_on(async {
        for &(symbol, pressed) in symbols {
            let result = notify_key(proxy, session, symbol, pressed, keycodes).await;
            if let Err(error) = result {
                for &(symbol, pressed) in symbols.iter().rev() {
                    if pressed {
                        let _ = notify_key(proxy, session, symbol, false, keycodes).await;
                    }
                }
                return Err(error.to_string());
            }
        }
        Ok(())
    })
}

pub fn refresh_portal_hotkeys() {
    HOTKEY_GENERATION.fetch_add(1, Ordering::SeqCst);
}

fn shortcut_description(language: &str, action: &str) -> String {
    let system_ru = std::env::var("LANG").is_ok_and(|value| value.starts_with("ru"));
    let language = if language == "ru" || language == "system" && system_ru {
        "ru"
    } else {
        "en"
    };
    serde_json::from_str::<serde_json::Value>(include_str!("../../locales/linux.json"))
        .ok()
        .and_then(|locales| locales[language][action].as_str().map(str::to_owned))
        .unwrap_or_else(|| "Vispeak".to_string())
}

fn portal_trigger(hotkey: &str) -> Result<String, String> {
    let mut parts = Vec::new();
    for part in hotkey.split('+') {
        parts.push(match part {
            "Control" | "ControlLeft" | "ControlRight" => "CTRL",
            "Shift" | "ShiftLeft" | "ShiftRight" => "SHIFT",
            "Alt" | "AltLeft" | "AltRight" => "ALT",
            "Meta" | "MetaLeft" | "MetaRight" => "LOGO",
            "Space" => "space",
            "Enter" => "Return",
            "ArrowLeft" => "Left",
            "ArrowRight" => "Right",
            "ArrowUp" => "Up",
            "ArrowDown" => "Down",
            other => other,
        });
    }
    if parts.iter().any(|part| part.is_empty())
        || parts
            .iter()
            .filter(|part| !matches!(**part, "CTRL" | "SHIFT" | "ALT" | "LOGO"))
            .count()
            != 1
    {
        return Err("err_linux_shortcut_key".into());
    }
    Ok(parts.join("+"))
}

pub fn validate_hotkey_update(app: &tauri::AppHandle, hotkey: &str) -> Result<(), String> {
    if !is_wayland() {
        return Ok(());
    }
    portal_trigger(hotkey)?;
    let state = app.state::<Arc<Mutex<crate::audio::AudioState>>>();
    let state = state.lock().map_err(|_| "Audio state lock failed")?;
    if state.is_recording || state.is_processing {
        return Err("err_linux_shortcut_busy".into());
    }
    Ok(())
}

pub(crate) fn start_portal_hotkeys(
    app: tauri::AppHandle,
    tx: Sender<crate::hotkeys::HotkeyAction>,
) {
    tauri::async_runtime::spawn(async move {
        register_portal_app().await;
        loop {
            let generation = HOTKEY_GENERATION.load(Ordering::SeqCst);
            if let Err(error) = portal_hotkeys(&app, &tx, generation).await {
                report_error("err_linux_shortcuts", &error);
                // Retry only after the user changes a shortcut, not on every timer tick.
                while generation == HOTKEY_GENERATION.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(250)).await;
                }
            }
        }
    });
}

async fn bind_shortcut(
    proxy: &GlobalShortcuts,
    id: &str,
    hotkey: &str,
    description: &str,
) -> Result<Session<GlobalShortcuts>, String> {
    let trigger = portal_trigger(hotkey)?;
    let parent = match APP.get() {
        Some(app) => portal_parent::get(app).await,
        None => None,
    };
    let _dialog = PortalDialogGuard::new();
    let session = proxy
        .create_session(Default::default())
        .await
        .map_err(|e| e.to_string())?;
    let result = proxy
        .bind_shortcuts(
            &session,
            &[NewShortcut::new(id, description).preferred_trigger(trigger.as_str())],
            parent,
            Default::default(),
        )
        .await
        .and_then(|request| request.response());
    match result {
        Ok(response) => {
            if id == "record" {
                update_record_shortcut(response.shortcuts());
            }
            Ok(session)
        }
        Err(error) => {
            let _ = session.close().await;
            Err(error.to_string())
        }
    }
}

fn shortcut_session_path(session: &Session<GlobalShortcuts>) -> Result<String, String> {
    serde_json::to_value(session)
        .map_err(|error| error.to_string())?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "Invalid portal session handle".into())
}

async fn portal_hotkeys(
    app: &tauri::AppHandle,
    tx: &Sender<crate::hotkeys::HotkeyAction>,
    generation: u64,
) -> Result<(), String> {
    use crate::hotkeys::HotkeyAction;
    enum PortalEvent {
        Key(String, String, bool),
        Changed(String, Vec<ashpd::desktop::global_shortcuts::Shortcut>),
    }
    let proxy = Arc::new(GlobalShortcuts::new().await.map_err(|e| e.to_string())?);
    let activated = proxy
        .receive_activated()
        .await
        .map_err(|e| e.to_string())?
        .map(|event| {
            PortalEvent::Key(
                event.session_handle().to_string(),
                event.shortcut_id().to_string(),
                true,
            )
        });
    let deactivated = proxy
        .receive_deactivated()
        .await
        .map_err(|e| e.to_string())?
        .map(|event| {
            PortalEvent::Key(
                event.session_handle().to_string(),
                event.shortcut_id().to_string(),
                false,
            )
        });
    let changed = proxy
        .receive_shortcuts_changed()
        .await
        .map_err(|e| e.to_string())?;
    let changed = changed.map(|event| {
        PortalEvent::Changed(
            event.session_handle().to_string(),
            event.shortcuts().to_vec(),
        )
    });
    let events = stream::select(stream::select(activated, deactivated), changed);
    futures_util::pin_mut!(events);
    let settings = crate::settings::load_settings();
    let description = shortcut_description(&settings.app_language, "record");
    let session = Arc::new(bind_shortcut(&proxy, "record", &settings.hotkey, &description).await?);
    let session_path = shortcut_session_path(&session)?;
    if let Ok(mut slot) = RECORD_PORTAL.lock() {
        *slot = Some((proxy.clone(), session.clone()));
    }
    if let Ok(mut errors) = ERRORS.lock() {
        errors.retain(|code| code != "err_linux_shortcuts");
    }
    let cancel_proxy = GlobalShortcuts::with_connection(proxy.connection().clone())
        .await
        .map_err(|error| error.to_string())?;
    let cancel_active = Arc::new(AtomicBool::new(true));
    let cancel_active_task = cancel_active.clone();
    let cancel_app = app.clone();
    // Authorization dialogs must not block record/release events (Push-to-Talk).
    tauri::async_runtime::spawn(async move {
        manage_cancel_shortcut(cancel_proxy, cancel_app, generation, cancel_active_task).await;
    });
    let mut started = None;
    while generation == HOTKEY_GENERATION.load(Ordering::SeqCst) {
        let (recording, processing) = {
            let state = app.state::<std::sync::Arc<Mutex<crate::audio::AudioState>>>();
            let state = state.lock().map_err(|_| "Audio state lock failed")?;
            (state.is_recording, state.is_processing)
        };
        let Ok(event) = tokio::time::timeout(Duration::from_millis(100), events.next()).await
        else {
            continue;
        };
        let (id, pressed) = match event {
            Some(PortalEvent::Key(path, id, pressed)) => {
                let belongs = if id == "record" {
                    path == session_path
                } else if id == "cancel" {
                    CANCEL_SESSION
                        .lock()
                        .ok()
                        .is_some_and(|value| value.as_deref() == Some(path.as_str()))
                } else {
                    false
                };
                if !belongs {
                    continue;
                }
                (id, pressed)
            }
            Some(PortalEvent::Changed(path, shortcuts)) => {
                if path != session_path {
                    continue;
                }
                if shortcuts.iter().any(|shortcut| shortcut.id() == "record") {
                    update_record_shortcut(&shortcuts);
                }
                continue;
            }
            None => break,
        };
        crate::log_debug(&format!(
            "[linux] shortcut {id}: {}",
            if pressed { "pressed" } else { "released" }
        ));
        let settings = crate::settings::load_settings();
        let action = match (id.as_str(), pressed) {
            ("cancel", true) if recording || processing => Some(HotkeyAction::CancelRecording),
            ("record", true) if !recording && !processing && started.is_none() => {
                started = Some(Instant::now());
                Some(HotkeyAction::StartRecording)
            }
            ("record", true) if recording && !settings.push_to_talk && started.is_none() => {
                started = Some(Instant::now());
                Some(HotkeyAction::StopRecording)
            }
            ("record", false) if settings.push_to_talk => started.take().map(|start| {
                if start.elapsed() < Duration::from_millis(300) {
                    HotkeyAction::CancelRecordingSilently
                } else {
                    HotkeyAction::StopRecording
                }
            }),
            ("record", false) => {
                started = None;
                None
            }
            _ => None,
        };
        if let Some(action) = action {
            tx.send(action).map_err(|e| e.to_string())?;
        }
    }
    cancel_active.store(false, Ordering::SeqCst);
    if let Ok(mut slot) = RECORD_PORTAL.lock() {
        *slot = None;
    }
    session.close().await.map_err(|e| e.to_string())
}

async fn manage_cancel_shortcut(
    proxy: GlobalShortcuts,
    app: tauri::AppHandle,
    generation: u64,
    alive: Arc<AtomicBool>,
) {
    let mut session = None;
    let mut failed = false;
    while alive.load(Ordering::SeqCst) && generation == HOTKEY_GENERATION.load(Ordering::SeqCst) {
        let busy = {
            let state = app.state::<Arc<Mutex<crate::audio::AudioState>>>();
            let busy = state
                .lock()
                .map(|state| state.is_recording || state.is_processing)
                .unwrap_or(false);
            busy
        };
        if busy && session.is_none() && !failed {
            let settings = crate::settings::load_settings();
            let description = shortcut_description(&settings.app_language, "cancel");
            match bind_shortcut(&proxy, "cancel", &settings.cancel_hotkey, &description).await {
                Ok(bound) => {
                    if let Ok(mut slot) = CANCEL_SESSION.lock() {
                        *slot = shortcut_session_path(&bound).ok();
                    }
                    session = Some(bound);
                }
                Err(error) => {
                    failed = true;
                    report_error("err_linux_shortcuts", &error);
                }
            }
        } else if !busy {
            if let Some(bound) = session.take() {
                if let Ok(mut slot) = CANCEL_SESSION.lock() {
                    *slot = None;
                }
                let _ = bound.close().await;
            }
            failed = false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if let Some(bound) = session {
        if let Ok(mut slot) = CANCEL_SESSION.lock() {
            *slot = None;
        }
        let _ = bound.close().await;
    }
}

pub fn show_overlay(app: tauri::AppHandle) {
    overlay::show(app);
}

#[cfg(test)]
mod tests {
    #[test]
    fn hidden_vispeak_window_with_stale_focus_does_not_block_insertion() {
        assert!(!super::blocks_dictation(false, true));
        assert!(!super::blocks_dictation(false, false));
        assert!(!super::blocks_dictation(true, false));
        assert!(super::blocks_dictation(true, true));
    }
    use super::portal_trigger;
    #[test]
    fn portal_shortcuts_use_xdg_names() {
        assert_eq!(portal_trigger("ControlLeft+Space").unwrap(), "CTRL+space");
        assert_eq!(
            portal_trigger("Meta+Shift+Enter").unwrap(),
            "LOGO+SHIFT+Return"
        );
        assert!(portal_trigger("AltRight").is_err());
        assert!(portal_trigger("").is_err());
        assert!(portal_trigger("Control+").is_err());
        assert!(portal_trigger("Control+A+B").is_err());
    }
}
