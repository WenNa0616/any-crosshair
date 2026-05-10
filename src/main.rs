#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use image::{DynamicImage, GenericImageView};
use std::cmp::min;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Registry::*;
use windows::Win32::Storage::FileSystem::*;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::*;

const WM_TRAYICON: u32 = WM_USER + 1;
const ID_TRAYICON: u32 = 1;
const IDM_SHOW_HIDE: u32 = 1001;
const IDM_AUTOSTART: u32 = 1002;
const IDM_EXIT: u32 = 1003;
const IDM_HOTKEY_F9: u32 = 1010;
const IDM_HOTKEY_F10: u32 = 1011;
const IDM_HOTKEY_F11: u32 = 1012;
const IDM_HOTKEY_F12: u32 = 1013;
const IDM_HOTKEY_CF9: u32 = 1014;
const IDM_HOTKEY_CF10: u32 = 1015;
const IDM_HOTKEY_CF11: u32 = 1016;
const IDM_HOTKEY_CF12: u32 = 1017;

const TRAY_ICON_PNG: &[u8] = include_bytes!("../crosshair.png");
const DEFAULT_CROSSHAIR: &[u8] = include_bytes!("../default.png");

static WINDOW_VISIBLE: AtomicBool = AtomicBool::new(false);
static TRAY_ADDED: AtomicBool = AtomicBool::new(false);
static HOTKEY_ID: AtomicI32 = AtomicI32::new(0);
static CURRENT_HOTKEY: AtomicI32 = AtomicI32::new(0); // 0=F9, 1=F10, ..., 4=Ctrl+F9, ...

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }

fn get_exe_dir() -> std::path::PathBuf {
    std::env::current_exe().ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn get_hotkey_info(idx: i32) -> (&'static str, HOT_KEY_MODIFIERS, u32) {
    match idx {
        0 => ("F9", HOT_KEY_MODIFIERS(0), 0x78),
        1 => ("F10", HOT_KEY_MODIFIERS(0), 0x79),
        2 => ("F11", HOT_KEY_MODIFIERS(0), 0x7A),
        3 => ("F12", HOT_KEY_MODIFIERS(0), 0x7B),
        4 => ("Ctrl+F9", MOD_CONTROL, 0x78),
        5 => ("Ctrl+F10", MOD_CONTROL, 0x79),
        6 => ("Ctrl+F11", MOD_CONTROL, 0x7A),
        7 => ("Ctrl+F12", MOD_CONTROL, 0x7B),
        _ => ("F9", HOT_KEY_MODIFIERS(0), 0x78),
    }
}

fn get_autostart() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        let path = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = wide("AnyCrosshair");
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), Some(0), KEY_READ, &mut hkey).is_err() {
            return false;
        }
        let mut dtype = REG_NONE;
        let mut dsize = 0u32;
        let r = RegQueryValueExW(hkey, PCWSTR(name.as_ptr()), None, Some(&mut dtype), None, Some(&mut dsize));
        let _ = RegCloseKey(hkey);
        r.is_ok() && dsize > 0
    }
}

fn set_autostart(enable: bool) {
    unsafe {
        let mut hkey = HKEY::default();
        let path = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = wide("AnyCrosshair");
        if RegOpenKeyExW(HKEY_CURRENT_USER, PCWSTR(path.as_ptr()), Some(0), KEY_WRITE, &mut hkey).is_err() {
            return;
        }
        if enable {
            if let Ok(exe) = std::env::current_exe() {
                let v = wide(&exe.to_string_lossy());
                let _ = RegSetValueExW(hkey, PCWSTR(name.as_ptr()), Some(0), REG_SZ,
                    Some(std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * 2)));
            }
        } else {
            let _ = RegDeleteValueW(hkey, PCWSTR(name.as_ptr()));
        }
        let _ = RegCloseKey(hkey);
    }
}

fn load_hotkey_setting() -> i32 {
    let path = get_exe_dir().join(".ac_hotkey");
    std::fs::read_to_string(path).ok()
        .and_then(|s| s.trim().parse().ok())
        .filter(|&v| v >= 0 && v <= 7)
        .unwrap_or(0)
}

fn save_hotkey_setting(idx: i32) {
    let path = get_exe_dir().join(".ac_hotkey");
    let _ = std::fs::write(&path, idx.to_string());
    unsafe {
        let w = wide(path.to_str().unwrap_or_default());
        let _ = SetFileAttributesW(PCWSTR(w.as_ptr()), FILE_ATTRIBUTE_HIDDEN);
    }
}

