//! Win32-based window rendering for Windows
//!
//! This module provides the Win32 implementation for rendering overlay windows
//! on Windows systems. It supports Alert components with text, images, badges,
//! and custom styling.

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
use crate::core::message::{Alert, AlertComponent, Layout};

static REGISTER_CLASS: Once = Once::new();

/// Data stored in window user data for rendering
pub struct WindowData {
    /// Alert being rendered
    pub alert: Alert,
    
    /// Current progress (0.0 - 1.0)
    pub progress: f64,
    
    /// Y offset for scrolling/animation
    pub y_offset: i32,
}

pub struct Win32Window {
    id: String,
    hwnd: HWND,
    created: Instant,
    duration: Duration,
}

impl Win32Window {
    /// Create a window from an Alert (the main entry point for Win32 rendering)
    pub fn from_alert(alert: &Alert, config: &WindowConfig) -> Result<Self, RenderError> {
        unsafe {
            let class_name = wide_string("OverlayWindow");
            let hinstance = GetModuleHandleW(null_mut());

            REGISTER_CLASS.call_once(|| {
                let wc = WNDCLASSW {
                    style: 0, // Removed CS_HREDRAW | CS_VREDRAW to prevent flickering
                    lpfnWndProc: Some(window_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: hinstance,
                    hIcon: null_mut(),
                    hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                    hbrBackground: null_mut(), // We'll handle painting ourselves
                    lpszMenuName: null_mut(),
                    lpszClassName: class_name.as_ptr(),
                };
                RegisterClassW(&wc);
            });

            // Calculate window position based on config
            let (x, y) = config.position;
            let (width, height) = config.size;

            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT,
                class_name.as_ptr(),
                wide_string("Overlay").as_ptr(),
                WS_POPUP,
                x,
                y,
                width,
                height,
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

            // Apply opacity
            let opacity = alert.style.opacity.unwrap_or(config.opacity);
            SetLayeredWindowAttributes(hwnd, 0, (opacity * 255.0) as u8, LWA_ALPHA);

            // Store alert data
            let window_data = Box::into_raw(Box::new(WindowData {
                alert: alert.clone(),
                progress: 0.0,
                y_offset: 0,
            }));

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_data as isize);
            ShowWindow(hwnd, SW_SHOW);
            UpdateWindow(hwnd);

            let duration = alert
                .duration
                .map(Duration::from_secs)
                .unwrap_or(config.duration);

            Ok(Self {
                id: alert.id.clone(),
                hwnd,
                created: Instant::now(),
                duration,
            })
        }
    }

    /// Create a window from an overlay element (legacy support)
    #[allow(dead_code)]
    pub fn from_element(
        element: &crate::core::OverlayElement,
        config: &WindowConfig,
    ) -> Result<Self, RenderError> {
        let alert = element.to_alert();
        Self::from_alert(&alert, config)
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
            // Only redraw the progress bar area instead of entire window to prevent flickering
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            GetClientRect(self.hwnd, &mut rect);
            let progress_height = 4i32;
            let rect_to_invalidate = RECT {
                left: 0,
                top: rect.bottom - progress_height - 4,
                right: rect.right,
                bottom: rect.bottom,
            };
            InvalidateRect(self.hwnd, &rect_to_invalidate, 0);
        }
    }

    fn is_valid(&self) -> bool {
        !self.hwnd.is_null() && unsafe { IsWindow(self.hwnd) != 0 }
    }

    fn close(self) {
        unsafe {
            let data_ptr = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut WindowData;
            if !data_ptr.is_null() {
                // Clear the user data pointer first to prevent double-free
                SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(data_ptr);
            }
            DestroyWindow(self.hwnd);
        }
    }

    fn created_at(&self) -> Instant {
        self.created
    }
}

