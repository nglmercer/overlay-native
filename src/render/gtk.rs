//! GTK-based window rendering for Linux
//!
//! This module provides the GTK implementation for rendering overlay windows
//! on Linux systems. It uses GTK3 and integrates with X11 for transparent
//! overlay windows.

use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{gdk, glib};

use super::{PlatformWindow, WindowConfig};
use crate::core::{ChatMessageElement, GiftElement, ImageElement, OverlayElement};

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
    /// Create a new GTK overlay window from a chat message
    pub fn from_chat_message(
        message: &ChatMessageElement,
        config: &WindowConfig,
    ) -> Result<Self, RenderError> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);

        // Use Utility/Dock hints to avoid decorations and behavior like normal windows
        window.set_type_hint(gdk::WindowTypeHint::Utility);

        // Configure window
        window.set_title(&format!("Overlay - {}", message.username));
        window.set_decorated(false);
        window.set_skip_taskbar_hint(true);
        window.set_skip_pager_hint(true);
        window.set_keep_above(true);
        window.set_accept_focus(false);
        window.set_resizable(false);

        // Set position and size
        window.move_(config.position.0, config.position.1);
        window.set_default_size(config.size.0, config.size.1);

        // Set opacity (convert f32 to f64)
        window.set_opacity(config.opacity as f64);

        // Create layout
        let layout = gtk::Box::new(gtk::Orientation::Vertical, 5);
        layout.set_margin_start(10);
        layout.set_margin_end(10);
        layout.set_margin_top(5);
        layout.set_margin_bottom(5);

        // Username label
        let username = gtk::Label::new(None);
        let username_markup = format!(
            "<span foreground=\"{}\" weight=\"bold\">{}</span>",
            message.user_color.as_deref().unwrap_or("#9147ff"),
            glib::markup_escape_text(&message.username)
        );
        username.set_markup(&username_markup);
        layout.add(&username);

        // Message content
        let label = gtk::Label::new(Some(&message.content));
        label.set_line_wrap(true);
        label.set_max_width_chars(50);
        layout.add(&label);

        // Progress bar
        let progress = gtk::ProgressBar::new();
        progress.set_show_text(false);
        progress.set_fraction(0.0);
        layout.add(&progress);

        window.add(&layout);

        Ok(Self {
            id: message.id.clone(),
            window,
            progress,
            created: Instant::now(),
            duration: config.duration,
        })
    }

    /// Create a new GTK overlay window from a gift event
    pub fn from_gift(gift: &GiftElement, config: &WindowConfig) -> Result<Self, RenderError> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_type_hint(gdk::WindowTypeHint::Utility);

        window.set_title(&format!("Overlay Gift - {}", gift.from_user));
        window.set_decorated(false);
        window.set_skip_taskbar_hint(true);
        window.set_skip_pager_hint(true);
        window.set_keep_above(true);
        window.set_accept_focus(false);
        window.set_resizable(false);

        window.move_(config.position.0, config.position.1);
        window.set_default_size(config.size.0, config.size.1);
        window.set_opacity(config.opacity as f64);

        let layout = gtk::Box::new(gtk::Orientation::Vertical, 5);
        layout.set_margin_start(10);
        layout.set_margin_end(10);
        layout.set_margin_top(5);
        layout.set_margin_bottom(5);

        // Gift text
        let gift_text = match &gift.to_user {
            Some(to_user) => format!(
                "🎁 {} gifted {} to {}!",
                gift.from_user,
                match &gift.gift_type {
                    crate::core::GiftType::Subscription => "a subscription".to_string(),
                    crate::core::GiftType::GiftSubscription => "a gifted sub".to_string(),
                    crate::core::GiftType::Bits => format!("{} bits", gift.amount.unwrap_or(1)),
                    crate::core::GiftType::Donation =>
                        format!("${} donation", gift.amount.unwrap_or(0)),
                    other => format!("{:?}", other),
                },
                to_user
            ),
            None => format!(
                "🎁 {} gifted {} to the community!",
                gift.from_user,
                match &gift.gift_type {
                    crate::core::GiftType::Subscription => "a subscription".to_string(),
                    crate::core::GiftType::GiftSubscription => "a gifted sub".to_string(),
                    crate::core::GiftType::Bits => format!("{} bits", gift.amount.unwrap_or(1)),
                    crate::core::GiftType::Donation =>
                        format!("${} donation", gift.amount.unwrap_or(0)),
                    other => format!("{:?}", other),
                }
            ),
        };

        let label = gtk::Label::new(Some(&gift_text));
        layout.add(&label);

        let progress = gtk::ProgressBar::new();
        progress.set_show_text(false);
        progress.set_fraction(0.0);
        layout.add(&progress);

        window.add(&layout);

        Ok(Self {
            id: gift.id.clone(),
            window,
            progress,
            created: Instant::now(),
            duration: config.duration,
        })
    }

    /// Create a new GTK overlay window from an image event
    pub fn from_image(image: &ImageElement, config: &WindowConfig) -> Result<Self, RenderError> {
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_type_hint(gdk::WindowTypeHint::Utility);

        window.set_title(&format!("Overlay - Image {}", image.name));
        window.set_decorated(false);
        window.set_skip_taskbar_hint(true);
        window.set_skip_pager_hint(true);
        window.set_keep_above(true);
        window.set_accept_focus(false);
        window.set_resizable(false);

        window.move_(config.position.0, config.position.1);
        window.set_default_size(config.size.0, config.size.1);
        window.set_opacity(config.opacity as f64);

        let layout = gtk::Box::new(gtk::Orientation::Vertical, 5);
        layout.set_margin_start(10);
        layout.set_margin_end(10);
        layout.set_margin_top(5);
        layout.set_margin_bottom(5);

        // Image
        let img = gtk::Image::new();
        // TODO: Load image from URL asynchronously (requires reqwest + image crate back)
        layout.add(&img);

        // Sender label
        if let Some(sender) = &image.sender {
            let label = gtk::Label::new(Some(&format!("Sent by {}", sender)));
            layout.add(&label);
        }

        let progress = gtk::ProgressBar::new();
        progress.set_show_text(false);
        progress.set_fraction(0.0);
        layout.add(&progress);

        window.add(&layout);

        Ok(Self {
            id: image.id.clone(),
            window,
            progress,
            created: Instant::now(),
            duration: config.duration,
        })
    }

    /// Create from any overlay element
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
        // window.get_visible() is the proper way to check if it's still there
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
