//! Transport layer module for receiving messages from external platform handlers
//!
//! This module provides IPC and WebSocket transport mechanisms for receiving
//! messages from external platform handlers. The overlay is now platform-agnostic
//! and receives messages via these transport connections.
//!
//! ## Architecture
//!
//! ```text
//! External Platforms (Twitch, Kick, YouTube, etc.)
//!         │
//!         ▼
//! ┌─────────────────────────────────────────────────┐
//! │              TRANSPORT LAYER                     │
//! │  ┌─────────┐  ┌───────────┐  ┌──────────────┐  │
//! │  │   IPC   │  │ WebSocket │  │  HTTP/REST   │  │
//! │  └────┬────┘  └─────┬─────┘  └──────┬───────┘  │
//! │       │             │                │          │
//! │       └─────────────┼────────────────┘          │
//! │                     ▼                           │
//! │              ┌───────────┐                      │
//! │              │  Schema   │ (Validation)         │
//! │              └─────┬─────┘                      │
//! └────────────────────┼───────────────────────────┘
//!                      ▼
//!              ┌───────────────┐
//!              │    Bridge     │
//!              └───────┬───────┘
//!                      ▼
//! ┌───────────────────────────────────────────────────┐
//! │                CORE RENDERER                       │
//! │         (Platform-Agnostic Rendering)              │
//! └───────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! The transport layer receives JSON messages that conform to the schema
//! defined in `schema.rs`. Messages are validated, normalized, and then
//! forwarded to the core renderer via the bridge.

pub mod bridge;
pub mod ipc;
pub mod schema;
pub mod websocket;

#[allow(unused_imports)]
pub use bridge::{BridgeError, BridgeEvent, BridgeStatus, TransportBridge};
#[allow(unused_imports)]
pub use schema::*;
