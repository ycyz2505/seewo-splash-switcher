use std::ptr::null;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GetClassNameW, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, ShowWindow, SW_HIDE,
    SW_RESTORE, SW_SHOW,
};

#[derive(Clone)]
pub struct WindowInfo {
    pub hwnd: isize,
    pub pid: u32,
    pub class_name: String,
    pub title: String,
    pub visible: bool,
    pub rect: RECT,
}

impl WindowInfo {
    pub fn width(&self) -> i32 {
        self.rect.right - self.rect.left
    }

    pub fn height(&self) -> i32 {
        self.rect.bottom - self.rect.top
    }

    pub fn area(&self) -> i64 {
        (self.width().max(0) as i64) * (self.height().max(0) as i64)
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = &mut *(lparam as *mut Vec<WindowInfo>);
    let mut pid = 0u32;
    GetWindowThreadProcessId(hwnd, &mut pid);
    let mut class_buf = [0u16; 256];
    let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
    let mut title_buf = [0u16; 512];
    let title_len = GetWindowTextW(hwnd, title_buf.as_mut_ptr(), title_buf.len() as i32);
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let _ = GetWindowRect(hwnd, &mut rect);
    out.push(WindowInfo {
        hwnd: hwnd as isize,
        pid,
        class_name: String::from_utf16_lossy(&class_buf[..class_len.max(0) as usize]),
        title: String::from_utf16_lossy(&title_buf[..title_len.max(0) as usize]),
        visible: IsWindowVisible(hwnd) != 0,
        rect,
    });
    TRUE
}

pub fn enum_windows() -> Vec<WindowInfo> {
    let mut out: Vec<WindowInfo> = Vec::new();
    unsafe {
        EnumWindows(Some(enum_proc), &mut out as *mut _ as LPARAM);
    }
    out
}

pub fn find_window_by_class(class_name: &str) -> Option<isize> {
    let class = wide(class_name);
    let hwnd = unsafe { FindWindowW(class.as_ptr(), null()) };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd as isize)
    }
}

pub fn hide_splash_windows() -> usize {
    let mut hidden = 0;
    for window in enum_windows() {
        if window.class_name == "Splash Image" || window.title == "Splash Image" {
            unsafe {
                ShowWindow(window.hwnd as HWND, SW_HIDE);
            }
            hidden += 1;
        }
    }
    hidden
}

pub fn find_main_window() -> Option<WindowInfo> {
    let windows = enum_windows();
    let mut candidates: Vec<WindowInfo> = windows
        .into_iter()
        .filter(|w| {
            let is_splash = w.class_name == "Splash Image" || w.title == "Splash Image";
            let looks_main = w.class_name.contains("HwndWrapper[EasiNote")
                || w.title.contains("希沃白板")
                || w.title.contains("EasiNote");
            !is_splash && looks_main && w.width() > 300 && w.height() > 200
        })
        .collect();
    candidates.sort_by_key(|w| std::cmp::Reverse(w.area()));
    candidates.into_iter().next()
}

pub fn reveal_window(hwnd: isize) {
    unsafe {
        ShowWindow(hwnd as HWND, SW_RESTORE);
        ShowWindow(hwnd as HWND, SW_SHOW);
        SetForegroundWindow(hwnd as HWND);
    }
}

pub fn is_seewo_running() -> bool {
    enum_windows().into_iter().any(|w| {
        w.class_name == "Splash Image"
            || w.title == "Splash Image"
            || w.class_name.contains("HwndWrapper[EasiNote")
            || w.title.contains("希沃白板")
    })
}
