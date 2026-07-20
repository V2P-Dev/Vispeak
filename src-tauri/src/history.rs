use chrono::Utc;
use hound::{SampleFormat, WavSpec, WavWriter};
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::command;

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryRecord {
    pub id: i64,
    pub timestamp: String,
    pub text: String,
    pub model_id: String,
    pub target_app_name: String,
    pub target_app_icon: String, // base64
    pub duration_sec: f32,
    pub has_audio: bool,
}

#[derive(Serialize, Deserialize)]
pub struct HistorySize {
    pub size_mb: f64,
}

pub fn get_history_dir() -> PathBuf {
    crate::settings::get_app_data_dir()
}

pub fn get_audio_dir() -> PathBuf {
    let mut path = get_history_dir();
    path.push("history");
    path.push("audio");
    path
}

pub fn get_db_path() -> PathBuf {
    let mut path = get_history_dir();
    path.push("history.db");
    path
}

fn get_conn() -> SqlResult<Connection> {
    Connection::open(get_db_path())
}

pub fn init_db() {
    let _ = fs::create_dir_all(get_audio_dir());
    if let Ok(conn) = get_conn() {
        let _ = conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                text TEXT NOT NULL,
                model_id TEXT NOT NULL,
                target_app_name TEXT NOT NULL,
                target_app_icon TEXT NOT NULL,
                duration_sec REAL NOT NULL
            )",
            [],
        );
        cleanup_orphans(&conn);
    }
}

pub fn cleanup_orphans(conn: &Connection) {
    let audio_dir = get_audio_dir();

    let mut valid_ids = std::collections::HashSet::new();
    if let Ok(mut stmt) = conn.prepare("SELECT id FROM history") {
        let iter = stmt.query_map([], |row| row.get::<_, i64>(0));
        if let Ok(ids) = iter {
            for id in ids.flatten() {
                valid_ids.insert(id);
            }
        }
    }

    if let Ok(entries) = fs::read_dir(&audio_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "wav" {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(id) = stem.parse::<i64>() {
                            if !valid_ids.contains(&id) {
                                let _ = fs::remove_file(path);
                            }
                        }
                    }
                }
            }
        }
    }

    let mut to_delete = Vec::new();
    for id in valid_ids {
        let mut audio_path = audio_dir.clone();
        audio_path.push(format!("{}.wav", id));
        if !audio_path.exists() {
            to_delete.push(id);
        }
    }
    for id in to_delete {
        let _ = conn.execute("DELETE FROM history WHERE id = ?1", params![id]);
    }
}

pub fn add_record(
    text: String,
    model_id: String,
    target_app_name: String,
    target_app_icon: String,
    samples: Vec<f32>,
) {
    let limit = crate::settings::load_settings().history_limit;
    if limit == 0 {
        return;
    }

    let timestamp = Utc::now().to_rfc3339();
    let duration_sec = samples.len() as f32 / 16000.0;

    let conn = match get_conn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[error] Failed to open DB: {}", e);
            return;
        }
    };

    let res = conn.execute(
        "INSERT INTO history (timestamp, text, model_id, target_app_name, target_app_icon, duration_sec)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![timestamp, text, model_id, target_app_name, target_app_icon, duration_sec],
    );

    if let Ok(_) = res {
        let id = conn.last_insert_rowid();
        let mut audio_path = get_audio_dir();
        audio_path.push(format!("{}.wav", id));

        let spec = WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        if let Ok(mut writer) = WavWriter::create(&audio_path, spec) {
            for sample in samples {
                let s = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                let _ = writer.write_sample(s);
            }
            let _ = writer.finalize();
        }

        enforce_limit_with_conn(&conn, limit);
    }
}

pub fn enforce_limit(limit: u32) {
    if let Ok(conn) = get_conn() {
        enforce_limit_with_conn(&conn, limit);
    }
}

