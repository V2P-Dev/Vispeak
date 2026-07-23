use std::collections::HashSet;
use std::sync::{mpsc::channel, Mutex, OnceLock};
use std::time::Duration;
use windows::Win32::Foundation::{BOOL, HWND, POINT, RECT};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, MonitorFromPoint, MonitorFromRect, MONITOR_DEFAULTTONULL,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
    SAFEARRAY,
};
use windows::Win32::System::Ole::{
    SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound,
    SafeArrayUnaccessData,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, IUIAutomationTextPattern2,
    IUIAutomationTextRange, UIA_TextPattern2Id, UIA_TextPatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, GUITHREADINFO,
};

// Maximum width (px) for a single-line or small input field (search box, address bar, single-line chat).
// Elements wider than this or taller than FIELD_MAX_HEIGHT are classified as Area.
pub const FIELD_MAX_WIDTH: i32 = 4000;
// Maximum height (px) for a small input field.
pub const FIELD_MAX_HEIGHT: i32 = 250;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CaretKind {
    Caret,
    Field,
    Area,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CaretRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub kind: CaretKind,
}

impl CaretRect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        let w = right - left;
        let h = bottom - top;
        let kind = if w <= 4 || (h <= 40 && w <= h * 2) {
            CaretKind::Caret
        } else if w <= FIELD_MAX_WIDTH && h <= FIELD_MAX_HEIGHT {
            CaretKind::Field
        } else {
            CaretKind::Area
        };
        Self {
            left,
            top,
            right,
            bottom,
            kind,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CaretMethod {
    Uia,
    Win32,
    Fallback,
}

unsafe fn safearray_to_vec_f64(psa: *mut SAFEARRAY) -> Vec<f64> {
    if psa.is_null() {
        return Vec::new();
    }
    let lbound = match SafeArrayGetLBound(psa, 1) {
        Ok(v) => v,
        Err(_) => {
            let _ = SafeArrayDestroy(psa);
            return Vec::new();
        }
    };
    let ubound = match SafeArrayGetUBound(psa, 1) {
        Ok(v) => v,
        Err(_) => {
            let _ = SafeArrayDestroy(psa);
            return Vec::new();
        }
    };
    let count = (ubound - lbound + 1) as usize;
    if count == 0 {
        let _ = SafeArrayDestroy(psa);
        return Vec::new();
    }
    let mut ptr: *mut std::ffi::c_void = std::ptr::null_mut();
    if SafeArrayAccessData(psa, &mut ptr).is_err() || ptr.is_null() {
        let _ = SafeArrayDestroy(psa);
        return Vec::new();
    }
    let slice = std::slice::from_raw_parts(ptr as *const f64, count);
    let result = slice.to_vec();
    let _ = SafeArrayUnaccessData(psa);
    let _ = SafeArrayDestroy(psa);
    result
}

fn get_window_info(hwnd: HWND) -> (String, String, String) {
    unsafe {
        let mut title = [0u16; 512];
        let len = GetWindowTextW(hwnd, &mut title);
        let title_str = String::from_utf16_lossy(&title[..len as usize]);

        let mut class = [0u16; 512];
        let len = GetClassNameW(hwnd, &mut class);
        let class_str = String::from_utf16_lossy(&class[..len as usize]);

        let mut process_id = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));

        let mut process_str = String::new();
        if process_id != 0 {
            if let Ok(hproc) = windows::Win32::System::Threading::OpenProcess(
                windows::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
                false,
                process_id,
            ) {
                let mut path = [0u16; 1024];
                let len = windows::Win32::System::ProcessStatus::GetProcessImageFileNameW(
                    hproc, &mut path,
                );
                if len > 0 {
                    let path_str = String::from_utf16_lossy(&path[..len as usize]);
                    if let Some(idx) = path_str.rfind('\\') {
                        process_str = path_str[idx + 1..].to_string();
                    } else {
                        process_str = path_str;
                    }
                }
                let _ = windows::Win32::Foundation::CloseHandle(hproc);
            }
        }

        (title_str, class_str, process_str)
    }
}

