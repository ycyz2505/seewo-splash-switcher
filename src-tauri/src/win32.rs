use std::ptr::null;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT, TRUE};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GetClassNameW, GetWindowRect, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow, SetWindowPos, ShowWindow,
    SW_HIDE, SW_SHOWNORMAL, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW,
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
        if (window.class_name == "Splash Image" || window.title == "Splash Image")
            && window.visible
        {
            unsafe {
                ShowWindow(window.hwnd as HWND, SW_HIDE);
            }
            hidden += 1;
        }
    }
    hidden
}

fn is_helper_title(title: &str) -> bool {
    let title = title.trim().to_lowercase();
    title.is_empty()
        || title.contains("hidden window")
        || title.contains("mediacontextnotificationwindow")
        || title.contains("systemresourcenotifywindow")
        || title.contains("broadcasteventwindow")
        || title.contains("gdi+ window")
        || title.contains("default ime")
        || title.contains("msctfime ui")
        || title.contains("chrome_systemmessagewindow")
        || title.contains("splash image")
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
            let title_ok = !is_helper_title(&w.title)
                || (w.title.trim().is_empty()
                    && (w.visible || w.class_name.contains("HwndWrapper[EasiNote")));
            !is_splash && looks_main && title_ok && w.width() > 300 && w.height() > 200
        })
        .collect();
    let score = |w: &WindowInfo| -> i32 {
        let mut score = 0;
        if w.title.contains("希沃白板") {
            score += 2;
        }
        if w.class_name.contains("HwndWrapper[EasiNote") {
            score += 1;
        }
        if w.visible {
            score += 4;
        }
        score
    };
    candidates.sort_by(|a, b| {
        score(b)
            .cmp(&score(a))
            .then_with(|| b.area().cmp(&a.area()))
    });
    candidates.into_iter().next()
}

pub fn move_window_offscreen(hwnd: isize) {
    unsafe {
        SetWindowPos(
            hwnd as HWND,
            std::ptr::null_mut(),
            -32000,
            -32000,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        );
    }
}

pub fn restore_window(hwnd: isize, x: i32, y: i32, width: i32, height: i32) {
    // 之前可能被挪到屏幕外；恢复时避免继续留在 -32000。
    let (x, y) = if x <= -10000 || y <= -10000 { (100, 80) } else { (x, y) };
    unsafe {
        SetWindowPos(
            hwnd as HWND,
            std::ptr::null_mut(),
            x,
            y,
            width,
            height,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        ShowWindow(hwnd as HWND, SW_SHOWNORMAL);
        SetForegroundWindow(hwnd as HWND);
    }
}

pub fn reveal_window(hwnd: isize) {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    unsafe {
        let _ = GetWindowRect(hwnd as HWND, &mut rect);
    }
    let width = (rect.right - rect.left).max(800);
    let height = (rect.bottom - rect.top).max(600);
    let offscreen = rect.left <= -10000
        || rect.top <= -10000
        || rect.right <= 0
        || rect.bottom <= 0;
    unsafe {
        if offscreen {
            SetWindowPos(
                hwnd as HWND,
                std::ptr::null_mut(),
                100,
                80,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        } else {
            SetWindowPos(
                hwnd as HWND,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
            );
        }
        ShowWindow(hwnd as HWND, SW_SHOWNORMAL);
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
