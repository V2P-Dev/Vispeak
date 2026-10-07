//! Read the active client rectangle without changing compositor settings.
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub struct ActiveWindow {
    pub pid: u32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub caret: Option<InputCaret>,
}

#[derive(Clone, Copy, Debug, serde::Deserialize)]
pub struct InputCaret {
    pub available: bool,
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub height: f64,
}

impl ActiveWindow {
    pub fn text_caret(&self) -> Option<crate::caret_position::CaretRect> {
        let caret = self.caret?;
        if !caret.available
            || ![caret.x, caret.y, caret.width, caret.height]
                .iter()
                .all(|v| v.is_finite())
            || caret.width < 0.0
            || caret.width > 8.0
            || caret.height <= 0.0
            || caret.x < self.x
            || caret.y < self.y
            || caret.x + caret.width > self.x + self.width
            || caret.y + caret.height > self.y + self.height
        {
            return None;
        }
        let left = caret.x.round() as i32;
        let top = caret.y.round() as i32;
        Some(crate::caret_position::CaretRect {
            left,
            top,
            right: left.checked_add(1)?,
            bottom: top.checked_add(caret.height.ceil() as i32)?,
            kind: crate::caret_position::CaretKind::Caret,
        })
    }

    pub fn screen_caret(
        &self,
        rect: crate::caret_position::CaretRect,
    ) -> Option<crate::caret_position::CaretRect> {
        if rect.left < 0
            || rect.top < 0
            || rect.left as f64 >= self.width
            || rect.bottom as f64 > self.height
            || rect.bottom <= rect.top
        {
            return None;
        }
        let x = self.x.round() as i32;
        let y = self.y.round() as i32;
        Some(crate::caret_position::CaretRect {
            left: rect.left.checked_add(x)?,
            top: rect.top.checked_add(y)?,
            right: rect.right.checked_add(x)?,
            bottom: rect.bottom.checked_add(y)?,
            kind: rect.kind,
        })
    }
}

struct Receiver {
    owner: String,
    sender: Mutex<Option<oneshot::Sender<Option<ActiveWindow>>>>,
}

#[zbus::interface(name = "app.vispeak.WindowProbe")]
impl Receiver {
    fn report(&self, data: &str, #[zbus(header)] header: zbus::message::Header<'_>) {
        if header.sender().map(|sender| sender.as_str()) != Some(self.owner.as_str()) {
            return;
        }
        let window = serde_json::from_str::<Option<ActiveWindow>>(data)
            .ok()
            .flatten()
            .filter(|window| {
                window.pid > 0
                    && window.width.is_finite()
                    && window.height.is_finite()
                    && window.x.is_finite()
                    && window.y.is_finite()
                    && window.width > 0.0
                    && window.height > 0.0
            });
        if let Some(sender) = self.sender.lock().ok().and_then(|mut sender| sender.take()) {
            let _ = sender.send(window);
        }
    }
}

pub async fn active_window() -> Option<ActiveWindow> {
    if !super::is_wayland()
        || !std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|value| {
            value
                .split(':')
                .any(|value| value.eq_ignore_ascii_case("KDE"))
        })
    {
        return None;
    }
    // Production loads only an explicitly configured module. Development can
    // use the bridge deliberately built in this checkout's ignored target dir.
    let module = std::env::var_os("VISPEAK_KWIN_CARET_MODULE")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            if cfg!(debug_assertions) {
                Some(
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("target/kwin-caret/Vispeak/CaretBridge"),
                )
            } else {
                None
            }
        })
        .filter(|path| {
            path.is_absolute()
                && path.join("qmldir").is_file()
                && path.join("libVispeakCaretBridge.so").is_file()
        });
    let window = probe(module.clone()).await;
    if window.is_none() && module.is_some() {
        crate::log_debug("[caret] native KWin probe failed; using ordinary window geometry");
        probe(None).await
    } else {
        window
    }
}

