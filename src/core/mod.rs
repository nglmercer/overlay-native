//! Core module - Platform-agnostic rendering and message handling

pub mod builder;
pub mod config;
pub mod factory;
pub mod message;
pub mod patterns;
pub mod renderer;

pub use builder::*;
pub use config::*;
pub use factory::*;
pub use message::*;
pub use patterns::*;
pub use renderer::*;
