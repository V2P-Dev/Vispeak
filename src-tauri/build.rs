use std::{env, fs, path::PathBuf};

fn main() {
    if let Some(runtime_dir) = env::var_os("DEP_TRANSCRIBE_CPP_RUNTIME_DIR") {
        let dest = PathBuf::from(env::var("OUT_DIR").unwrap()).join("../../../"); // root of target dir, roughly

        let runtime_dir = PathBuf::from(runtime_dir);
        for entry in fs::read_dir(&runtime_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name.ends_with(".dll") {
                // Copy to both debug/release root and deps folder to be safe for dev and bundle
                let _ = fs::copy(&path, dest.join(name));
                let _ = fs::copy(&path, dest.join("deps").join(name));
            }
        }
    }
    println!("cargo:rerun-if-env-changed=DEP_TRANSCRIBE_CPP_RUNTIME_DIR");

    tauri_build::build()
}

// Trigger rebuild