/// Convert a string to wide string (UTF-16)
fn wide_string(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

/// Parse a hex color string (#RRGGBB or #RGB) to RGB values
/// Also handles simple rgba() and rgb() formats
fn parse_hex_color(color: &str) -> (u8, u8, u8) {
    let hex = color.trim_start_matches('#');
    
    // Handle rgb() and rgba() formats
    if hex.starts_with("rgb") || hex.starts_with("rgba") {
        // Parse rgb(r, g, b) or rgba(r, g, b, a)
        if let Some(hex_part) = hex.strip_prefix("rgba(") {
            if let Some(parts) = hex_part.strip_suffix(")") {
                let nums: Vec<&str> = parts.split(',').collect();
                if nums.len() >= 3 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        nums[0].trim().parse::<u8>(),
                        nums[1].trim().parse::<u8>(),
                        nums[2].trim().parse::<u8>()
                    ) {
                        return (r, g, b);
                    }
                }
            }
        } else if let Some(hex_part) = hex.strip_prefix("rgb(") {
            if let Some(parts) = hex_part.strip_suffix(")") {
                let nums: Vec<&str> = parts.split(',').collect();
                if nums.len() >= 3 {
                    if let (Ok(r), Ok(g), Ok(b)) = (
                        nums[0].trim().parse::<u8>(),
                        nums[1].trim().parse::<u8>(),
                        nums[2].trim().parse::<u8>()
                    ) {
                        return (r, g, b);
                    }
                }
            }
        }
        // Default dark gray for rgba/rgb parsing failures
        return (30, 30, 30);
    }
    
    // Handle linear-gradient - return a special marker for gradient handling
    if hex.contains("linear-gradient") {
        // Return dark gray - gradient will be handled separately
        return (30, 30, 30);
    }
    
    match hex.len() {
        3 => {
            // #RGB format
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(30);
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(30);
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(30);
            (r, g, b)
        }
        6 => {
            // #RRGGBB format
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(30);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(30);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(30);
            (r, g, b)
        }
        _ => (30, 30, 30), // Default dark gray for unknown formats
    }
}

/// Check if a color string is a linear gradient
fn is_linear_gradient(color: &str) -> bool {
    color.contains("linear-gradient")
}

/// Parse linear-gradient and return (start_color, end_color, angle)
/// Returns None if parsing fails
fn parse_linear_gradient(color: &str) -> Option<((u8, u8, u8), (u8, u8, u8), i32)> {
    // Look for the gradient pattern: linear-gradient(Xdeg, #COLOR1, #COLOR2)
    if !color.contains("linear-gradient") {
        return None;
    }
    
    // Extract the content between parentheses
    let start = color.find('(')?;
    let end = color.rfind(')')?;
    let gradient_content = &color[start+1..end];
    
    // Parse angle if present (e.g., "135deg,")
    let mut angle = 135; // Default diagonal
    let mut colors_part = gradient_content;
    
    if let Some(deg_pos) = gradient_content.find("deg") {
        let angle_str = &gradient_content[..deg_pos];
        // Find the last number before "deg"
        if let Some(comma_pos) = angle_str.rfind(',') {
            let num_str = angle_str[comma_pos+1..].trim();
            if let Ok(a) = num_str.parse::<i32>() {
                angle = a;
            }
        }
        // Find where the colors start
        if let Some(colors_start) = gradient_content.find(',') {
            colors_part = &gradient_content[colors_start+1..];
        }
    }
    
    // Extract color values - handle colors with optional percentage like "#1a1a1a 0%"
    let mut colors: Vec<(u8, u8, u8)> = Vec::new();
    
    // Find all hex colors (#RRGGBB or #RGB) and extract just the hex part (before any percentage)
    let mut search_idx = 0;
    while let Some(hash_pos) = colors_part[search_idx..].find('#') {
        let absolute_pos = search_idx + hash_pos;
        let hex_part = &colors_part[absolute_pos+1..];
        
        // Extract hex color - handle formats like #RRGGBB, #RRGGBB 0%, #RGB
        let mut hex_len = 0;
        for (i, c) in hex_part.char_indices() {
            if c.is_ascii_hexdigit() {
                hex_len = i + 1;
                if hex_len >= 6 {
                    break;
                }
            } else {
                break;
            }
        }
        
        if hex_len >= 6 {
            let hex6 = &hex_part[..6];
            if let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&hex6[0..2], 16),
                u8::from_str_radix(&hex6[2..4], 16),
                u8::from_str_radix(&hex6[4..6], 16)
            ) {
                colors.push((r, g, b));
            }
        } else if hex_len >= 3 {
            let hex3 = &hex_part[..3];
            let r = u8::from_str_radix(&hex3[0..1].repeat(2), 16).unwrap_or(30);
            let g = u8::from_str_radix(&hex3[1..2].repeat(2), 16).unwrap_or(30);
            let b = u8::from_str_radix(&hex3[2..3].repeat(2), 16).unwrap_or(30);
            colors.push((r, g, b));
        }
        
        search_idx = absolute_pos + 1;
    }
    
    if colors.len() >= 2 {
        Some((colors[0], colors[1], angle))
    } else if colors.len() == 1 {
        // If only one color, use it for both start and end
        Some((colors[0], colors[0], angle))
    } else {
        None
    }
}

