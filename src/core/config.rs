//! Core configuration - Platform-agnostic settings
//!
//! This module defines the configuration for the core renderer.
//! All settings here are independent of any streaming platform.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the core renderer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct CoreConfig {
    /// Window/message display settings
    pub window: WindowSettings,

    /// Animation settings
    pub animation: AnimationSettings,

    /// Display preferences
    pub display: DisplaySettings,

    /// Message processing settings
    pub processing: ProcessingSettings,
}


/// Window display settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    /// How long messages are displayed (in seconds)
    pub message_duration_seconds: u64,

    /// Maximum number of active windows/overlays
    pub max_windows: usize,

    /// Enable window animations
    pub animation_enabled: bool,

    /// Fade in duration in milliseconds
    pub fade_in_duration_ms: u64,

    /// Fade out duration in milliseconds  
    pub fade_out_duration_ms: u64,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            message_duration_seconds: 10,
            max_windows: 50,
            animation_enabled: true,
            fade_in_duration_ms: 300,
            fade_out_duration_ms: 500,
        }
    }
}

impl WindowSettings {
    /// Get message duration as Duration
    pub fn message_duration(&self) -> Duration {
        Duration::from_secs(self.message_duration_seconds)
    }
}

/// Animation settings for overlay elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationSettings {
    /// Enable animations
    pub enabled: bool,

    /// Animation speed multiplier (0.5 = half speed, 2.0 = double speed)
    pub speed_multiplier: f32,

    /// Enable entrance animations
    pub entrance_enabled: bool,

    /// Enable exit animations
    pub exit_enabled: bool,

    /// Enable emote animations
    pub emote_animation: bool,
}

impl Default for AnimationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            speed_multiplier: 1.0,
            entrance_enabled: true,
            exit_enabled: true,
            emote_animation: true,
        }
    }
}

/// Display settings for rendered elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplaySettings {
    /// Font family to use
    pub font_family: String,

    /// Base font size
    pub font_size: u32,

    /// Background color (hex format)
    pub background_color: String,

    /// Primary text color (hex format)
    pub text_color: String,

    /// Username text color (hex format)
    pub username_color: String,

    /// Border radius in pixels
    pub border_radius: u32,

    /// Window opacity (0.0 - 1.0)
    pub opacity: f32,

    /// Monitor margin in pixels
    pub monitor_margin: i32,

    /// Window size in pixels
    pub window_size: i32,

    /// Grid size for positioning
    pub grid_size: i32,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            font_family: "Segoe UI, Arial, sans-serif".to_string(),
            font_size: 14,
            background_color: "#1a1a2e".to_string(),
            text_color: "#ffffff".to_string(),
            username_color: "#9147ff".to_string(),
            border_radius: 8,
            opacity: 0.9,
            monitor_margin: 10,
            window_size: 400,
            grid_size: 10,
        }
    }
}

/// Message processing settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingSettings {
    /// Maximum message length to display
    pub max_message_length: usize,

    /// Maximum emotes per message
    pub max_emotes_per_message: usize,

    /// Maximum badges per message
    pub max_badges_per_message: usize,

    /// Enable message deduplication
    pub deduplicate_messages: bool,

    /// Deduplication window in milliseconds
    pub deduplication_window_ms: u64,
}

impl Default for ProcessingSettings {
    fn default() -> Self {
        Self {
            max_message_length: 500,
            max_emotes_per_message: 50,
            max_badges_per_message: 10,
            deduplicate_messages: true,
            deduplication_window_ms: 5000,
        }
    }
}

/// Filter configuration for incoming messages
/// These are applied AFTER validation by the transport layer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFilter {
    /// Minimum message length
    pub min_length: Option<usize>,

    /// Maximum message length
    pub max_length: Option<usize>,

    /// Blocked usernames
    pub blocked_users: Vec<String>,

    /// Allowed usernames (if not empty, only these can send)
    pub allowed_users: Vec<String>,

    /// Blocked words (partial match)
    pub blocked_words: Vec<String>,

    /// Only allow commands (messages starting with ! or /)
    pub commands_only: bool,

    /// Only allow subscribers
    pub subscribers_only: bool,

    /// Only allow VIPs
    pub vip_only: bool,
}

impl Default for MessageFilter {
    fn default() -> Self {
        Self {
            min_length: None,
            max_length: Some(500),
            blocked_users: vec![],
            allowed_users: vec![],
            blocked_words: vec![],
            commands_only: false,
            subscribers_only: false,
            vip_only: false,
        }
    }
}

impl MessageFilter {
    /// Check if a message should be accepted based on filters
    pub fn accepts(&self, content: &str, username: &str, badges: &[super::message::Badge]) -> bool {
        // Check length
        if let Some(min) = self.min_length {
            if content.len() < min {
                return false;
            }
        }
        if let Some(max) = self.max_length {
            if content.len() > max {
                return false;
            }
        }

        // Check blocked users
        if self
            .blocked_users
            .iter()
            .any(|u| u.eq_ignore_ascii_case(username))
        {
            return false;
        }

        // Check allowed users
        if !self.allowed_users.is_empty()
            && !self
                .allowed_users
                .iter()
                .any(|u| u.eq_ignore_ascii_case(username))
        {
            return false;
        }

        // Check blocked words
        let content_lower = content.to_lowercase();
        if self
            .blocked_words
            .iter()
            .any(|w| content_lower.contains(&w.to_lowercase()))
        {
            return false;
        }

        // Check commands only
        if self.commands_only && !content.starts_with('!') && !content.starts_with('/') {
            return false;
        }

        // Check subscriber-only
        if self.subscribers_only {
            let has_sub = badges.iter().any(|b| {
                b.id.contains("subscriber")
                    || b.id.contains("subscription")
                    || b.name.to_lowercase().contains("sub")
            });
            if !has_sub {
                return false;
            }
        }

        // Check VIP-only
        if self.vip_only {
            let has_vip = badges
                .iter()
                .any(|b| b.id.contains("vip") || b.name.to_lowercase().contains("vip"));
            if !has_vip {
                return false;
            }
        }

        true
    }
}
