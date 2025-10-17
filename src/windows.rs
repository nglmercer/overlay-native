use std::collections::HashMap;
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;

use std::ptr::null_mut;
use std::sync::{Arc, Mutex, Once};
use tokio::time::Instant;
use twitch_irc::message::Emote;
use winapi::shared::windef::{HBITMAP, HDC, HWND, RECT};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::wingdi::*;
use winapi::um::wingdi::{BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, RGBQUAD};
use winapi::um::winuser::*;

static REGISTER_CLASS: Once = Once::new();

// Window data structure to store with each window
#[repr(C)]
pub struct WindowData {
    pub progress: f64,
    pub created_time: u64,
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

#[derive(Clone)]
pub struct WindowsWindow {
    pub hwnd: HWND,
    pub created: Instant,
    pub progress: f64,
    pub username: String,
    pub message: String,
    pub emotes: Vec<twitch_irc::message::Emote>,
}

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

#[derive(Clone, Copy, Debug)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WindowsWindow {
    /// Preload emote images (simplified version that creates placeholders)
    fn preload_emotes(emotes: &[Emote]) -> Vec<EmoteImage> {
        let mut emote_images = Vec::new();

        for (index, emote) in emotes.iter().enumerate() {
            // For now, create placeholder emote images
            // In a production version, you would download and cache actual images
            emote_images.push(EmoteImage {
                id: emote.id.clone(),
                image_data: None, // Will be downloaded asynchronously later
                width: 32,
                height: 32,
                x: 10 + (index as i32 * 36), // Position emotes horizontally
                y: 25,
            });
        }

        emote_images
    }

    /// Get emote URL based on source
    fn get_emote_url(emote: &Emote) -> String {
        format!(
            "https://static-cdn.jtvnw.net/emoticons/v2/{}/default/dark/1.0",
            emote.id
        )
    }

