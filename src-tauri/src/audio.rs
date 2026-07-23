use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

pub struct AudioState {
    pub is_recording: bool,
    pub is_processing: bool,
    pub is_previewing: bool,
    pub target_hwnd: Option<isize>,
    pub caret_pos: Option<crate::caret_position::CaretRect>,
    pub caret_method: crate::caret_position::CaretMethod,
    pub caret_trace: String,
    pub app_info: Option<crate::paste::AppInfo>,
    stop_tx: Option<Sender<()>>,
}

impl Default for AudioState {
    fn default() -> Self {
        Self {
            is_recording: false,
            is_processing: false,
            is_previewing: false,
            target_hwnd: None,
            caret_pos: None,
            caret_method: crate::caret_position::CaretMethod::Fallback,
            caret_trace: String::new(),
            app_info: None,
            stop_tx: None,
        }
    }
}

pub fn start_recording(app: AppHandle) -> Result<(), String> {
    let state_arc = app.state::<Arc<Mutex<AudioState>>>();
    let mut state = state_arc.inner().lock().unwrap();
    if state.is_recording {
        return Ok(());
    }

    let mut had_old_worker = false;
    if let Some(old_tx) = state.stop_tx.take() {
        let _ = old_tx.send(());
        had_old_worker = true;
    }
    state.is_previewing = false;
    state.is_processing = false;

    let (stop_tx, stop_rx) = unbounded();
    state.is_recording = true;
    state.stop_tx = Some(stop_tx);
    drop(state);

    if had_old_worker {
        std::thread::sleep(std::time::Duration::from_millis(80));
    }

    let app_clone = app.clone();
    spawn_audio_thread(app_clone, stop_rx, false);

    let settings = crate::settings::load_settings();
    if let Some(model_id) = &settings.active_model {
        if !crate::transcribe::is_model_loaded(model_id) {
            let _ = app.emit("model-loading", ());
            crate::transcribe::preload_model(&app, model_id.clone());
        }
    }

    Ok(())
}

pub fn play_cue(start: bool) {
    std::thread::spawn(move || {
        if let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() {
            if let Ok(sink) = rodio::Sink::try_new(&stream_handle) {
                let sample_rate = 44100u32;

                // Премиальный звук: каждая нота — это 3 слегка расстроенных слоя (chorus),
                // создающих эффект глубины и "дорогого" звучания
                let notes: [(f32, u32); 2] = if start {
                    [(523.25, 100), (659.25, 130)] // C5 -> E5
                } else {
                    [(659.25, 100), (523.25, 130)] // E5 -> C5
                };

                let mut samples = Vec::new();

                for (base_freq, dur_ms) in notes {
                    let num_samples = (sample_rate * dur_ms) / 1000;
                    let freqs = [base_freq, base_freq + 1.5, base_freq - 1.5];
                    let mut phases = [0.0f32; 3];

                    for i in 0..num_samples {
                        let t = i as f32 / num_samples as f32;

                        let mut sample = 0.0f32;
                        for (k, &freq) in freqs.iter().enumerate() {
                            phases[k] += freq * 2.0 * std::f32::consts::PI / sample_rate as f32;
                            let weight = if k == 0 { 0.5 } else { 0.25 };
                            sample += weight * phases[k].sin();
                        }

                        let attack_time = 0.020;
                        let attack_samples = (sample_rate as f32 * attack_time) as u32;
                        let mut env = (-2.2 * t).exp() * (1.0 - t * 0.7);
                        if i < attack_samples {
                            env *= i as f32 / attack_samples as f32;
                        }

                        // Уровень громкости 0.1, чтобы звук был мягким и ненавязчивым
                        samples.push(sample * env * 0.1);
                    }
                }

                let source = rodio::buffer::SamplesBuffer::new(1, sample_rate, samples);
                sink.append(source);
                sink.sleep_until_end();
            }
        }
    });
}

enum WorkerResult {
    Completed,
    StalledNoSamples,
}

struct DuckingGuard(bool);
impl Drop for DuckingGuard {
    fn drop(&mut self) {
        if self.0 {
            crate::ducking::restore_audio();
        }
    }
}