async fn probe(module: Option<std::path::PathBuf>) -> Option<ActiveWindow> {
    let connection = zbus::Connection::session().await.ok()?;
    let bus = zbus::Proxy::new(
        &connection,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .await
    .ok()?;
    let owner: String = bus.call("GetNameOwner", &("org.kde.KWin",)).await.ok()?;
    let (sender, receiver) = oneshot::channel();
    connection
        .object_server()
        .at(
            "/app/vispeak/WindowProbe",
            Receiver {
                owner,
                sender: Mutex::new(Some(sender)),
            },
        )
        .await
        .ok()?;
    let destination = serde_json::to_string(connection.unique_name()?.as_str()).ok()?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let name = format!("vispeak-window-probe-{}-{nonce}", std::process::id());
    let directory = std::env::temp_dir().join(&name);
    std::fs::create_dir(&directory).ok()?;
    let script_path = directory.join(if module.is_some() {
        "probe.qml"
    } else {
        "probe.js"
    });
    let script = if let Some(module) = &module {
        let url = tauri::Url::from_directory_path(module).ok()?;
        let import = serde_json::to_string(url.as_str()).ok()?;
        format!("import QtQuick\nimport org.kde.kwin\nimport {import} as Bridge\nDBusCall {{ id: probe; service: {destination}; path: '/app/vispeak/WindowProbe'; dbusInterface: 'app.vispeak.WindowProbe'; method: 'Report'; Component.onCompleted: {{ var w = Workspace.activeWindow; var r = w ? w.clientGeometry : null; probe.arguments = [JSON.stringify(w ? {{pid:w.pid,x:r.x,y:r.y,width:r.width,height:r.height,caret:Bridge.CaretReader.read(w)}} : null)]; probe.call(); }} }}")
    } else {
        format!("var w = workspace.activeWindow; var r = w ? w.clientGeometry : null; callDBus({destination}, '/app/vispeak/WindowProbe', 'app.vispeak.WindowProbe', 'Report', JSON.stringify(w ? {{pid:w.pid,x:r.x,y:r.y,width:r.width,height:r.height}} : null));")
    };
    let result = async {
        std::fs::write(&script_path, script).ok()?;
        let scripting = zbus::Proxy::new(
            &connection,
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting",
        )
        .await
        .ok()?;
        let index: i32 = scripting
            .call(
                if module.is_some() {
                    "loadDeclarativeScript"
                } else {
                    "loadScript"
                },
                &(script_path.to_str()?, &name),
            )
            .await
            .ok()?;
        if index < 0 {
            return None;
        }
        let path = format!("/Scripting/Script{index}");
        let proxy = zbus::Proxy::new(
            &connection,
            "org.kde.KWin",
            path.as_str(),
            "org.kde.kwin.Script",
        )
        .await
        .ok()?;
        proxy.call::<_, _, ()>("run", &()).await.ok()?;
        receiver.await.ok().flatten()
    };
    let window = tokio::time::timeout(Duration::from_millis(300), result)
        .await
        .ok()
        .flatten();
    // Clean up even if KWin does not answer the probe.
    if let Ok(scripting) = zbus::Proxy::new(
        &connection,
        "org.kde.KWin",
        "/Scripting",
        "org.kde.kwin.Scripting",
    )
    .await
    {
        let _ = tokio::time::timeout(
            Duration::from_millis(100),
            scripting.call::<_, _, bool>("unloadScript", &(&name,)),
        )
        .await;
    }
    let _ = std::fs::remove_file(script_path);
    let _ = std::fs::remove_dir(directory);
    window
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::caret_position::{CaretKind, CaretRect};

    #[test]
    fn compositor_caret_is_global_and_rejects_stale_or_invalid_geometry() {
        let caret = InputCaret {
            available: true,
            x: 508.0,
            y: 240.0,
            width: 0.0,
            height: 19.5,
        };
        let window = ActiveWindow {
            pid: 1,
            x: 8.0,
            y: 40.0,
            width: 1904.0,
            height: 1032.0,
            caret: Some(caret),
        };
        let rect = window.text_caret().unwrap();
        assert_eq!((rect.left, rect.top, rect.bottom), (508, 240, 260));
        for invalid in [
            InputCaret {
                available: false,
                ..caret
            },
            InputCaret {
                x: f64::NAN,
                ..caret
            },
            InputCaret { x: 0.0, ..caret },
            InputCaret {
                height: 0.0,
                ..caret
            },
            InputCaret {
                width: 300.0,
                ..caret
            },
            InputCaret { y: 1071.0, ..caret },
        ] {
            assert!(ActiveWindow {
                caret: Some(invalid),
                ..window
            }
            .text_caret()
            .is_none());
        }
        assert!(ActiveWindow {
            caret: None,
            ..window
        }
        .text_caret()
        .is_none());
    }

    #[test]
    fn window_coordinates_include_client_origin_and_reject_outside_caret() {
        let window = ActiveWindow {
            pid: 1,
            x: 8.0,
            y: 40.0,
            width: 1904.0,
            height: 1032.0,
            caret: None,
        };
        let rect = CaretRect {
            left: 100,
            top: 200,
            right: 101,
            bottom: 220,
            kind: CaretKind::Caret,
        };
        let screen = window.screen_caret(rect).unwrap();
        assert_eq!((screen.left, screen.top, screen.bottom), (108, 240, 260));
        assert!(window.screen_caret(CaretRect { top: -1, ..rect }).is_none());
        assert!(window
            .screen_caret(CaretRect {
                bottom: 1040,
                ..rect
            })
            .is_none());
    }

    #[test]
    #[ignore = "requires a running KDE Wayland session"]
    fn reads_active_kde_client_and_unloads_probe() {
        let window = tauri::async_runtime::block_on(active_window())
            .expect("KWin did not return active client geometry");
        assert!(window.pid > 0 && window.width > 0.0 && window.height > 0.0);
    }
}
