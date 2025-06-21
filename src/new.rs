use std::io;
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

pub fn main() -> Result<()> {
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
		let screen_width = GetSystemMetrics(SM_CXSCREEN);
		let screen_height = GetSystemMetrics(SM_CYSCREEN);
		let hwnd = CreateWindowExW(
			// WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE|WS_EX_APPWINDOW,
			// WS_EX_TOPMOST | WS_EX_NOACTIVATE|WS_EX_LAYERED|WS_EX_APPWINDOW,
			// WS_EX_LAYERED,
			WINDOW_EX_STYLE(0)|WS_EX_TRANSPARENT|WS_EX_LAYERED,
			class_name,
			w!("Crosshair Overlay"),
			WS_POPUP|WS_VISIBLE,
			0,
			0,
			screen_width,
			screen_height,
			None,
			None,
			Some(HINSTANCE::from(instance)),
			None,
		)?;
		if hwnd.is_invalid() {
			let error = GetLastError();
			println!("{:?}", error);
		}else { 
			println!("window created successfully");
		}

		// 注册F1-F12热键
		// for key in 0x70..=0x7B {
		// 	RegisterHotKey(Option::from(hwnd), key as i32, MOD_NOREPEAT, key)?;
		// }

		// 加载默认图片
		load_and_display_image(hwnd, "default.png").unwrap();
		
		let hdc = GetDC(Some(hwnd));
		let red_bursh = CreateSolidBrush(COLORREF(0xFFFF0000));
		let mut rect = RECT {
			left: 0,
			top: 0,
			right: screen_width,
			bottom: screen_height,
		};
		FillRect(hdc, &rect, red_bursh);
		ReleaseDC(Option::from(hwnd), hdc);
		DeleteObject(HGDIOBJ::from(red_bursh));
		
		// 显示窗口
		ShowWindow(hwnd, SW_SHOW);
		UpdateWindow(hwnd);
		// 强制前台显示（调试用）
		// SetForegroundWindow(hwnd);
		BringWindowToTop(hwnd).unwrap();
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
			println!("[MSG] WM_DESTROY - 窗口销毁");
			PostQuitMessage(0);
			LRESULT(0)
		}
		WM_CREATE => {
			println!("[MSG] WM_CREATE - 窗口已创建");
			LRESULT(0)
		}
		WM_PAINT => {
			println!("[MSG] WM_PAINT - 重绘请求");
			let mut ps = PAINTSTRUCT::default();
			let hdc = BeginPaint(hwnd, &mut ps);

			// 绘制绿色边框验证绘图功能
			let green_pen = CreatePen(PS_SOLID, 3, COLORREF(0xFF0000));
			let old_pen = SelectObject(hdc, HGDIOBJ::from(green_pen));
			Rectangle(hdc, 50, 50, 350, 350);
			SelectObject(hdc, old_pen);
			DeleteObject(HGDIOBJ::from(green_pen));

			EndPaint(hwnd, &ps);
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
		let center_x = screen_width / 2;
		let center_y = screen_height / 2;

		// 创建内存DC
		let hdc_screen = GetDC(None);
		let hdc_mem = CreateCompatibleDC(Option::from(hdc_screen));
		ReleaseDC(None, hdc_screen);

		// 创建32位ARGB位图
		let mut bmi = BITMAPINFO {
			bmiHeader: BITMAPINFOHEADER {
				biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
				biWidth: screen_width,
				biHeight: -screen_height, // 负值表示从上到下的位图
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

		let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
		let hbitmap = CreateDIBSection(
			Option::from(hdc_mem),
			&bmi,
			DIB_RGB_COLORS,
			&mut bits_ptr as *mut _,
			None,
			0,
		)?;

		if hbitmap.is_invalid() {
			return Err(Error::from_win32());
		}

		let _old_bitmap = SelectObject(hdc_mem, HGDIOBJ::from(hbitmap));

		// 初始化位图为全透明
		if !bits_ptr.is_null() {
			let size = (screen_width * screen_height * 4) as usize;
			std::ptr::write_bytes(bits_ptr as *mut u8, 0, size);
		}

		let red_brush = CreateSolidBrush(COLORREF(0xFF0000));
		let red_pen = CreatePen(PS_SOLID, 2, COLORREF(0xFF0000));

		let old_pen = SelectObject(hdc_mem, HGDIOBJ::from(red_pen));
		let old_brush = SelectObject(hdc_mem, HGDIOBJ::from(red_brush));

		// 绘制水平线
		MoveToEx(hdc_mem, center_x - 20, center_y, None);
		LineTo(hdc_mem, center_x + 20, center_y);

		// 绘制垂直线
		MoveToEx(hdc_mem, center_x, center_y - 20, None);
		LineTo(hdc_mem, center_x, center_y + 20);

		// 绘制中心点
		Ellipse(hdc_mem, center_x - 5, center_y - 5, center_x + 5, center_y + 5);

		SelectObject(hdc_mem, old_pen);
		SelectObject(hdc_mem, old_brush);
		DeleteObject(HGDIOBJ::from(red_pen));
		DeleteObject(HGDIOBJ::from(red_brush));
		
		// 加载PNG图片
		let img_path = Path::new(file_name);
		if img_path.exists() {
			if let Ok(img) = image::open(img_path) {
				let (img_width, img_height) = img.dimensions();

				// 计算居中位置（不缩放）
				let x = (screen_width - img_width as i32) / 2;
				let y = (screen_height - img_height as i32) / 2;

				// 裁剪超出屏幕的部分
				let draw_width = img_width.min(screen_width as u32);
				let draw_height = img_height.min(screen_height as u32);

				// 创建临时位图
				if let Ok(img_hbitmap) = create_bitmap_from_image(img) {
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
							BlendOp: AC_SRC_OVER as u8,
							BlendFlags: 0,
							SourceConstantAlpha: 255,
							AlphaFormat: AC_SRC_ALPHA as u8,
						},
					);

					DeleteObject(HGDIOBJ::from(img_hbitmap));
				}
			} else {
				eprintln!("Failed to decode image: {}", file_name);
			}
		} else {
			eprintln!("Image not found: {}", file_name);
			return Err(io::Error::new(io::ErrorKind::NotFound, "Image not found").into());
		}

		// 更新分层窗口
		let mut pt_dst = POINT { x: 0, y: 0 };
		let mut sz = SIZE {
			cx: screen_width,
			cy: screen_height,
		};
		let mut pt_src = POINT { x: 0, y: 0 };
		let mut blend = BLENDFUNCTION {
			BlendOp: AC_SRC_OVER as u8,
			BlendFlags: 0,
			SourceConstantAlpha: 255,
			AlphaFormat: AC_SRC_ALPHA as u8,
		};

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
		).unwrap();

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
				biCompression: BI_RGB.0,
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
			let dest_slice = std::slice::from_raw_parts_mut(
				bits_ptr as *mut u8,
				(width * height * 4) as usize,
			);
			dest_slice.copy_from_slice(pixels);
		}

		Ok(hbitmap)
	}
}
