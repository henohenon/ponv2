//! Win32 の薄い殻。コンソール無しの exe から、親のコンソールへの出力・メッセージ窓・入力窓を出す。

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, COLOR_WINDOW, DEFAULT_GUI_FONT};
use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_ESCAPE, VK_RETURN};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// ターミナルから呼ばれていれば、そのコンソールに標準出力を繋ぐ。無ければ何もしない。
pub fn attach_parent_console() {
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

pub fn enable_dpi_awareness() {
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

pub fn message_box(text: &str) {
    let text = wide(text);
    let title = wide("pon");
    unsafe {
        MessageBoxW(
            GetForegroundWindow(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_CLOSE => {
                DestroyWindow(hwnd);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// 前面の窓（Zed）に所有させた小さな入力窓。Enter で確定、Esc か閉じるでキャンセル。
pub fn input_box(title: &str) -> Option<String> {
    unsafe {
        let hinst = GetModuleHandleW(std::ptr::null());
        let class = wide("pon-input");
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as usize as _,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc);

        let owner = GetForegroundWindow();
        let (w, h) = (420, 70);
        let (mut x, mut y) = (CW_USEDEFAULT, CW_USEDEFAULT);
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if !owner.is_null() && GetWindowRect(owner, &mut r) != 0 {
            x = r.left + (r.right - r.left - w) / 2;
            y = r.top + (r.bottom - r.top - h) / 2;
        }
        let title_w = wide(title);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class.as_ptr(),
            title_w.as_ptr(),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            x,
            y,
            w,
            h,
            owner,
            null_mut(),
            hinst,
            std::ptr::null(),
        );
        if hwnd.is_null() {
            return None;
        }
        let edit_class = wide("EDIT");
        let edit = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            10,
            10,
            w - 20,
            26,
            hwnd,
            null_mut(),
            hinst,
            std::ptr::null(),
        );
        SendMessageW(edit, WM_SETFONT, GetStockObject(DEFAULT_GUI_FONT) as usize, 1);
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetFocus(edit);

        let mut result: Option<String> = None;
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if msg.message == WM_KEYDOWN {
                let key = msg.wParam as u16;
                if key == VK_RETURN {
                    result = Some(window_text(edit));
                    DestroyWindow(hwnd);
                    continue;
                }
                if key == VK_ESCAPE {
                    DestroyWindow(hwnd);
                    continue;
                }
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        result.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
    }
}

unsafe fn window_text(hwnd: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(hwnd) as usize;
        let mut buf = vec![0u16; len + 1];
        let n = GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32) as usize;
        String::from_utf16_lossy(&buf[..n])
    }
}