fn run_audio_session(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    stop_rx: Receiver<()>,
    app_clone: AppHandle,
    is_preview: bool,
) -> Result<WorkerResult, String> {
    let channels = if config.channels() == 0 {
        eprintln!("[warn][audio] config.channels() returned 0! Defaulting to 1 channel.");
        1
    } else {
        config.channels()
    };
    let sample_rate = config.sample_rate().0;
    let dev_name = device
        .name()
        .unwrap_or_else(|_| "Unknown Device".to_string());
    eprintln!("[info][audio] Creating cpal input stream: device='{}', channels={}, sample_rate={}, format={:?}", dev_name, channels, sample_rate, config.sample_format());

    let (tx, rx): (Sender<f32>, Receiver<f32>) = unbounded();

    let err_fn = move |err| {
        eprintln!("[error][audio] cpal input stream error: {}", err);
    };

    let stream_result = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.config(),
            move |data: &[f32], _: &_| {
                for &sample in data {
                    let _ = tx.send(sample);
                }
            },
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.config(),
            move |data: &[i16], _: &_| {
                for &sample in data {
                    let f_sample = (sample as f32) / (i16::MAX as f32);
                    let _ = tx.send(f_sample);
                }
            },
            err_fn,
            None,
        ),
        _ => {
            eprintln!(
                "[error][audio] Unsupported sample format: {:?}",
                config.sample_format()
            );
            return Err("Unsupported sample format".to_string());
        }
    };

    let stream = match stream_result {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[error][audio] Failed to build stream: {}", e);
            return Err(format!("Failed to build stream: {}", e));
        }
    };

    if let Err(e) = stream.play() {
        eprintln!("[error][audio] Failed to call play() on stream: {}", e);
        return Err(format!("Failed to play stream: {}", e));
    }
    eprintln!("[info][audio] Stream play() succeeded. Entering worker loop...");

    let res = worker_process(rx, stop_rx, channels, sample_rate, app_clone, is_preview);
    eprintln!("[info][audio] Worker loop finished. Dropping cpal stream.");
    Ok(res)
}

fn spawn_audio_thread(app_clone: AppHandle, stop_rx: Receiver<()>, is_preview: bool) {
    std::thread::spawn(move || {
        let settings = crate::settings::load_settings();
        let host = cpal::default_host();

        let device = if let Some(mic_name) = settings.microphone.as_ref() {
            if let Ok(mut devices) = host.input_devices() {
                devices
                    .find(|d| d.name().unwrap_or_default() == *mic_name)
                    .or_else(|| host.default_input_device())
            } else {
                host.default_input_device()
            }
        } else {
            host.default_input_device()
        };

        let device = match device {
            Some(d) => d,
            None => {
                eprintln!("[error][audio] No input device available");
                let _ = app_clone.emit("show-error", "err_mic_not_found".to_string());
                return;
            }
        };
        let config = match device.default_input_config() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[error][audio] Error getting config: {}", e);
                return;
            }
        };

        match run_audio_session(
            &device,
            &config,
            stop_rx.clone(),
            app_clone.clone(),
            is_preview,
        ) {
            Ok(WorkerResult::StalledNoSamples) => {
                eprintln!("[warn][audio] Audio watchdog: stream stalled/no samples. Recreating cpal input stream (attempt 2 of 2)...");
                std::thread::sleep(std::time::Duration::from_millis(150));

                let device_retry = if let Some(mic_name) = settings.microphone.as_ref() {
                    if let Ok(mut devices) = host.input_devices() {
                        devices
                            .find(|d| d.name().unwrap_or_default() == *mic_name)
                            .or_else(|| host.default_input_device())
                    } else {
                        host.default_input_device()
                    }
                } else {
                    host.default_input_device()
                }
                .unwrap_or(device);

                match run_audio_session(
                    &device_retry,
                    &config,
                    stop_rx,
                    app_clone.clone(),
                    is_preview,
                ) {
                    Ok(WorkerResult::StalledNoSamples) | Err(_) => {
                        eprintln!("[error][audio] Watchdog recovery failed: stream still silent after restart.");
                        let _ = app_clone.emit("show-error", "err_mic_not_found".to_string());
                        let state_arc = app_clone.state::<Arc<Mutex<AudioState>>>();
                        let mut state = state_arc.inner().lock().unwrap();
                        state.is_recording = false;
                        state.is_processing = false;
                        if let Some(window) = app_clone.get_webview_window("overlay") {
                            crate::log_debug("[OVERLAY_EVENT] Window HIDE (reason: watchdog timeout worker aborted)");
                            let _ = window.hide();
                        }
                    }
                    Ok(WorkerResult::Completed) => {
                        eprintln!("[info][audio] Watchdog recovery successful!");
                    }
                }
            }
            Err(e) => {
                eprintln!("[error][audio] Failed to start audio session: {}", e);
                let _ = app_clone.emit("show-error", "err_mic_not_found".to_string());
                let state_arc = app_clone.state::<Arc<Mutex<AudioState>>>();
                let mut state = state_arc.inner().lock().unwrap();
                state.is_recording = false;
                state.is_processing = false;
                state.is_previewing = false;
                let _ = state.stop_tx.take();
                if let Some(window) = app_clone.get_webview_window("overlay") {
                    crate::log_debug(
                        "[OVERLAY_EVENT] Window HIDE (reason: audio session start failed)",
                    );
                    let _ = window.hide();
                }
            }
            Ok(WorkerResult::Completed) => {}
        }
    });
}