pub fn validate_caret_rect(
    rect: &CaretRect,
    target_hwnd: HWND,
    _method_name: &str,
) -> Result<(), String> {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    if width < 0 || width > 5000 {
        return Err(format!("invalid width: {}", width));
    }
    if height <= 0 || height > 3000 {
        return Err(format!(
            "invalid height: {} (must be 0 < h <= 3000)",
            height
        ));
    }

    let mut win_rc: RECT = unsafe { std::mem::zeroed() };
    let mut has_win_rc = false;

    if !target_hwnd.0.is_null() {
        unsafe {
            if GetWindowRect(target_hwnd, &mut win_rc).is_ok() {
                has_win_rc = true;
                let tolerance = 64;
                let min_left = win_rc.left - tolerance;
                let max_right = win_rc.right + tolerance;
                let min_top = win_rc.top - tolerance;
                let max_bottom = win_rc.bottom + tolerance;

                if rect.left < min_left
                    || rect.right > max_right
                    || rect.top < min_top
                    || rect.bottom > max_bottom
                {
                    return Err(format!(
                        "rect {:?} outside target window rect {:?} (tolerance {})",
                        rect, win_rc, tolerance
                    ));
                }
            }
        }
    }

    if has_win_rc {
        if (rect.left == 0 && rect.top == 0) || (rect.left == win_rc.left && rect.top == win_rc.top)
        {
            return Err(format!(
                "[CARET SUSPECT] rect {:?} at 0,0 or matches window top-left {:?}",
                rect, win_rc
            ));
        }
    } else if rect.left == 0 && rect.top == 0 {
        return Err(format!("[CARET SUSPECT] rect {:?} at 0,0", rect));
    }

    unsafe {
        let win_rect = RECT {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        };
        let hmonitor = MonitorFromRect(&win_rect, MONITOR_DEFAULTTONULL);
        if hmonitor.0.is_null() {
            let pt = POINT {
                x: (rect.left + rect.right) / 2,
                y: (rect.top + rect.bottom) / 2,
            };
            let hmon_pt = MonitorFromPoint(pt, MONITOR_DEFAULTTONULL);
            if hmon_pt.0.is_null() {
                return Err(format!("rect {:?} is outside all monitors", rect));
            }
        }
    }

    Ok(())
}

