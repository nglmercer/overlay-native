//! Core module - Platform-agnostic rendering and message handling
//!
//! This module provides the core functionality for rendering overlay elements
//! without any knowledge of specific streaming platforms. All platform-specific
//! logic is handled externally via IPC or WebSocket connections.
//!
//! ## Architecture
//!
//! ```text
//! +------------------+     +------------------+     +------------------+
//! |   Platform       |     |   Transport      |     |      Core        |
//! |   (External)     | --> |   (IPC/WS)       | --> |   (Renderer)     |
//! |                  |     |                  |     |                  |
//! | - Twitch Bridge  |     | - Validates      |     | - Renders        |
//! | - Kick Bridge    |     | - Normalizes     |     | - Animates       |
//! | - YouTube Bridge |     | - Schema Check   |     | - Manages        |
//! +------------------+     +------------------+     +------------------+
//! ```
//!
//! ## Usage
//!
//! The core module receives validated messages from the transport layer
//! and handles all rendering decisions independently of the source platform.

pub mod config;
pub mod message;
pub mod renderer;

pub use config::*;
pub use message::*;
pub use renderer::*;
