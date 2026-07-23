use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use transcribe_rs::onnx::canary::{CanaryModel, CanaryParams};
use transcribe_rs::onnx::gigaam::{GigaAMModel, GigaAMParams};
use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity};
use transcribe_rs::onnx::Quantization;

lazy_static::lazy_static! {
    pub static ref LOADED_MODEL_ID: Mutex<Option<String>> = Mutex::new(None);
    pub static ref DIAGNOSTIC_LAST_ACTIVITY: Mutex<std::time::Instant> = Mutex::new(std::time::Instant::now());
    pub static ref DIAGNOSTIC_HOTKEY_PRESS_COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    pub static ref DIAGNOSTIC_MODEL_STATE: Mutex<String> = Mutex::new("Unloaded".to_string());
    pub static ref DIAGNOSTIC_HOTKEY_TIME: Mutex<Option<std::time::Instant>> = Mutex::new(None);
}

pub fn is_model_loaded(model_id: &str) -> bool {
    let guard = LOADED_MODEL_ID.lock().unwrap();
    guard.as_deref() == Some(model_id)
}

enum ActiveModel {
    Whisper(WhisperContext),
    Parakeet(ParakeetModel),
    Canary(CanaryModel),
    GigaAM(GigaAMModel),
    Ggml(transcribe_cpp::Model),
}

pub enum TranscribeMsg {
    Request(TranscribeRequest),
    Preload { app: AppHandle, model_id: String },
    UnloadModel,
}

pub struct TranscriberState {
    tx: Sender<TranscribeMsg>,
}

pub struct TranscribeRequest {
    pub samples: Vec<f32>,
    pub model_id: String,
    pub app: AppHandle,
    pub is_retranscription: Option<i64>,
}

pub fn init_transcriber(app: &tauri::App) {
    let (tx, rx) = unbounded::<TranscribeMsg>();

    std::thread::spawn(move || {
        transcriber_worker(rx);
    });

    app.manage(TranscriberState { tx });
}

pub fn preload_model(app: &AppHandle, model_id: String) {
    if let Some(state) = app.try_state::<TranscriberState>() {
        let _ = state.tx.send(TranscribeMsg::Preload {
            app: app.clone(),
            model_id,
        });
    }
}

pub fn unload_model(app: &AppHandle) {
    if let Some(state) = app.try_state::<TranscriberState>() {
        let _ = state.tx.send(TranscribeMsg::UnloadModel);
    }
}

pub fn request_transcription(
    app: &AppHandle,
    samples: Vec<f32>,
    model_id: String,
    is_retranscription: Option<i64>,
) {
    let state = app.state::<TranscriberState>();
    let req = TranscribeRequest {
        samples,
        model_id,
        app: app.clone(),
        is_retranscription,
    };
    let _ = state.tx.send(TranscribeMsg::Request(req));
}

