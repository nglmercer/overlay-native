//! Platform-agnostic message types
//!
//! This module defines the core message types used throughout the overlay.
//! These types are completely independent of any streaming platform and
//! are used after normalization by the transport layer.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// Get current system time
fn system_time_now() -> SystemTime {
    SystemTime::now()
}


/// Unified message type for all overlay elements
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "element_type", content = "data", rename_all = "snake_case")]
pub enum OverlayElement {
    /// A chat message (legacy support)
    ChatMessage(ChatMessageElement),

    /// A gift/subscription event (legacy support)
    Gift(GiftElement),

    /// A single image/sticker event (legacy support)
    Image(ImageElement),

    /// A generic alert built with components
    Alert(Alert),
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

/// Components that can make up an Alert
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AlertComponent {
    /// Text component
    Text {
        content: String,
        color: Option<String>,
        weight: Option<String>,
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

/// Chat message element - the most common overlay element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageElement {
    /// Unique identifier for this message
    pub id: String,

    /// The username who sent the message
    pub username: String,

    /// Display name (may differ from username)
    pub display_name: Option<String>,

    /// The message content/text
    pub content: String,

    /// User color in hex format (e.g., "#FF0000")
    pub user_color: Option<String>,

    /// Badges/roles of the user
    #[serde(default)]
    pub badges: Vec<Badge>,

    /// Source platform (informational only)
    #[serde(default)]
    pub platform: Option<String>,

    /// Timestamp when message was received
    #[serde(default = "system_time_now")]
    pub timestamp: SystemTime,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ChatMessageElement {
    /// Create a new chat message element
    pub fn new(id: String, username: String, content: String) -> Self {
        Self {
            id,
            username: username.clone(),
            display_name: None,
            content,
            user_color: None,
            badges: vec![],
            platform: None,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        }
    }

    /// Get the effective display name
    pub fn effective_display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
    }

    /// Convert to a modern Alert
    pub fn to_alert(&self) -> Alert {
        let mut components = Vec::new();
        
        // Add badges
        for badge in &self.badges {
            components.push(AlertComponent::Badge {
                id: badge.id.clone(),
                name: badge.name.clone(),
                url: badge.url.clone(),
            });
        }

        // Add username
        components.push(AlertComponent::Text {
            content: self.effective_display_name().to_string(),
            color: self.user_color.clone(),
            weight: Some("bold".to_string()),
            size: None,
        });

        // Add content
        components.push(AlertComponent::Text {
            content: self.content.clone(),
            color: None,
            weight: None,
            size: None,
        });

        Alert {
            id: self.id.clone(),
            components,
            layout: Layout::Vertical,
            style: AlertStyle::default(),
            timestamp: self.timestamp,
            duration: None,
        }
    }
}

/// Gift/subscription event element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftElement {
    /// Unique identifier
    pub id: String,

    /// User who sent the gift
    pub from_user: String,

    /// User who received the gift (None for community gifts)
    pub to_user: Option<String>,

    /// Type of gift
    pub gift_type: GiftType,

    /// Quantity (subscription months, bits amount, etc.)
    pub amount: Option<u32>,

    /// Subscription tier (Tier 1, Tier 2, Tier 3)
    pub tier: Option<String>,

    /// Optional message from the gifter
    pub message: Option<String>,

    /// Timestamp
    #[serde(default = "system_time_now")]
    pub timestamp: SystemTime,
}

/// Type of gift
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GiftType {
    Subscription,
    GiftSubscription,
    Bits,
    Cheer,
    Donation,
    Other(String),
}

/// Single image/sticker event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageElement {
    /// Unique identifier
    pub id: String,

    /// Image name/code
    pub name: String,

    /// URL to the image
    pub url: String,

    /// Whether the image is animated
    #[serde(default)]
    pub is_animated: bool,

    /// Width in pixels
    pub width: Option<u32>,

    /// Height in pixels
    pub height: Option<u32>,

    /// User who sent the image
    pub sender: Option<String>,

    /// Timestamp
    #[serde(default = "system_time_now")]
    pub timestamp: SystemTime,
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
