//! Export the GTK main window for portal dialogs. The export lives with the window.
use gtk::{glib::translate::ToGlibPtr, prelude::*};
use tauri::Manager;

static PARENT: tokio::sync::OnceCell<Option<ashpd::WindowIdentifier>> =
    tokio::sync::OnceCell::const_new();

pub async fn get(app: &tauri::AppHandle) -> Option<&'static ashpd::WindowIdentifier> {
    PARENT
        .get_or_init(|| async {
            let (sender, receiver) = tokio::sync::oneshot::channel::<String>();
            let handle = app.clone();
            if app
                .run_on_main_thread(move || {
                    let Some(window) = handle.get_webview_window("main") else {
                        return;
                    };
                    let Ok(native) = window.gtk_window() else {
                        return;
                    };
                    native.realize();
                    let Some(surface) = native.window() else {
                        return;
                    };
                    // GDK retains user_data until destroy_notify, including on failure.
                    let data = Box::into_raw(Box::new(Some(sender)));
                    let pointer: *mut gtk::gdk::ffi::GdkWindow = surface.to_glib_none().0;
                    unsafe {
                        gdk_wayland_sys::gdk_wayland_window_export_handle(
                            pointer.cast(),
                            Some(exported),
                            data.cast(),
                            Some(destroy),
                        );
                    }
                })
                .is_err()
            {
                return None;
            }
            let name = tokio::time::timeout(std::time::Duration::from_secs(3), receiver)
                .await
                .ok()?
                .ok()?;
            let identifier: ashpd::WindowIdentifierType = format!("wayland:{name}").parse().ok()?;
            Some(identifier.into())
        })
        .await
        .as_ref()
}

type Sender = Option<tokio::sync::oneshot::Sender<String>>;
unsafe extern "C" fn exported(
    _: *mut gdk_wayland_sys::GdkWaylandWindow,
    handle: *const std::ffi::c_char,
    data: *mut std::ffi::c_void,
) {
    if handle.is_null() {
        return;
    }
    let sender = &mut *(data as *mut Sender);
    if let Some(sender) = sender.take() {
        let _ = sender.send(
            std::ffi::CStr::from_ptr(handle)
                .to_string_lossy()
                .into_owned(),
        );
    }
}
unsafe extern "C" fn destroy(data: *mut std::ffi::c_void) {
    drop(Box::from_raw(data as *mut Sender));
}