/// Window procedure for processing Windows messages
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
                // Clear the user data pointer first to prevent double-free
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let _ = Box::from_raw(data_ptr);
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Main rendering function - renders the Alert with all its components
unsafe fn render_content(hdc: HDC, rect: &RECT, hwnd: HWND) {
    let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;
    if data_ptr.is_null() {
        return;
    }

    let data = &*data_ptr;
    let alert = &data.alert;
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    // Draw background - check for gradient first, then solid color
    let bg_color_str = alert.style.background_color.as_deref();
    
    if let Some(color_str) = bg_color_str {
        if is_linear_gradient(color_str) {
            // Parse gradient colors - use average of both colors as solid fallback
            if let Some((start_color, end_color, _angle)) = parse_linear_gradient(color_str) {
                // Use blended color as solid fallback
                let r = ((start_color.0 as u16 + end_color.0 as u16) / 2) as u8;
                let g = ((start_color.1 as u16 + end_color.1 as u16) / 2) as u8;
                let b = ((start_color.2 as u16 + end_color.2 as u16) / 2) as u8;
                let bg_brush = CreateSolidBrush(RGB(r, g, b));
                FillRect(hdc, rect, bg_brush);
                DeleteObject(bg_brush as *mut _);
            } else {
                // Fallback to solid color if parsing fails
                let bg_color = parse_hex_color(color_str);
                let bg_brush = CreateSolidBrush(RGB(bg_color.0, bg_color.1, bg_color.2));
                FillRect(hdc, rect, bg_brush);
                DeleteObject(bg_brush as *mut _);
            }
        } else {
            // Solid color
            let bg_color = parse_hex_color(color_str);
            let bg_brush = CreateSolidBrush(RGB(bg_color.0, bg_color.1, bg_color.2));
            FillRect(hdc, rect, bg_brush);
            DeleteObject(bg_brush as *mut _);
        }
    } else {
        // Default dark gray background when no color is specified
        let bg_brush = CreateSolidBrush(RGB(30, 30, 30));
        FillRect(hdc, rect, bg_brush);
        DeleteObject(bg_brush as *mut _);
    }
    
    let border_color = alert
        .style
        .border_color
        .as_ref()
        .map(|c| parse_hex_color(c));
    
    let _border_radius = alert.style.border_radius.unwrap_or(8);

    // Draw border if specified
    if let Some(bc) = border_color {
        let pen = CreatePen(PS_SOLID as i32, 2, RGB(bc.0, bc.1, bc.2));
        let old_pen = SelectObject(hdc, pen as *mut _);
        
        // Draw a simple rectangle border
        Rectangle(hdc, rect.left, rect.top, rect.right, rect.bottom);
        
        SelectObject(hdc, old_pen);
        DeleteObject(pen as *mut _);
    }

    // Set up text rendering
    SetBkMode(hdc, TRANSPARENT as i32);

    // Calculate padding
    let padding = alert.style.padding.unwrap_or(10) as i32;
    let current_y = padding;
    let content_width = width - (padding * 2);

    // Render components based on layout
    match alert.layout {
        Layout::Vertical => {
            render_vertical_layout(hdc, rect, &alert.components, padding, current_y, content_width);
        }
        Layout::Horizontal => {
            render_horizontal_layout(hdc, rect, &alert.components, padding, current_y, content_width);
        }
        Layout::Stacked => {
            render_stacked_layout(hdc, rect, &alert.components, padding, current_y, content_width);
        }
    }

    // Draw progress bar at the bottom
    let progress_height = 4i32;
    let progress_width = ((width - padding * 2) as f64 * data.progress) as i32;
    
    if data.progress > 0.0 {
        let progress_bg = CreateSolidBrush(RGB(80, 80, 80));
        let progress_rect = RECT {
            left: padding,
            top: height - progress_height - 2,
            right: width - padding,
            bottom: height - 2,
        };
        FillRect(hdc, &progress_rect, progress_bg);
        DeleteObject(progress_bg as *mut _);

        let progress_fg = CreateSolidBrush(RGB(0, 150, 255));
        let progress_fg_rect = RECT {
            left: padding,
            top: height - progress_height - 2,
            right: padding + progress_width,
            bottom: height - 2,
        };
        FillRect(hdc, &progress_fg_rect, progress_fg);
        DeleteObject(progress_fg as *mut _);
    }
}