    /// Schedule async download of emote images (works from sync context)
    fn schedule_emote_downloads(emotes: Vec<Emote>) {
        // Use a blocking task to avoid the Tokio runtime issue
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let cache = get_emote_cache();

                for emote in emotes {
                    // Check if already cached
                    if let Ok(cache_guard) = cache.lock() {
                        if cache_guard.contains_key(&emote.id) {
                            continue;
                        }
                    }

                    // Download emote with timeout
                    let url = WindowsWindow::get_emote_url(&emote);
                    if let Ok(image_data) = WindowsWindow::download_emote_async(&url).await {
                        // Store in cache
                        if let Ok(mut cache_guard) = cache.lock() {
                            cache_guard.insert(emote.id.clone(), image_data);
                        }
                    }
                }
            });
        });
    }

    /// Download emote image asynchronously with timeout
    async fn download_emote_async(url: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()?;

        let response = client.get(url).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
    pub fn new(user: &str, message: &str, emotes: &[Emote], pos: (i32, i32)) -> Self {
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

            // Create emote images data structure
            let emote_images = Box::new(Self::preload_emotes(emotes));

            // Schedule async download of emote images in background
            Self::schedule_emote_downloads(emotes.to_vec());

            // Store window data
            let window_data = Box::new(WindowData {
                progress: 0.0,
                created_time: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                emote_images: Box::into_raw(emote_images),
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
                emotes: emotes.to_vec(),
            }
        }
    }

    pub fn close(&self) {
        unsafe {
            // Clean up window data before destroying
            let window_data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() {
                let window_data = Box::from_raw(window_data_ptr);
                // Clean up emote images
                if !window_data.emote_images.is_null() {
                    let _ = Box::from_raw(window_data.emote_images);
                }
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            }
            DestroyWindow(self.hwnd);
        }
    }

    pub fn set_progress(&mut self, progress: f64) {
        // Only update if progress changed significantly to reduce flickering
        let progress_diff = (self.progress - progress).abs();
        if progress_diff < 0.02 {
            return; // Skip update if change is less than 2%
        }

        self.progress = progress;
        unsafe {
            // Update the stored window data
            let window_data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() {
                (*window_data_ptr).progress = progress;
            }

            // Only invalidate the progress bar area to reduce flickering
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

            // Restore original font and delete bold font
            SelectObject(hdc, old_font);
            DeleteObject(bold_font as *mut _);

            // Draw emotes first (if any)
            let window_data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
            if !window_data_ptr.is_null() && !(*window_data_ptr).emote_images.is_null() {
                let emote_images = &*(*window_data_ptr).emote_images;
                let cache = get_emote_cache();

                for emote_image in emote_images {
                    // Try to get image from cache
                    let image_data = if let Ok(cache_guard) = cache.lock() {
                        cache_guard.get(&emote_image.id).cloned()
                    } else {
                        None
                    };

                    render_emote_image(
                        hdc,
                        image_data.as_deref().unwrap_or(&[]),
                        emote_image.x,
                        emote_image.y,
                        emote_image.width,
                        emote_image.height,
                    );
                }
            }

            // Draw message (adjust position if there are emotes)
            let message_y =
                if !window_data_ptr.is_null() && !(*window_data_ptr).emote_images.is_null() {
                    let emote_images = &*(*window_data_ptr).emote_images;
                    if !emote_images.is_empty() {
                        60 // Space for emotes
                    } else {
                        25
                    }
                } else {
                    25
                };

            let message_wide = wide_string(message);
            let mut message_rect = RECT {
                left: 10,
                top: message_y,
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

/// Render an emote image using Windows GDI with real image decoding

unsafe fn render_emote_image(hdc: HDC, image_data: &[u8], x: i32, y: i32, width: u32, height: u32) {
    // If no image data, render placeholder
    if image_data.is_empty() {
        render_emote_placeholder(hdc, x, y, width, height);
        return;
    }

    // Try to decode the image
    if let Ok(image) = image::load_from_memory(image_data) {
        let rgba_image = image.to_rgba8();
        let (img_width, img_height) = rgba_image.dimensions();

        // Scale to target size if needed
        let (target_width, target_height) = (width as u32, height as u32);
        let scaled_image = if img_width != target_width || img_height != target_height {
            image::imageops::resize(
                &rgba_image,
                target_width,
                target_height,
                image::imageops::FilterType::Lanczos3,
            )
        } else {
            rgba_image
        };

        // Create bitmap from image data
        let bitmap_info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: target_width as i32,
                biHeight: -(target_height as i32), // Negative for top-down bitmap
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [RGBQUAD {
                rgbBlue: 0,
                rgbGreen: 0,
                rgbRed: 0,
                rgbReserved: 0,
            }],
        };

        let mut bitmap: HBITMAP = null_mut();
        let mut bitmap_bits: *mut winapi::ctypes::c_void = null_mut();

        bitmap = CreateDIBSection(
            hdc,
            &bitmap_info,
            DIB_RGB_COLORS,
            &mut bitmap_bits,
            null_mut(),
            0,
        );

        if !bitmap.is_null() && !bitmap_bits.is_null() {
            // Copy image data to bitmap
            let pixels = std::slice::from_raw_parts_mut(
                bitmap_bits as *mut u8,
                (target_width * target_height * 4) as usize,
            );

            // Convert RGBA to BGRA format for Windows
            for (i, pixel) in scaled_image.chunks_exact(4).enumerate() {
                if i * 4 < pixels.len() {
                    pixels[i * 4] = pixel[2]; // B
                    pixels[i * 4 + 1] = pixel[1]; // G
                    pixels[i * 4 + 2] = pixel[0]; // R
                    pixels[i * 4 + 3] = pixel[3]; // A
                }
            }

            // Create memory DC and select bitmap
            let mem_dc = CreateCompatibleDC(hdc);
            let old_bitmap = SelectObject(mem_dc, bitmap as *mut _);

            // Draw the bitmap
            let success = BitBlt(
                hdc,
                x,
                y,
                target_width as i32,
                target_height as i32,
                mem_dc,
                0,
                0,
                SRCCOPY,
            );

            // Cleanup
            SelectObject(mem_dc, old_bitmap);
            DeleteDC(mem_dc);
            DeleteObject(bitmap as _);

            if success == 0 {
                // Fallback to rectangle if BitBlt failed
                render_emote_placeholder(hdc, x, y, width, height);
            }
        } else {
            // Fallback to rectangle if bitmap creation failed
            render_emote_placeholder(hdc, x, y, width, height);
        }
    } else {
        // Fallback to rectangle if image decoding failed
        render_emote_placeholder(hdc, x, y, width, height);
    }
}

/// Fallback function to render a placeholder rectangle when image rendering fails
unsafe fn render_emote_placeholder(hdc: HDC, x: i32, y: i32, width: u32, height: u32) {
    let emote_rect = RECT {
        left: x,
        top: y,
        right: x + width as i32,
        bottom: y + height as i32,
    };

    // Draw a purple rectangle as emote placeholder
    let emote_brush = CreateSolidBrush(RGB(128, 0, 128));
    FillRect(hdc, &emote_rect, emote_brush);
    DeleteObject(emote_brush as _);

    // Draw border
    let border_brush = CreateSolidBrush(RGB(255, 255, 255));
    FrameRect(hdc, &emote_rect, border_brush);
    DeleteObject(border_brush as _);
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

            // Get window dimensions
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            GetClientRect(hwnd, &mut rect);

            // Create memory DC for double buffering to reduce flickering
            let mem_dc = CreateCompatibleDC(hdc);
            let mem_bitmap =
                CreateCompatibleBitmap(hdc, rect.right - rect.left, rect.bottom - rect.top);
            let old_bitmap = SelectObject(mem_dc, mem_bitmap as *mut _);

            // Render to memory DC instead of directly to screen
            render_window_content(mem_dc, &rect, hwnd);

            // Copy from memory DC to screen DC (this reduces flickering)
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
                let window_data = Box::from_raw(window_data_ptr);
                // Clean up emote images
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

pub fn get_monitor_geometry() -> WindowGeometry {
    unsafe {
        let desktop = GetDesktopWindow();
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
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
