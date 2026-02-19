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
    /// A chat message with text, emotes, and badges
    ChatMessage(ChatMessageElement),

    /// A gift/subscription event
    Gift(GiftElement),

    /// A single emote/sticker event
    Emote(EmoteElement),
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

    /// Emotes present in the message
    #[serde(default)]
    pub emotes: Vec<Emote>,

    /// Badges/roles of the user
    #[serde(default)]
    pub badges: Vec<Badge>,

    /// Source platform (informational only, not used for rendering)
    #[serde(default)]
    pub platform: Option<String>,

    /// Source channel (informational only)
    #[serde(default)]
    pub channel: Option<String>,

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
            emotes: vec![],
            badges: vec![],
            platform: None,
            channel: None,
            timestamp: SystemTime::now(),
            metadata: HashMap::new(),
        }
    }

    /// Get the effective display name
    pub fn effective_display_name(&self) -> &str {
        self.display_name.as_deref().unwrap_or(&self.username)
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

    /// Source platform
    #[serde(default)]
    pub platform: Option<String>,

    /// Timestamp
    #[serde(default = "system_time_now")]
    pub timestamp: SystemTime,
}

/// Type of gift
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GiftType {
    /// Subscription gift
    Subscription,

    /// Gifted subscription (to random user)
    GiftSubscription,

    /// Bits/cheer
    Bits,

    /// Cheer (YouTube)
    Cheer,

    /// Donation
    Donation,

    /// Other/unknown type
    Other(String),
}

/// Single emote/sticker event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmoteElement {
    /// Unique identifier
    pub id: String,

    /// Emote name/code
    pub name: String,

    /// URL to the emote image
    pub url: String,

    /// Whether the emote is animated (GIF)
    #[serde(default)]
    pub is_animated: bool,

    /// Width in pixels
    pub width: Option<u32>,

    /// Height in pixels
    pub height: Option<u32>,

    /// User who sent the emote
    pub sender: Option<String>,

    /// Source platform
    #[serde(default)]
    pub platform: Option<String>,

    /// Timestamp
    #[serde(default = "system_time_now")]
    pub timestamp: SystemTime,
}

/// Emote representation (used in chat messages)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emote {
    /// Emote ID
    pub id: String,

    /// Emote name
    pub name: String,

    /// Source of the emote (e.g., "bttv", "7tv", custom)
    pub source: EmoteSource,

    /// Positions in the text where this emote appears
    #[serde(default)]
    pub positions: Vec<TextPosition>,

    /// URL to the emote image
    pub url: Option<String>,

    /// Whether the emote is animated
    #[serde(default)]
    pub is_animated: bool,

    /// Width in pixels
    pub width: Option<u32>,

    /// Height in pixels
    pub height: Option<u32>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: EmoteMetadata,
}

impl Default for Emote {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            source: EmoteSource::Local,
            positions: vec![],
            url: None,
            is_animated: false,
            width: None,
            height: None,
            metadata: EmoteMetadata::default(),
        }
    }
}

/// Emote source/origin
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum EmoteSource {
    /// Platform-specific emote
    Platform(String),

    /// Third-party emote (BTTV, FFZ, 7TV)
    ThirdParty(String),

    /// Local/custom emote
    #[default]
    Local,
}

impl std::fmt::Display for EmoteSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmoteSource::Platform(p) => write!(f, "platform:{}", p),
            EmoteSource::ThirdParty(p) => write!(f, "third_party:{}", p),
            EmoteSource::Local => write!(f, "local"),
        }
    }
}

/// Badge representation (user roles, subscriptions, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    /// Badge ID
    pub id: String,

    /// Badge name
    pub name: String,

    /// Badge version (e.g., "1", "2" for sub tier)
    pub version: String,

    /// URL to badge image
    pub url: Option<String>,

    /// Badge title/description
    pub title: Option<String>,

    /// Badge source
    pub source: EmoteSource,
}

