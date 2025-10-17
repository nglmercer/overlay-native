//! Overlay Native - Library exports for testing and binaries

pub mod config;
pub mod connection;
pub mod emotes;
pub mod mapping;
pub mod platforms;

#[cfg(unix)]
pub mod window;

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod x11;
