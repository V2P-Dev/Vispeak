use futures_util::StreamExt;
use reqwest::Client;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::settings::{get_app_data_dir, load_settings};

#[derive(Serialize, Clone)]
#[serde(tag = "type", content = "langs")]
pub enum TranslationSupport {
    None,
    ToEnglishOnly,
    Pairs(&'static [&'static str]),
}

#[derive(Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub size_mb: u32,
    pub is_downloaded: bool,
    pub is_active: bool,
    pub accuracy_rating: u8,
    pub speed_rating: u8,
    pub languages: u32,
    pub quantization: String,
    pub engine: String,
    pub translation: TranslationSupport,
}

pub struct ModelDef {
    pub id: &'static str,
    pub name: &'static str,
    pub size_mb: u32,
    pub accuracy_rating: u8,
    pub speed_rating: u8,
    pub languages: u32,
    pub quantization: &'static str,
    pub engine: &'static str,
    pub translation: TranslationSupport,
    pub files: &'static [(&'static str, &'static str)],
}

pub const MODELS: &[ModelDef] = &[
    ModelDef {
        id: "tiny",
        name: "Whisper Tiny",
        size_mb: 75,
        accuracy_rating: 1,
        speed_rating: 5,
        languages: 99,
        quantization: "FP16",
        engine: "whisper",
        translation: TranslationSupport::ToEnglishOnly,
        files: &[
            ("ggml-tiny.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin")
        ],
    },
    ModelDef {
        id: "base",
        name: "Whisper Base",
        size_mb: 142,
        accuracy_rating: 2,
        speed_rating: 4,
        languages: 99,
        quantization: "FP16",
        engine: "whisper",
        translation: TranslationSupport::ToEnglishOnly,
        files: &[
            ("ggml-base.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin")
        ],
    },
    ModelDef {
        id: "small",
        name: "Whisper Small",
        size_mb: 466,
        accuracy_rating: 3,
        speed_rating: 3,
        languages: 99,
        quantization: "FP16",
        engine: "whisper",
        translation: TranslationSupport::ToEnglishOnly,
        files: &[
            ("ggml-small.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin")
        ],
    },
    ModelDef {
        id: "medium",
        name: "Whisper Medium",
        size_mb: 540,
        accuracy_rating: 4,
        speed_rating: 2,
        languages: 99,
        quantization: "Q5_0",
        engine: "whisper",
        translation: TranslationSupport::ToEnglishOnly,
        files: &[
            ("ggml-medium-q5_0.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium-q5_0.bin")
        ],
    },
    ModelDef {
        id: "large-v3-turbo",
        name: "Whisper Large V3 Turbo",
        size_mb: 574,
        accuracy_rating: 5,
        speed_rating: 3,
        languages: 99,
        quantization: "Q5_0",
        engine: "whisper",
        translation: TranslationSupport::ToEnglishOnly,
        files: &[
            ("ggml-large-v3-turbo-q5_0.bin", "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin")
        ],
    },
    ModelDef {
        id: "parakeet",
        name: "Parakeet TDT 0.6B v3",
        size_mb: 640,
        accuracy_rating: 4,
        speed_rating: 4,
        languages: 99,
        quantization: "INT8",
        engine: "onnx",
        translation: TranslationSupport::None,
        files: &[
            ("encoder-model.int8.onnx", "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/encoder-model.int8.onnx"),
            ("decoder_joint-model.int8.onnx", "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/decoder_joint-model.int8.onnx"),
            ("nemo128.onnx", "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/nemo128.onnx"),
            ("vocab.txt", "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/vocab.txt"),
        ],
    },
    ModelDef {
        id: "canary",
        name: "Canary 1B v2",
        size_mb: 980,
        accuracy_rating: 5,
        speed_rating: 2,
        languages: 4,
        quantization: "INT8",
        engine: "onnx",
        translation: TranslationSupport::Pairs(&["EN", "DE", "FR", "ES"]),
        files: &[
            ("encoder-model.int8.onnx", "https://huggingface.co/istupakov/canary-1b-v2-onnx/resolve/main/encoder-model.int8.onnx"),
            ("decoder-model.int8.onnx", "https://huggingface.co/istupakov/canary-1b-v2-onnx/resolve/main/decoder-model.int8.onnx"),
            ("nemo128.onnx", "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/nemo128.onnx"),
            ("vocab.txt", "https://huggingface.co/istupakov/canary-1b-v2-onnx/resolve/main/vocab.txt"),
        ],
    },
    ModelDef {
        id: "gigaam",
        name: "GigaAM v3 E2E",
        size_mb: 220,
        accuracy_rating: 4,
        speed_rating: 4,
        languages: 1,
        quantization: "INT8",
        engine: "onnx",
        translation: TranslationSupport::None,
        files: &[
            ("model.int8.onnx", "https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc.int8.onnx"),
            ("vocab.txt", "https://huggingface.co/istupakov/gigaam-v3-onnx/resolve/main/v3_e2e_ctc_vocab.txt"),
        ],
    },
    ModelDef {
        id: "nemotron",
        name: "Nemotron 3.5 ASR 0.6B",
        size_mb: 716,
        accuracy_rating: 4,
        speed_rating: 4,
        languages: 40,
        quantization: "Q8_0",
        engine: "ggml",
        translation: TranslationSupport::None,
        files: &[
            ("nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf", "https://huggingface.co/handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf/resolve/main/nemotron-3.5-asr-streaming-0.6b-Q8_0.gguf"),
        ],
    },
    ModelDef {
        id: "qwen",
        name: "Qwen3-ASR 0.6B",
        size_mb: 811,
        accuracy_rating: 4,
        speed_rating: 4,
        languages: 40,
        quantization: "Q8_0",
        engine: "ggml",
        translation: TranslationSupport::None,
        files: &[
            ("Qwen3-ASR-0.6B-Q8_0.gguf", "https://huggingface.co/handy-computer/Qwen3-ASR-0.6B-gguf/resolve/main/Qwen3-ASR-0.6B-Q8_0.gguf"),
        ],
    },
];

