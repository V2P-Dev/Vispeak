use std::collections::HashMap;
use std::sync::Mutex;
use windows::core::{Interface, Result as WinResult};
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDevice,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

lazy_static::lazy_static! {
    static ref ORIGINAL_VOLUMES: Mutex<HashMap<String, f32>> = Mutex::new(HashMap::new());
    static ref IS_DUCKED: Mutex<bool> = Mutex::new(false);
}

fn get_process_name(pid: u32) -> String {
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_INFORMATION,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };
    if pid == 0 {
        return "System Idle Process".to_string();
    }
    if pid == 4 {
        return "System".to_string();
    }
    unsafe {
        if let Ok(handle) = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid)
            .or_else(|_| OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid))
        {
            let mut buffer = [0u16; MAX_PATH as usize];
            let mut len = MAX_PATH;
            if QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buffer.as_mut_ptr()),
                &mut len,
            )
            .is_ok()
                && len > 0
            {
                let _ = windows::Win32::Foundation::CloseHandle(handle);
                let path_str = String::from_utf16_lossy(&buffer[..len as usize]);
                return std::path::Path::new(&path_str)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("PID: {}", pid));
            }
            let len = GetModuleFileNameExW(handle, None, &mut buffer);
            let _ = windows::Win32::Foundation::CloseHandle(handle);
            if len > 0 {
                let path_str = String::from_utf16_lossy(&buffer[..len as usize]);
                return std::path::Path::new(&path_str)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| format!("PID: {}", pid));
            }
        }
    }
    format!("PID: {} (System/Protected)", pid)
}

