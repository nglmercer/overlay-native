//! Win32-based window rendering for Windows
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::Once;
use std::time::{Duration, Instant};

use winapi::shared::windef::{HDC, HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

use super::{PlatformWindow, WindowConfig};
use crate::core::{ChatMessageElement, GiftElement, ImageElement, OverlayElement};

static REGISTER_CLASS: Once = Once::new();

#[repr(C)]
pub struct WindowData {
    pub progress: f64,
    pub username: String,
    pub message: String,
}

pub struct Win32Window {
    id: String,
    hwnd: HWND,
    created: Instant,
    duration: Duration,
}

impl Win32Window {
    pub fn from_chat_message(
        message: &ChatMessageElement,
        config: &WindowConfig,
    ) -> Result<Self, RenderError> {
        unsafe {
            let class_name = wide_string("OverlayWindow");
            let hinstance = GetModuleHandleW(null_mut());

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

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                class_name.as_ptr(),
                wide_string("Overlay").as_ptr(),
                WS_POPUP,
                config.position.0,
                config.position.1,
                config.size.0,
                config.size.1,
                null_mut(),
                null_mut(),
                hinstance,
                null_mut(),
            );

            if hwnd.is_null() {
                return Err(RenderError::WindowCreation(
                    "Failed to create window".to_string(),
                ));
            }

            SetLayeredWindowAttributes(hwnd, 0, (config.opacity * 255.0) as u8, LWA_ALPHA);

            let window_data = Box::into_raw(Box::new(WindowData {
                progress: 0.0,
                username: message.username.clone(),
                message: message.content.clone(),
            }));

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_data as isize);
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            Ok(Self {
                id: message.id.clone(),
                hwnd,
                created: Instant::now(),
                duration: config.duration,
            })
        }
    }

    pub fn from_gift(gift: &GiftElement, config: &WindowConfig) -> Result<Self, RenderError> {
        unsafe {
            let class_name = wide_string("OverlayWindow");
            let hinstance = GetModuleHandleW(null_mut());

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

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                class_name.as_ptr(),
                wide_string("Overlay Gift").as_ptr(),
                WS_POPUP,
                config.position.0,
                config.position.1,
                config.size.0,
                config.size.1,
                null_mut(),
                null_mut(),
                hinstance,
                null_mut(),
            );

            if hwnd.is_null() {
                return Err(RenderError::WindowCreation("Failed".to_string()));
            }
            SetLayeredWindowAttributes(hwnd, 0, (config.opacity * 255.0) as u8, LWA_ALPHA);

            let window_data = Box::into_raw(Box::new(WindowData {
                progress: 0.0,
                username: gift.from_user.clone(),
                message: format!("Gift: {:?}", gift.gift_type),
            }));

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_data as isize);
            ShowWindow(hwnd, SW_SHOW);
            Ok(Self {
                id: gift.id.clone(),
                hwnd,
                created: Instant::now(),
                duration: config.duration,
            })
        }
    }

    pub fn from_image(image: &ImageElement, config: &WindowConfig) -> Result<Self, RenderError> {
        unsafe {
            let class_name = wide_string("OverlayWindow");
            let hinstance = GetModuleHandleW(null_mut());
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                class_name.as_ptr(),
                wide_string("Overlay Image").as_ptr(),
                WS_POPUP,
                config.position.0,
                config.position.1,
                config.size.0,
                config.size.1,
                null_mut(),
                null_mut(),
                hinstance,
                null_mut(),
            );

            if hwnd.is_null() {
                return Err(RenderError::WindowCreation("Failed".to_string()));
            }
            SetLayeredWindowAttributes(hwnd, 0, (config.opacity * 255.0) as u8, LWA_ALPHA);

            let window_data = Box::into_raw(Box::new(WindowData {
                progress: 0.0,
                username: image.sender.clone().unwrap_or_default(),
                message: image.name.clone(),
            }));

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_data as isize);
            ShowWindow(hwnd, SW_SHOW);
            Ok(Self {
                id: image.id.clone(),
                hwnd,
                created: Instant::now(),
                duration: config.duration,
            })
        }
    }

    pub fn from_element(
        element: &OverlayElement,
        config: &WindowConfig,
    ) -> Result<Self, RenderError> {
        match element {
            OverlayElement::ChatMessage(msg) => Self::from_chat_message(msg, config),
            OverlayElement::Gift(gift) => Self::from_gift(gift, config),
            OverlayElement::Image(image) => Self::from_image(image, config),
        }
    }
}