pub fn get_models_dir() -> PathBuf {
    let mut path = get_app_data_dir();
    path.push("models");
    path
}

pub fn get_model_path(id: &str) -> PathBuf {
    let mut path = get_models_dir();
    path.push(id);
    path
}

#[tauri::command]
pub fn get_models() -> Vec<ModelInfo> {
    let settings = load_settings();
    let active_model = settings.active_model.unwrap_or_else(|| "".to_string());

    MODELS
        .iter()
        .map(|model| {
            let path = get_model_path(model.id);
            let is_downloaded = model.files.iter().all(|(f, _)| path.join(f).exists());

            ModelInfo {
                id: model.id.to_string(),
                name: model.name.to_string(),
                size_mb: model.size_mb,
                is_downloaded,
                is_active: model.id == active_model,
                accuracy_rating: model.accuracy_rating,
                speed_rating: model.speed_rating,
                languages: model.languages,
                quantization: model.quantization.to_string(),
                engine: model.engine.to_string(),
                translation: model.translation.clone(),
            }
        })
        .collect()
}

#[derive(Clone, Serialize)]
struct DownloadProgress {
    id: String,
    progress: f32, // 0.0 to 100.0
}

#[tauri::command]
pub async fn download_model(app: AppHandle, id: String) -> Result<(), String> {
    let model = MODELS
        .iter()
        .find(|m| m.id == id)
        .ok_or("Model not found")?;

    let model_dir = get_model_path(&id);
    if model_dir.exists() {
        fs::remove_dir_all(&model_dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&model_dir).map_err(|e| e.to_string())?;

    let client = Client::new();
    let mut total_size: u64 = 0;
    let mut responses = Vec::new();

    // 1. Resolve sizes and initiate requests
    for &(filename, url) in model.files {
        let res = client.get(url).send().await.map_err(|e| e.to_string())?;
        if !res.status().is_success() {
            let _ = fs::remove_dir_all(&model_dir);
            return Err(format!(
                "Download failed for {}: status {}",
                filename,
                res.status()
            ));
        }
        let size = res.content_length().unwrap_or(0);
        total_size += size;
        responses.push((filename, res));
    }

    // 2. Download contents
    let mut downloaded: u64 = 0;
    for (filename, res) in responses {
        let file_path = model_dir.join(filename);
        let mut file = tokio::fs::File::create(&file_path).await.map_err(|e| {
            let _ = fs::remove_dir_all(&model_dir);
            e.to_string()
        })?;

        let mut stream = res.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                let _ = fs::remove_dir_all(&model_dir);
                e.to_string()
            })?;
            if let Err(e) = file.write_all(&chunk).await {
                let _ = fs::remove_dir_all(&model_dir);
                return Err(e.to_string());
            }

            downloaded += chunk.len() as u64;
            if total_size > 0 {
                let progress = (downloaded as f32 / total_size as f32) * 100.0;
                let _ = app.emit(
                    "download-progress",
                    DownloadProgress {
                        id: id.clone(),
                        progress,
                    },
                );
            }
        }

        file.flush().await.map_err(|e| {
            let _ = fs::remove_dir_all(&model_dir);
            e.to_string()
        })?;
        drop(file);
    }

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            id,
            progress: 100.0,
        },
    );

    Ok(())
}

#[tauri::command]
pub fn delete_model(id: String) -> Result<(), String> {
    let path = get_model_path(&id);
    if path.exists() {
        std::fs::remove_dir_all(path).map_err(|e| e.to_string())?;
    }

    let mut settings = load_settings();
    if let Some(active) = &settings.active_model {
        if active == &id {
            settings.active_model = None;
            let _ = crate::settings::save_settings(&settings);
        }
    }
    Ok(())
}
