use crossbeam_channel::{unbounded, Receiver, Sender};
use tauri::{AppHandle, Emitter, Manager};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use transcribe_rs::onnx::canary::{CanaryModel, CanaryParams};
use transcribe_rs::onnx::gigaam::{GigaAMModel, GigaAMParams};
use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity};
use transcribe_rs::onnx::Quantization;

enum ActiveModel {
    Whisper(WhisperContext),
    Parakeet(ParakeetModel),
    Canary(CanaryModel),
    GigaAM(GigaAMModel),
    Ggml(transcribe_cpp::Model),
}

pub struct TranscriberState {
    tx: Sender<TranscribeRequest>,
}

pub struct TranscribeRequest {
    pub samples: Vec<f32>,
    pub model_id: String,
    pub app: AppHandle,
    pub is_retranscription: Option<i64>,
}

pub fn init_transcriber(app: &tauri::App) {
    let (tx, rx) = unbounded::<TranscribeRequest>();

    std::thread::spawn(move || {
        transcriber_worker(rx);
    });

    app.manage(TranscriberState { tx });
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
    let _ = state.tx.send(req);
}

fn transcriber_worker(rx: Receiver<TranscribeRequest>) {
    let mut active_model: Option<ActiveModel> = None;
    let mut active_model_id: Option<String> = None;

    loop {
        let req = match rx.recv() {
            Ok(r) => r,
            Err(_) => break,
        };

        let app = req.app;

        if req.is_retranscription.is_none() {
            let _ = app.emit("processing-started", ());
        }

        if active_model_id.as_deref() != Some(&req.model_id) || active_model.is_none() {
            let model_def = crate::models::MODELS.iter().find(|m| m.id == req.model_id);
            if model_def.is_none() {
                let _ = app.emit(
                    "transcription-done",
                    "Error: err_model_not_found".to_string(),
                );
                continue;
            }
            let model_def = model_def.unwrap();
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
                        Ok(ctx) => {
                            active_model = Some(ActiveModel::Whisper(ctx));
                            active_model_id = Some(req.model_id.clone());
                        }
                        Err(_e) => {
                            let _ = app.emit(
                                "transcription-done",
                                str::to_string("Error: err_load_failed"),
                            );
                            continue;
                        }
                    }
                }
                "onnx" => match model_def.id {
                    "parakeet" => match ParakeetModel::load(&model_dir, &Quantization::Int8) {
                        Ok(m) => {
                            active_model = Some(ActiveModel::Parakeet(m));
                            active_model_id = Some(req.model_id.clone());
                        }
                        Err(_e) => {
                            let _ = app.emit(
                                "transcription-done",
                                str::to_string("Error: err_load_failed"),
                            );
                            continue;
                        }
                    },
                    "canary" => match CanaryModel::load(&model_dir, &Quantization::Int8) {
                        Ok(m) => {
                            active_model = Some(ActiveModel::Canary(m));
                            active_model_id = Some(req.model_id.clone());
                        }
                        Err(_e) => {
                            let _ = app.emit(
                                "transcription-done",
                                str::to_string("Error: err_load_failed"),
                            );
                            continue;
                        }
                    },
                    "gigaam" => match GigaAMModel::load(&model_dir, &Quantization::Int8) {
                        Ok(m) => {
                            active_model = Some(ActiveModel::GigaAM(m));
                            active_model_id = Some(req.model_id.clone());
                        }
                        Err(_e) => {
                            let _ = app.emit(
                                "transcription-done",
                                str::to_string("Error: err_load_failed"),
                            );
                            continue;
                        }
                    },
                    _ => {
                        let _ = app.emit(
                            "transcription-done",
                            "Error: err_unknown_engine".to_string(),
                        );
                        continue;
                    }
                },
                "ggml" => {
                    let path_str = model_dir
                        .join(model_def.files[0].0)
                        .to_string_lossy()
                        .to_string();
                    match transcribe_cpp::Model::load(&path_str) {
                        Ok(m) => {
                            active_model = Some(ActiveModel::Ggml(m));
                            active_model_id = Some(req.model_id.clone());
                        }
                        Err(_e) => {
                            let _ = app.emit(
                                "transcription-done",
                                str::to_string("Error: err_load_failed"),
                            );
                            continue;
                        }
                    }
                }
                _ => {
                    let _ = app.emit(
                        "transcription-done",
                        "Error: err_unknown_engine".to_string(),
                    );
                    continue;
                }
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

                    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
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
                    if model_settings.language != "auto" && !model_settings.language.is_empty() {
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
            continue;
        }

        // Check cancellation
        let is_processing = {
            let state_arc =
                app.state::<std::sync::Arc<std::sync::Mutex<crate::audio::AudioState>>>();
            let mut state = state_arc.inner().lock().unwrap();
            let was_processing = state.is_processing;
            state.is_processing = false;
            was_processing
        };

        if is_processing {
            if text.is_empty() {
                let _ = app.emit(
                    "transcription-done",
                    "Error: err_speech_not_recognized".to_string(),
                );
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
                    let state_arc =
                        app.state::<std::sync::Arc<std::sync::Mutex<crate::audio::AudioState>>>();
                    let state = state_arc.inner().lock().unwrap();
                    state.target_hwnd
                };
                is_copy = crate::paste::paste_text(&text, target_hwnd);
            }

            // Save to history
            let (target_app_name, target_app_icon) = {
                let state_arc =
                    app.state::<std::sync::Arc<std::sync::Mutex<crate::audio::AudioState>>>();
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

            if is_copy {
                let _ = app.emit("transcription-done", format!("COPIED:{}", text));
            } else {
                let _ = app.emit("transcription-done", text);
            }
        }
    }
}
