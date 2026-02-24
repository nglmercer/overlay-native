//! Platform-agnostic message types
//!
//! This module defines the core message types used throughout the overlay.
//! These types are completely independent of any streaming platform and
//! are used after normalization by the transport layer.

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Get current system time
fn system_time_now() -> SystemTime {
    SystemTime::now()
}

/// Unified message type for all overlay elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverlayElement {
    ChatMessage(ChatMessageElement),
    Gift(GiftElement),
    Image(ImageElement),
}

impl OverlayElement {
    /// Convert to Alert for compatibility with existing code
    pub fn to_alert(&self) -> Alert {
        match self {
            OverlayElement::ChatMessage(msg) => Alert::chat(
                msg.id.clone(),
                msg.username.clone(),
                msg.content.clone(),
                msg.color.clone(),
                msg.badges.clone(),
            ),
            OverlayElement::Gift(gift) => Alert::gift(
                gift.id.clone(),
                gift.from_user.clone(),
                gift.gift_type.clone(),
                gift.message.clone(),
            ),
            OverlayElement::Image(img) => {
                let mut components = Vec::new();
                if let Some(url) = &img.url {
                    components.push(AlertComponent::Image {
                        url: url.clone(),
                        width: img.width,
                        height: img.height,
                        is_animated: false,
                    });
                }
                components.push(AlertComponent::Text {
                    content: img.name.clone(),
                    color: None,
                    weight: None,
                    style: None,
                    size: None,
                });
                Alert {
                    id: img.id.clone(),
                    components,
                    layout: Layout::Vertical,
                    style: AlertStyle::default(),
                    timestamp: SystemTime::now(),
                    duration: None,
                }
            }
        }
    }
}

/// Chat message element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageElement {
    pub id: String,
    pub username: String,
    pub content: String,
    pub color: Option<String>,
    pub badges: Vec<Badge>,
}

/// Gift element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftElement {
    pub id: String,
    pub from_user: String,
    pub gift_type: String,
    pub message: Option<String>,
}

/// Image element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageElement {
    pub id: String,
    pub name: String,
    pub url: Option<String>,
    pub sender: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Generic Alert structure for flexible overlay elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Unique identifier
    pub id: String,

    /// List of components to render
    pub components: Vec<AlertComponent>,

    /// Layout for the components
    pub layout: Layout,

    /// Custom style for this alert
    pub style: AlertStyle,

    /// Timestamp when created
    #[serde(default = "system_time_now")]
    pub timestamp: SystemTime,

    /// Custom duration override
    pub duration: Option<u64>,
}

impl Alert {
    /// Create a standard chat message alert
    pub fn chat(
        id: String,
        username: String,
        content: String,
        color: Option<String>,
        badges: Vec<Badge>,
    ) -> Self {
        let mut components = Vec::new();

        // Add badges
        for badge in badges {
            components.push(AlertComponent::Badge {
                id: badge.id,
                name: badge.name,
                url: badge.url,
            });
        }

        // Add username
        components.push(AlertComponent::Text {
            content: username,
            color,
            weight: Some("bold".to_string()),
            style: None,
            size: None,
        });

        // Add separator (for horizontal layout maybe, but here we stay vertical)
        // Add content
        components.push(AlertComponent::Text {
            content,
            color: None,
            weight: None,
            style: None,
            size: None,
        });

        Self {
            id,
            components,
            layout: Layout::Vertical,
            style: AlertStyle::default(),
            timestamp: SystemTime::now(),
            duration: None,
        }
    }

    /// Create a standard gift alert
    pub fn gift(id: String, from: String, gift_desc: String, message: Option<String>) -> Self {
        let mut components = Vec::new();

        // Gift Icon/Header
        components.push(AlertComponent::Text {
            content: format!("🎁 {} gifted {}!", from, gift_desc),
            color: Some("#ffd700".to_string()), // Gold
            weight: Some("bold".to_string()),
            style: None,
            size: Some(16),
        });

        if let Some(msg) = message {
            components.push(AlertComponent::Text {
                content: msg,
                color: None,
                weight: None,
                style: None,
                size: None,
            });
        }

        Self {
            id,
            components,
            layout: Layout::Vertical,
            style: AlertStyle {
                background_color: Some("#2c3e50".to_string()),
                border_color: Some("#ffd700".to_string()),
                border_radius: Some(12),
                ..Default::default()
            },
            timestamp: SystemTime::now(),
            duration: Some(15),
        }
    }
}

/// Components that can make up an Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertComponent {
    /// Text component
    Text {
        content: String,
        color: Option<String>,
        weight: Option<String>,
        style: Option<String>,
        size: Option<u32>,
    },
    /// Image component (supports GIFs)
    Image {
        url: String,
        width: Option<u32>,
        height: Option<u32>,
        is_animated: bool,
    },
    /// Badge/Icon component
    Badge {
        id: String,
        name: String,
        url: Option<String>,
    },
}

/// Layout options for Alert components
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    #[default]
    Vertical,
    Horizontal,
    Stacked,
}

/// Styling options for Alerts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AlertStyle {
    pub background_color: Option<String>,
    pub border_color: Option<String>,
    pub border_radius: Option<u32>,
    pub padding: Option<u32>,
    pub opacity: Option<f32>,
    pub custom_css: Option<String>,
}

/// Badge representation (user roles, subscriptions, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    /// Badge ID
    pub id: String,

    /// Badge name
    pub name: String,

    /// URL to badge image
    pub url: Option<String>,

    /// Badge title/description
    pub title: Option<String>,
}
