use image::{DynamicImage, GenericImageView};
use std::cmp::min;
use std::ffi::c_void;
use std::io;
use std::path::Path;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    HOT_KEY_MODIFIERS, RegisterHotKey, UnregisterHotKey, VK_F1,
};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::LibraryLoader::GetModuleHandleW,
        UI::{HiDpi::*, WindowsAndMessaging::*},
    },
    core::*,
};


fn load_image(file_name: &str) -> Result<DynamicImage> {
    let img_path = Path::new(file_name);
    if img_path.exists() {
        if let Ok(img) = image::open(img_path) {
            return Ok(img);
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "图片解码失败").into());
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "Image not found").into())
}

fn main() -> Result<()> {
    let img = load_image("default.png")?;

    unsafe {
        // 设置DPI感知
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2).expect("识别DPI失败");

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

        if RegisterClassW(&wnd_class) == 0 {
            let error = GetLastError();
            println!("注册窗口类失败: {:?}", error);
            return Err(Error::from_win32());
        }

        // 获取屏幕尺寸
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);

        let (window_width, window_height) = img.dimensions();
        let (window_width, window_height) = (
            min(window_width as _, screen_width),
            min(window_height as _, screen_height),
        );
        let center_x = (screen_width - window_width) / 2;
        let center_y = (screen_height - window_height) / 2;
        
        // 创建窗口 - 使用正确的样式
        let hwnd = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_APPWINDOW,
            class_name,
            w!("Crosshair Overlay"),
            WS_POPUP | WS_VISIBLE,
            center_x,
            center_y,
            screen_width,
            screen_height,
            None,
            None,
            Some(HINSTANCE::from(instance)),
            None,
        )
        .unwrap();

        if hwnd.is_invalid() {
            let error = GetLastError();
            println!("窗口创建失败: {:?}", error);
            return Err(Error::from_win32());
        }

        println!(
            "窗口创建成功: {:?} {}x{}",
            hwnd.0, window_width, window_height
        );

        // 调试：绘制红色背景验证窗口可见性
        let hdc = GetDC(Option::from(hwnd));
        if hdc.is_invalid() {
            println!("获取设备上下文失败");
        } else {
            let red_brush = CreateSolidBrush(COLORREF(0xFFFF0000));
            let rect = RECT {
                left: 0,
                top: 0,
                right: screen_width,
                bottom: screen_height,
            };
            FillRect(hdc, &rect, red_brush);
            ReleaseDC(Option::from(hwnd), hdc);
            DeleteObject(HGDIOBJ::from(red_brush));
            println!("已绘制红色背景");
        }

        // 显示窗口
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
        // 计算初始窗口位置
        let initial_rect = RECT {
            left: center_x,
            top: center_y,
            right: center_x + window_width,
            bottom: center_y + window_height,
        };
        // 加载默认图片
        if let Err(e) = load_and_display_image(hwnd, img, &initial_rect) {
            println!("加载图片失败: {:?}", e);
        }

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
) -> LRESULT { unsafe {
    match message {
        WM_CREATE => {
            // 注册热键
            for i in 0..12 {
                let hotkey_id = 0x7000 + i;
                if RegisterHotKey(
                    Option::from(hwnd),
                    hotkey_id,
                    HOT_KEY_MODIFIERS(0),
                    VK_F1.0 as u32 + i as u32,
                )
                .is_ok()
                {
                    println!("注册热键 F{} 成功", i + 1);
                } else {
                    println!("注册热键 F{} 失败", i + 1);
                }
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            // 注销所有热键
            for i in 0..12 {
                UnregisterHotKey(Option::from(hwnd), 0x7000 + i);
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        WM_HOTKEY => {
            let hotkey_id = wparam.0 as i32;
            let function_key = hotkey_id - 0x7000;
            println!("切换到F{}: ID={}, ", function_key + 1, hotkey_id);
            
            if (0..12).contains(&function_key) {
                let file_name = format!("F{}.png", function_key + 1);
                if let Ok(img) = load_image(&file_name) {
                    // 调整窗口大小
                    let screen_width = GetSystemMetrics(SM_CXSCREEN);
                    let screen_height = GetSystemMetrics(SM_CYSCREEN);
                    let (window_width, window_height) = img.dimensions();
                    let (window_width, window_height) = (
                        min(window_width as _, screen_width),
                        min(window_height as _, screen_height),
                    );
                    let center_x = (screen_width - window_width) / 2;
                    let center_y = (screen_height - window_height) / 2;
                    println!("{center_x}:{center_y} {window_width}:{window_height}");
                    match SetWindowPos(
                        hwnd,
                        Option::from(HWND_TOPMOST),
                        center_x,
                        center_y,
                        window_width,
                        window_height,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    ) {
                        Ok(_) => {
                            // 重新加载图片
                            // 强制立即更新窗口位置
                            let mut new_rect = RECT {
                                left: center_x,
                                top: center_y,
                                right: center_x + window_width,
                                bottom: center_y + window_height,
                            };
                            if let Err(e) = load_and_display_image(hwnd, img, &new_rect) {
                                println!("热键切换图片失败: {:?}", e);
                            }
                        }
                        Err(e) => {
                            println!("<UNK>: {:?}", e);
                        }
                    }
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}}

// 修改 load_and_display_image 函数
fn load_and_display_image(hwnd: HWND, img: DynamicImage, window_rect: &RECT) -> Result<()> {
    unsafe {
        // println!("加载图片: {}", file_name);

        // 获取屏幕尺寸
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);

        // 获取窗口当前尺寸
        // let mut window_rect = RECT::default();
        // GetWindowRect(hwnd, &mut window_rect);
        let window_width = window_rect.right - window_rect.left;
        let window_height = window_rect.bottom - window_rect.top;
        // let center_x = screen_width / 2;
        // let center_y = screen_height / 2;
        // println!("屏幕尺寸: {}x{}", screen_width, screen_height);

        // 创建内存DC
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        ReleaseDC(None, hdc_screen);

        if hdc_mem.is_invalid() {
            println!("创建内存DC失败");
            return Err(Error::from_win32());
        }

        // 创建32位ARGB位图
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: window_width,
                biHeight: -window_height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD::default()],
        };

        let mut bits_ptr: *mut c_void = std::ptr::null_mut();
        let hbitmap = CreateDIBSection(
            Some(hdc_mem),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits_ptr as *mut _ as *mut *mut c_void,
            None,
            0,
        )
        .unwrap();

        if hbitmap.is_invalid() {
            let error = GetLastError();
            println!("CreateDIBSection失败: {:?}", error);
            return Err(Error::from_win32());
        }
        println!("位图创建成功");

        // 选定位图到内存DC
        let _old_bitmap = SelectObject(hdc_mem, HGDIOBJ::from(hbitmap));

        // 初始化位图为全透明
        if !bits_ptr.is_null() {
            let size = (window_width * window_height * 4) as usize;
            std::ptr::write_bytes(bits_ptr, 0, size);
        }
        println!("位图初始化为透明");

        // 加载PNG图片

        let (img_width, img_height) = img.dimensions();
        println!("图片尺寸: {}x{}", img_width, img_height);

        // 计算居中位置（不缩放）
        let x = (window_width - img_width as i32) / 2;
        let y = (window_height - img_height as i32) / 2;

        // 裁剪超出屏幕的部分
        let src_x = if x < 0 { -x } else { 0 };
        let src_y = if y < 0 { -y } else { 0 };
        let draw_width = img_width.min(window_width as u32) - src_x as u32;
        let draw_height = img_height.min(window_height as u32) - src_y as u32;
        let dst_x = x.max(0);
        let dst_y = y.max(0);

        println!(
            "绘制位置: ({}, {}) 尺寸: {}x{}",
            dst_x, dst_y, draw_width, draw_height
        );

        // 创建图片位图
        if let Ok(img_hbitmap) = create_bitmap_from_image(img) {
            // 创建临时内存DC用于图片
            let hdc_img = CreateCompatibleDC(Some(hdc_mem));
            let _old_img_bitmap = SelectObject(hdc_img, HGDIOBJ::from(img_hbitmap));

            // 使用AlphaBlend绘制图片
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let _ = AlphaBlend(
                hdc_mem, // 目标DC
                dst_x,
                dst_y,
                draw_width as i32,
                draw_height as i32,
                hdc_img, // 源DC
                src_x,
                src_y,
                draw_width as i32,
                draw_height as i32,
                blend,
            );

            // 清理临时资源
            SelectObject(hdc_img, _old_img_bitmap);
            DeleteDC(hdc_img);
            DeleteObject(HGDIOBJ::from(img_hbitmap));
            println!("图片绘制完成");
        }

        // 更新分层窗口
        // 更新分层窗口 - 使用提供的窗口位置
        let mut pt_dst = POINT {
            x: window_rect.left,
            y: window_rect.top
        };
        let mut sz = SIZE {
            cx: window_width,
            cy: window_height,
        };
        let mut pt_src = POINT { x: 0, y: 0 };

        let mut blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as _,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as _,
        };

        println!("调用UpdateLayeredWindow...");
        let result = UpdateLayeredWindow(
            hwnd,
            None,
            Some(&mut pt_dst),
            Some(&mut sz),
            Some(hdc_mem),
            Some(&mut pt_src),
            COLORREF(0),
            Some(&mut blend),
            ULW_ALPHA,
        );

        match result {
            Ok(()) => println!("UpdateLayeredWindow成功!"),
            Err(e) => println!("UpdateLayeredWindow失败: {:?}", e),
        }

        // 清理资源 - 注意这些资源在窗口更新后不再需要
        SelectObject(hdc_mem, _old_bitmap); // 恢复原始位图
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
        assert!(!rgba.is_empty());

        // 创建BITMAPINFO
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // 负值表示从上到下的位图
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            bmiColors: [RGBQUAD::default()],
        };

        // 创建DIB
        let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let hdc = GetDC(None);
        let hbitmap = CreateDIBSection(
            Option::from(hdc),
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
            let dest_slice =
                std::slice::from_raw_parts_mut(bits_ptr as *mut u8, (width * height * 4) as usize);
            dest_slice.copy_from_slice(pixels);
        }

        Ok(hbitmap)
    }
}