#[tauri::command]
pub fn start_preview(app: AppHandle) -> Result<(), String> {
    let state_arc = app.state::<Arc<Mutex<AudioState>>>();
    let mut state = state_arc.inner().lock().unwrap();
    if state.is_previewing || state.is_recording {
        return Ok(());
    }

    let mut had_old_worker = false;
    if let Some(old_tx) = state.stop_tx.take() {
        let _ = old_tx.send(());
        had_old_worker = true;
    }
    state.is_processing = false;

    let (stop_tx, stop_rx) = unbounded();
    state.is_previewing = true;
    state.stop_tx = Some(stop_tx);
    drop(state);

    if had_old_worker {
        std::thread::sleep(std::time::Duration::from_millis(80));
    }

    let app_clone = app.clone();
    spawn_audio_thread(app_clone, stop_rx, true);

    Ok(())
}

#[tauri::command]
pub fn stop_preview(app: AppHandle) -> Result<(), String> {
    let state_arc = app.state::<Arc<Mutex<AudioState>>>();
    let mut state = state_arc.inner().lock().unwrap();
    if !state.is_previewing {
        return Ok(());
    }

    state.is_previewing = false;
    if let Some(tx) = state.stop_tx.take() {
        let _ = tx.send(());
    }

    Ok(())
}

pub fn stop_recording(app: AppHandle) -> Result<(), String> {
    let state_arc = app.state::<Arc<Mutex<AudioState>>>();
    let mut state = state_arc.inner().lock().unwrap();
    if !state.is_recording {
        return Ok(());
    }

    state.is_recording = false;
    state.is_processing = true;
    if let Some(tx) = state.stop_tx.take() {
        let _ = tx.send(());
    }

    let settings = crate::settings::load_settings();
    if settings.duck_audio {
        crate::ducking::restore_audio();
    }
    if settings.sound_cues {
        play_cue(false);
    }

    let _ = app.emit("recording-stopped", ());
    Ok(())
}

pub fn cancel_action(app: AppHandle) {
    let state_arc = app.state::<Arc<Mutex<AudioState>>>();
    let mut state = state_arc.inner().lock().unwrap();

    state.is_recording = false;
    state.is_processing = false;

    if let Some(tx) = state.stop_tx.take() {
        let _ = tx.send(());
    }

    let settings = crate::settings::load_settings();
    if settings.duck_audio {
        crate::ducking::restore_audio();
    }
    // We don't play a cue on cancel, or maybe we do? Let's not play it to differentiate.

    let _ = app.emit("recording-cancelled", ());
    if let Some(window) = app.get_webview_window("overlay") {
        crate::log_debug("[OVERLAY_EVENT] Window HIDE (reason: cancel_action)");
        let _ = window.hide();
    }
}

pub fn cancel_action_silently(app: AppHandle) {
    let state_arc = app.state::<Arc<Mutex<AudioState>>>();
    let mut state = state_arc.inner().lock().unwrap();

    state.is_recording = false;
    state.is_processing = false;

    if let Some(tx) = state.stop_tx.take() {
        let _ = tx.send(());
    }

    let settings = crate::settings::load_settings();
    if settings.duck_audio {
        crate::ducking::restore_audio();
    }

    // We emit a silent cancel so the frontend can reset its UI state without showing an error
    let _ = app.emit("recording-cancelled-silently", ());
    if let Some(window) = app.get_webview_window("overlay") {
        crate::log_debug("[OVERLAY_EVENT] Window HIDE (reason: cancel_action_silently)");
        let _ = window.hide();
    }
}

// Simple nearest-neighbor manual resampler
struct NearestResampler {
    ratio: f32,
    counter: f32,
    input_count: usize,
    output_count: usize,
}

impl NearestResampler {
    fn new(in_sr: u32, out_sr: u32) -> Self {
        let safe_in_sr = if in_sr == 0 { 44100 } else { in_sr };
        let safe_out_sr = if out_sr == 0 { 16000 } else { out_sr };
        let ratio = safe_in_sr as f32 / safe_out_sr as f32;
        eprintln!(
            "[info][audio] Initializing NearestResampler: in={}Hz, out={}Hz, ratio={:.4}",
            safe_in_sr, safe_out_sr, ratio
        );
        Self {
            ratio,
            counter: 0.0,
            input_count: 0,
            output_count: 0,
        }
    }

