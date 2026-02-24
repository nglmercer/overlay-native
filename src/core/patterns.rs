//! Reusable alert patterns and templates
//!
//! This module provides predefined patterns for common overlay elements
//! ensuring consistency across different platforms and providers.

use super::builder::AlertBuilder;
use super::message::{Alert, AlertStyle, Badge, Layout};

pub struct AlertTemplates;

impl AlertTemplates {
    /// A premium-looking chat message pattern
    pub fn modern_chat(
        id: String,
        username: String,
        content: String,
        color: Option<String>,
        badges: Vec<Badge>,
    ) -> Alert {
        let mut builder = AlertBuilder::new(id).with_layout(Layout::Horizontal);

        // Add badges first
        for badge in badges {
            builder = builder.with_badge(badge.id, badge.name, badge.url);
        }

        // Username and message in a specific style
        builder
            .with_styled_text(format!("{}: ", username), color, Some("bold".to_string()))
            .with_text(content)
            .with_style(AlertStyle {
                padding: Some(12),
                border_radius: Some(8),
                background_color: Some("rgba(0, 0, 0, 0.7)".to_string()),
                ..Default::default()
            })
            .build()
    }

    /// A notification banner (centered, big text)
    pub fn notification_banner(id: String, title: String, subtitle: Option<String>) -> Alert {
        let mut builder = AlertBuilder::new(id)
            .with_layout(Layout::Vertical)
            .with_styled_text(title, Some("#FFFFFF".to_string()), Some("bold".to_string()));

        if let Some(sub) = subtitle {
            builder = builder.with_styled_text(sub, Some("#CCCCCC".to_string()), None);
        }

        builder
            .with_style(AlertStyle {
                padding: Some(20),
                background_color: Some("linear-gradient(90deg, #6441a5, #2a0845)".to_string()), // Twitch colors placeholder
                border_radius: Some(0), // Full width banner style
                ..Default::default()
            })
            .build()
    }

    /// Achievement/Goal reached pattern
    pub fn achievement(id: String, title: String, icon_url: String) -> Alert {
        AlertBuilder::new(id)
            .with_layout(Layout::Horizontal)
            .with_image(icon_url)
            .with_styled_text(
                " UNLOCKED: ",
                Some("#ffd700".to_string()),
                Some("bold".to_string()),
            )
            .with_text(title)
            .with_duration(10)
            .with_style(AlertStyle {
                border_color: Some("#ffd700".to_string()),
                border_radius: Some(25),
                ..Default::default()
            })
            .build()
    }
}