fn enforce_limit_with_conn(conn: &Connection, limit: u32) {
    if limit == 0 {
        return;
    }

    let audio_dir = get_audio_dir();

    if let Ok(mut stmt) = conn.prepare("SELECT id FROM history ORDER BY id DESC LIMIT -1 OFFSET ?1")
    {
        let iter = stmt.query_map(params![limit], |row| row.get::<_, i64>(0));
        if let Ok(ids) = iter {
            for id_res in ids {
                if let Ok(id) = id_res {
                    let _ = conn.execute("DELETE FROM history WHERE id = ?1", params![id]);
                    let mut audio_path = audio_dir.clone();
                    audio_path.push(format!("{}.wav", id));
                    let _ = fs::remove_file(audio_path);
                }
            }
        }
    }
}

#[command]
pub fn get_history() -> Result<Vec<HistoryRecord>, String> {
    let conn = get_conn().map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, timestamp, text, model_id, target_app_name, target_app_icon, duration_sec FROM history ORDER BY id DESC").map_err(|e| e.to_string())?;

    let iter = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let mut audio_path = get_audio_dir();
            audio_path.push(format!("{}.wav", id));

            Ok(HistoryRecord {
                id,
                timestamp: row.get(1)?,
                text: row.get(2)?,
                model_id: row.get(3)?,
                target_app_name: row.get(4)?,
                target_app_icon: row.get(5)?,
                duration_sec: row.get(6)?,
                has_audio: audio_path.exists(),
            })
        })
        .map_err(|e| e.to_string())?;

    let mut res = Vec::new();
    for r in iter {
        if let Ok(record) = r {
            res.push(record);
        }
    }

    Ok(res)
}

#[command]
pub fn delete_history_record(id: i64) -> Result<(), String> {
    let conn = get_conn().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    let mut audio_path = get_audio_dir();
    audio_path.push(format!("{}.wav", id));
    let _ = fs::remove_file(audio_path);

    Ok(())
}

#[command]
pub fn clear_history() -> Result<(), String> {
    let conn = get_conn().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM history", [])
        .map_err(|e| e.to_string())?;

    if let Ok(entries) = fs::read_dir(get_audio_dir()) {
        for entry in entries.flatten() {
            let _ = fs::remove_file(entry.path());
        }
    }
    Ok(())
}

#[command]
pub fn get_history_size() -> Result<HistorySize, String> {
    let mut size: u64 = 0;

    if let Ok(md) = fs::metadata(get_db_path()) {
        size += md.len();
    }

    if let Ok(entries) = fs::read_dir(get_audio_dir()) {
        for entry in entries.flatten() {
            if let Ok(md) = entry.metadata() {
                size += md.len();
            }
        }
    }

    let size_mb = size as f64 / 1_048_576.0;
    Ok(HistorySize { size_mb })
}

#[command]
pub fn get_history_audio_path(id: i64) -> Result<String, String> {
    let mut audio_path = get_audio_dir();
    audio_path.push(format!("{}.wav", id));
    if audio_path.exists() {
        Ok(audio_path.to_string_lossy().to_string())
    } else {
        Err("Audio file not found".into())
    }
}

#[command]
pub fn retranscribe_history_record(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    let audio_path = get_history_audio_path(id)?;
    let mut reader = hound::WavReader::open(&audio_path).map_err(|e| e.to_string())?;
    let samples: Vec<f32> = reader
        .samples::<i16>()
        .map(|s| {
            if let Ok(val) = s {
                val as f32 / i16::MAX as f32
            } else {
                0.0
            }
        })
        .collect();

    let settings = crate::settings::load_settings();
    if let Some(model_id) = settings.active_model {
        crate::transcribe::request_transcription(&app, samples, model_id, Some(id));
        Ok(())
    } else {
        Err("err_no_model_selected".into())
    }
}

#[command]
pub fn update_history_record_text(
    id: i64,
    new_text: String,
    new_model_id: String,
) -> Result<(), String> {
    let conn = get_conn().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE history SET text = ?1, model_id = ?2 WHERE id = ?3",
        params![new_text, new_model_id, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub fn repeat_paste_history_record(id: i64) -> Result<(), String> {
    let conn = get_conn().map_err(|e| e.to_string())?;
    let text: String = conn
        .query_row(
            "SELECT text FROM history WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // We spawn a thread to paste after 1.5s
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let target_hwnd =
            unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize };
        crate::paste::paste_text(&text, Some(target_hwnd));
    });

    Ok(())
}