    fn process(&mut self, sample: f32) -> Option<f32> {
        self.input_count += 1;
        if self.ratio.is_nan() || self.ratio.is_infinite() || self.ratio <= 0.0 {
            eprintln!(
                "[error][audio] Resampler error: invalid ratio ({})",
                self.ratio
            );
            return None;
        }
        if self.counter.is_nan() || self.counter.is_infinite() {
            eprintln!(
                "[error][audio] Resampler error: invalid counter state ({}), resetting to 0",
                self.counter
            );
            self.counter = 0.0;
        }

        self.counter += 1.0;
        if self.counter >= self.ratio {
            self.counter -= self.ratio;
            self.output_count += 1;
            Some(sample)
        } else {
            None
        }
    }
}

fn worker_process(
    rx: Receiver<f32>,
    stop_rx: Receiver<()>,
    mut channels: u16,
    in_sample_rate: u32,
    app: AppHandle,
    is_preview: bool,
) -> WorkerResult {
    if channels == 0 {
        channels = 1;
    }
    let settings = crate::settings::load_settings();
    let should_duck = !is_preview && settings.duck_audio;
    let _ducking_guard = DuckingGuard(should_duck);
    let mut ducked_already = false;
    let mut total_samples_received: usize = 0;
    let start_time = std::time::Instant::now();
    let mut last_sample_time = std::time::Instant::now();
    let mut last_log_time = std::time::Instant::now();

    let gain = settings.microphone_gain;
    let target_sample_rate = 16000;
    let mut resampler = NearestResampler::new(in_sample_rate, target_sample_rate);
    eprintln!("[info][audio] Resampler initialized for worker: in_sr={}Hz ({}ch) -> out_sr={}Hz (mono 1ch)", in_sample_rate, channels, target_sample_rate);

    let mut accumulated_samples = Vec::new();
    let max_samples = 16000 * 120; // 120 seconds

    let mut rms_sum = 0.0;
    let mut rms_count = 0;
    let rms_report_interval = in_sample_rate as usize / 30; // 30 times a second
    let mut current_frame_samples = Vec::with_capacity(channels as usize);

    loop {
        match stop_rx.try_recv() {
            Ok(()) | Err(crossbeam_channel::TryRecvError::Disconnected) => {
                eprintln!("[info][audio] Stop/Disconnect signal received via stop_rx. Worker exiting loop.");
                break;
            }
            Err(crossbeam_channel::TryRecvError::Empty) => {}
        }

        let sample = match rx.recv_timeout(std::time::Duration::from_millis(10)) {
            Ok(s) => {
                if total_samples_received == 0 {
                    if !is_preview {
                        let _ = app.emit("recording-started", ());
                        crate::show_overlay(app.clone());
                        let settings = crate::settings::load_settings();
                        if settings.sound_cues {
                            play_cue(true);
                        }
                    }
                }
                total_samples_received += 1;
                last_sample_time = std::time::Instant::now();

                if !ducked_already
                    && should_duck
                    && total_samples_received >= channels as usize * 20
                {
                    ducked_already = true;
                    eprintln!("[info][audio] Audio stream confirmed flowing (received {} raw samples). Triggering duck_audio()...", total_samples_received);
                    crate::ducking::duck_audio();
                }

                if last_log_time.elapsed() >= std::time::Duration::from_secs(1) {
                    if !is_preview {
                        eprintln!("[info][audio] Recording active: {} raw samples received, {} resampled accumulated (resampler in: {}, out: {}).", total_samples_received, accumulated_samples.len(), resampler.input_count, resampler.output_count);
                    }
                    last_log_time = std::time::Instant::now();
                }

                s
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                if total_samples_received == 0
                    && start_time.elapsed() >= std::time::Duration::from_millis(500)
                {
                    eprintln!("[warn][audio] Watchdog triggered: 0 samples received within first 500ms after play()!");
                    return WorkerResult::StalledNoSamples;
                }
                if total_samples_received > 0
                    && last_sample_time.elapsed() >= std::time::Duration::from_millis(500)
                {
                    eprintln!("[warn][audio] Watchdog triggered: stream gap > 500ms during active recording!");
                    return WorkerResult::StalledNoSamples;
                }
                continue;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                eprintln!("[info][audio] rx channel disconnected.");
                break;
            }
        };

        current_frame_samples.push(sample);

        if current_frame_samples.len() == channels as usize {
            let sum: f32 = current_frame_samples.iter().sum();
            let mut mono_sample = sum / channels as f32;
            mono_sample *= gain;
            mono_sample = mono_sample.clamp(-1.0, 1.0);

            current_frame_samples.clear();

            rms_sum += mono_sample * mono_sample;
            rms_count += 1;

            if rms_count >= rms_report_interval {
                let rms = (rms_sum / rms_count as f32).sqrt();
                let _ = app.emit("audio-level", rms);
                rms_sum = 0.0;
                rms_count = 0;
            }

            if is_preview {
                continue;
            }

            if let Some(resampled) = resampler.process(mono_sample) {
                accumulated_samples.push(resampled);
            }

            if accumulated_samples.len() >= max_samples {
                let _ = app.emit("recording-stopped", ());
                // Do not hide the window, because transcription is starting.
                let state_arc = app.state::<Arc<Mutex<AudioState>>>();
                let mut state = state_arc.inner().lock().unwrap();
                state.is_recording = false;
                state.is_processing = true;
                let _ = state.stop_tx.take();
                break;
            }
        }
    }

    eprintln!("[info][audio] Worker loop exited: received {} raw samples, accumulated {} resampled mono samples (resampler in: {}, out: {}).", total_samples_received, accumulated_samples.len(), resampler.input_count, resampler.output_count);

    if !is_preview {
        let state_arc = app.state::<Arc<Mutex<AudioState>>>();
        let mut state = state_arc.inner().lock().unwrap();
        if state.is_recording {
            state.is_recording = false;
            let _ = state.stop_tx.take();
        }
    } else {
        let state_arc = app.state::<Arc<Mutex<AudioState>>>();
        let mut state = state_arc.inner().lock().unwrap();
        if state.is_previewing {
            state.is_previewing = false;
            let _ = state.stop_tx.take();
        }
    }

    if is_preview {
        return WorkerResult::Completed;
    }

    // Call transcription
    if accumulated_samples.len() < 4000 {
        // < 0.25s at 16kHz
        println!(
            "Rejected: accumulated_samples.len() < 4000 ({})",
            accumulated_samples.len()
        );
        let _ = app.emit(
            "transcription-done",
            "Error: err_speech_not_recognized".to_string(),
        );
        let state_arc = app.state::<Arc<Mutex<AudioState>>>();
        let mut state = state_arc.inner().lock().unwrap();
        state.is_processing = false;
        return WorkerResult::Completed;
    }

    let mut vad = match crate::vad::VadSession::new() {
        Ok(v) => v,
        Err(_e) => {
            let _ = app.emit("show-error", "err_vad_failed".to_string());
            let state_arc = app.state::<Arc<Mutex<AudioState>>>();
            let mut state = state_arc.inner().lock().unwrap();
            state.is_processing = false;
            return WorkerResult::Completed;
        }
    };

    let trimmed_samples = match vad.trim_silence(&accumulated_samples, target_sample_rate) {
        Ok(Some(s)) => s,
        Ok(None) => {
            let _ = app.emit(
                "transcription-done",
                "Error: err_speech_not_recognized".to_string(),
            );
            let state_arc = app.state::<Arc<Mutex<AudioState>>>();
            let mut state = state_arc.inner().lock().unwrap();
            state.is_processing = false;
            return WorkerResult::Completed;
        }
        Err(e) => {
            eprintln!("VAD error: {}", e);
            accumulated_samples
        }
    };

    if trimmed_samples.len() < 4000 {
        println!(
            "Rejected: trimmed_samples.len() < 4000 ({})",
            trimmed_samples.len()
        );
        let _ = app.emit(
            "transcription-done",
            "Error: err_speech_not_recognized".to_string(),
        );
        let state_arc = app.state::<Arc<Mutex<AudioState>>>();
        let mut state = state_arc.inner().lock().unwrap();
        state.is_processing = false;
        return WorkerResult::Completed;
    }

    println!(
        "Accepting audio for transcription, trimmed length: {}",
        trimmed_samples.len()
    );

    let active_model = crate::settings::load_settings().active_model;
    if let Some(id) = active_model {
        crate::transcribe::request_transcription(&app, trimmed_samples, id, None);
    } else {
        let _ = app.emit(
            "transcription-done",
            "Error: err_no_model_selected".to_string(),
        );
        let state_arc = app.state::<Arc<Mutex<AudioState>>>();
        let mut state = state_arc.inner().lock().unwrap();
        state.is_processing = false;
    }
    WorkerResult::Completed
}

#[tauri::command]
pub fn get_microphones() -> Vec<String> {
    let host = cpal::default_host();
    let mut mics = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for device in devices {
            if let Ok(name) = device.name() {
                mics.push(name);
            }
        }
    }
    mics
}
