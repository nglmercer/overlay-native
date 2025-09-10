use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::Once;
use tokio::time::Instant;
use winapi::shared::windef::{HWND, RECT};
use winapi::um::winuser::*;
use winapi::um::wingdi::*;
use winapi::shared::windef::HDC;
use winapi::um::libloaderapi::GetModuleHandleW;
use twitch_irc::message::Emote;

static REGISTER_CLASS: Once = Once::new();

// Window data structure to store with each window
#[repr(C)]
struct WindowData {
    progress: f64,
    created_time: u64,
}

#[derive(Clone)]
pub struct WindowsWindow {
    pub hwnd: HWND,
    pub created: Instant,
    pub progress: f64,
    pub username: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WindowsWindow {
    pub fn new(user: &str, message: &str, _emotes: &[Emote], pos: (i32, i32)) -> Self {
        unsafe {
            let class_name = wide_string("OverlayWindow");
            let window_name = wide_string(&format!("{}: {}", user, message));
            
            let hinstance = GetModuleHandleW(null_mut());
            
            // Register window class only once
            REGISTER_CLASS.call_once(|| {
                let wc = WNDCLASSW {
                    style: CS_HREDRAW | CS_VREDRAW,
                    lpfnWndProc: Some(window_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: hinstance,
                    hIcon: null_mut(),
                    hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                    hbrBackground: CreateSolidBrush(RGB(30, 30, 30)) as *mut _,
                    lpszMenuName: null_mut(),
                    lpszClassName: class_name.as_ptr(),
                };
                
                RegisterClassW(&wc);
            });
            
            // Calculate window size based on text length
            let text_width = (user.len() + message.len()).max(20) * 8 + 20;
            let window_width = text_width.min(400).max(200);
            
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                class_name.as_ptr(),
                window_name.as_ptr(),
                WS_POPUP,
                pos.0,
                pos.1,
                window_width as i32,
                80,
                null_mut(),
                null_mut(),
                hinstance,
                null_mut(),
            );
            
            // Make window semi-transparent
            SetLayeredWindowAttributes(hwnd, 0, 220, LWA_ALPHA);
            
            // Store window data
            let window_data = Box::new(WindowData {
                progress: 0.0,
                created_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            });
            
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(window_data) as isize);
            
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);
            
            WindowsWindow {
                hwnd,
                created: Instant::now(),
                progress: 0.0,
                username: user.to_string(),
                message: message.to_string(),
            }
        }
    }
    
    pub fn close(&self) {
        unsafe {
            // Clean up window data before destroying
            let window_data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() {
                let _ = Box::from_raw(window_data_ptr);
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            }
            DestroyWindow(self.hwnd);
        }
    }
    
    pub fn set_progress(&mut self, progress: f64) {
        // Only update if progress changed significantly to reduce flickering
        let progress_diff = (self.progress - progress).abs();
        if progress_diff < 0.01 {
            return; // Skip update if change is less than 1%
        }
        
        self.progress = progress;
        unsafe {
            // Update the stored window data
            let window_data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() {
                (*window_data_ptr).progress = progress;
            }
            
            // Only invalidate the progress bar area to reduce flickering
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(self.hwnd, &mut rect);
            let progress_rect = RECT {
                left: 10,
                top: rect.bottom - 15,
                right: rect.right - 10,
                bottom: rect.bottom - 5,
            };
            InvalidateRect(self.hwnd, &progress_rect, 0); // Don't erase background
        }
    }
}

fn wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