pub fn duck_audio() {
    {
        let mut is_ducked = IS_DUCKED.lock().unwrap();
        if *is_ducked {
            eprintln!("[info][ducking] duck_audio() called, but already ducked (guard flag active). Skipping duplicate duck.");
            return;
        }
        *is_ducked = true;
    }
    let _ = std::panic::catch_unwind(|| {
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        if let Ok(enumerator) = get_enumerator() {
            if let Ok(device) = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) } {
                if let Ok(session_manager) = get_session_manager(&device) {
                    if let Ok(session_enumerator) =
                        unsafe { session_manager.GetSessionEnumerator() }
                    {
                        let count = unsafe { session_enumerator.GetCount() }.unwrap_or(0);
                        let mut volumes = ORIGINAL_VOLUMES.lock().unwrap();
                        let mut snapshot = HashMap::new();
                        let my_pid = std::process::id();
                        eprintln!("[info][ducking] Enumerating {} eRender sessions for ducking (our PID={})...", count, my_pid);
                        for i in 0..count {
                            if let Ok(session) = unsafe { session_enumerator.GetSession(i) } {
                                if let Ok(session2) = session.cast::<IAudioSessionControl2>() {
                                    if let Ok(pid) = unsafe { session2.GetProcessId() } {
                                        let proc_name = get_process_name(pid);
                                        let is_sys_sound = unsafe {
                                            session2.IsSystemSoundsSession()
                                                == windows::Win32::Foundation::S_OK
                                        };
                                        if pid == my_pid
                                            || pid == 0
                                            || pid == 4
                                            || is_sys_sound
                                            || proc_name.to_lowercase().contains("audiodg")
                                            || proc_name.to_lowercase().contains("svchost")
                                            || proc_name.contains("System/Protected")
                                        {
                                            eprintln!("[info][ducking] Skipping system/self/protected session: {} (PID={}, IsSystemSounds={})", proc_name, pid, is_sys_sound);
                                            continue;
                                        }
                                        if let Ok(simple_volume) =
                                            session.cast::<ISimpleAudioVolume>()
                                        {
                                            let current_vol =
                                                unsafe { simple_volume.GetMasterVolume() }
                                                    .unwrap_or(1.0);
                                            let id = format!("{}_{}", pid, i);
                                            let snapshot_id = format!("{}_{}", proc_name, pid);
                                            volumes.insert(id.clone(), current_vol);
                                            snapshot.insert(snapshot_id, current_vol);

                                            let ducked = current_vol * 0.2;
                                            eprintln!("[info][ducking] Ducking render session: proc={} (PID={}), vol before={:.2}, after={:.2}", proc_name, pid, current_vol, ducked);
                                            if let Err(e) = unsafe {
                                                simple_volume
                                                    .SetMasterVolume(ducked, std::ptr::null())
                                            } {
                                                eprintln!("[warn][ducking] Failed to set master volume for {}: {:?}", id, e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if let Ok(json) = serde_json::to_string(&snapshot) {
                            let path =
                                crate::settings::get_app_data_dir().join("ducked_volumes.json");
                            let _ = std::fs::write(path, json);
                        }
                    }
                }
            }
        }
    });
}

pub fn restore_audio() {
    let _ = std::panic::catch_unwind(|| {
        {
            let mut is_ducked = IS_DUCKED.lock().unwrap();
            if !*is_ducked {
                let volumes = ORIGINAL_VOLUMES.lock().unwrap();
                if volumes.is_empty() {
                    return;
                }
            }
            *is_ducked = false;
        }
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let mut volumes = ORIGINAL_VOLUMES.lock().unwrap();
        if volumes.is_empty() {
            return;
        }

        if let Ok(enumerator) = get_enumerator() {
            if let Ok(device) = unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) } {
                if let Ok(session_manager) = get_session_manager(&device) {
                    if let Ok(session_enumerator) =
                        unsafe { session_manager.GetSessionEnumerator() }
                    {
                        let count = unsafe { session_enumerator.GetCount() }.unwrap_or(0);
                        let my_pid = std::process::id();
                        for i in 0..count {
                            if let Ok(session) = unsafe { session_enumerator.GetSession(i) } {
                                if let Ok(session2) = session.cast::<IAudioSessionControl2>() {
                                    if let Ok(pid) = unsafe { session2.GetProcessId() } {
                                        let proc_name = get_process_name(pid);
                                        let is_sys_sound = unsafe {
                                            session2.IsSystemSoundsSession()
                                                == windows::Win32::Foundation::S_OK
                                        };
                                        if pid == my_pid
                                            || pid == 0
                                            || pid == 4
                                            || is_sys_sound
                                            || proc_name.to_lowercase().contains("audiodg")
                                            || proc_name.to_lowercase().contains("svchost")
                                            || proc_name.contains("System/Protected")
                                        {
                                            continue;
                                        }
                                        if let Ok(simple_volume) =
                                            session.cast::<ISimpleAudioVolume>()
                                        {
                                            let id = format!("{}_{}", pid, i);
                                            if let Some(&orig_vol) = volumes.get(&id) {
                                                eprintln!("[info][ducking] Restoring render session: proc={} (PID={}), target vol={:.2}", proc_name, pid, orig_vol);
                                                let _ = unsafe {
                                                    simple_volume
                                                        .SetMasterVolume(orig_vol, std::ptr::null())
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        volumes.clear();
        let path = crate::settings::get_app_data_dir().join("ducked_volumes.json");
        let _ = std::fs::remove_file(path);
    });
}

pub fn restore_all_on_startup() {
    {
        let mut is_ducked = IS_DUCKED.lock().unwrap();
        *is_ducked = false;
    }
    {
        let mut volumes = ORIGINAL_VOLUMES.lock().unwrap();
        volumes.clear();
    }
    let path = crate::settings::get_app_data_dir().join("ducked_volumes.json");
    if !path.exists() {
        return;
    }
    eprintln!("[info][ducking] Found previous ducked_volumes.json snapshot on startup. Restoring volumes...");
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(snapshot) = serde_json::from_str::<HashMap<String, f32>>(&content) {
            let _ = std::panic::catch_unwind(|| {
                let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
                if let Ok(enumerator) = get_enumerator() {
                    if let Ok(device) =
                        unsafe { enumerator.GetDefaultAudioEndpoint(eRender, eConsole) }
                    {
                        if let Ok(session_manager) = get_session_manager(&device) {
                            if let Ok(session_enumerator) =
                                unsafe { session_manager.GetSessionEnumerator() }
                            {
                                let count = unsafe { session_enumerator.GetCount() }.unwrap_or(0);
                                let my_pid = std::process::id();
                                for i in 0..count {
                                    if let Ok(session) = unsafe { session_enumerator.GetSession(i) }
                                    {
                                        if let Ok(session2) =
                                            session.cast::<IAudioSessionControl2>()
                                        {
                                            if let Ok(pid) = unsafe { session2.GetProcessId() } {
                                                if pid == my_pid || pid == 0 || pid == 4 {
                                                    continue;
                                                }
                                                let proc_name = get_process_name(pid);
                                                let snapshot_id = format!("{}_{}", proc_name, pid);
                                                if let Some(&orig_vol) = snapshot.get(&snapshot_id)
                                                {
                                                    if let Ok(simple_volume) =
                                                        session.cast::<ISimpleAudioVolume>()
                                                    {
                                                        eprintln!("[info][ducking] Startup restore for: proc={} (PID={}), target vol={:.2}", proc_name, pid, orig_vol);
                                                        let _ = unsafe {
                                                            simple_volume.SetMasterVolume(
                                                                orig_vol,
                                                                std::ptr::null(),
                                                            )
                                                        };
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }
    let _ = std::fs::remove_file(path);
}

fn get_enumerator() -> WinResult<IMMDeviceEnumerator> {
    use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
    unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
}

fn get_session_manager(device: &IMMDevice) -> WinResult<IAudioSessionManager2> {
    use windows::Win32::System::Com::CLSCTX_ALL;
    unsafe {
        let obj: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None)?;
        Ok(obj)
    }
}