/// Text position (start and end indices)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPosition {
    /// Start index (inclusive)
    pub start: usize,

    /// End index (exclusive)
    pub end: usize,
}

/// Additional emote metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmoteMetadata {
    /// Whether this is a zero-width emote
    #[serde(default)]
    pub is_zero_width: bool,

    /// Whether this is a modifier emote
    #[serde(default)]
    pub modifier: bool,

    /// Emote set ID (for subscriber emotes)
    #[serde(default)]
    pub emote_set_id: Option<String>,

    /// Subscription tier (for subscriber emotes)
    #[serde(default)]
    pub tier: Option<String>,
}

/// Convert from transport schema types to core types
impl From<crate::transport::schema::ChatMessagePayload> for ChatMessageElement {
    fn from(payload: crate::transport::schema::ChatMessagePayload) -> Self {
        Self {
            id: payload.get_or_generate_id(),
            username: payload.username.clone(),
            display_name: payload.display_name.clone(),
            content: payload.content.clone(),
            user_color: payload.user_color.clone(),
            emotes: payload.emotes.into_iter().map(|e| e.into()).collect(),
            badges: payload.badges.into_iter().map(|b| b.into()).collect(),
            platform: payload.platform.clone(),
            channel: payload.channel.clone(),
            timestamp: SystemTime::now(),
            metadata: payload.metadata,
        }
    }
}

impl From<crate::transport::schema::EmotePayload> for Emote {
    fn from(payload: crate::transport::schema::EmotePayload) -> Self {
        Self {
            id: payload.id.clone(),
            name: payload.name.clone(),
            source: EmoteSource::Local, // Will be enriched by emote system
            positions: payload.positions.into_iter().map(|p| p.into()).collect(),
            url: payload.url.clone(),
            is_animated: payload.is_animated,
            width: payload.width,
            height: payload.height,
            metadata: EmoteMetadata::default(),
        }
    }
}

impl From<crate::transport::schema::EmotePosition> for TextPosition {
    fn from(pos: crate::transport::schema::EmotePosition) -> Self {
        Self {
            start: pos.start,
            end: pos.end,
        }
    }
}

impl From<crate::transport::schema::BadgePayload> for Badge {
    fn from(payload: crate::transport::schema::BadgePayload) -> Self {
        Self {
            id: payload.id.clone(),
            name: payload.name.clone(),
            version: "1".to_string(),
            url: payload.url.clone(),
            title: payload.title.clone(),
            source: EmoteSource::Local,
        }
    }
}

impl From<crate::transport::schema::GiftPayload> for GiftElement {
    fn from(payload: crate::transport::schema::GiftPayload) -> Self {
        let gift_type = match payload.gift_type {
            crate::transport::schema::GiftType::Subscription => GiftType::Subscription,
            crate::transport::schema::GiftType::GiftSubscription => GiftType::GiftSubscription,
            crate::transport::schema::GiftType::Bits => GiftType::Bits,
            crate::transport::schema::GiftType::Cheer => GiftType::Cheer,
            crate::transport::schema::GiftType::Donation => GiftType::Donation,
            crate::transport::schema::GiftType::Other(s) => GiftType::Other(s),
        };

        Self {
            id: format!(
                "gift_{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            ),
            from_user: payload.from_user,
            to_user: payload.to_user,
            gift_type,
            amount: payload.amount,
            tier: payload.tier,
            message: payload.message,
            platform: payload.platform,
            timestamp: SystemTime::now(),
        }
    }
}

impl From<crate::transport::schema::EmoteEventPayload> for EmoteElement {
    fn from(payload: crate::transport::schema::EmoteEventPayload) -> Self {
        Self {
            id: payload.id.clone(),
            name: payload.name.clone(),
            url: payload.url.clone(),
            is_animated: payload.is_animated,
            width: payload.width,
            height: payload.height,
            sender: payload.sender,
            platform: None,
            timestamp: SystemTime::now(),
        }
    }
}
