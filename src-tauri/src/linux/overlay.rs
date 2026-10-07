use gtk::prelude::*;
use gtk_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use tauri::Manager;

pub fn setup(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window("overlay") else {
        return;
    };
    if let Ok(native) = window.gtk_window() {
        native.set_accept_focus(false);
        native.set_focus_on_map(false);
        native.set_app_paintable(true);
        if let Some(visual) =
            gtk::prelude::WidgetExt::screen(&native).and_then(|screen| screen.rgba_visual())
        {
            native.set_visual(Some(&visual));
        }
        native.connect_draw(|native, context| {
            // Tao restores accept-focus on its first draw for initially hidden
            // windows; keep this overlay non-focusable after that handler.
            native.set_accept_focus(false);
            let _ = context.save();
            context.set_operator(gtk::cairo::Operator::Source);
            context.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            let _ = context.paint();
            let _ = context.restore();
            gtk::glib::Propagation::Proceed
        });
        native.set_widget_name("vispeak-overlay");
        let provider = gtk::CssProvider::new();
        let _ = provider.load_from_data(
            b"#vispeak-overlay, #vispeak-overlay * { background-color: transparent; background-image: none; box-shadow: none; }",
        );
        native
            .style_context()
            .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_USER);
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::StyleContext::add_provider_for_screen(
                &display.default_screen(),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_USER,
            );
        }
        native.connect_realize(|native| {
            if let Some(surface) = native.window() {
                surface.set_opaque_region(None);
            }
        });

        if super::is_wayland() && gtk_layer_shell::is_supported() {
            if native.is_realized() {
                native.unrealize();
            }
            native.init_layer_shell();
            native.set_namespace("vispeak-overlay");
            native.set_layer(Layer::Overlay);
            native.set_keyboard_mode(KeyboardMode::None);
            native.set_exclusive_zone(0);
            native.set_anchor(Edge::Bottom, true);
            native.set_layer_shell_margin(Edge::Bottom, 12);
            eprintln!("[linux] overlay: layer-shell, keyboard mode none");
        }
    }
    let _ = window.with_webview(|webview| {
        use webkit2gtk::{SettingsExt, WebViewExt};
        // Avoid GPU compositing in the small overlay while using the
        // Wayland DMA-BUF fallback. GTK alpha is configured separately.
        if let Some(settings) = WebViewExt::settings(&webview.inner()) {
            settings
                .set_hardware_acceleration_policy(webkit2gtk::HardwareAccelerationPolicy::Never);
        }
        webview
            .inner()
            .set_background_color(&gtk::gdk::RGBA::new(0.0, 0.0, 0.0, 0.0));
        webview.inner().set_app_paintable(true);
    });
}

