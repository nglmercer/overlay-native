//! Overlay Native - Platform-agnostic overlay rendering system
//!
//! This library provides a modular, platform-agnostic overlay system for streaming.
//! The architecture separates concerns into distinct layers:
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    EXTERNAL PLATFORM SERVICES                    │
//! │   (Twitch Bridge, Kick Bridge, YouTube Bridge, Custom Clients)  │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      TRANSPORT LAYER                            │
//! │              (IPC, WebSocket, HTTP API)                         │
//! │   - Message validation via schema                               │
//! │   - Protocol-agnostic interface                                 │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        CORE LAYER                               │
//! │              (Platform-Agnostic Rendering)                      │
//! │   - Message types and validation                                │
//! │   - Core renderer with filtering                                │
//! │   - Configuration management                                    │
//! └────────────────────────────┬────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                       RENDER LAYER                              │
//! │              (Platform-Specific Rendering)                      │
//! │   - Linux: GTK/X11                                              │
//! │   - Windows: Win32/GDI                                          │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`core`] - Platform-agnostic rendering logic and message types
//! - [`transport`] - IPC and WebSocket transport for receiving messages
//! - [`render`] - Platform-specific window rendering
//! - [`config`] - Configuration management
//!
//! ## Usage
//!
//! The overlay receives messages from external handlers via the
//! transport layer. Messages are validated, normalized, and rendered
//! using the platform-specific render layer.

// Allow dead code for modules that provide APIs for future use
#![allow(dead_code)]

// Core modules - platform-agnostic
pub mod config;
pub mod core;

// Transport layer - receiving messages from external sources
pub mod transport;

// Rendering - platform-specific window management
pub mod render;

// Re-exports for convenience
pub use core::{ChatMessageElement, CoreRenderer, GiftElement, ImageElement, OverlayElement};
#[cfg(unix)]
pub use render::GtkWindow;
pub use render::{PlatformWindow, WindowConfig};
pub use transport::{bridge::TransportBridge, ChatMessagePayload, GiftPayload, IncomingMessage};
