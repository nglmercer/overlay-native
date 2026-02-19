//! GTK-based window rendering for Linux
//!
//! This module provides the GTK implementation for rendering overlay windows
//! on Linux systems. It uses GTK3 and integrates with X11 for transparent
//! overlay windows.

use std::time::{Duration, Instant};

use gtk::prelude::*;
use gtk::{gdk, glib};

use super::{PlatformWindow, WindowConfig};
use crate::core::{ChatMessageElement, EmoteElement, GiftElement, OverlayElement};

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
        // Use Popup window type for overlay windows on Linux
        #[cfg(target_os = "linux")]
        let window = gtk::Window::new(gtk::WindowType::Popup);
        #[cfg(not(target_os = "linux"))]
        let window = gtk::Window::new(gtk::WindowType::Toplevel);

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

        // Message content with emotes
        let message_box = gtk::Box::new(gtk::Orientation::Horizontal, 2);
        build_message_content(&message_box, &message.content, &message.emotes);
        layout.add(&message_box);

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
        #[cfg(target_os = "linux")]
        let window = gtk::Window::new(gtk::WindowType::Popup);
        #[cfg(not(target_os = "linux"))]
        let window = gtk::Window::new(gtk::WindowType::Toplevel);

        window.set_title(&format!("Overlay - Gift from {}", gift.from_user));
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

    /// Create a new GTK overlay window from an emote event
    pub fn from_emote(emote: &EmoteElement, config: &WindowConfig) -> Result<Self, RenderError> {
        #[cfg(target_os = "linux")]
        let window = gtk::Window::new(gtk::WindowType::Popup);
        #[cfg(not(target_os = "linux"))]
        let window = gtk::Window::new(gtk::WindowType::Toplevel);

        window.set_title(&format!("Overlay - Emote {}", emote.name));
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

        // Emote image
        let emote_img = gtk::Image::new();
        // TODO: Load emote from URL asynchronously
        layout.add(&emote_img);

        // Sender label
        if let Some(sender) = &emote.sender {
            let label = gtk::Label::new(Some(&format!("Sent by {}", sender)));
            layout.add(&label);
        }

        let progress = gtk::ProgressBar::new();
        progress.set_show_text(false);
        progress.set_fraction(0.0);
        layout.add(&progress);

        window.add(&layout);

        Ok(Self {
            id: emote.id.clone(),
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
            OverlayElement::Emote(emote) => Self::from_emote(emote, config),
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
        self.window.get_visible()
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

/// Build the message content with emotes
fn build_message_content(container: &gtk::Box, content: &str, emotes: &[crate::core::Emote]) {
    // Sort emotes by position
    let mut sorted_emotes = emotes.to_vec();
    sorted_emotes.sort_by_key(|e| e.positions.first().map(|p| p.start).unwrap_or(0));

    let mut current_pos = 0;

    for emote in sorted_emotes {
        if let Some(pos) = emote.positions.first() {
            // Add text before emote
            if pos.start > current_pos {
                let text = &content[current_pos..pos.start];
                let label = gtk::Label::new(Some(text));
                container.add(&label);
            }

            // Add emote image
            let img = gtk::Image::new();
            // TODO: Load emote from cache or URL
            if let Some(_url) = &emote.url {
                // Set placeholder for now
                img.set_from_icon_name(Some("face-smile-symbolic"), gtk::IconSize::Button);
            }
            container.add(&img);

            current_pos = pos.end;
        }
    }

    // Add remaining text
    if current_pos < content.len() {
        let text = &content[current_pos..];
        let label = gtk::Label::new(Some(text));
        container.add(&label);
    }
}

/// Errors during rendering
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Failed to create window: {0}")]
    WindowCreation(String),

    #[error("Failed to load emote: {0}")]
    EmoteLoad(String),

    #[error("GTK error: {0}")]
    Gtk(String),
}

/// Window geometry for X11 integration
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Get the primary monitor geometry
pub fn get_primary_monitor_geometry() -> Option<(i32, i32, i32, i32)> {
    let display = gdk::Display::default()?;
    let monitor = display.primary_monitor()?;
    let geom = monitor.geometry();
    Some((geom.x(), geom.y(), geom.width(), geom.height()))
}

/// Initialize GTK for overlay windows
pub fn init_gtk() -> Result<(), RenderError> {
    gtk::init().map_err(|e| RenderError::Gtk(e.to_string()))?;

    // Load CSS for styling
    let css_provider = gtk::CssProvider::new();
    css_provider
        .load_from_data(include_bytes!("../../style.css"))
        .map_err(|e| RenderError::Gtk(e.to_string()))?;

    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::default().ok_or_else(|| RenderError::Gtk("No default screen".to_string()))?,
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    Ok(())
}

/// Run a single GTK iteration (non-blocking)
pub fn gtk_iteration() -> bool {
    gtk::main_iteration_do(false)
}
