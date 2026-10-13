//! Win32 の薄い殻。コンソール無しの exe から、親のコンソールへの出力・メッセージ窓・入力窓を出す。

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{CreateFontIndirectW, COLOR_WINDOW};
use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
use windows_sys::Win32::System::Diagnostics::Debug::MessageBeep;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, GetDpiForSystem, SetProcessDpiAwarenessContext,
    SystemParametersInfoForDpi, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
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

/// exe 本体のアイコン（build.rs の set_icon、名前 ID は winresource の既定で 1）。入力窓に出す。
const APP_ICON_ID: u16 = 1;
/// 失敗のメッセージ窓に出すアイコン（build.rs で埋めた、色違いの pon）の名前 ID。
const ERROR_ICON_ID: u16 = 2;

pub fn message_box(text: &str) {
    let text = wide(text);
    let title = wide("pon");
    unsafe {
        // MB_USERICON は警告音を鳴らさないので、自分で鳴らす。
        MessageBeep(MB_ICONERROR);
        let params = MSGBOXPARAMSW {
            cbSize: std::mem::size_of::<MSGBOXPARAMSW>() as u32,
            // 持ち主は付けない。持ち主を付けると、窓を閉じるまで持ち主（そのとき前面にある Zed や Chrome）が
            // 操作不能になり、窓が後ろに隠れたり pon が落ちたりすると、その窓が触れないまま残る。
            hwndOwner: null_mut(),
            hInstance: GetModuleHandleW(std::ptr::null()),
            lpszText: text.as_ptr(),
            lpszCaption: title.as_ptr(),
            // 標準の「×」の代わりに色違いの pon。MB_USERICON は警告音も鳴らさない。
            // 持ち主が無いぶん、最前面に出して見失わないようにする。
            dwStyle: MB_OK | MB_USERICON | MB_SETFOREGROUND | MB_TOPMOST,
            lpszIcon: ERROR_ICON_ID as usize as *const u16,
            dwContextHelpId: 0,
            lpfnMsgBoxCallback: None,
            dwLanguageId: 0,
        };
        MessageBoxIndirectW(&params);
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
        let app_icon = LoadIconW(hinst, APP_ICON_ID as usize as *const u16);
        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinst,
            hIcon: app_icon,
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_WINDOW + 1) as usize as _,
            lpszMenuName: std::ptr::null(),
            lpszClassName: class.as_ptr(),
        };
        RegisterClassW(&wc);

        // DPI 対応を宣言しているので、寸法は 96dpi の値を画面の倍率で伸ばす。
        let dpi = GetDpiForSystem();
        let s = |v: i32| (v * dpi as i32 + 48) / 96;
        let (pad, edit_h) = (s(10), s(26));
        let (client_w, client_h) = (s(400), edit_h + pad * 2);
        // タイトルバーに pon のアイコンを出すので、ツールウィンドウにはしない（所有窓なのでタスクバーには出ない）。
        let style = WS_POPUP | WS_CAPTION | WS_SYSMENU;
        let ex_style = WS_EX_TOPMOST;
        let mut frame = RECT { left: 0, top: 0, right: client_w, bottom: client_h };
        AdjustWindowRectExForDpi(&mut frame, style, 0, ex_style, dpi);
        let (w, h) = (frame.right - frame.left, frame.bottom - frame.top);

        let owner = GetForegroundWindow();
        let (mut x, mut y) = (CW_USEDEFAULT, CW_USEDEFAULT);
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        if !owner.is_null() && GetWindowRect(owner, &mut r) != 0 {
            x = r.left + (r.right - r.left - w) / 2;
            y = r.top + (r.bottom - r.top - h) / 2;
        }
        let title_w = wide(title);
        let hwnd = CreateWindowExW(
            ex_style,
            class.as_ptr(),
            title_w.as_ptr(),
            style,
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
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, app_icon as isize);
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, app_icon as isize);
        let edit_class = wide("EDIT");
        let edit = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            edit_class.as_ptr(),
            std::ptr::null(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            pad,
            pad,
            client_w - pad * 2,
            edit_h,
            hwnd,
            null_mut(),
            hinst,
            std::ptr::null(),
        );
        // メッセージ用のシステムフォントを、この DPI の大きさで作る。
        let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        if SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, ncm.cbSize, &mut ncm as *mut _ as _, 0, dpi) != 0 {
            let font = CreateFontIndirectW(&ncm.lfMessageFont);
            if !font.is_null() {
                SendMessageW(edit, WM_SETFONT, font as usize, 1);
            }
        }
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

/// 前面の窓が Zed（Zed.exe）なら、そのタイトル。違えば None。
pub fn foreground_zed_title() -> Option<String> {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
        CloseHandle(process);
        if ok == 0 {
            return None;
        }
        let exe = String::from_utf16_lossy(&buf[..len as usize]);
        if !exe.to_ascii_lowercase().ends_with("\\zed.exe") {
            return None;
        }
        Some(window_text(hwnd))
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