pub fn try_get_uia_caret_rect(target_hwnd: HWND) -> (Option<CaretRect>, String) {
    let mut log = String::new();
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let initialized = hr.is_ok();

        let res = (|| -> Option<CaretRect> {
            let automation: IUIAutomation =
                match CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
                    Ok(a) => a,
                    Err(e) => {
                        log.push_str(&format!("Failed CoCreateInstance: {:?}\n", e));
                        return None;
                    }
                };
            let mut attempt = 0;
            loop {
                let element = match automation.GetFocusedElement() {
                    Ok(e) => e,
                    Err(e) => {
                        log.push_str(&format!("Failed GetFocusedElement: {:?}\n", e));
                        return None;
                    }
                };

                let mut range_opt: Option<IUIAutomationTextRange> = None;
                let mut pattern_source = "none";

                if let Ok(pattern2) =
                    element.GetCurrentPatternAs::<IUIAutomationTextPattern2>(UIA_TextPattern2Id)
                {
                    let mut is_active = BOOL(0);
                    if let Ok(r) = pattern2.GetCaretRange(&mut is_active) {
                        range_opt = Some(r);
                        pattern_source = "TextPattern2.GetCaretRange";
                    }
                }

                if range_opt.is_none() {
                    if let Ok(pattern) =
                        element.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
                    {
                        if let Ok(selection) = pattern.GetSelection() {
                            if let Ok(len) = selection.Length() {
                                if len > 0 {
                                    if let Ok(r) = selection.GetElement(0) {
                                        range_opt = Some(r);
                                        pattern_source = "TextPattern.GetSelection";
                                    }
                                }
                            }
                        }
                    }
                }

                let range = match range_opt {
                    Some(r) => r,
                    None => {
                        log.push_str("No TextPattern available\n");
                        return None;
                    }
                };

                let mut slice_data: Vec<f64> = Vec::new();
                if let Ok(psa) = range.GetBoundingRectangles() {
                    let data = safearray_to_vec_f64(psa);
                    if data.len() >= 4 {
                        slice_data.extend_from_slice(&data[0..4]);
                    }
                }

                if slice_data.is_empty()
                    || (slice_data.len() >= 4 && slice_data[2] == 0.0 && slice_data[3] == 0.0)
                {
                    if let Ok(rect) = element.CurrentBoundingRectangle() {
                        let left = rect.left as f64;
                        let top = rect.top as f64;
                        let width = (rect.right - rect.left) as f64;
                        let height = (rect.bottom - rect.top) as f64;
                        slice_data = vec![left, top, width, height];
                        pattern_source = "element bounds fallback";
                    }
                }

                if slice_data.len() >= 4 {
                    let left = slice_data[0];
                    let top = slice_data[1];
                    let width = slice_data[2];
                    let height = slice_data[3];

                    let raw_rect = CaretRect::new(
                        left.round() as i32,
                        top.round() as i32,
                        (left + width).round() as i32,
                        (top + height).round() as i32,
                    );

                    if raw_rect.kind != CaretKind::Caret && attempt < 2 {
                        attempt += 1;
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        continue;
                    }

                    log.push_str(&format!(
                        "2. UIA branch: SUCCESS (source={})\n",
                        pattern_source
                    ));

                    let classification_reason = format!(
                        "w={}, h={}",
                        (raw_rect.right - raw_rect.left),
                        (raw_rect.bottom - raw_rect.top)
                    );
                    log.push_str(&format!(
                        "3. Raw Rect: left={}, top={}, right={}, bottom={}\n",
                        raw_rect.left, raw_rect.top, raw_rect.right, raw_rect.bottom
                    ));
                    log.push_str(&format!(
                        "5. Classification: {:?} (reason: {})\n",
                        raw_rect.kind, classification_reason
                    ));

                    if let Err(reason) = validate_caret_rect(&raw_rect, target_hwnd, "uia") {
                        if reason.contains("[CARET SUSPECT]") {
                            log.push_str(&format!("4. Validation: SUSPECT ({})\n", reason));
                        } else {
                            log.push_str(&format!("4. Validation: FAILED ({})\n", reason));
                        }
                        return None;
                    }

                    log.push_str("4. Validation: PASSED\n");
                    return Some(raw_rect);
                } else {
                    log.push_str(&format!(
                        "Invalid slice_data length: {}\n",
                        slice_data.len()
                    ));
                    return None;
                }
            }
        })();

        if initialized {
            CoUninitialize();
        }

        (res, log)
    }
}