impl PlatformWindow for Win32Window {
    fn id(&self) -> &str {
        &self.id
    }
    fn set_progress(&mut self, progress: f64) {
        unsafe {
            let data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !data_ptr.is_null() {
                (*data_ptr).progress = progress;
            }
            InvalidateRect(self.hwnd, null_mut(), 0);
        }
    }
    fn is_valid(&self) -> bool {
        !self.hwnd.is_null() && unsafe { IsWindow(self.hwnd) != 0 }
    }
    fn close(self) {
        unsafe {
            let data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !data_ptr.is_null() {
                let _ = Box::from_raw(data_ptr);
            }
            DestroyWindow(self.hwnd);
        }
    }
    fn created_at(&self) -> Instant {
        self.created
    }
}

fn wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT {
                hdc: null_mut(),
                fErase: 0,
                rcPaint: RECT {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                fRestore: 0,
                fIncUpdate: 0,
                rgbReserved: [0; 32],
            };
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            GetClientRect(hwnd, &mut rect);
            render_content(hdc, &rect, hwnd);
            EndPaint(hwnd, &ps);
            0
        }
        WM_DESTROY => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
            if !data_ptr.is_null() {
                let _ = Box::from_raw(data_ptr);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn render_content(hdc: HDC, rect: &RECT, hwnd: HWND) {
    let bg_brush = CreateSolidBrush(RGB(40, 40, 40));
    FillRect(hdc, rect, bg_brush);
    DeleteObject(bg_brush as *mut _);

    let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
    if !data_ptr.is_null() {
        let data = &*data_ptr;
        SetTextColor(hdc, RGB(255, 255, 255));
        SetBkMode(hdc, TRANSPARENT as i32);

        let user_wide = wide_string(&data.username);
        let mut user_rect = RECT {
            left: 10,
            top: 5,
            right: rect.right - 10,
            bottom: 25,
        };
        DrawTextW(
            hdc,
            user_wide.as_ptr(),
            user_wide.len() as i32 - 1,
            &mut user_rect,
            DT_LEFT | DT_SINGLELINE,
        );

        let msg_wide = wide_string(&data.message);
        let mut msg_rect = RECT {
            left: 10,
            top: 25,
            right: rect.right - 10,
            bottom: rect.bottom - 20,
        };
        DrawTextW(
            hdc,
            msg_wide.as_ptr(),
            msg_wide.len() as i32 - 1,
            &mut msg_rect,
            DT_LEFT | DT_WORDBREAK,
        );

        let progress_rect = RECT {
            left: 10,
            top: rect.bottom - 10,
            right: 10 + ((rect.right - 20) as f64 * data.progress) as i32,
            bottom: rect.bottom - 5,
        };
        let brush = CreateSolidBrush(RGB(0, 150, 255));
        FillRect(hdc, &progress_rect, brush);
        DeleteObject(brush as *mut _);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Failed to create window: {0}")]
    WindowCreation(String),
}

/// Get the primary monitor geometry
pub fn get_primary_monitor_geometry() -> (i32, i32) {
    unsafe {
        let screen_width = GetSystemMetrics(SM_CXSCREEN);
        let screen_height = GetSystemMetrics(SM_CYSCREEN);
        (screen_width, screen_height)
    }
}

/// Process Windows messages (non-blocking)
/// Returns false to indicate the message loop should exit
pub fn process_messages() -> bool {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        // PeekMessage is non-blocking - returns immediately
        if PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
            if msg.message == WM_QUIT {
                return false;
            }
            true
        } else {
            // No messages waiting
            true
        }
    }
}
