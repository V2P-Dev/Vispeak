use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    pub language: String,
    pub initial_prompt: Option<String>,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            language: "auto".to_string(),
            initial_prompt: None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub active_model: Option<String>,
    pub hotkey: String,
    pub microphone: Option<String>,
    #[serde(default = "default_microphone_gain")]
    pub microphone_gain: f32,
    #[serde(default)]
    pub model_settings: HashMap<String, ModelSettings>,
    pub autostart: bool,
    #[serde(default)]
    pub silent_start: bool,
    #[serde(default = "default_true")]
    pub sound_cues: bool,
    #[serde(default)]
    pub duck_audio: bool,
    #[serde(default = "default_text_input_method")]
    pub text_input_method: String,
    #[serde(default = "default_clipboard_after")]
    pub clipboard_after: String,
    #[serde(default)]
    pub trailing_space: bool,
    #[serde(default = "default_send_after")]
    pub send_after: String,
    #[serde(default = "default_push_to_talk")]
    pub push_to_talk: bool,
    #[serde(default = "default_cancel_hotkey")]
    pub cancel_hotkey: String,
    #[serde(default = "default_overlay_skin")]
    pub overlay_skin: String,
    #[serde(default = "default_overlay_position")]
    pub overlay_position: String,
    #[serde(default = "default_app_language")]
    pub app_language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_history_limit")]
    pub history_limit: u32,
}

fn default_push_to_talk() -> bool {
    false
}
fn default_cancel_hotkey() -> String {
    "Escape".to_string()
}
fn default_overlay_skin() -> String {
    "full".to_string()
}
fn default_overlay_position() -> String {
    "bottom-center".to_string()
}
fn default_microphone_gain() -> f32 {
    1.0
}
fn default_app_language() -> String {
    "system".to_string()
}
fn default_theme() -> String {
    "system".to_string()
}
fn default_true() -> bool {
    true
}
fn default_text_input_method() -> String {
    "paste".to_string()
}
fn default_clipboard_after() -> String {
    "restore".to_string()
}
fn default_send_after() -> String {
    "none".to_string()
}
fn default_history_limit() -> u32 {
    10
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            active_model: None,
            hotkey: "Control+Space".to_string(),
            microphone: None,
            microphone_gain: default_microphone_gain(),
            model_settings: HashMap::new(),
            autostart: false,
            silent_start: false,
            sound_cues: true,
            duck_audio: false,
            text_input_method: default_text_input_method(),
            clipboard_after: default_clipboard_after(),
            trailing_space: false,
            send_after: default_send_after(),
            push_to_talk: default_push_to_talk(),
            cancel_hotkey: default_cancel_hotkey(),
            overlay_skin: default_overlay_skin(),
            overlay_position: default_overlay_position(),
            app_language: default_app_language(),
            theme: default_theme(),
            history_limit: default_history_limit(),
        }
    }
}

pub fn get_app_data_dir() -> PathBuf {
    let base_dirs = directories::BaseDirs::new().expect("No base dirs");
    let mut path = base_dirs.data_local_dir().to_path_buf();
    path.push("app.vispeak");
    path
}

pub fn get_settings_path() -> PathBuf {
    get_app_data_dir().join("settings.json")
}

pub fn load_settings() -> Settings {
    let path = get_settings_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            let mut legacy_language = None;
            let mut legacy_autostart_min = None;
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(lang) = val.get("language").and_then(|v| v.as_str()) {
                    legacy_language = Some(lang.to_string());
                }
                if let Some(am) = val.get("autostart_minimized").and_then(|v| v.as_bool()) {
                    legacy_autostart_min = Some(am);
                }
            }

            if let Ok(mut settings) = serde_json::from_str::<Settings>(&content) {
                let mut mutated = false;
                // Backward compatibility: replace old Tauri shortcut names
                if settings.hotkey.contains("CommandOrControl") {
                    settings.hotkey = settings.hotkey.replace("CommandOrControl", "Control");
                    mutated = true;
                }

                // Migrate legacy language
                if let Some(lang) = legacy_language {
                    if settings.model_settings.is_empty() {
                        for model in crate::models::MODELS {
                            settings.model_settings.insert(
                                model.id.to_string(),
                                ModelSettings {
                                    language: lang.clone(),
                                    initial_prompt: None,
                                },
                            );
                        }
                        mutated = true;
                    }
                }

                // Migrate autostart_minimized
                if let Some(am) = legacy_autostart_min {
                    if am && !settings.silent_start {
                        settings.silent_start = true;
                        mutated = true;
                    }
                }

                if mutated {
                    let _ = save_settings(&settings);
                }
                return settings;
            }
        }
    }
    Settings::default()
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let dir = get_app_data_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }

    let path = get_settings_path();
    let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_active_model() -> Option<String> {
    let settings = load_settings();
    settings.active_model
}

#[tauri::command]
pub fn set_active_model(model_id: String) -> Result<(), String> {
    let mut settings = load_settings();
    settings.active_model = Some(model_id);
    save_settings(&settings)?;
    Ok(())
}

#[tauri::command]
pub fn get_settings() -> Settings {
    load_settings()
}

static SETTINGS_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[tauri::command]
pub fn update_settings(new_settings: Settings) -> Result<(), String> {
    let _guard = SETTINGS_LOCK.lock().unwrap();

    save_settings(&new_settings)?;
    crate::history::enforce_limit(new_settings.history_limit);
    Ok(())
}

#[tauri::command]
pub fn update_single_setting(key: String, value: serde_json::Value) -> Result<(), String> {
    let _guard = SETTINGS_LOCK.lock().unwrap();

    let settings = load_settings();
    let mut val = serde_json::to_value(&settings).map_err(|e| e.to_string())?;

    if let Some(obj) = val.as_object_mut() {
        obj.insert(key, value);
    }

    let new_settings: Settings = serde_json::from_value(val).map_err(|e| e.to_string())?;
    save_settings(&new_settings)?;

    crate::history::enforce_limit(new_settings.history_limit);
    Ok(())
}
