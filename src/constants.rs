//! Application constants and enums
//!
//! This module centralizes all magic strings and constants used throughout
//! the application to avoid hardcoded values scattered across the codebase.

use std::time::Duration;

/// IPC configuration constants
pub mod ipc {
    /// Default Unix socket path for IPC communication (Linux/macOS)
    pub const DEFAULT_SOCKET_PATH: &str = "/tmp/overlay-native.sock";

    /// Default Named Pipe path for IPC communication (Windows)
    pub const DEFAULT_PIPE_NAME: &str = r"\\.\pipe\overlay-native";

    /// Protocol name for logging
    pub const PROTOCOL_NAME: &str = "JSON por línea (newline-delimited JSON)";
}

/// WebSocket configuration constants
pub mod websocket {
    /// Default WebSocket bind address
    pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:9001";

    /// Protocol name for logging
    pub const PROTOCOL_NAME: &str = "JSON sobre WebSocket";

    /// Broadcast channel capacity
    pub const BROADCAST_CAPACITY: usize = 256;
}

/// Message type identifiers used in JSON communication
pub mod message_types {
    /// Chat message type identifier
    pub const CHAT_MESSAGE: &str = "chat_message";

    /// Gift/subscription message type identifier
    pub const GIFT: &str = "gift";

    /// Emote event message type identifier
    pub const EMOTE: &str = "emote";

    /// Ping message type identifier
    pub const PING: &str = "ping";

    /// Status request message type identifier
    pub const STATUS: &str = "status";

    /// Acknowledge message type identifier
    pub const ACK: &str = "ack";

    /// Pong response type identifier
    pub const PONG: &str = "pong";

    /// Error message type identifier
    pub const ERROR: &str = "error";
}

/// Error codes used in error responses
pub mod error_codes {
    /// Validation error code
    pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";

    /// Connection error code
    pub const CONNECTION_ERROR: &str = "CONNECTION_ERROR";

    /// Authentication error code
    pub const AUTH_ERROR: &str = "AUTH_ERROR";

    /// Internal error code
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
}

/// JSON field names used in message schemas
pub mod field_names {
    // Chat message fields
    pub const ID: &str = "id";
    pub const USERNAME: &str = "username";
    pub const DISPLAY_NAME: &str = "display_name";
    pub const CONTENT: &str = "content";
    pub const USER_COLOR: &str = "user_color";
    pub const EMOTES: &str = "emotes";
    pub const BADGES: &str = "badges";
    pub const PLATFORM: &str = "platform";
    pub const CHANNEL: &str = "channel";
    pub const METADATA: &str = "metadata";

    // Gift fields
    pub const FROM_USER: &str = "from_user";
    pub const TO_USER: &str = "to_user";
    pub const GIFT_TYPE: &str = "gift_type";
    pub const AMOUNT: &str = "amount";
    pub const TIER: &str = "tier";
    pub const MESSAGE: &str = "message";

    // Emote fields
    pub const NAME: &str = "name";
    pub const URL: &str = "url";
    pub const IS_ANIMATED: &str = "is_animated";
    pub const WIDTH: &str = "width";
    pub const HEIGHT: &str = "height";
    pub const SENDER: &str = "sender";
    pub const POSITIONS: &str = "positions";

    // Badge fields
    pub const BADGE_ID: &str = "id";
    pub const BADGE_NAME: &str = "name";
    pub const VERSION: &str = "version";
    pub const TITLE: &str = "title";

    // Position fields
    pub const START: &str = "start";
    pub const END: &str = "end";

    // Outgoing message fields
    pub const TYPE: &str = "type";
    pub const DATA: &str = "data";
    pub const CODE: &str = "code";

    // Overlay element fields
    pub const ELEMENT_TYPE: &str = "element_type";

    // Status fields
    pub const ACTIVE_WINDOWS: &str = "active_windows";
    pub const CONNECTED_CLIENTS: &str = "connected_clients";
    pub const VERSION: &str = "version";
}

/// Default configuration values
pub mod defaults {
    /// Default message display duration in seconds
    pub const MESSAGE_DURATION_SECONDS: u64 = 10;

    /// Maximum number of active windows/overlays
    pub const MAX_WINDOWS: usize = 50;

    /// Default fade in duration in milliseconds
    pub const FADE_IN_DURATION_MS: u64 = 300;

    /// Default fade out duration in milliseconds
    pub const FADE_OUT_DURATION_MS: u64 = 500;

    /// Default animation speed multiplier
    pub const ANIMATION_SPEED_MULTIPLIER: f32 = 1.0;

    /// Default font family
    pub const FONT_FAMILY: &str = "Segoe UI, Arial, sans-serif";

    /// Default font size
    pub const FONT_SIZE: u32 = 14;

    /// Default background color
    pub const BACKGROUND_COLOR: &str = "#1a1a2e";

    /// Default text color
    pub const TEXT_COLOR: &str = "#ffffff";

    /// Default username color
    pub const USERNAME_COLOR: &str = "#9147ff";

    /// Default border radius in pixels
    pub const BORDER_RADIUS: u32 = 8;

    /// Default window opacity
    pub const OPACITY: f32 = 0.9;

    /// Default monitor margin in pixels
    pub const MONITOR_MARGIN: i32 = 10;

    /// Default window size in pixels
    pub const WINDOW_SIZE: i32 = 400;

    /// Default grid size for positioning
    pub const GRID_SIZE: i32 = 10;

    /// Default maximum message length
    pub const MAX_MESSAGE_LENGTH: usize = 500;

    /// Default maximum emotes per message
    pub const MAX_EMOTES_PER_MESSAGE: usize = 50;

    /// Default maximum badges per message
    pub const MAX_BADGES_PER_MESSAGE: usize = 10;

    /// Default deduplication window in milliseconds
    pub const DEDUPLICATION_WINDOW_MS: u64 = 5000;

    /// Default maximum username length
    pub const MAX_USERNAME_LENGTH: usize = 256;

    /// Default maximum content length
    pub const MAX_CONTENT_LENGTH: usize = 4096;

    /// Default badge version
    pub const BADGE_VERSION: &str = "1";
}

/// Timing constants
pub mod timing {
    /// Default message polling interval in milliseconds
    pub const MESSAGE_POLL_INTERVAL_MS: u64 = 5;

    /// Default Windows message processing interval in milliseconds
    pub const WINDOWS_POLL_INTERVAL_MS: u64 = 10;

    /// Default timer tick interval in milliseconds
    pub const TIMER_TICK_INTERVAL_MS: u64 = 100;

    /// Event channel capacity
    pub const EVENT_CHANNEL_CAPACITY: usize = 1000;
}

/// ID prefix constants
pub mod id_prefixes {
    /// Prefix for generated message IDs
    pub const MESSAGE: &str = "msg_";

    /// Prefix for generated gift IDs
    pub const GIFT: &str = "gift_";
}

/// Badge identifier patterns for filtering
pub mod badge_patterns {
    /// Patterns that indicate a subscriber
    pub const SUBSCRIBER_PATTERNS: &[&str] = &["subscriber", "subscription"];

    /// Patterns that indicate a VIP
    pub const VIP_PATTERN: &str = "vip";

    /// Sub pattern for subscriber check
    pub const SUB_PATTERN: &str = "sub";
}

/// Command prefixes for chat commands
pub mod commands {
    /// Chat command prefixes
    pub const PREFIXES: &[char] = &['!', '/'];
}