/// Render components in vertical layout
#[allow(clippy::too_many_lines)]
unsafe fn render_vertical_layout(
    hdc: HDC,
    rect: &RECT,
    components: &[AlertComponent],
    padding: i32,
    mut current_y: i32,
    content_width: i32,
) {
    let height = rect.bottom - rect.top;

    for component in components {
        match component {
            AlertComponent::Text {
                content,
                color,
                weight,
                style: _style,
                size,
            } => {
                let text_color = color
                    .as_ref()
                    .map(|c| parse_hex_color(c))
                    .unwrap_or((255, 255, 255));
                
                let font_size = size.unwrap_or(14) as i32;
                let font_weight = if weight.as_deref() == Some("bold") { FW_BOLD } else { FW_NORMAL };

                // Create font
                let font = CreateFontW(
                    font_size,
                    0,
                    0,
                    0,
                    font_weight,
                    0u32,
                    0u32,
                    0u32,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    DEFAULT_PITCH | FF_DONTCARE,
                    wide_string("Arial").as_ptr(),
                );
                
                let old_font = SelectObject(hdc, font as *mut _);
                SetTextColor(hdc, RGB(text_color.0, text_color.1, text_color.2));

                let text_wide = wide_string(content);
                let mut text_rect = RECT {
                    left: padding,
                    top: current_y,
                    right: padding + content_width,
                    bottom: height - 10,
                };

                DrawTextW(
                    hdc,
                    text_wide.as_ptr(),
                    text_wide.len() as i32 - 1,
                    &mut text_rect,
                    DT_LEFT | DT_WORDBREAK | DT_CALCRECT,
                );

                // Draw the text
                let mut draw_rect = RECT {
                    left: padding,
                    top: current_y,
                    right: padding + content_width,
                    bottom: text_rect.bottom,
                };
                
                DrawTextW(
                    hdc,
                    text_wide.as_ptr(),
                    text_wide.len() as i32 - 1,
                    &mut draw_rect,
                    DT_LEFT | DT_WORDBREAK,
                );

                current_y = text_rect.bottom + 5;

                SelectObject(hdc, old_font);
                DeleteObject(font as *mut _);
            }
            AlertComponent::Image {
                url: _url,
                width,
                height: img_height,
                is_animated: _,
            } => {
                // Basic image placeholder - in a full implementation, we'd load the image
                let w = width.unwrap_or(64) as i32;
                let h = img_height.unwrap_or(64) as i32;
                
                // Draw a placeholder rectangle
                let img_brush = CreateSolidBrush(RGB(60, 60, 60));
                let img_rect = RECT {
                    left: padding,
                    top: current_y,
                    right: padding + w,
                    bottom: current_y + h,
                };
                FillRect(hdc, &img_rect, img_brush);
                DeleteObject(img_brush as *mut _);
                
                // Draw border
                let border_pen = CreatePen(PS_SOLID as i32, 1, RGB(100, 100, 100));
                let old_pen = SelectObject(hdc, border_pen as *mut _);
                Rectangle(hdc, padding, current_y, padding + w, current_y + h);
                SelectObject(hdc, old_pen);
                DeleteObject(border_pen as *mut _);

                current_y += h + 5;
            }
            AlertComponent::Badge {
                id: _id,
                name,
                url: _url,
            } => {
                // Render badge as a small colored rectangle with text
                let badge_bg = CreateSolidBrush(RGB(100, 100, 180));
                let badge_w = 60i32;
                let badge_h = 20i32;
                let badge_rect = RECT {
                    left: padding,
                    top: current_y,
                    right: padding + badge_w,
                    bottom: current_y + badge_h,
                };
                FillRect(hdc, &badge_rect, badge_bg);
                DeleteObject(badge_bg as *mut _);

                // Badge text
                let font = CreateFontW(
                    10,
                    0,
                    0,
                    0,
                    FW_NORMAL,
                    0u32,
                    0u32,
                    0u32,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    DEFAULT_PITCH | FF_DONTCARE,
                    wide_string("Arial").as_ptr(),
                );
                let old_font = SelectObject(hdc, font as *mut _);
                SetTextColor(hdc, RGB(255, 255, 255));

                let badge_wide = wide_string(name);
                let mut badge_rect = RECT {
                    left: padding + 2,
                    top: current_y + 2,
                    right: padding + badge_w - 2,
                    bottom: current_y + badge_h - 2,
                };
                DrawTextW(
                    hdc,
                    badge_wide.as_ptr(),
                    badge_wide.len() as i32 - 1,
                    &mut badge_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );

                current_y += badge_h + 5;

                SelectObject(hdc, old_font);
                DeleteObject(font as *mut _);
            }
        }
    }
}

