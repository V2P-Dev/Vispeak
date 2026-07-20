use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, DeleteDC, GetDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassLongPtrW, GetIconInfo, SendMessageTimeoutW, GCLP_HICON, GCLP_HICONSM, HICON, ICONINFO,
    ICON_BIG, ICON_SMALL, ICON_SMALL2, SMTO_ABORTIFHUNG, WM_GETICON,
};

pub fn get_window_icon_base64(hwnd: HWND) -> String {
    unsafe {
        if let Some(hicon) = get_hicon(hwnd) {
            if let Some(base64_str) = hicon_to_png_base64(hicon) {
                return base64_str;
            }
        }
    }
    "".to_string()
}

unsafe fn get_hicon(hwnd: HWND) -> Option<HICON> {
    let mut res: usize = 0;
    if SendMessageTimeoutW(
        hwnd,
        WM_GETICON,
        WPARAM(ICON_SMALL2 as _),
        LPARAM(0),
        SMTO_ABORTIFHUNG,
        100,
        Some(&mut res as *mut usize),
    )
    .0 != 0
        && res != 0
    {
        return Some(HICON(res as _));
    }
    if SendMessageTimeoutW(
        hwnd,
        WM_GETICON,
        WPARAM(ICON_SMALL as _),
        LPARAM(0),
        SMTO_ABORTIFHUNG,
        100,
        Some(&mut res as *mut usize),
    )
    .0 != 0
        && res != 0
    {
        return Some(HICON(res as _));
    }
    if SendMessageTimeoutW(
        hwnd,
        WM_GETICON,
        WPARAM(ICON_BIG as _),
        LPARAM(0),
        SMTO_ABORTIFHUNG,
        100,
        Some(&mut res as *mut usize),
    )
    .0 != 0
        && res != 0
    {
        return Some(HICON(res as _));
    }

    #[cfg(target_pointer_width = "64")]
    {
        res = GetClassLongPtrW(hwnd, GCLP_HICONSM) as usize;
        if res != 0 {
            return Some(HICON(res as _));
        }
        res = GetClassLongPtrW(hwnd, GCLP_HICON) as usize;
        if res != 0 {
            return Some(HICON(res as _));
        }
    }

    #[cfg(target_pointer_width = "32")]
    {
        use windows::Win32::UI::WindowsAndMessaging::GetClassLongW;
        res = GetClassLongW(hwnd, GCLP_HICONSM) as usize;
        if res != 0 {
            return Some(HICON(res as _));
        }
        res = GetClassLongW(hwnd, GCLP_HICON) as usize;
        if res != 0 {
            return Some(HICON(res as _));
        }
    }

    None
}

unsafe fn hicon_to_png_base64(hicon: HICON) -> Option<String> {
    let mut icon_info: ICONINFO = std::mem::zeroed();
    if GetIconInfo(hicon, &mut icon_info).is_err() {
        return None;
    }

    let hbm_color = icon_info.hbmColor;
    if hbm_color.is_invalid() {
        // Monochrome icon, not handled yet
        return None;
    }

    let hdc = CreateCompatibleDC(None);
    if hdc.is_invalid() {
        return None;
    }

    let mut bmi: BITMAPINFO = std::mem::zeroed();
    bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;

    if GetDIBits(hdc, hbm_color, 0, 0, None, &mut bmi, DIB_RGB_COLORS) == 0 {
        let _ = DeleteDC(hdc);
        return None;
    }

    let width = bmi.bmiHeader.biWidth;
    let height = bmi.bmiHeader.biHeight.abs();

    bmi.bmiHeader.biHeight = -height;
    bmi.bmiHeader.biCompression = BI_RGB.0 as u32;
    bmi.bmiHeader.biBitCount = 32;

    let mut pixels = vec![0u8; (width * height * 4) as usize];

    if GetDIBits(
        hdc,
        hbm_color,
        0,
        height as u32,
        Some(pixels.as_mut_ptr() as _),
        &mut bmi,
        DIB_RGB_COLORS,
    ) == 0
    {
        let _ = DeleteDC(hdc);
        return None;
    }

    let _ = DeleteDC(hdc);

    let mut has_alpha = false;
    for chunk in pixels.chunks_exact_mut(4) {
        let b = chunk[0];
        let r = chunk[2];
        chunk[0] = r;
        chunk[2] = b;
        if chunk[3] > 0 {
            has_alpha = true;
        }
    }

    if !has_alpha {
        for chunk in pixels.chunks_exact_mut(4) {
            chunk[3] = 255;
        }
    }

    let img_buffer =
        image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width as u32, height as u32, pixels)?;
    let mut cursor = std::io::Cursor::new(Vec::new());
    img_buffer
        .write_to(&mut cursor, image::ImageFormat::Png)
        .ok()?;

    use base64::{engine::general_purpose, Engine as _};
    Some(general_purpose::STANDARD.encode(cursor.into_inner()))
}