fn load_model_instance(app: &AppHandle, model_id: &str) -> Option<ActiveModel> {
    let model_def = crate::models::MODELS.iter().find(|m| m.id == model_id)?;
    let model_dir = crate::models::get_model_path(model_def.id);

    match model_def.engine {
        "whisper" => {
            let path_str = model_dir
                .join(model_def.files[0].0)
                .to_string_lossy()
                .to_string();
            match WhisperContext::new_with_params(
                &path_str,
                WhisperContextParameters::default(),
            ) {
                Ok(ctx) => Some(ActiveModel::Whisper(ctx)),
                Err(_e) => {
                    let _ = app.emit(
                        "transcription-done",
                        "Error: err_load_failed".to_string(),
                    );
                    None
                }
            }
        }
        "onnx" => match model_def.id {
            "parakeet" => match ParakeetModel::load(&model_dir, &Quantization::Int8) {
                Ok(m) => Some(ActiveModel::Parakeet(m)),
                Err(_e) => {
                    let _ = app.emit(
                        "transcription-done",
                        "Error: err_load_failed".to_string(),
                    );
                    None
                }
            },
            "canary" => match CanaryModel::load(&model_dir, &Quantization::Int8) {
                Ok(m) => Some(ActiveModel::Canary(m)),
                Err(_e) => {
                    let _ = app.emit(
                        "transcription-done",
                        "Error: err_load_failed".to_string(),
                    );
                    None
                }
            },
            "gigaam" => match GigaAMModel::load(&model_dir, &Quantization::Int8) {
                Ok(m) => Some(ActiveModel::GigaAM(m)),
                Err(_e) => {
                    let _ = app.emit(
                        "transcription-done",
                        "Error: err_load_failed".to_string(),
                    );
                    None
                }
            },
            _ => {
                let _ = app.emit(
                    "transcription-done",
                    "Error: err_unknown_engine".to_string(),
                );
                None
            }
        },
        "ggml" => {
            let path_str = model_dir
                .join(model_def.files[0].0)
                .to_string_lossy()
                .to_string();
            match transcribe_cpp::Model::load(&path_str) {
                Ok(m) => Some(ActiveModel::Ggml(m)),
                Err(_e) => {
                    let _ = app.emit(
                        "transcription-done",
                        "Error: err_load_failed".to_string(),
                    );
                    None
                }
            }
        }
        _ => {
            let _ = app.emit(
                "transcription-done",
                "Error: err_unknown_engine".to_string(),
            );
            None
        }
    }
}