// Separate rendering function to reduce flickering with double buffering
unsafe fn render_window_content(hdc: HDC, rect: &RECT, hwnd: HWND) {
    // Background
    let bg_brush = CreateSolidBrush(RGB(40, 40, 40));
    FillRect(hdc, rect, bg_brush);
    DeleteObject(bg_brush as *mut _);
    
    // Set text properties
    SetTextColor(hdc, RGB(255, 255, 255));
    SetBkMode(hdc, TRANSPARENT as i32);
    
    // Get window title to extract username and message
    let mut title_buffer = [0u16; 512];
    let title_len = GetWindowTextW(hwnd, title_buffer.as_mut_ptr(), 512);
    if title_len > 0 {
        let title = String::from_utf16_lossy(&title_buffer[..title_len as usize]);
        if let Some(colon_pos) = title.find(": ") {
            let username = &title[..colon_pos];
            let message = &title[colon_pos + 2..];
            
            // Draw username (bold)
            let username_wide = wide_string(username);
            let mut username_rect = RECT {
                left: 10,
                top: 5,
                right: rect.right - 10,
                bottom: 25,
            };
            
            // Create bold font for username
            let bold_font = CreateFontW(
                14, 0, 0, 0, FW_BOLD, 0, 0, 0,
                DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
                CLIP_DEFAULT_PRECIS, DEFAULT_QUALITY,
                DEFAULT_PITCH | FF_DONTCARE,
                wide_string("Arial").as_ptr(),
            );
            let old_font = SelectObject(hdc, bold_font as *mut _);
            
            DrawTextW(
                hdc,
                username_wide.as_ptr(),
                username_wide.len() as i32 - 1,
                &mut username_rect,
                DT_LEFT | DT_TOP | DT_SINGLELINE,
            );
            
            // Restore original font and delete bold font
            SelectObject(hdc, old_font);
            DeleteObject(bold_font as *mut _);
            
            // Draw message
            let message_wide = wide_string(message);
            let mut message_rect = RECT {
                left: 10,
                top: 25,
                right: rect.right - 10,
                bottom: rect.bottom - 25,
            };
            
            DrawTextW(
                hdc,
                message_wide.as_ptr(),
                message_wide.len() as i32 - 1,
                &mut message_rect,
                DT_LEFT | DT_TOP | DT_WORDBREAK,
            );
        }
    }
    
    // Draw progress bar
    let progress_bg_rect = RECT {
        left: 10,
        top: rect.bottom - 15,
        right: rect.right - 10,
        bottom: rect.bottom - 5,
    };
    
    // Progress background
    let progress_bg_brush = CreateSolidBrush(RGB(60, 60, 60));
    FillRect(hdc, &progress_bg_rect, progress_bg_brush);
    DeleteObject(progress_bg_brush as *mut _);
    
    // Get progress from stored window data
    let window_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
    let progress = if !window_data_ptr.is_null() {
        (*window_data_ptr).progress
    } else {
        0.0
    };
    
    let progress_width = ((progress_bg_rect.right - progress_bg_rect.left) as f64 * progress) as i32;
    
    if progress_width > 0 {
        let progress_rect = RECT {
            left: progress_bg_rect.left,
            top: progress_bg_rect.top,
            right: progress_bg_rect.left + progress_width,
            bottom: progress_bg_rect.bottom,
        };
        
        let progress_brush = CreateSolidBrush(RGB(0, 150, 255));
        FillRect(hdc, &progress_rect, progress_brush);
        DeleteObject(progress_brush as *mut _);
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> isize {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT {
                hdc: null_mut(),
                fErase: 0,
                rcPaint: RECT { left: 0, top: 0, right: 0, bottom: 0 },
                fRestore: 0,
                fIncUpdate: 0,
                rgbReserved: [0; 32],
            };
            
            let hdc = BeginPaint(hwnd, &mut ps);
            
            // Get window dimensions
            let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
            GetClientRect(hwnd, &mut rect);
            
            // Create memory DC for double buffering to reduce flickering
            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bitmap = CreateCompatibleBitmap(hdc, rect.right - rect.left, rect.bottom - rect.top);
            let old_bitmap = SelectObject(mem_dc, mem_bitmap as *mut _);
            
            // Render to memory DC instead of directly to screen
            render_window_content(mem_dc, &rect, hwnd);
            
            // Copy from memory DC to screen DC (this reduces flickering)
            BitBlt(
                hdc,
                0, 0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                mem_dc,
                0, 0,
                SRCCOPY,
            );
            
            // Clean up memory DC resources
            SelectObject(mem_dc, old_bitmap);
            DeleteObject(mem_bitmap as *mut _);
            DeleteDC(mem_dc);
            
            EndPaint(hwnd, &ps);
            0
        }
        WM_DESTROY => {
            // Clean up window data to prevent memory leak
            let window_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() {
                let _ = Box::from_raw(window_data_ptr);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub fn get_monitor_geometry() -> WindowGeometry {
    unsafe {
        let desktop = GetDesktopWindow();
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        GetWindowRect(desktop, &mut rect);
        
        WindowGeometry {
            x: rect.left,
            y: rect.top,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
        }
    }
}

pub fn process_messages() -> bool {
    unsafe {
        let mut msg = MSG {
            hwnd: null_mut(),
            message: 0,
            wParam: 0,
            lParam: 0,
            time: 0,
            pt: winapi::shared::windef::POINT { x: 0, y: 0 },
        };
        
        while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
            if msg.message == WM_QUIT {
                return false;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        true
    }
}