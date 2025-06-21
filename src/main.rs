mod new;

use new::main;
/*
use std::path::Path;
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{
            Gdi::*,
            Imaging::{IWICBitmapDecoder, IWICBitmapFrameDecode, IWICImagingFactory},
        },
        System::{
            LibraryLoader::GetModuleHandleW,
            // SystemServices::{MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, MOD_WIN},
            Threading::GetCurrentProcess,
        },
        UI::{
            Input::KeyboardAndMouse::RegisterHotKey,
            WindowsAndMessaging::*,
            HiDpi::*,
            Input::KeyboardAndMouse::*,
        },
    },
};
use image::GenericImageView;

const WM_HOTKEY: u32 = 0x0312;

fn main() -> Result<()> {
    unsafe {
        // 设置DPI感知
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        // 注册窗口类
        let instance = GetModuleHandleW(None)?;
        let class_name = w!("CrosshairWindow");

        let wnd_class = WNDCLASSW {
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hInstance: HINSTANCE::from(instance),
            lpszClassName: class_name,
            lpfnWndProc: Some(wndproc),
            ..Default::default()
        };

        RegisterClassW(&wnd_class);

        // 创建透明窗口
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            class_name,
            w!("Crosshair Overlay"),
            WS_POPUP,
            0,
            0,
            GetSystemMetrics(SM_CXSCREEN),
            GetSystemMetrics(SM_CYSCREEN),
            None,
            None,
            Some(HINSTANCE::from(instance)),
            None,
        )?;

        // 设置窗口透明度
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA)?;

        // 注册F1-F12热键
        for key in 0x70..=0x7B {
            RegisterHotKey(Option::from(hwnd), key as i32, MOD_NOREPEAT, key)?;
        }

        // 加载默认图片
        load_and_display_image(hwnd, "default.png")?;

        // 显示窗口
        ShowWindow(hwnd, SW_SHOW);

        // 消息循环
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        Ok(())
    }
}

unsafe extern "system" fn wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_HOTKEY => {
            // 处理热键事件
            let key = wparam.0 as u32;
            if (0x70..=0x7B).contains(&key) {
                let file_name = format!("F{}.png", key - 0x70 + 1);
                if let Err(e) = load_and_display_image(hwnd, &file_name) {
                    eprintln!("Failed to load image: {:?}", e);
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

fn load_and_display_image(hwnd: HWND, file_name: &str) -> Result<()> {
    unsafe {
        // 获取屏幕尺寸
        let screen_width = GetSystemMetrics(SM_CXSCREEN) as i32;
        let screen_height = GetSystemMetrics(SM_CYSCREEN) as i32;

        // 创建内存DC
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        ReleaseDC(None, hdc_screen);

        // 创建屏幕大小的位图
        let hbitmap = CreateCompatibleBitmap(hdc_screen, screen_width, screen_height);
        let _old_bitmap = SelectObject(hdc_mem, HGDIOBJ::from(hbitmap));

        // 清除背景为透明
        let mut blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as _,
            BlendFlags: 0,
            SourceConstantAlpha: 0,
            AlphaFormat: AC_SRC_ALPHA as _,
        };
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: screen_width,
            bottom: screen_height,
        };
        FillRect(hdc_mem, &rect, GetStockObject(BLACK_BRUSH));

        // 加载PNG图片
        if let Ok(img) = image::open(file_name) {
            let (img_width, img_height) = img.dimensions();

            // 计算居中位置（不缩放）
            let x = (screen_width - img_width as i32) / 2;
            let y = (screen_height - img_height as i32) / 2;

            // 裁剪超出屏幕的部分
            let draw_width = img_width.min(screen_width as u32);
            let draw_height = img_height.min(screen_height as u32);

            // 创建临时位图
            let img_hbitmap = create_bitmap_from_image(img)?;
            let _old_img_bitmap = SelectObject(hdc_mem, HGDIOBJ::from(img_hbitmap));

            // 绘制到内存DC
            AlphaBlend(
                hdc_mem,
                x.max(0),
                y.max(0),
                draw_width as i32,
                draw_height as i32,
                hdc_mem,
                0,
                0,
                draw_width as i32,
                draw_height as i32,
                BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as _,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as _,
                },
            );

            // DeleteObject(img_hbitmap);

        } else {
            eprintln!("Image not found: {}", file_name);
        }

        // 更新分层窗口
        let mut pt_dst = POINT { x: 0, y: 0 };
        let mut sz = SIZE {
            cx: screen_width,
            cy: screen_height,
        };
        let mut pt_src = POINT { x: 0, y: 0 };

        UpdateLayeredWindow(
            hwnd,
            None,
            Some(&mut pt_dst),
            Some(&mut sz),
            Some(hdc_mem),
            Some(&mut pt_src),
            COLORREF(0),
            Some(&mut blend),
            ULW_ALPHA,
        )?;

        // 清理资源
        DeleteObject(HGDIOBJ::from(hbitmap));
        DeleteDC(hdc_mem);

        Ok(())
    }
}

fn create_bitmap_from_image(img: image::DynamicImage) -> Result<HBITMAP> {
    unsafe {
        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8();
        let pixels = rgba.as_raw();

        // 创建BITMAPINFO
        let mut bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // 负值表示从上到下的位图
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.into(),
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        // 创建DIB
        let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hdc = GetDC(None);
        let hbitmap = CreateDIBSection(
            Some(hdc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr as *mut _,
            None,
            0,
        )?;
        ReleaseDC(None, hdc);

        if hbitmap.is_invalid() {
            return Err(Error::from_win32());
        }

        // 复制像素数据
        if !bits_ptr.is_null() {
            let dest_slice = std::slice::from_raw_parts_mut(
                bits_ptr as *mut u8,
                (width * height * 4) as usize,
            );
            dest_slice.copy_from_slice(pixels);
        }

        Ok(hbitmap)
    }
}
*/