fn transcriber_worker(rx: Receiver<TranscribeMsg>) {
    let mut active_model: Option<ActiveModel> = None;
    let mut active_model_id: Option<String> = None;
    let mut last_activity = std::time::Instant::now();
    let mut last_app: Option<AppHandle> = None;

    loop {
        let msg = match rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(m) => m,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                let settings = crate::settings::load_settings();
                if settings.auto_unload_idle_minutes > 0 && active_model.is_some() {
                    let idle_dur = std::time::Duration::from_secs(
                        settings.auto_unload_idle_minutes as u64 * 60,
                    );
                    if last_activity.elapsed() >= idle_dur {
                        let mut is_busy = false;
                        if let Some(app) = &last_app {
                            let state_arc = app.state::<std::sync::Arc<std::sync::Mutex<
                                crate::audio::AudioState,
                            >>>();
                            let state = state_arc.inner().lock().unwrap();
                            is_busy = state.is_recording || state.is_processing;
                        }

                        if is_busy {
                            last_activity = std::time::Instant::now();
                        } else {
                            active_model = None;
                            active_model_id = None;
                            *LOADED_MODEL_ID.lock().unwrap() = None;
                            *DIAGNOSTIC_MODEL_STATE.lock().unwrap() = "Unloaded".to_string();
                            DIAGNOSTIC_HOTKEY_PRESS_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
                            if let Some(app) = &last_app {
                                let _ = app.emit("model-unloaded", ());
                            }
                            println!("[info][transcribe] Model unloaded due to idle timeout");
                        }
                    }
                }
                continue;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
        };

        match msg {
            TranscribeMsg::UnloadModel => {
                let had_model = active_model.is_some();
                active_model = None;
                active_model_id = None;
                *LOADED_MODEL_ID.lock().unwrap() = None;
                if had_model {
                    if let Some(app) = &last_app {
                        let _ = app.emit("model-unloaded", ());
                    }
                }
                println!("[info][transcribe] Model explicitly unloaded");
            }
            TranscribeMsg::Preload { app, model_id } => {
                last_app = Some(app.clone());
                if active_model_id.as_deref() != Some(&model_id) || active_model.is_none() {
                    let _ = app.emit("model-loading", ());
                    let loaded = load_model_instance(&app, &model_id);
                    if let Some(m) = loaded {
                        active_model = Some(m);
                        active_model_id = Some(model_id.clone());
                        *LOADED_MODEL_ID.lock().unwrap() = Some(model_id);
                        let _ = app.emit("model-loaded", ());
                    }
                }
                last_activity = std::time::Instant::now();
            }
            TranscribeMsg::Request(req) => {
                let app = req.app.clone();
                last_app = Some(app.clone());

                if req.is_retranscription.is_none() {
                    let _ = app.emit("processing-started", ());
                }

                if active_model_id.as_deref() != Some(&req.model_id) || active_model.is_none() {
                    let _ = app.emit("model-loading", ());
                    let loaded = load_model_instance(&app, &req.model_id);
                    if let Some(m) = loaded {
                        active_model = Some(m);
                        active_model_id = Some(req.model_id.clone());
                        *LOADED_MODEL_ID.lock().unwrap() = Some(req.model_id.clone());
                        let _ = app.emit("model-loaded", ());
                    } else {
                        continue;
                    }
                }

                let mut result_text = String::new();

                if let Some(am) = active_model.as_mut() {
                    let settings = crate::settings::load_settings();
                    let model_settings = settings
                        .model_settings
                        .get(&req.model_id)
                        .cloned()
                        .unwrap_or_default();

                    match am {
                        ActiveModel::Whisper(ctx) => {
                            let mut state = ctx.create_state().expect("failed to create state");

                            let mut params =
                                FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
                            params.set_language(Some(&model_settings.language));
                            params.set_print_special(false);
                            params.set_print_progress(false);
                            params.set_print_realtime(false);
                            params.set_print_timestamps(false);

                            if let Some(prompt) = &model_settings.initial_prompt {
                                if !prompt.trim().is_empty() {
                                    params.set_initial_prompt(prompt);
                                }
                            }

                            if let Err(_e) = state.full(params, &req.samples) {
                                let _ = app.emit(
                                    "transcription-done",
                                    str::to_string("Error: err_transcription_failed"),
                                );
                                continue;
                            }

                            let num_segments = state.full_n_segments();
                            for i in 0..num_segments {
                                if let Some(segment) = state.get_segment(i) {
                                    if let Ok(text) = segment.to_str() {
                                        result_text.push_str(text);
                                    }
                                }
                            }
                        }
                        ActiveModel::Parakeet(model) => {
                            let lang = if model_settings.language == "auto" {
                                None
                            } else {
                                Some(model_settings.language.clone())
                            };
                            match model.transcribe_with(
                                &req.samples,
                                &ParakeetParams {
                                    language: lang,
                                    timestamp_granularity: Some(TimestampGranularity::Segment),
                                    ..Default::default()
                                },
                            ) {
                                Ok(res) => result_text = res.text,
                                Err(_e) => {
                                    let _ = app.emit(
                                        "transcription-done",
                                        str::to_string("Error: err_transcription_failed"),
                                    );
                                    continue;
                                }
                            }
                        }
                        ActiveModel::Canary(model) => {
                            let lang = if model_settings.language == "auto" {
                                "en".to_string()
                            } else {
                                model_settings.language.clone()
                            };

                            match model.transcribe_with(
                                &req.samples,
                                &CanaryParams {
                                    language: Some(lang),
                                    use_pnc: true,
                                    use_itn: true,
                                    ..Default::default()
                                },
                            ) {
                                Ok(res) => result_text = res.text,
                                Err(_e) => {
                                    let _ = app.emit(
                                        "transcription-done",
                                        str::to_string("Error: err_transcription_failed"),
                                    );
                                    continue;
                                }
                            }
                        }
                        ActiveModel::GigaAM(model) => {
                            match model.transcribe_with(
                                &req.samples,
                                &GigaAMParams {
                                    language: Some("ru".to_string()),
                                    ..Default::default()
                                },
                            ) {
                                Ok(res) => result_text = res.text,
                                Err(_e) => {
                                    let _ = app.emit(
                                        "transcription-done",
                                        str::to_string("Error: err_transcription_failed"),
                                    );
                                    continue;
                                }
                            }
                        }
                        ActiveModel::Ggml(model) => {
                            let mut session = match model.session() {
                                Ok(s) => s,
                                Err(_e) => {
                                    let _ = app.emit(
                                        "transcription-done",
                                        str::to_string("Error: err_transcription_failed"),
                                    );
                                    continue;
                                }
                            };

                            let mut options = transcribe_cpp::RunOptions::default();
                            if model_settings.language != "auto"
                                && !model_settings.language.is_empty()
                            {
                                options.language = Some(model_settings.language.clone());
                            }

                            match session.run(&req.samples, &options) {
                                Ok(res) => result_text = res.text,
                                Err(_e) => {
                                    let _ = app.emit(
                                        "transcription-done",
                                        str::to_string("Error: err_transcription_failed"),
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }

                let text = result_text.trim().to_string();

                if let Some(history_id) = req.is_retranscription {
                    if text.is_empty() {
                        let _ = app.emit(
                            "retranscription-error",
                            (history_id, "Error: err_speech_not_recognized".to_string()),
                        );
                    } else {
                        let _ = app.emit("retranscription-done", (history_id, text));
                    }
                    last_activity = std::time::Instant::now();
                    continue;
                }

                *DIAGNOSTIC_LAST_ACTIVITY.lock().unwrap() = std::time::Instant::now();
                
                let is_processing = {
                    let state_arc = app.state::<std::sync::Arc<std::sync::Mutex<
                        crate::audio::AudioState,
                    >>>();
                    let mut state = state_arc.inner().lock().unwrap();
                    let was_processing = state.is_processing;
                    state.is_processing = false;
                    was_processing
                };

                if active_model.is_none() {
                    *DIAGNOSTIC_MODEL_STATE.lock().unwrap() = "Loading".to_string();
                    let loaded = load_model_instance(&app, &req.model_id);
                    if let Some(m) = loaded {
                        active_model = Some(m);
                        active_model_id = Some(req.model_id.clone());
                        *LOADED_MODEL_ID.lock().unwrap() = Some(req.model_id.clone());
                        let _ = app.emit("model-loaded", ());
                        *DIAGNOSTIC_MODEL_STATE.lock().unwrap() = "Loaded".to_string();
                    } else {
                        *DIAGNOSTIC_MODEL_STATE.lock().unwrap() = "Unloaded (Load Failed)".to_string();
                        continue;
                    }
                }

                if is_processing {
                    if text.is_empty() {
                        let _ = app.emit(
                            "transcription-done",
                            "Error: err_speech_not_recognized".to_string(),
                        );
                        last_activity = std::time::Instant::now();
                        continue;
                    }

                    let mut should_paste = true;
                    if let Some(main_win) = app.get_webview_window("main") {
                        if main_win.is_focused().unwrap_or(false) {
                            should_paste = false;
                        }
                    }

                    let mut is_copy = false;
                    if should_paste {
                        let target_hwnd = {
                            let state_arc = app.state::<std::sync::Arc<std::sync::Mutex<
                                crate::audio::AudioState,
                            >>>();
                            let state = state_arc.inner().lock().unwrap();
                            state.target_hwnd
                        };
                        is_copy = crate::paste::paste_text(&text, target_hwnd);
                    }

                    let (target_app_name, target_app_icon) = {
                        let state_arc = app.state::<std::sync::Arc<std::sync::Mutex<
                            crate::audio::AudioState,
                        >>>();
                        let state = state_arc.inner().lock().unwrap();
                        if let Some(app_info) = &state.app_info {
                            (app_info.title.clone(), app_info.icon_base64.clone())
                        } else {
                            ("Unknown".to_string(), "".to_string())
                        }
                    };

                    crate::history::add_record(
                        text.clone(),
                        req.model_id.clone(),
                        target_app_name,
                        target_app_icon,
                        req.samples,
                    );

                    let is_recording_now = {
                        let state_arc = app.state::<std::sync::Arc<std::sync::Mutex<
                            crate::audio::AudioState,
                        >>>();
                        let state = state_arc.inner().lock().unwrap();
                        state.is_recording
                    };

                    if !is_recording_now {
                        if is_copy {
                            let _ = app.emit("transcription-done", format!("COPIED:{}", text));
                        } else {
                            let _ = app.emit("transcription-done", text);
                        }
                    }
                }
                last_activity = std::time::Instant::now();
            }
        }
    }
}
