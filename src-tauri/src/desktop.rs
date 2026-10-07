//! Capabilities exposed to settings and the updater UI.
#[derive(serde::Serialize)]
pub struct DesktopCapabilities {
    pub native_updater: bool,
    pub live_typing: bool,
    pub wayland: bool,
    pub errors: Vec<String>,
    pub record_shortcut: Option<String>,
    pub input_ready: bool,
    pub caret_available: Option<bool>,
}

#[tauri::command]
pub fn get_desktop_capabilities() -> DesktopCapabilities {
    #[cfg(target_os = "linux")]
    let (wayland, errors) = (
        crate::linux::is_wayland(),
        crate::linux::integration_errors(),
    );
    #[cfg(windows)]
    let (wayland, errors) = (false, Vec::new());
    DesktopCapabilities {
        native_updater: cfg!(windows),
        live_typing: crate::paste::supports_live_typing(),
        wayland,
        errors,
        caret_available: {
            #[cfg(target_os = "linux")]
            {
                crate::caret_position::availability()
            }
            #[cfg(windows)]
            {
                None
            }
        },
        input_ready: {
            #[cfg(target_os = "linux")]
            {
                crate::linux::input_ready()
            }
            #[cfg(windows)]
            {
                true
            }
        },
        record_shortcut: {
            #[cfg(target_os = "linux")]
            {
                crate::linux::record_shortcut()
            }
            #[cfg(windows)]
            {
                None
            }
        },
    }
}

#[tauri::command]
pub async fn configure_desktop_shortcuts() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        crate::linux::configure_record_shortcut().await
    }
    #[cfg(windows)]
    {
        Err("err_linux_shortcuts".into())
    }
}
