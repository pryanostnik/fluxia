//! Оформление системного окна на Windows: цвет заголовка под палитру,
//! тёмная/светлая рамка и скруглённые углы (Windows 11, DWM).

use crate::theme::Palette;

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    type Hwnd = isize;

    #[link(name = "user32")]
    extern "system" {
        fn EnumWindows(cb: extern "system" fn(Hwnd, isize) -> i32, lparam: isize) -> i32;
        fn GetWindowThreadProcessId(hwnd: Hwnd, pid: *mut u32) -> u32;
        fn IsWindowVisible(hwnd: Hwnd) -> i32;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
    }

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(hwnd: Hwnd, attr: u32, value: *const c_void, size: u32) -> i32;
    }

    extern "system" fn find_cb(hwnd: Hwnd, lparam: isize) -> i32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        // своё окно, видимое и с заголовком
        if pid == std::process::id()
            && unsafe { IsWindowVisible(hwnd) } != 0
            && unsafe { GetWindowTextLengthW(hwnd) } > 0
        {
            unsafe { *(lparam as *mut Hwnd) = hwnd };
            return 0;
        }
        1
    }

    pub fn hwnd() -> Option<Hwnd> {
        let mut found: Hwnd = 0;
        unsafe { EnumWindows(find_cb, &mut found as *mut Hwnd as isize) };
        (found != 0).then_some(found)
    }

    pub fn set_u32(hwnd: Hwnd, attr: u32, value: u32) -> i32 {
        let v = value as i32;
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                attr,
                &v as *const i32 as *const c_void,
                std::mem::size_of::<i32>() as u32,
            )
        }
    }
}

/// Текущий час по локальному времени (для автоцвета «по времени суток»).
/// Без chrono: один вызов Win32.
#[cfg(windows)]
pub fn local_hour() -> u32 {
    #[repr(C)]
    #[allow(non_snake_case)]
    struct SYSTEMTIME {
        wYear: u16,
        wMonth: u16,
        wDayOfWeek: u16,
        wDay: u16,
        wHour: u16,
        wMinute: u16,
        wSecond: u16,
        wMilliseconds: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GetLocalTime(st: *mut SYSTEMTIME);
    }

    let mut st = SYSTEMTIME {
        wYear: 0,
        wMonth: 0,
        wDayOfWeek: 0,
        wDay: 0,
        wHour: 0,
        wMinute: 0,
        wSecond: 0,
        wMilliseconds: 0,
    };
    unsafe { GetLocalTime(&mut st) };
    st.wHour as u32
}

#[cfg(not(windows))]
pub fn local_hour() -> u32 {
    12
}

/// COLORREF: 0x00BBGGRR
fn colorref(c: eframe::egui::Color32) -> u32 {
    (c.r() as u32) | ((c.g() as u32) << 8) | ((c.b() as u32) << 16)
}

/// Перекрашивает рамку окна под палитру.
/// Возвращает false, если окно ещё не найдено — тогда надо попробовать позже.
pub fn style_window(palette: &Palette) -> bool {
    #[cfg(windows)]
    {
        const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
        const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
        const DWMWA_BORDER_COLOR: u32 = 34;
        const DWMWA_CAPTION_COLOR: u32 = 35;
        const DWMWA_TEXT_COLOR: u32 = 36;
        const DWMWCP_ROUND: u32 = 2;

        let Some(hwnd) = imp::hwnd() else { return false };

        imp::set_u32(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, u32::from(palette.is_dark()));
        imp::set_u32(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
        // заголовок и рамка — в цветах интерфейса
        let rc = [
            imp::set_u32(hwnd, DWMWA_CAPTION_COLOR, colorref(palette.panel)),
            imp::set_u32(hwnd, DWMWA_BORDER_COLOR, colorref(palette.border)),
            imp::set_u32(hwnd, DWMWA_TEXT_COLOR, colorref(palette.text)),
        ];

        #[cfg(debug_assertions)]
        eprintln!("fluxia-dwm: hwnd={hwnd} rc={rc:?}");

        let _ = rc;
        true
    }

    #[cfg(not(windows))]
    {
        let _ = palette;
        false
    }
}