pub fn try_get_win32_caret_rect(target_hwnd: Option<HWND>) -> (Option<CaretRect>, String) {
    let mut log = String::new();
    unsafe {
        let hwnd = target_hwnd.unwrap_or_else(|| GetForegroundWindow());
        if hwnd.0.is_null() {
            log.push_str("target_hwnd/foreground is null\n");
            return (None, log);
        }

        let thread_id = GetWindowThreadProcessId(hwnd, None);
        if thread_id == 0 {
            log.push_str("GetWindowThreadProcessId returned 0\n");
            return (None, log);
        }

        let mut gti: GUITHREADINFO = std::mem::zeroed();
        gti.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;

        if GetGUIThreadInfo(thread_id, &mut gti).is_ok() {
            let rc = gti.rcCaret;

            if gti.hwndCaret.0.is_null() {
                log.push_str("gti.hwndCaret is null -> Win32 caret not active\n");
                return (None, log);
            }

            let caret_hwnd = gti.hwndCaret;

            if (rc.right - rc.left) >= 0
                && (rc.bottom - rc.top) > 0
                && !(rc.left == 0 && rc.top == 0 && rc.right == 0 && rc.bottom == 0)
            {
                let mut pt_top_left = POINT {
                    x: rc.left,
                    y: rc.top,
                };
                let mut pt_bottom_right = POINT {
                    x: rc.right,
                    y: rc.bottom,
                };

                let ok_tl = ClientToScreen(caret_hwnd, &mut pt_top_left).as_bool();
                let ok_br = ClientToScreen(caret_hwnd, &mut pt_bottom_right).as_bool();

                if ok_tl && ok_br {
                    let raw_rect = CaretRect::new(
                        pt_top_left.x,
                        pt_top_left.y,
                        pt_bottom_right.x,
                        pt_bottom_right.y,
                    );

                    log.push_str("2. Win32 branch: SUCCESS\n");

                    let classification_reason = format!(
                        "w={}, h={}",
                        (raw_rect.right - raw_rect.left),
                        (raw_rect.bottom - raw_rect.top)
                    );
                    log.push_str(&format!(
                        "3. Raw Rect: left={}, top={}, right={}, bottom={}\n",
                        raw_rect.left, raw_rect.top, raw_rect.right, raw_rect.bottom
                    ));
                    log.push_str(&format!(
                        "5. Classification: {:?} (reason: {})\n",
                        raw_rect.kind, classification_reason
                    ));

                    if let Err(reason) = validate_caret_rect(&raw_rect, hwnd, "win32") {
                        if reason.contains("[CARET SUSPECT]") {
                            log.push_str(&format!("4. Validation: SUSPECT ({})\n", reason));
                        } else {
                            log.push_str(&format!("4. Validation: FAILED ({})\n", reason));
                        }
                        return (None, log);
                    }

                    log.push_str("4. Validation: PASSED\n");
                    return (Some(raw_rect), log);
                } else {
                    log.push_str("2. Win32 branch: ClientToScreen failed\n");
                }
            } else {
                log.push_str(&format!(
                    "2. Win32 branch: Invalid rcCaret dimensions: {:?}\n",
                    rc
                ));
            }
        } else {
            log.push_str("2. Win32 branch: GetGUIThreadInfo failed\n");
        }
        (None, log)
    }
}

static SEEN_HWNDS: OnceLock<Mutex<HashSet<isize>>> = OnceLock::new();

fn is_hwnd_seen(hwnd: isize) -> bool {
    let mut set = SEEN_HWNDS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap();
    if set.contains(&hwnd) {
        true
    } else {
        set.insert(hwnd);
        false
    }
}