fn create_hicon() -> HICON {
    unsafe {
        let img = image::load_from_memory(TRAY_ICON_PNG)
            .unwrap_or_else(|_| DynamicImage::new_rgba8(32, 32))
            .resize_exact(32, 32, image::imageops::FilterType::Lanczos3);
        let rgba = img.to_rgba8();
        let hdc = GetDC(None);
        let bi = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: 32, biHeight: -32, biPlanes: 1, biBitCount: 32,
            biCompression: BI_RGB.0, ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let color = CreateDIBSection(Some(hdc), &BITMAPINFO { bmiHeader: bi, ..Default::default() }, DIB_RGB_COLORS, &mut bits, None, 0).unwrap();
        ReleaseDC(None, hdc);
        if !bits.is_null() {
            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, 32 * 32 * 4);
            let px = rgba.as_raw();
            for i in (0..dst.len()).step_by(4) {
                dst[i] = px[i+2]; dst[i+1] = px[i+1]; dst[i+2] = px[i]; dst[i+3] = px[i+3];
            }
        }
        let mask = CreateBitmap(32, 32, 1, 1, None);
        let icon = CreateIconIndirect(&ICONINFO {
            fIcon: TRUE, xHotspot: 16, yHotspot: 16, hbmMask: mask, hbmColor: color
        }).unwrap();
        let _ = DeleteObject(HGDIOBJ::from(color));
        let _ = DeleteObject(HGDIOBJ::from(mask));
        icon
    }
}

fn add_tray(hwnd: HWND) {
    unsafe {
        let icon = create_hicon();
        let (name, _, _) = get_hotkey_info(CURRENT_HOTKEY.load(Ordering::Relaxed));
        let tip = wide(&format!("Any Crosshair - {}", name));

        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = 936;
        nid.hWnd = hwnd;
        nid.uID = ID_TRAYICON;
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);

        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = icon;
        nid.szTip[..min(tip.len(), 128)].copy_from_slice(&tip[..min(tip.len(), 128)]);

        if Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
            TRAY_ADDED.store(true, Ordering::Relaxed);
        }
        let _ = DestroyIcon(icon);
    }
}

fn remove_tray(hwnd: HWND) {
    unsafe {
        if !TRAY_ADDED.load(Ordering::Relaxed) { return; }
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = 936;
        nid.hWnd = hwnd;
        nid.uID = ID_TRAYICON;
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        TRAY_ADDED.store(false, Ordering::Relaxed);
    }
}

