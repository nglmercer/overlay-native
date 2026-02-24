//! GTK-based window rendering for Linux
//!
//! This module provides the GTK implementation for rendering overlay windows
//! on Linux systems. It uses GTK3 and integrates with X11 for transparent
//! overlay windows.

use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{gdk, glib};

use super::{PlatformWindow, WindowConfig};
use crate::core::{Alert, OverlayElement};

/// GTK-based overlay window
pub struct GtkWindow {
    /// Window ID
    id: String,

    /// GTK window
    window: gtk::Window,

    /// Progress bar widget
    progress: gtk::ProgressBar,

    /// Creation time
    created: Instant,

    /// Display duration
    #[allow(dead_code)]
    duration: Duration,
}

impl GtkWindow {
    /// Create a window from an overlay element
    pub fn from_element(
        element: &OverlayElement,
        config: &WindowConfig,
    ) -> Result<Self, RenderError> {
        Self::from_alert(&element.0, config)
    }

    /// Create a window from a generic Alert
    pub fn from_alert(alert: &Alert, config: &WindowConfig) -> Result<Self, RenderError> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_type_hint(gdk::WindowTypeHint::Utility);

        window.set_title(&format!("Overlay Alert - {}", alert.id));
        window.set_decorated(false);
        window.set_skip_taskbar_hint(true);
        window.set_skip_pager_hint(true);
        window.set_keep_above(true);
        window.set_accept_focus(false);
        window.set_resizable(false);

        window.move_(config.position.0, config.position.1);
        window.set_default_size(config.size.0, config.size.1);

        // Apply style defaults or overrides
        let opacity = alert.style.opacity.unwrap_or(config.opacity);
        window.set_opacity(opacity as f64);

        // Main container
        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

        // Apply custom alert background if specified
        if let Some(ref bg) = alert.style.background_color {
            let provider = gtk::CssProvider::new();
            let css = format!(
                ".alert-{} {{ background-color: {}; border-radius: {}px; }}",
                alert.id,
                bg,
                alert.style.border_radius.unwrap_or(config.border_radius)
            );
            let _ = provider.load_from_data(css.as_bytes());
            main_box
                .style_context()
                .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
            main_box
                .style_context()
                .add_class(&format!("alert-{}", alert.id));
        }

        // Layout container
        let layout = match alert.layout {
            crate::core::Layout::Vertical => gtk::Box::new(gtk::Orientation::Vertical, 5),
            crate::core::Layout::Horizontal => gtk::Box::new(gtk::Orientation::Horizontal, 8),
            crate::core::Layout::Stacked => gtk::Box::new(gtk::Orientation::Vertical, 0), // Basic stack
        };
        layout.set_margin_start(alert.style.padding.unwrap_or(10) as i32);
        layout.set_margin_end(alert.style.padding.unwrap_or(10) as i32);
        layout.set_margin_top(5);
        layout.set_margin_bottom(5);

        for component in &alert.components {
            match component {
                crate::core::AlertComponent::Text {
                    content,
                    color,
                    weight,
                    style,
                    size,
                } => {
                    let label = gtk::Label::new(None);
                    let mut markup = String::from("<span");

                    if let Some(c) = color {
                        markup.push_str(&format!(" foreground=\"{}\"", c));
                    }
                    if let Some(w) = weight {
                        markup.push_str(&format!(" weight=\"{}\"", w));
                    }
                    if let Some(s) = style {
                        markup.push_str(&format!(" style=\"{}\"", s));
                    }
                    if let Some(sz) = size {
                        markup.push_str(&format!(" font_size=\"{}\"", sz * 1024));
                    }

                    markup.push_str(&format!(
                        ">{}</span>",
                        glib::markup_escape_text(content.as_str()).as_str()
                    ));
                    label.set_markup(&markup);
                    label.set_line_wrap(true);
                    layout.add(&label);
                }
                crate::core::AlertComponent::Image {
                    url, width, height, ..
                } => {
                    let image = gtk::Image::new();
                    let w = width.unwrap_or(64) as i32;
                    let h = height.unwrap_or(64) as i32;

                    // Basic blocking image load for demo purposes
                    if url.starts_with("http") {
                        if let Ok(resp) = reqwest::blocking::get(url) {
                            if let Ok(bytes) = resp.bytes() {
                                let loader = gdk_pixbuf::PixbufLoader::new();
                                if loader.write(&bytes).is_ok() && loader.close().is_ok() {
                                    if let Some(pixbuf) = loader.pixbuf() {
                                        let scaled = pixbuf
                                            .scale_simple(w, h, gdk_pixbuf::InterpType::Bilinear)
                                            .expect("Failed to scale pixbuf");
                                        image.set_from_pixbuf(Some(&scaled));
                                    }
                                }
                            }
                        }
                    }

                    image.set_size_request(w, h);
                    layout.add(&image);
                }
                crate::core::AlertComponent::Badge { url, name, .. } => {
                    let badge_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
                    if let Some(_u) = url {
                        let img = gtk::Image::new();
                        badge_box.add(&img);
                    }
                    let label = gtk::Label::new(Some(name.as_str()));
                    badge_box.add(&label);
                    layout.add(&badge_box);
                }
            }
        }

        main_box.add(&layout);

        // Progress bar
        let progress = gtk::ProgressBar::new();
        progress.set_show_text(false);
        progress.set_fraction(0.0);
        main_box.add(&progress);

        window.add(&main_box);

        Ok(Self {
            id: alert.id.clone(),
            window,
            progress,
            created: Instant::now(),
            duration: alert
                .duration
                .map(Duration::from_secs)
                .unwrap_or(config.duration),
        })
    }

    /// Show the window
    pub fn show(&self) {
        self.window.show_all();
    }
}

impl PlatformWindow for GtkWindow {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_progress(&mut self, progress: f64) {
        self.progress.set_fraction(progress.clamp(0.0, 1.0));
    }

    fn is_valid(&self) -> bool {
        self.window.is_visible()
    }

    fn close(self) {
        self.window.close();
    }

    fn created_at(&self) -> Instant {
        self.created
    }
}

impl Clone for GtkWindow {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            window: self.window.clone(),
            progress: self.progress.clone(),
            created: self.created,
            duration: self.duration,
        }
    }
}

/// Errors during rendering
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Failed to create window: {0}")]
    WindowCreation(String),

    #[error("GTK error: {0}")]
    Gtk(String),
}

/// Get the primary monitor geometry
pub fn get_primary_monitor_geometry() -> Option<(i32, i32, i32, i32)> {
    let display = gdk::Display::default()?;
    let monitor = display.primary_monitor().or_else(|| display.monitor(0))?;
    let geom = monitor.geometry();
    Some((geom.x(), geom.y(), geom.width(), geom.height()))
}

/// Initialize GTK for overlay windows
pub fn init_gtk() -> Result<(), RenderError> {
    gtk::init().map_err(|e| RenderError::Gtk(e.to_string()))?;

    // Load CSS for styling
    let css_provider = gtk::CssProvider::new();
    let css_data = include_bytes!("../../style.css");
    css_provider
        .load_from_data(css_data)
        .map_err(|e| RenderError::Gtk(e.to_string()))?;

    if let Some(screen) = gdk::Screen::default() {
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &css_provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    Ok(())
}

/// Run a single GTK iteration (non-blocking)
pub fn gtk_iteration() -> bool {
    gtk::main_iteration_do(false)
}
