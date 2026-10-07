//! Capabilities exposed to settings and the updater UI.
#[derive(serde::Serialize)]
pub struct DesktopCapabilities {
    pub native_updater: bool,
    pub updater_target: Option<String>,
    pub live_typing: bool,
    pub wayland: bool,
    pub errors: Vec<String>,
    pub record_shortcut: Option<String>,
    pub input_ready: bool,
    pub caret_available: Option<bool>,
}

#[tauri::command]
pub fn get_desktop_capabilities() -> DesktopCapabilities {
    let updater_target = updater_target();
    #[cfg(target_os = "linux")]
    let (wayland, errors) = (
        crate::linux::is_wayland(),
        crate::linux::integration_errors(),
    );
    #[cfg(windows)]
    let (wayland, errors) = (false, Vec::new());
    DesktopCapabilities {
        native_updater: cfg!(windows) || updater_target.is_some(),
        updater_target,
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

fn updater_target() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let id = release
            .lines()
            .find_map(|line| line.strip_prefix("ID="))
            .unwrap_or("")
            .trim_matches('"');
        return linux_updater_target(tauri::utils::platform::bundle_type(), id).map(str::to_owned);
    }
    #[cfg(not(target_os = "linux"))]
    None
}

#[cfg(target_os = "linux")]
fn linux_updater_target(
    bundle: Option<tauri::utils::config::BundleType>,
    id: &str,
) -> Option<&'static str> {
    use tauri::utils::config::BundleType;
    match (bundle, id) {
        (Some(BundleType::Rpm), "fedora") => Some("linux-x86_64-rpm"),
        (Some(BundleType::Deb), "ubuntu") => Some("linux-x86_64-ubuntu-deb"),
        (Some(BundleType::Deb), "debian") => Some("linux-x86_64-debian-deb"),
        _ => None,
    }
}

#[cfg(all(test, target_os = "linux"))]
mod updater_tests {
    use super::*;
    use tauri::utils::config::BundleType;
    #[test]
    fn updates_only_supported_installed_packages_with_matching_distribution() {
        assert_eq!(
            linux_updater_target(Some(BundleType::Rpm), "fedora"),
            Some("linux-x86_64-rpm")
        );
        assert_eq!(
            linux_updater_target(Some(BundleType::Deb), "ubuntu"),
            Some("linux-x86_64-ubuntu-deb")
        );
        assert_eq!(
            linux_updater_target(Some(BundleType::Deb), "debian"),
            Some("linux-x86_64-debian-deb")
        );
        assert_eq!(linux_updater_target(Some(BundleType::Rpm), "ubuntu"), None);
        assert_eq!(linux_updater_target(Some(BundleType::Deb), "unknown"), None);
        assert_eq!(linux_updater_target(None, "fedora"), None);
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
