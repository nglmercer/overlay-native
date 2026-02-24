//! Platform-specific rendering module
//!
//! This module provides the actual window rendering implementation for each
//! operating system. It is completely decoupled from platform logic and only
//! receives normalized messages from the core renderer.

#[cfg(unix)]
pub mod gtk;

#[cfg(windows)]
pub mod win32;

use std::time::{Duration, Instant};

/// Trait for platform-specific window rendering
pub trait PlatformWindow {
    /// Get the window ID (for tracking)
    fn id(&self) -> &str;

    /// Update the progress bar (0.0 - 1.0)
    fn set_progress(&mut self, progress: f64);

    /// Check if the window is still valid/visible
    fn is_valid(&self) -> bool;

    /// Close and destroy the window
    fn close(self);

    /// Get the creation time
    fn created_at(&self) -> Instant;
}

/// Configuration for window rendering
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub position: (i32, i32),
    pub size: (i32, i32),
    pub duration: Duration,
    pub opacity: f32,
    pub border_radius: u32,
    pub font_family: String,
    pub font_size: u32,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            position: (0, 0),
            size: (400, 100),
            duration: Duration::from_secs(10),
            opacity: 0.9,
            border_radius: 8,
            font_family: "Arial".to_string(),
            font_size: 14,
        }
    }
}

// Re-export platform-specific types
#[cfg(unix)]
pub use gtk::GtkWindow;
#[cfg(unix)]
pub use gtk::GtkWindow as PlatformWindowImpl;

#[cfg(windows)]
pub use win32::Win32Window as PlatformWindowImpl;