pub fn get_caret_position(target_hwnd: Option<HWND>) -> (Option<CaretRect>, CaretMethod, String) {
    let hwnd = unsafe { target_hwnd.unwrap_or_else(|| GetForegroundWindow()) };
    let hwnd_val = hwnd.0 as usize;
    let seen = is_hwnd_seen(hwnd_val as isize);

    let (title, class, process) = get_window_info(hwnd);

    let mut trace = format!(
        "[CARET DIAGNOSTICS]\n1. Window: process='{}', class='{}', title='{}'\n",
        process, class, title
    );

    let max_attempts = if seen { 1 } else { 3 };
    let mut attempt = 0;

    let mut final_selected_rect = None;
    let mut final_selected_method = CaretMethod::Fallback;

    while attempt < max_attempts {
        attempt += 1;
        if attempt == 2 {
            trace.push_str("\n[RETRY] Attempt 2 for new HWND (waiting 150ms)...\n");
            std::thread::sleep(Duration::from_millis(150));
        } else if attempt == 3 {
            trace.push_str("\n[RETRY] Attempt 3 for new HWND (waiting 300ms)...\n");
            std::thread::sleep(Duration::from_millis(300));
        }

        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let res = try_get_uia_caret_rect(HWND(hwnd_val as *mut _));
            let _ = tx.send(res);
        });

        let mut selected_rect = None;
        let mut selected_method = CaretMethod::Fallback;
        let mut uia_is_caret = false;

        if let Ok((res_opt, log)) = rx.recv_timeout(Duration::from_millis(150)) {
            if let Some(rect) = res_opt {
                trace.push_str(&log);
                selected_rect = Some(rect);
                selected_method = CaretMethod::Uia;
                if rect.kind == CaretKind::Caret {
                    uia_is_caret = true;
                }
            } else {
                trace.push_str("2. UIA branch: Failed. ");
                trace.push_str(&log);
            }
        } else {
            trace.push_str("2. UIA branch: Failed (timeout 150ms)\n");
        }

        if selected_rect.is_none() || !uia_is_caret {
            let (res_opt, log) = try_get_win32_caret_rect(Some(hwnd));
            if let Some(rect) = res_opt {
                if rect.kind == CaretKind::Caret {
                    if selected_rect.is_some() {
                        trace.push_str(
                            "2. UIA yielded Field/Area, but Win32 found Caret! Preferring Win32.\n",
                        );
                    }
                    trace.push_str(&log);
                    selected_rect = Some(rect);
                    selected_method = CaretMethod::Win32;
                } else {
                    trace.push_str("2. Win32 branch: Fallback attempted, but also not Caret.\n");
                    trace.push_str(&log);
                }
            } else {
                if selected_rect.is_some() {
                    trace.push_str("2. Win32 branch: Fallback failed. Keeping UIA Field/Area.\n");
                } else {
                    trace.push_str(&log);
                }
            }
        } else {
            trace.push_str("2. Win32 branch: Skipped (UIA found Caret)\n");
        }

        final_selected_rect = selected_rect;
        final_selected_method = selected_method;

        if final_selected_rect.is_some() {
            trace.push_str(&format!("-> Success on attempt {}\n", attempt));
            break;
        }
    }

    if final_selected_rect.is_none() {
        trace.push_str("2. Fallback branch: Selected\n");
    }

    (final_selected_rect, final_selected_method, trace)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MiniPositionResult {
    pub x: f64,
    pub y: f64,
    pub placement: &'static str,
    pub clamped_x: bool,
    pub anchor_x: f64,
}

