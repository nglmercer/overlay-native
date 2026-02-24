/// Incoming message schema for the transport layer (IPC/WebSocket)
///
/// This module defines the format of messages that clients must send
/// to the overlay. The platform is agnostic: any source can send
/// messages as long as they comply with this schema.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Message validation error
#[derive(Debug, Error)]
pub enum SchemaError {
    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid value for '{field}': {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("Invalid JSON: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Unknown message type: {0}")]
    UnknownMessageType(String),
}

/// Incoming message from any transport (WebSocket, IPC)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum IncomingMessage {
    /// Standard chat message (text and badges)
    ChatMessage(ChatMessagePayload),

    /// Gift/Subscription event (sub, bits, etc.)
    Gift(GiftPayload),

    /// Single image event (sticker, full-screen image)
    Image(ImagePayload),

    /// Ping to keep connection alive
    Ping,

    /// Request for overlay status
    Status,
}

/// Chat message payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessagePayload {
    /// Unique message ID (optional, generated if not provided)
    pub id: Option<String>,

    /// Username (required)
    pub username: String,

    /// Display name (optional, defaults to username)
    pub display_name: Option<String>,

    /// Message content (required)
    pub content: String,

    /// User color in hex format (e.g., "#FF0000")
    pub user_color: Option<String>,

    /// List of user badges
    #[serde(default)]
    pub badges: Vec<BadgePayload>,

    /// Origin platform (informational)
    pub platform: Option<String>,

    /// Origin channel (informational)
    pub channel: Option<String>,

    /// Arbitrary additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Gift/subscription payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftPayload {
    /// Who sent the gift
    pub from_user: String,

    /// Who received the gift (None = random/community)
    pub to_user: Option<String>,

    /// Type of gift (subscription, bits, etc.)
    pub gift_type: GiftType,

    /// Amount (subscription months, bits, etc.)
    pub amount: Option<u32>,

    /// Tier/plan name (Tier 1, Tier 2, etc.)
    pub tier: Option<String>,

    /// Personal message
    pub message: Option<String>,

    /// Origin platform (informational)
    pub platform: Option<String>,
}

/// Gift type
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

/// Single image event payload (sticker, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImagePayload {
    /// Image ID
    pub id: String,

    /// Image name
    pub name: String,

    /// Image URL
    pub url: String,

    /// Whether the image is animated (GIF)
    #[serde(default)]
    pub is_animated: bool,

    /// Width in pixels
    pub width: Option<u32>,

    /// Height in pixels
    pub height: Option<u32>,

    /// User who sent the image (optional)
    pub sender: Option<String>,
}

/// User badge definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadgePayload {
    /// Badge ID
    pub id: String,

    /// Human-readable badge name
    pub name: String,

    /// Badge image URL
    pub url: Option<String>,

    /// Badge title/description
    pub title: Option<String>,
}

/// Overlay response to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum OutgoingMessage {
    /// Acknowledgment of message received
    Ack { id: String },

    /// Response to ping
    Pong,

    /// Current overlay status
    Status(OverlayStatus),

    /// Processing error
    Error { code: String, message: String },
}

/// Overlay status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayStatus {
    /// Number of active windows
    pub active_windows: usize,

    /// Number of connected clients
    pub connected_clients: usize,

    /// Overlay version
    pub version: String,
}

impl IncomingMessage {
    /// Parse a message from JSON and validate it
    pub fn parse_and_validate(raw: &str) -> Result<Self, SchemaError> {
        let msg: IncomingMessage = serde_json::from_str(raw)?;
        msg.validate()?;
        Ok(msg)
    }

    /// Validate business rules for the message
    pub fn validate(&self) -> Result<(), SchemaError> {
        match self {
            IncomingMessage::ChatMessage(payload) => payload.validate(),
            IncomingMessage::Gift(payload) => payload.validate(),
            IncomingMessage::Image(payload) => payload.validate(),
            IncomingMessage::Ping | IncomingMessage::Status => Ok(()),
        }
    }

    /// Decompose the message into its type, ID and data for dynamic processing
    pub fn into_parts(self) -> (String, String, serde_json::Value) {
        let serialized = serde_json::to_value(&self).unwrap_or(serde_json::Value::Null);
        let msg_type = serialized
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let data = serialized
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        let id = match &self {
            IncomingMessage::ChatMessage(p) => p.get_or_generate_id(),
            IncomingMessage::Image(p) => p.id.clone(),
            _ => data
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_else(|| Self::generate_id()),
        };

        (msg_type, id, data)
    }

    /// Generate a unique ID for messages that don't have one
    pub fn generate_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        format!("msg_{}_{}", ts, rand::random::<u32>())
    }
}

impl ChatMessagePayload {
    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.username.trim().is_empty() {
            return Err(SchemaError::MissingField("username".to_string()));
        }

        if self.content.trim().is_empty() {
            return Err(SchemaError::MissingField("content".to_string()));
        }

        if self.username.len() > 256 {
            return Err(SchemaError::InvalidValue {
                field: "username".to_string(),
                reason: "too long (max 256 characters)".to_string(),
            });
        }

        if self.content.len() > 4096 {
            return Err(SchemaError::InvalidValue {
                field: "content".to_string(),
                reason: "too long (max 4096 characters)".to_string(),
            });
        }

        if let Some(color) = &self.user_color {
            if !is_valid_hex_color(color) {
                return Err(SchemaError::InvalidValue {
                    field: "user_color".to_string(),
                    reason: format!("'{}' is not a valid hex color (eg: #FF0000)", color),
                });
            }
        }

        Ok(())
    }

    /// Generate an ID if not provided
    pub fn get_or_generate_id(&self) -> String {
        self.id.clone().unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            format!("msg_{}_{}", ts, rand::random::<u32>())
        })
    }
}

impl GiftPayload {
    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.from_user.trim().is_empty() {
            return Err(SchemaError::MissingField("from_user".to_string()));
        }
        Ok(())
    }
}

impl ImagePayload {
    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.id.trim().is_empty() {
            return Err(SchemaError::MissingField("id".to_string()));
        }
        if self.name.trim().is_empty() {
            return Err(SchemaError::MissingField("name".to_string()));
        }
        if self.url.trim().is_empty() {
            return Err(SchemaError::MissingField("url".to_string()));
        }
        Ok(())
    }
}

/// Validate if a string is a valid hex color (#RGB, #RRGGBB, #RRGGBBAA)
fn is_valid_hex_color(color: &str) -> bool {
    if !color.starts_with('#') {
        return false;
    }
    let hex = &color[1..];
    let valid_length = matches!(hex.len(), 3 | 4 | 6 | 8);
    let valid_chars = hex.chars().all(|c| c.is_ascii_hexdigit());
    valid_length && valid_chars
}

// Tests moved to separate test file for cleaner compilation
