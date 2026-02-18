//! Win32-based window rendering for Windows
//!
//! This module provides the Win32 API implementation for rendering overlay windows
//! on Windows systems. It uses native Windows GDI for rendering.

use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex, Once};
use std::time::{Duration, Instant};

use winapi::shared::windef::{HBITMAP, HDC, HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::*;
use winapi::um::winuser::*;

use super::{PlatformWindow, WindowConfig};
use crate::core::{ChatMessageElement, EmoteElement, GiftElement, OverlayElement};

static REGISTER_CLASS: Once = Once::new();

// Global cache for emote images
static EMOTE_CACHE: Once = Once::new();
static mut EMOTE_IMAGES: Option<Arc<Mutex<HashMap<String, Vec<u8>>>>> = None;

fn get_emote_cache() -> Arc<Mutex<HashMap<String, Vec<u8>>>> {
    unsafe {
        EMOTE_CACHE.call_once(|| {
            EMOTE_IMAGES = Some(Arc::new(Mutex::new(HashMap::new())));
        });
        EMOTE_IMAGES.as_ref().unwrap().clone()
    }
}

// Window data structure to store with each window
#[repr(C)]
pub struct WindowData {
    pub progress: f64,
    pub created_time: u64,
    pub username: String,
    pub message: String,
    pub emote_images: *mut Vec<EmoteImage>,
}

#[derive(Clone)]
pub struct EmoteImage {
    pub id: String,
    pub image_data: Option<Vec<u8>>,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

/// Win32-based overlay window
pub struct Win32Window {
    /// Window ID
    id: String,

    /// Window handle
    hwnd: HWND,

    /// Creation time
    created: Instant,

    /// Display duration
    duration: Duration,
}

impl Win32Window {
    /// Create a new Win32 overlay window from a chat message
    pub fn from_chat_message(
        message: &ChatMessageElement,
        config: &WindowConfig,
    ) -> Result<Self, RenderError> {
        unsafe {
            let class_name = wide_string("OverlayWindow");
            let window_title = format!("{}: {}", message.username, message.content);
            let window_name = wide_string(&window_title);

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

            // Calculate window size
            let window_width = config.size.0;
            let window_height = config.size.1;

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                class_name.as_ptr(),
                window_name.as_ptr(),
                WS_POPUP,
                config.position.0,
                config.position.1,
                window_width,
                window_height,
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

            // Make window semi-transparent
            let alpha = (config.opacity * 255.0) as u8;
            SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);

            // Create emote images data
            let emote_images = Box::new(Self::preload_emotes(&message.emotes));

            // Schedule async download of emote images
            Self::schedule_emote_downloads(message.emotes.clone());

            // Store window data
            let window_data = Box::new(WindowData {
                progress: 0.0,
                created_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                username: message.username.clone(),
                message: message.content.clone(),
                emote_images: Box::into_raw(emote_images),
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(window_data) as isize);

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

    /// Create a new Win32 overlay window from a gift event
    pub fn from_gift(gift: &GiftElement, config: &WindowConfig) -> Result<Self, RenderError> {
        unsafe {
            let class_name = wide_string("OverlayWindow");
            let gift_text = format!("Gift from {}", gift.from_user);
            let window_name = wide_string(&gift_text);

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
                window_name.as_ptr(),
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

            let alpha = (config.opacity * 255.0) as u8;
            SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);

            let window_data = Box::new(WindowData {
                progress: 0.0,
                created_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                username: gift.from_user.clone(),
                message: format!("{:?}", gift.gift_type),
                emote_images: null_mut(),
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(window_data) as isize);

            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            Ok(Self {
                id: gift.id.clone(),
                hwnd,
                created: Instant::now(),
                duration: config.duration,
            })
        }
    }

    /// Create a new Win32 overlay window from an emote event
    pub fn from_emote(emote: &EmoteElement, config: &WindowConfig) -> Result<Self, RenderError> {
        unsafe {
            let class_name = wide_string("OverlayWindow");
            let window_name = wide_string(&format!("Emote: {}", emote.name));

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
                window_name.as_ptr(),
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

            let alpha = (config.opacity * 255.0) as u8;
            SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA);

            let window_data = Box::new(WindowData {
                progress: 0.0,
                created_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                username: emote.sender.clone().unwrap_or_default(),
                message: emote.name.clone(),
                emote_images: null_mut(),
            });

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(window_data) as isize);

            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            Ok(Self {
                id: emote.id.clone(),
                hwnd,
                created: Instant::now(),
                duration: config.duration,
            })
        }
    }

    /// Create from any overlay element
    pub fn from_element(
        element: &OverlayElement,
        config: &WindowConfig,
    ) -> Result<Self, RenderError> {
        match element {
            OverlayElement::ChatMessage(msg) => Self::from_chat_message(msg, config),
            OverlayElement::Gift(gift) => Self::from_gift(gift, config),
            OverlayElement::Emote(emote) => Self::from_emote(emote, config),
        }
    }

    /// Preload emote images
    fn preload_emotes(emotes: &[crate::core::Emote]) -> Vec<EmoteImage> {
        emotes
            .iter()
            .enumerate()
            .map(|(index, emote)| EmoteImage {
                id: emote.id.clone(),
                image_data: None,
                width: 32,
                height: 32,
                x: 10 + (index as i32 * 36),
                y: 25,
            })
            .collect()
    }

    /// Schedule async download of emote images
    fn schedule_emote_downloads(emotes: Vec<crate::core::Emote>) {
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = get_emote_cache();

                for emote in emotes {
                    if let Some(url) = &emote.url {
                        if let Ok(cache_guard) = cache.lock() {
                            if cache_guard.contains_key(&emote.id) {
                                continue;
                            }
                        }

                        if let Ok(image_data) = Self::download_emote_async(url).await {
                            if let Ok(mut cache_guard) = cache.lock() {
                                cache_guard.insert(emote.id.clone(), image_data);
                            }
                        }
                    }
                }
            });
        });
    }

    /// Download emote image asynchronously
    async fn download_emote_async(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()?;

        let response = client.get(url).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

impl PlatformWindow for Win32Window {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_progress(&mut self, progress: f64) {
        unsafe {
            let window_data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() {
                (*window_data_ptr).progress = progress;
            }

            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            GetClientRect(self.hwnd, &mut rect);

            let progress_rect = RECT {
                left: 10,
                top: rect.bottom - 15,
                right: rect.right - 10,
                bottom: rect.bottom - 5,
            };
            InvalidateRect(self.hwnd, &progress_rect, 0);
        }
    }

    fn is_valid(&self) -> bool {
        !self.hwnd.is_null() && unsafe { IsWindow(self.hwnd) != 0 }
    }

    fn close(self) {
        unsafe {
            let window_data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() {
                let window_data = Box::from_raw(window_data_ptr);
                if !window_data.emote_images.is_null() {
                    let _ = Box::from_raw(window_data.emote_images);
                }
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            }
            DestroyWindow(self.hwnd);
        }
    }

    fn created_at(&self) -> Instant {
        self.created
    }
}

/// Convert string to wide string for Windows API
fn wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

/// Window procedure for handling Windows messages
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

            // Double buffering
            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bitmap =
                CreateCompatibleBitmap(hdc, rect.right - rect.left, rect.bottom - rect.top);
            let old_bitmap = SelectObject(mem_dc, mem_bitmap as *mut _);

            render_window_content(mem_dc, &rect, hwnd);

            BitBlt(
                hdc,
                0,
                0,
                rect.right - rect.left,
                rect.bottom - rect.top,
                mem_dc,
                0,
                0,
                SRCCOPY,
            );

            SelectObject(mem_dc, old_bitmap);
            DeleteObject(mem_bitmap as *mut _);
            DeleteDC(mem_dc);

            EndPaint(hwnd, &ps);
            0
        }
        WM_DESTROY => {
            let window_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() {
                let window_data = Box::from_raw(window_data_ptr);
                if !window_data.emote_images.is_null() {
                    let _ = Box::from_raw(window_data.emote_images);
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Render window content
unsafe fn render_window_content(hdc: HDC, rect: &RECT, hwnd: HWND) {
    use std::collections::HashMap;

    // Background
    let bg_brush = CreateSolidBrush(RGB(40, 40, 40));
    FillRect(hdc, rect, bg_brush);
    DeleteObject(bg_brush as *mut _);

    // Set text properties
    SetTextColor(hdc, RGB(255, 255, 255));
    SetBkMode(hdc, TRANSPARENT as i32);

    // Get window data
    let window_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
    if !window_data_ptr.is_null() {
        let data = &*window_data_ptr;

        // Draw username (bold)
        let username_wide = wide_string(&data.username);
        let mut username_rect = RECT {
            left: 10,
            top: 5,
            right: rect.right - 10,
            bottom: 25,
        };

        let bold_font = CreateFontW(
            14,
            0,
            0,
            0,
            FW_BOLD,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            DEFAULT_QUALITY,
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

        SelectObject(hdc, old_font);
        DeleteObject(bold_font as *mut _);

        // Draw message
        let message_wide = wide_string(&data.message);
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

    // Draw progress bar
    let progress_bg_rect = RECT {
        left: 10,
        top: rect.bottom - 15,
        right: rect.right - 10,
        bottom: rect.bottom - 5,
    };

    let progress_bg_brush = CreateSolidBrush(RGB(60, 60, 60));
    FillRect(hdc, &progress_bg_rect, progress_bg_brush);
    DeleteObject(progress_bg_brush as *mut _);

    let progress = if !window_data_ptr.is_null() {
        (*window_data_ptr).progress
    } else {
        0.0
    };

    let progress_width =
        ((progress_bg_rect.right - progress_bg_rect.left) as f64 * progress) as i32;

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

/// Errors during rendering
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Failed to create window: {0}")]
    WindowCreation(String),

    #[error("Failed to load emote: {0}")]
    EmoteLoad(String),

    #[error("Windows API error: {0}")]
    Win32(String),
}

/// Get the primary monitor geometry
pub fn get_primary_monitor_geometry() -> (i32, i32, i32, i32) {
    unsafe {
        let desktop = GetDesktopWindow();
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(desktop, &mut rect);
        (
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        )
    }
}

/// Process Windows messages (non-blocking)
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