fn update_tip(hwnd: HWND) {
    unsafe {
        if !TRAY_ADDED.load(Ordering::Relaxed) { return; }
        let (name, _, _) = get_hotkey_info(CURRENT_HOTKEY.load(Ordering::Relaxed));
        let tip = wide(&format!("Any Crosshair - {}", name));
        let mut nid = NOTIFYICONDATAW::default();
        nid.cbSize = 936;
        nid.hWnd = hwnd;
        nid.uID = ID_TRAYICON;
        nid.uFlags = NIF_TIP;
        nid.szTip[..min(tip.len(), 128)].copy_from_slice(&tip[..min(tip.len(), 128)]);
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

fn register_hotkey(hwnd: HWND) {
    unsafe {
        let old = HOTKEY_ID.load(Ordering::Relaxed);
        if old != 0 { let _ = UnregisterHotKey(Some(hwnd), old); }

        let idx = CURRENT_HOTKEY.load(Ordering::Relaxed);
        let (_, mods, vk) = get_hotkey_info(idx);
        if RegisterHotKey(Some(hwnd), 0x7000, mods, vk).is_ok() {
            HOTKEY_ID.store(0x7000, Ordering::Relaxed);
        }
    }
}

fn show_menu(hwnd: HWND) {
    unsafe {
        let auto = get_autostart();
        let cur = CURRENT_HOTKEY.load(Ordering::Relaxed);
        let menu = CreatePopupMenu().unwrap();

        // 显示/隐藏
        let txt = wide(if WINDOW_VISIBLE.load(Ordering::Relaxed) { "隐藏准心" } else { "显示准心" });
        let _ = AppendMenuW(menu, MF_STRING, IDM_SHOW_HIDE as usize, PCWSTR(txt.as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

        // 热键子菜单
        let hotkey_menu = CreatePopupMenu().unwrap();
        let items = [
            (0, "F9", IDM_HOTKEY_F9), (1, "F10", IDM_HOTKEY_F10),
            (2, "F11", IDM_HOTKEY_F11), (3, "F12", IDM_HOTKEY_F12),
            (4, "Ctrl+F9", IDM_HOTKEY_CF9), (5, "Ctrl+F10", IDM_HOTKEY_CF10),
            (6, "Ctrl+F11", IDM_HOTKEY_CF11), (7, "Ctrl+F12", IDM_HOTKEY_CF12),
        ];
        for (idx, name, id) in items {
            let flags = if idx == cur { MF_STRING | MF_CHECKED } else { MF_STRING };
            let _ = AppendMenuW(hotkey_menu, flags, id as usize, PCWSTR(wide(name).as_ptr()));
        }
        let _ = AppendMenuW(menu, MF_POPUP, hotkey_menu.0 as usize, PCWSTR(wide("热键").as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

        // 开机自启
        let flags = if auto { MF_STRING | MF_CHECKED } else { MF_STRING | MF_UNCHECKED };
        let _ = AppendMenuW(menu, flags, IDM_AUTOSTART as usize, PCWSTR(wide("开机自启").as_ptr()));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);

        // 退出
        let _ = AppendMenuW(menu, MF_STRING, IDM_EXIT as usize, PCWSTR(wide("退出").as_ptr()));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, Some(0), hwnd, None);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
    }
}

fn toggle(hwnd: HWND) {
    let v = WINDOW_VISIBLE.load(Ordering::Relaxed);
    WINDOW_VISIBLE.store(!v, Ordering::Relaxed);
    unsafe { let _ = ShowWindow(hwnd, if !v { SW_SHOW } else { SW_HIDE }); }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_TRAYICON => {
                match l.0 as u32 {
                    WM_LBUTTONUP => toggle(hwnd),
                    WM_RBUTTONUP => show_menu(hwnd),
                    _ => {}
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                match w.0 as u32 {
                    IDM_SHOW_HIDE => { toggle(hwnd); }
                    IDM_AUTOSTART => { set_autostart(!get_autostart()); }
                    IDM_HOTKEY_F9 | IDM_HOTKEY_F10 | IDM_HOTKEY_F11 | IDM_HOTKEY_F12 |
                    IDM_HOTKEY_CF9 | IDM_HOTKEY_CF10 | IDM_HOTKEY_CF11 | IDM_HOTKEY_CF12 => {
                        let idx = match w.0 as u32 {
                            IDM_HOTKEY_F9 => 0, IDM_HOTKEY_F10 => 1,
                            IDM_HOTKEY_F11 => 2, IDM_HOTKEY_F12 => 3,
                            IDM_HOTKEY_CF9 => 4, IDM_HOTKEY_CF10 => 5,
                            IDM_HOTKEY_CF11 => 6, IDM_HOTKEY_CF12 => 7,
                            _ => 0,
                        };
                        CURRENT_HOTKEY.store(idx, Ordering::Relaxed);
                        save_hotkey_setting(idx);
                        register_hotkey(hwnd);
                        update_tip(hwnd);
                    }
                    IDM_EXIT => {
                        remove_tray(hwnd);
                        let hk = HOTKEY_ID.load(Ordering::Relaxed);
                        if hk != 0 { let _ = UnregisterHotKey(Some(hwnd), hk); }
                        PostQuitMessage(0);
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_HOTKEY => {
                if w.0 as i32 == HOTKEY_ID.load(Ordering::Relaxed) { toggle(hwnd); }
                LRESULT(0)
            }
            WM_DESTROY => {
                remove_tray(hwnd);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, w, l),
        }
    }
}

fn main() -> Result<()> {
    CURRENT_HOTKEY.store(load_hotkey_setting(), Ordering::Relaxed);

    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
        let inst = GetModuleHandleW(None)?;

        let cls = w!("AnyCrosshair");
        RegisterClassW(&WNDCLASSW {
            hInstance: HINSTANCE::from(inst),
            lpszClassName: cls,
            lpfnWndProc: Some(wndproc),
            ..Default::default()
        });

        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
            cls, w!("AnyCrosshair"), WS_POPUP,
            0, 0, 0, 0,
            None, None, Some(HINSTANCE::from(inst)), None,
        )?;

        // 加载准心图片（优先外部文件，否则用内嵌的）
        let img = std::fs::read(get_exe_dir().join("default.png"))
            .ok()
            .and_then(|d| image::load_from_memory(&d).ok())
            .or_else(|| image::load_from_memory(DEFAULT_CROSSHAIR).ok())
            .unwrap_or_else(|| DynamicImage::new_rgba8(32, 32));

        let sw = GetSystemMetrics(SM_CXSCREEN);
        let sh = GetSystemMetrics(SM_CYSCREEN);
        let (iw, ih) = img.dimensions();
        let ww = min(iw as i32, sw);
        let wh = min(ih as i32, sh);
        let cx = (sw - ww) / 2;
        let cy = (sh - wh) / 2;

        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), cx, cy, ww, wh, SWP_NOACTIVATE);
        let rect = RECT { left: cx, top: cy, right: cx + ww, bottom: cy + wh };
        let _ = display_image(hwnd, img, &rect);

        add_tray(hwnd);
        register_hotkey(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Ok(())
    }
}

fn display_image(hwnd: HWND, img: DynamicImage, rect: &RECT) -> Result<()> {
    unsafe {
        let w = rect.right - rect.left;
        let h = rect.bottom - rect.top;
        let hdc = GetDC(None);
        let mem = CreateCompatibleDC(Some(hdc));
        ReleaseDC(None, hdc);

        let bi = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32,
            biCompression: BI_RGB.0, ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bmp = CreateDIBSection(Some(mem), &BITMAPINFO { bmiHeader: bi, ..Default::default() }, DIB_RGB_COLORS, &mut bits, None, 0)?;
        let old = SelectObject(mem, HGDIOBJ::from(bmp));
        if !bits.is_null() { std::ptr::write_bytes(bits, 0, (w * h * 4) as usize); }

        let (iw, ih) = img.dimensions();
        let x = (w - iw as i32) / 2;
        let y = (h - ih as i32) / 2;
        let sx = if x < 0 { -x } else { 0 };
        let sy = if y < 0 { -y } else { 0 };
        let dw = iw.min(w as u32) - sx as u32;
        let dh = ih.min(h as u32) - sy as u32;

        if let Ok(ibmp) = make_bitmap(img) {
            let img_dc = CreateCompatibleDC(Some(mem));
            let oimg = SelectObject(img_dc, HGDIOBJ::from(ibmp));
            let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8, ..Default::default() };
            let _ = AlphaBlend(mem, x.max(0), y.max(0), dw as i32, dh as i32, img_dc, sx, sy, dw as i32, dh as i32, blend);
            SelectObject(img_dc, oimg);
            let _ = DeleteDC(img_dc);
            let _ = DeleteObject(HGDIOBJ::from(ibmp));
        }

        let pt = POINT { x: rect.left, y: rect.top };
        let sz = SIZE { cx: w, cy: h };
        let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as _, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as _, ..Default::default() };
        let _ = UpdateLayeredWindow(hwnd, None, Some(&pt), Some(&sz), Some(mem), Some(&POINT { x: 0, y: 0 }), COLORREF(0), Some(&blend), ULW_ALPHA);
        SelectObject(mem, old);
        let _ = DeleteObject(HGDIOBJ::from(bmp));
        let _ = DeleteDC(mem);
        Ok(())
    }
}

fn make_bitmap(img: DynamicImage) -> Result<HBITMAP> {
    unsafe {
        let (w, h) = img.dimensions();
        let rgba = img.to_rgba8();
        let px = rgba.as_raw();
        let bi = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32, biHeight: -(h as i32), biPlanes: 1, biBitCount: 32,
            biCompression: BI_RGB.0, ..Default::default()
        };
        let mut ptr: *mut c_void = std::ptr::null_mut();
        let hdc = GetDC(None);
        let bmp = CreateDIBSection(Some(hdc), &BITMAPINFO { bmiHeader: bi, ..Default::default() }, DIB_RGB_COLORS, &mut ptr, None, 0)?;
        ReleaseDC(None, hdc);
        if !ptr.is_null() {
            let dst = std::slice::from_raw_parts_mut(ptr as *mut u8, (w * h * 4) as usize);
            for i in (0..dst.len()).step_by(4) {
                dst[i] = px[i+2]; dst[i+1] = px[i+1]; dst[i+2] = px[i]; dst[i+3] = px[i+3];
            }
        }
        Ok(bmp)
    }
}