/// Render components in horizontal layout
#[allow(clippy::too_many_lines)]
unsafe fn render_horizontal_layout(
    hdc: HDC,
    rect: &RECT,
    components: &[AlertComponent],
    padding: i32,
    padding_vert: i32,
    _content_width: i32,
) {
    let height = rect.bottom - rect.top;
    let mut current_x = padding;
    let center_y = height / 2;

    for component in components {
        match component {
            AlertComponent::Text {
                content,
                color,
                weight,
                style: _style,
                size,
            } => {
                let text_color = color
                    .as_ref()
                    .map(|c| parse_hex_color(c))
                    .unwrap_or((255, 255, 255));
                
                let font_size = size.unwrap_or(14) as i32;
                let font_weight = if weight.as_deref() == Some("bold") { FW_BOLD } else { FW_NORMAL };

                let font = CreateFontW(
                    font_size,
                    0,
                    0,
                    0,
                    font_weight,
                    0u32,
                    0u32,
                    0u32,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    DEFAULT_PITCH | FF_DONTCARE,
                    wide_string("Arial").as_ptr(),
                );
                
                let old_font = SelectObject(hdc, font as *mut _);
                SetTextColor(hdc, RGB(text_color.0, text_color.1, text_color.2));

                let text_wide = wide_string(content);
                let mut text_rect = RECT {
                    left: current_x,
                    top: padding_vert,
                    right: current_x + 500, // Arbitrary large width
                    bottom: height - padding_vert,
                };

                DrawTextW(
                    hdc,
                    text_wide.as_ptr(),
                    text_wide.len() as i32 - 1,
                    &mut text_rect,
                    DT_LEFT | DT_SINGLELINE | DT_CALCRECT,
                );

                let text_height = text_rect.bottom - text_rect.top;
                let text_y = center_y - (text_height / 2);

                let mut draw_rect = RECT {
                    left: current_x,
                    top: text_y,
                    right: current_x + (text_rect.right - text_rect.left),
                    bottom: text_y + text_height,
                };
                
                DrawTextW(
                    hdc,
                    text_wide.as_ptr(),
                    text_wide.len() as i32 - 1,
                    &mut draw_rect,
                    DT_LEFT | DT_SINGLELINE,
                );

                current_x += (text_rect.right - text_rect.left) + 8;

                SelectObject(hdc, old_font);
                DeleteObject(font as *mut _);
            }
            AlertComponent::Image {
                url: _url,
                width,
                height: img_height,
                is_animated: _,
            } => {
                let w = width.unwrap_or(64) as i32;
                let h = img_height.unwrap_or(64) as i32;
                
                let img_brush = CreateSolidBrush(RGB(60, 60, 60));
                let img_rect = RECT {
                    left: current_x,
                    top: center_y - h / 2,
                    right: current_x + w,
                    bottom: center_y + h / 2,
                };
                FillRect(hdc, &img_rect, img_brush);
                DeleteObject(img_brush as *mut _);

                current_x += w + 8;
            }
            AlertComponent::Badge {
                id: _id,
                name,
                url: _url,
            } => {
                let badge_bg = CreateSolidBrush(RGB(100, 100, 180));
                let badge_w = 60i32;
                let badge_h = 20i32;
                let badge_rect = RECT {
                    left: current_x,
                    top: center_y - badge_h / 2,
                    right: current_x + badge_w,
                    bottom: center_y + badge_h / 2,
                };
                FillRect(hdc, &badge_rect, badge_bg);
                DeleteObject(badge_bg as *mut _);

                let font = CreateFontW(
                    10,
                    0,
                    0,
                    0,
                    FW_NORMAL,
                    0u32,
                    0u32,
                    0u32,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    DEFAULT_PITCH | FF_DONTCARE,
                    wide_string("Arial").as_ptr(),
                );
                let old_font = SelectObject(hdc, font as *mut _);
                SetTextColor(hdc, RGB(255, 255, 255));

                let badge_wide = wide_string(name);
                let mut badge_rect = RECT {
                    left: current_x + 2,
                    top: center_y - badge_h / 2 + 2,
                    right: current_x + badge_w - 2,
                    bottom: center_y + badge_h / 2 - 2,
                };
                DrawTextW(
                    hdc,
                    badge_wide.as_ptr(),
                    badge_wide.len() as i32 - 1,
                    &mut badge_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );

                current_x += badge_w + 8;

                SelectObject(hdc, old_font);
                DeleteObject(font as *mut _);
            }
        }
    }
}

