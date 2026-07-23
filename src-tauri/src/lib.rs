use std::sync::{Arc, Mutex};
use tauri::Manager;

pub mod audio;
pub mod caret_position;
pub mod ducking;
pub mod history;
pub mod hotkeys;
pub mod icon;
pub mod models;
pub mod paste;
pub mod settings;
pub mod transcribe;
pub mod vad;

use audio::AudioState;

pub fn log_debug(msg: &str) {
    eprintln!("{}", msg);
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open("vispeak_debug.log") {
        use std::io::Write;
        let _ = writeln!(file, "{}", msg);
    }
}

pub fn show_overlay(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("overlay") {
        use std::mem;
        use windows::Win32::Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        };
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

        let hwnd = unsafe { GetForegroundWindow() };
        let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };

        let mut mi: MONITORINFO = unsafe { mem::zeroed() };
        mi.cbSize = mem::size_of::<MONITORINFO>() as u32;
        unsafe { let _ = GetMonitorInfoW(hmonitor, &mut mi); };

        let rc_work = mi.rcWork;
        let rc_monitor = mi.rcMonitor;

        let mut scale_factor = 1.0;
        if let Ok(monitors) = window.available_monitors() {
            for monitor in monitors {
                let pos = monitor.position();
                if pos.x == rc_monitor.left && pos.y == rc_monitor.top {
                    scale_factor = monitor.scale_factor();
                    break;
                }
            }
        }

        let settings = settings::load_settings();
        let is_mini = settings.overlay_skin == "mini";
        let is_compact = settings.overlay_skin == "compact";

        let logical_width = if is_mini {
            144.0
        } else if is_compact {
            268.0
        } else {
            392.0
        };
        let logical_height = if is_mini {
            82.0
        } else if is_compact {
            96.0
        } else {
            162.0
        };
        let logical_size = tauri::LogicalSize::new(logical_width, logical_height);
        let _ = window.set_size(logical_size);

        let margin = 12.0;
        let glow_padding = 24.0;

        let (caret_pos_opt, _method, mut trace_log) = {
            let state_arc = app.state::<Arc<Mutex<AudioState>>>();
            let state = state_arc.inner().lock().unwrap();
            (state.caret_pos, state.caret_method, state.caret_trace.clone())
        };

        let effective_anchor = if is_mini {
            if let Some(rect) = caret_pos_opt {
                match rect.kind {
                    crate::caret_position::CaretKind::Caret => { trace_log.push_str("Classification: Caret
"); Some((rect, "caret")) },
                    crate::caret_position::CaretKind::Field => { trace_log.push_str("Classification: Field
"); Some((rect, "field")) },
                    crate::caret_position::CaretKind::Area => {
                        use windows::Win32::Foundation::POINT;
                        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
                        let mut pt = POINT { x: 0, y: 0 };
                        let ok = unsafe { GetCursorPos(&mut pt) };
                        if ok.is_ok()
                            && pt.x >= rect.left
                            && pt.x <= rect.right
                            && pt.y >= rect.top
                            && pt.y <= rect.bottom
                        {
                            let mouse_rect = crate::caret_position::CaretRect {
                                left: rect.left,
                                top: pt.y,
                                right: rect.right,
                                bottom: pt.y,
                                kind: crate::caret_position::CaretKind::Area,
                            };
                            trace_log.push_str("Classification: Area (mouse_anchor)
");
                            Some((mouse_rect, "area (mouse_anchor)"))
                        } else {
                            trace_log.push_str(&format!("Classification: Area -> Fallback (mouse outside area {:?})
", rect));
                            None
                        }
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let (x_phys, y_phys) = if let Some((effective_rect, _kind_str)) = effective_anchor {
            use windows::Win32::Foundation::POINT;
            use windows::Win32::Graphics::Gdi::MonitorFromPoint;

            let pt = POINT {
                x: (effective_rect.left + effective_rect.right) / 2,
                y: (effective_rect.top + effective_rect.bottom) / 2,
            };
            let hmonitor_caret = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
            let mut mi_caret: MONITORINFO = unsafe { mem::zeroed() };
            mi_caret.cbSize = mem::size_of::<MONITORINFO>() as u32;
            unsafe {
                let _ = GetMonitorInfoW(hmonitor_caret, &mut mi_caret);
            };

            let rc_work_caret = mi_caret.rcWork;
            let rc_monitor_caret = mi_caret.rcMonitor;

            let mut scale_caret = scale_factor;
            if let Ok(monitors) = window.available_monitors() {
                for monitor in monitors {
                    let pos = monitor.position();
                    if pos.x == rc_monitor_caret.left && pos.y == rc_monitor_caret.top {
                        scale_caret = monitor.scale_factor();
                        break;
                    }
                }
            }

            let pos_res = crate::caret_position::calculate_mini_position(
                &effective_rect,
                scale_caret,
                logical_width,
                logical_height,
                glow_padding,
                &rc_work_caret,
            );

            trace_log.push_str(&format!("6. DPI Scale: {}, Rect: {:?}\n", scale_caret, effective_rect));
            
            let dx = pos_res.x - effective_rect.left as f64;
            let dy = pos_res.y - effective_rect.top as f64;
            if dx.abs() > 300.0 || dy.abs() > 300.0 {
                trace_log.push_str("[CARET ANOMALY] Final position far from raw rect\n");
            }

            trace_log.push_str(&format!(
                "10. Final Capsule Position: ({}, {}) Clamped X: {}, Placement: {}\n",
                pos_res.x, pos_res.y, pos_res.clamped_x, pos_res.placement
            ));

            (pos_res.x, pos_res.y)
        } else {
            let phys_width = logical_width * scale_factor;
            let phys_height = logical_height * scale_factor;
            let phys_glow = glow_padding * scale_factor;
            let phys_margin = margin * scale_factor;

            let left = rc_work.left as f64;
            let right = rc_work.right as f64;
            let top = rc_work.top as f64;
            let bottom = rc_work.bottom as f64;

            let pos_str = settings.overlay_position.trim();

            let x = match pos_str {
                "bottom-right" => right - phys_width + phys_glow - phys_margin,
                "bottom-left" => left - phys_glow + phys_margin,
                _ => left + (right - left - phys_width) / 2.0,
            };

            let y = match pos_str {
                "top-center" => top - phys_glow + phys_margin,
                _ => bottom - phys_height + phys_glow - phys_margin,
            };

            trace_log.push_str(&format!(
                "6. DPI Scale: {} (fallback window)\n", scale_factor
            ));
            trace_log.push_str(&format!(
                "10. Final Capsule Position: ({}, {}) pos_str='{}' scale={}\n",
                x, y, pos_str, scale_factor
            ));
            trace_log.push_str("[CARET FALLBACK]\n");

            (x, y)
        };

        let hotkey_time_opt = *crate::transcribe::DIAGNOSTIC_HOTKEY_TIME.lock().unwrap();
        if let Some(hotkey_time) = hotkey_time_opt {
            let elapsed_ms = hotkey_time.elapsed().as_millis();
            trace_log.push_str(&format!("9. Timing: {}ms from hotkey press to overlay show\n", elapsed_ms));
        } else {
            trace_log.push_str("9. Timing: unknown (hotkey time not set)\n");
        }
    
    crate::log_debug(&format!("{}===================", trace_log));

    let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
        x_phys.round() as i32,
        y_phys.round() as i32,
    )));
    let _ = window.set_size(logical_size);
    let _ = window.set_always_on_top(true);

    crate::log_debug("[OVERLAY_EVENT] Window SHOW (reason: overlay display logic executed)");
    let _ = window.show();
}
}

#[tauri::command]
fn restart_app(app: tauri::AppHandle) {
    app.restart();
}

#[tauri::command]
fn hide_overlay(app: tauri::AppHandle) {
    {
        use tauri::Manager;
        let state_arc = app.state::<std::sync::Arc<std::sync::Mutex<crate::audio::AudioState>>>();
        let state = state_arc.inner().lock().unwrap();
        if state.is_recording || state.is_processing {
            crate::log_debug("[OVERLAY_EVENT] Window HIDE ignored (reason: currently recording/processing, delayed IPC command)");
            return;
        }
    }

    if let Some(window) = app.get_webview_window("overlay") {
        crate::log_debug("[OVERLAY_EVENT] Window HIDE (reason: hide_overlay command called)");
        if let Ok(hwnd) = window.hwnd() {
            use windows::Win32::Foundation::HWND;
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            unsafe {
                let _ = ShowWindow(HWND(hwnd.0 as _), SW_HIDE);
            }
        }
        let _ = window.hide();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(main_window) = app.get_webview_window("main") {
                let _ = main_window.show();
                let _ = main_window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if window.label() == "main" {
                    window.hide().unwrap();
                    api.prevent_close();
                }
            }
            _ => {}
        })
        .manage(Arc::new(Mutex::new(AudioState::default())))
        .setup(|app| {
            if let Some(window) = app.get_webview_window("overlay") {
                if let Ok(hwnd) = window.hwnd() {
                    use windows::Win32::Foundation::HWND;
                    use windows::Win32::UI::WindowsAndMessaging::{
                        GetWindowLongW, SetWindowLongW, GWL_EXSTYLE, WS_EX_NOACTIVATE,
                    };
                    unsafe {
                        let hwnd = HWND(hwnd.0 as _);
                        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_NOACTIVATE.0 as i32);
                    }
                }
            }

            use tauri::menu::{Menu, MenuItem};
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

            let settings = crate::settings::load_settings();
            let lang = if settings.app_language == "system" {
                "en"
            } else {
                &settings.app_language
            };
            let settings_text = if lang == "ru" {
                "Настройки"
            } else {
                "Settings"
            };
            let quit_text = if lang == "ru" { "Выход" } else { "Quit" };

            let quit_i = MenuItem::with_id(app, "quit", quit_text, true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", settings_text, true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| match event {
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } => {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            transcribe::init_transcriber(app);
            hotkeys::setup_hotkeys(app)?;
            ducking::restore_all_on_startup();
            history::init_db();

            let settings = settings::load_settings();
            let args: Vec<String> = std::env::args().collect();
            let is_autostart = args.iter().any(|a| a == "--autostart");

            if let Some(main_window) = app.get_webview_window("main") {
                if !is_autostart && !settings.silent_start {
                    let _ = main_window.show();
                    let _ = main_window.set_focus();
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hide_overlay,
            audio::get_microphones,
            audio::start_preview,
            audio::stop_preview,
            models::get_models,
            models::download_model,
            models::delete_model,
            settings::get_active_model,
            settings::get_model_status,
            settings::set_active_model,
            settings::get_settings,
            settings::update_settings,
            settings::update_single_setting,
            hotkeys::update_hotkey,
            hotkeys::update_cancel_hotkey,
            hotkeys::update_push_to_talk,
            update_tray_lang,
            update_tray_tooltip,
            restart_app,
            history::get_history,
            history::delete_history_record,
            history::clear_history,
            history::get_history_size,
            history::retranscribe_history_record,
            history::update_history_record_text,
            history::repeat_paste_history_record
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn update_tray_tooltip(app: tauri::AppHandle, tooltip: String) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

#[tauri::command]
fn update_tray_lang(app: tauri::AppHandle, lang: String) {
    use tauri::menu::{Menu, MenuItem};

    let settings_text = if lang == "ru" {
        "Настройки"
    } else {
        "Settings"
    };
    let quit_text = if lang == "ru" { "Выход" } else { "Quit" };

    if let Ok(quit_i) = MenuItem::with_id(&app, "quit", quit_text, true, None::<&str>) {
        if let Ok(settings_i) =
            MenuItem::with_id(&app, "settings", settings_text, true, None::<&str>)
        {
            if let Ok(menu) = Menu::with_items(&app, &[&settings_i, &quit_i]) {
                if let Some(tray) = app.tray_by_id("main") {
                    let _ = tray.set_menu(Some(menu));
                }
            }
        }
    }
}
