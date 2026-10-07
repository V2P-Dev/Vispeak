//! Best-effort per-stream attenuation via PulseAudio / PipeWire's Pulse server.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct Snapshot {
    index: u64,
    process: String,
    client: u64,
    volumes: Vec<u64>,
}
pub struct DuckingGuard {
    snapshots: Vec<Snapshot>,
}

fn recovery_path() -> PathBuf {
    crate::settings::get_app_data_dir().join("linux-ducking.json")
}
fn inputs() -> Result<Vec<Value>, String> {
    let bytes = super::paste::command("pactl", &["--format=json", "list", "sink-inputs"], None)?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}
fn set_volume(index: u64, volumes: &[u64]) -> Result<(), String> {
    let values: Vec<_> = volumes.iter().map(ToString::to_string).collect();
    let index = index.to_string();
    let mut args = vec!["set-sink-input-volume", index.as_str()];
    args.extend(values.iter().map(String::as_str));
    super::paste::command("pactl", &args, None).map(|_| ())
}
fn restore(snapshots: &[Snapshot]) -> bool {
    let Ok(current) = inputs() else { return false };
    let mut restored = true;
    for snapshot in snapshots {
        // Avoid restoring a reused stream id belonging to a different process.
        if current.iter().any(|input| {
            input["index"].as_u64() == Some(snapshot.index)
                && input["client"].as_u64() == Some(snapshot.client)
                && input["properties"]["application.process.id"].as_str()
                    == Some(snapshot.process.as_str())
        }) {
            if set_volume(snapshot.index, &snapshot.volumes).is_err() {
                restored = false;
            }
        }
    }
    restored
}
impl DuckingGuard {
    pub fn new() -> Self {
        let mut snapshots = Vec::new();
        let result = (|| {
            for input in inputs()? {
                let Some(process) = input["properties"]["application.process.id"].as_str() else {
                    continue;
                };
                if process == std::process::id().to_string() {
                    continue;
                }
                let (Some(index), Some(client), Some(volume)) = (
                    input["index"].as_u64(),
                    input["client"].as_u64(),
                    input["volume"].as_object(),
                ) else {
                    continue;
                };
                let volumes: Vec<_> = input["channel_map"]
                    .as_str()
                    .unwrap_or("")
                    .split(',')
                    .filter_map(|channel| volume.get(channel))
                    .filter_map(|channel| channel["value"].as_u64())
                    .collect();
                if !volumes.is_empty() {
                    snapshots.push(Snapshot {
                        index,
                        client,
                        process: process.into(),
                        volumes,
                    });
                }
            }
            // Save first, so abnormal termination can be recovered on next startup.
            let bytes = serde_json::to_vec(&snapshots).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(crate::settings::get_app_data_dir())
                .map_err(|e| e.to_string())?;
            std::fs::write(recovery_path(), bytes).map_err(|e| e.to_string())?;
            for snapshot in &snapshots {
                let ducked: Vec<_> = snapshot.volumes.iter().map(|value| value / 5).collect();
                set_volume(snapshot.index, &ducked)?;
            }
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            eprintln!("[linux][ducking] {error}");
        }
        Self { snapshots }
    }
}
impl Drop for DuckingGuard {
    fn drop(&mut self) {
        if restore(&self.snapshots) {
            let _ = std::fs::remove_file(recovery_path());
        }
    }
}
pub fn restore_all_on_startup() {
    if let Ok(bytes) = std::fs::read(recovery_path()) {
        if let Ok(snapshots) = serde_json::from_slice::<Vec<Snapshot>>(&bytes) {
            if restore(&snapshots) {
                let _ = std::fs::remove_file(recovery_path());
            }
        }
    }
}