/// Render components in stacked layout (similar to vertical but tighter spacing)
#[allow(clippy::too_many_lines)]
unsafe fn render_stacked_layout(
    hdc: HDC,
    rect: &RECT,
    components: &[AlertComponent],
    padding: i32,
    mut current_y: i32,
    content_width: i32,
) {
    let height = rect.bottom - rect.top;

    for component in components {
        match component {
            AlertComponent::Text {
                content,
                color,
                weight,
                style: _style,
                size,
            } => {
                let text_color = color
                    .as_ref()
                    .map(|c| parse_hex_color(c))
                    .unwrap_or((255, 255, 255));
                
                let font_size = size.unwrap_or(14) as i32;
                let font_weight = if weight.as_deref() == Some("bold") { FW_BOLD } else { FW_NORMAL };

                let font = CreateFontW(
                    font_size,
                    0,
                    0,
                    0,
                    font_weight,
                    0u32,
                    0u32,
                    0u32,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    DEFAULT_PITCH | FF_DONTCARE,
                    wide_string("Arial").as_ptr(),
                );
                
                let old_font = SelectObject(hdc, font as *mut _);
                SetTextColor(hdc, RGB(text_color.0, text_color.1, text_color.2));

                let text_wide = wide_string(content);
                let mut text_rect = RECT {
                    left: padding,
                    top: current_y,
                    right: padding + content_width,
                    bottom: height - 10,
                };

                DrawTextW(
                    hdc,
                    text_wide.as_ptr(),
                    text_wide.len() as i32 - 1,
                    &mut text_rect,
                    DT_LEFT | DT_SINGLELINE | DT_CALCRECT,
                );

                let mut draw_rect = RECT {
                    left: padding,
                    top: current_y,
                    right: padding + content_width,
                    bottom: text_rect.bottom,
                };
                
                DrawTextW(
                    hdc,
                    text_wide.as_ptr(),
                    text_wide.len() as i32 - 1,
                    &mut draw_rect,
                    DT_LEFT | DT_SINGLELINE,
                );

                current_y += text_rect.bottom - text_rect.top + 2;

                SelectObject(hdc, old_font);
                DeleteObject(font as *mut _);
            }
            AlertComponent::Image {
                url: _url,
                width,
                height: img_height,
                is_animated: _,
            } => {
                let w = width.unwrap_or(48) as i32;
                let h = img_height.unwrap_or(48) as i32;
                
                let img_brush = CreateSolidBrush(RGB(60, 60, 60));
                let img_rect = RECT {
                    left: padding,
                    top: current_y,
                    right: padding + w,
                    bottom: current_y + h,
                };
                FillRect(hdc, &img_rect, img_brush);
                DeleteObject(img_brush as *mut _);
                
                current_y += h + 2;
            }
            AlertComponent::Badge {
                id: _id,
                name,
                url: _url,
            } => {
                let badge_bg = CreateSolidBrush(RGB(100, 100, 180));
                let badge_w = 50i32;
                let badge_h = 16i32;
                let badge_rect = RECT {
                    left: padding,
                    top: current_y,
                    right: padding + badge_w,
                    bottom: current_y + badge_h,
                };
                FillRect(hdc, &badge_rect, badge_bg);
                DeleteObject(badge_bg as *mut _);

                let font = CreateFontW(
                    9,
                    0,
                    0,
                    0,
                    FW_NORMAL,
                    0u32,
                    0u32,
                    0u32,
                    DEFAULT_CHARSET,
                    OUT_DEFAULT_PRECIS,
                    CLIP_DEFAULT_PRECIS,
                    CLEARTYPE_QUALITY,
                    DEFAULT_PITCH | FF_DONTCARE,
                    wide_string("Arial").as_ptr(),
                );
                let old_font = SelectObject(hdc, font as *mut _);
                SetTextColor(hdc, RGB(255, 255, 255));

                let badge_wide = wide_string(name);
                let mut badge_rect = RECT {
                    left: padding + 2,
                    top: current_y + 1,
                    right: padding + badge_w - 2,
                    bottom: current_y + badge_h - 1,
                };
                DrawTextW(
                    hdc,
                    badge_wide.as_ptr(),
                    badge_wide.len() as i32 - 1,
                    &mut badge_rect,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );

                current_y += badge_h + 2;

                SelectObject(hdc, old_font);
                DeleteObject(font as *mut _);
            }
        }
    }
}

/// Errors during rendering
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
