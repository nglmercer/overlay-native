//! Transport layer module for receiving messages from external platform handlers
//!
//! This module provides IPC and WebSocket transport mechanisms for receiving
//! messages from external platform handlers. The overlay is now platform-agnostic
//! and receives messages via these transport connections.

pub mod bridge;
pub mod ipc;
pub mod schema;
pub mod websocket;

pub use schema::*;
pub use bridge::*;