pub fn calculate_mini_position(
    rect: &CaretRect,
    scale_caret: f64,
    logical_width: f64,
    logical_height: f64,
    glow_padding: f64,
    rc_work: &RECT,
) -> MiniPositionResult {
    let phys_width = logical_width * scale_caret;
    let phys_height = logical_height * scale_caret;
    let phys_glow = glow_padding * scale_caret;
    let gap = 8.0 * scale_caret;

    let work_left = rc_work.left as f64;
    let work_right = rc_work.right as f64;
    let work_top = rc_work.top as f64;
    let work_bottom = rc_work.bottom as f64;

    let caret_left = rect.left as f64;
    let caret_top = rect.top as f64;
    let caret_bottom = rect.bottom as f64;

    let anchor_x = caret_left;

    let raw_x = anchor_x - phys_glow;
    let min_x = work_left + gap - phys_glow;
    let max_x = work_right - gap - (phys_width - phys_glow);

    let clamped_x = raw_x < min_x || raw_x > max_x;
    let x = raw_x.clamp(min_x, max_x);

    let y_above = caret_top - gap - phys_height + phys_glow;
    let y_below = caret_bottom + gap - phys_glow;

    let fits_above = (y_above + phys_glow) >= work_top;
    let (chosen_y, placement) = if fits_above {
        (y_above, "above")
    } else {
        (y_below, "below")
    };

    let min_y = work_top + gap - phys_glow;
    let max_y = work_bottom - gap - (phys_height - phys_glow);
    let y = chosen_y.clamp(min_y, max_y);

    MiniPositionResult {
        x,
        y,
        placement,
        clamped_x,
        anchor_x,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_work_rect() -> RECT {
        RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        }
    }

    #[test]
    fn test_caret_rect_classification() {
        assert_eq!(CaretRect::new(100, 100, 102, 120).kind, CaretKind::Caret);
        assert_eq!(CaretRect::new(100, 100, 400, 130).kind, CaretKind::Field);
        assert_eq!(CaretRect::new(100, 100, 900, 600).kind, CaretKind::Area);
        assert_eq!(CaretRect::new(100, 100, 300, 200).kind, CaretKind::Area);
    }

    #[test]
    fn test_caret_center_screen() {
        let work = mock_work_rect();
        let rect = CaretRect {
            left: 959,
            top: 540,
            right: 961,
            bottom: 560,
            kind: CaretKind::Caret,
        };
        let res = calculate_mini_position(&rect, 1.0, 144.0, 82.0, 24.0, &work);
        assert_eq!(res.placement, "above");
        assert_eq!(res.clamped_x, false);
        assert_eq!(res.anchor_x, 959.0);
        assert_eq!(res.x, 935.0);
        assert_eq!(res.y, 474.0);
    }

    #[test]
    fn test_field_centered_on_left_edge() {
        let work = mock_work_rect();
        let rect = CaretRect {
            left: 500,
            top: 540,
            right: 800,
            bottom: 580,
            kind: CaretKind::Field,
        };
        let res = calculate_mini_position(&rect, 1.0, 144.0, 82.0, 24.0, &work);
        assert_eq!(res.placement, "above");
        assert_eq!(res.clamped_x, false);
        assert_eq!(res.anchor_x, 500.0);
        assert_eq!(res.x, 476.0);
    }

    #[test]
    fn test_caret_left_edge_clamped() {
        let work = mock_work_rect();
        let rect = CaretRect {
            left: 1,
            top: 540,
            right: 3,
            bottom: 560,
            kind: CaretKind::Caret,
        };
        let res = calculate_mini_position(&rect, 1.0, 144.0, 82.0, 24.0, &work);
        assert_eq!(res.placement, "above");
        assert_eq!(res.clamped_x, true);
        assert_eq!(res.x, -16.0);
    }

    #[test]
    fn test_caret_right_edge_clamped() {
        let work = mock_work_rect();
        let rect = CaretRect {
            left: 1917,
            top: 540,
            right: 1919,
            bottom: 560,
            kind: CaretKind::Caret,
        };
        let res = calculate_mini_position(&rect, 1.0, 144.0, 82.0, 24.0, &work);
        assert_eq!(res.placement, "above");
        assert_eq!(res.clamped_x, true);
        assert_eq!(res.x, 1792.0);
    }

    #[test]
    fn test_caret_top_row_below() {
        let work = mock_work_rect();
        let rect = CaretRect {
            left: 959,
            top: 5,
            right: 961,
            bottom: 25,
            kind: CaretKind::Caret,
        };
        let res = calculate_mini_position(&rect, 1.0, 144.0, 82.0, 24.0, &work);
        assert_eq!(res.placement, "below");
        assert_eq!(res.clamped_x, false);
        assert_eq!(res.y, 9.0);
    }

    #[test]
    fn test_caret_top_left_corner() {
        let work = mock_work_rect();
        let rect = CaretRect {
            left: 1,
            top: 2,
            right: 3,
            bottom: 22,
            kind: CaretKind::Caret,
        };
        let res = calculate_mini_position(&rect, 1.0, 144.0, 82.0, 24.0, &work);
        assert_eq!(res.placement, "below");
        assert_eq!(res.clamped_x, true);
        assert_eq!(res.x, -16.0);
        assert_eq!(res.y, 6.0);
    }

    #[test]
    fn test_area_with_mouse_centered_on_left_edge_not_mouse() {
        let work = mock_work_rect();
        let area = CaretRect::new(200, 100, 1200, 800);
        assert_eq!(area.kind, CaretKind::Area);

        let mouse_rect = CaretRect {
            left: area.left,
            top: 450,
            right: area.right,
            bottom: 450,
            kind: CaretKind::Area,
        };
        let res = calculate_mini_position(&mouse_rect, 1.0, 144.0, 82.0, 24.0, &work);
        assert_eq!(res.placement, "above");
        assert_eq!(res.clamped_x, false);
        assert_eq!(res.anchor_x, 200.0);
        assert_eq!(res.x, 176.0);
        assert_eq!(res.y, 450.0 - 8.0 - 82.0 + 24.0);
    }
}