pub fn show(app: tauri::AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(window) = handle.get_webview_window("overlay") else {
            return;
        };
        let settings = crate::settings::load_settings();
        let (width, height) = match settings.overlay_skin.as_str() {
            "mini" => (134.0, 82.0),
            "compact" => (247.0, 92.0),
            _ => (359.0, 151.0),
        };
        let _ = window.set_size(tauri::LogicalSize::new(width, height));
        let caret = if settings.overlay_skin == "mini" {
            let state =
                handle.state::<std::sync::Arc<std::sync::Mutex<crate::audio::AudioState>>>();
            state.lock().ok().and_then(|state| state.caret_pos)
        } else {
            None
        };
        if settings.overlay_skin == "mini" && caret.is_none() {
            crate::log_debug("[linux] overlay fallback: no verified text caret geometry");
        }
        if let Ok(native) = window.gtk_window() {
            native.set_default_size(width as i32, height as i32);
            if native.is_layer_window() {
                native.set_size_request(width as i32, height as i32);
            }
            native.set_accept_focus(false);
            native.set_focus_on_map(false);
            if native.is_layer_window() {
                if let (Some(caret), Some(display)) = (caret, gtk::gdk::Display::default()) {
                    for index in 0..display.n_monitors() {
                        let Some(monitor) = display.monitor(index) else {
                            continue;
                        };
                        let geometry = monitor.geometry();
                        if caret.left >= geometry.x()
                            && caret.left < geometry.x() + geometry.width()
                            && caret.top >= geometry.y()
                            && caret.top < geometry.y() + geometry.height()
                        {
                            let Some((x, y)) = mini_position(
                                caret.left - geometry.x(),
                                caret.top - geometry.y(),
                                width as i32,
                                height as i32,
                                geometry.width(),
                                geometry.height(),
                            ) else {
                                break;
                            };
                            native.set_monitor(&monitor);
                            native.set_anchor(Edge::Top, true);
                            native.set_anchor(Edge::Left, true);
                            native.set_anchor(Edge::Bottom, false);
                            native.set_anchor(Edge::Right, false);
                            // Clear margins left by the screen-edge fallback.
                            native.set_layer_shell_margin(Edge::Bottom, 0);
                            native.set_layer_shell_margin(Edge::Right, 0);
                            native.set_layer_shell_margin(Edge::Left, x);
                            native.set_layer_shell_margin(Edge::Top, y);
                            native.show_all();
                            return;
                        }
                    }
                }
                if settings.overlay_skin == "mini" {
                    crate::log_debug("[linux] overlay fallback: caret placement unavailable");
                }
                let top = settings.overlay_position == "top-center";
                native.set_anchor(Edge::Top, top);
                native.set_anchor(Edge::Bottom, !top);
                native.set_anchor(Edge::Left, settings.overlay_position == "bottom-left");
                native.set_anchor(Edge::Right, settings.overlay_position == "bottom-right");
                native.set_layer_shell_margin(Edge::Top, 12);
                native.set_layer_shell_margin(Edge::Bottom, 12);
                native.set_layer_shell_margin(Edge::Left, 12);
                native.set_layer_shell_margin(Edge::Right, 12);
                native.show_all();
                return;
            }
        }
        // X11 supports absolute placement; ordinary Wayland toplevels do not.
        if !super::is_wayland() {
            if let (Some(caret), Some(display)) = (caret, gtk::gdk::Display::default()) {
                if let Some(monitor) = display.monitor_at_point(caret.left, caret.top) {
                    let geometry = monitor.workarea();
                    if let Some((x, y)) = mini_position(
                        caret.left - geometry.x(),
                        caret.top - geometry.y(),
                        width as i32,
                        height as i32,
                        geometry.width(),
                        geometry.height(),
                    ) {
                        let _ = window.set_position(tauri::LogicalPosition::new(
                            x + geometry.x(),
                            y + geometry.y(),
                        ));
                        let _ = window.show();
                        return;
                    }
                }
            }
        }
        if settings.overlay_skin == "mini" {
            crate::log_debug("[linux] overlay fallback: compositor cannot place it at the caret");
        }
        if let Ok(Some(monitor)) = window
            .current_monitor()
            .or_else(|_| window.primary_monitor())
        {
            let scale = monitor.scale_factor();
            let origin = monitor.position();
            let size = monitor.size();
            let x = origin.x as f64 + (size.width as f64 - width * scale) / 2.0;
            let y = origin.y as f64 + size.height as f64 - (height + 12.0) * scale;
            let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
        }
        let _ = window.show();
    });
}

pub fn resize(app: tauri::AppHandle, height: f64) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = handle.get_webview_window("overlay") {
            let width = match crate::settings::load_settings().overlay_skin.as_str() {
                "mini" => 134.0,
                "compact" => 247.0,
                _ => 359.0,
            };
            let _ = window.set_size(tauri::LogicalSize::new(width, height));
            if let Ok(native) = window.gtk_window() {
                if native.is_layer_window() {
                    native.set_size_request(width as i32, height as i32);
                }
            }
        }
    });
}

// Center the overlay horizontally over the caret and keep the entire window,
// including its glow padding, at least 8 px above the caret.
fn mini_position(
    left: i32,
    top: i32,
    width: i32,
    height: i32,
    monitor_width: i32,
    monitor_height: i32,
) -> Option<(i32, i32)> {
    let gap = 8;
    let y = top.checked_sub(gap + height)?;
    if y < 0 || top >= monitor_height || left < 0 || left >= monitor_width {
        return None;
    }
    let x = (left - width / 2).clamp(0, (monitor_width - width).max(0));
    Some((x, y))
}

#[cfg(test)]
mod tests {
    use super::mini_position;

    #[test]
    fn capsule_is_centered_strictly_above_caret() {
        let (x, y) = mini_position(500, 400, 134, 82, 1920, 1080).unwrap();
        assert_eq!(x + 134 / 2, 500);
        assert_eq!(y + 82, 400 - 8);
    }

    #[test]
    fn never_moves_capsule_below_caret_or_to_screen_bottom() {
        assert_eq!(mini_position(500, 10, 134, 82, 1920, 1080), None);
        assert_eq!(mini_position(500, 89, 134, 82, 1920, 1080), None);
        assert_eq!(mini_position(500, 90, 134, 82, 1920, 1080), Some((433, 0)));
        assert_eq!(
            mini_position(1919, 1070, 134, 82, 1920, 1080),
            Some((1786, 980))
        );
        assert_eq!(mini_position(0, 0, 134, 82, 100, 60), None);
        assert_eq!(mini_position(500, 1080, 134, 82, 1920, 1080), None);
    }
}
