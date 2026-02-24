//! Overlay Native - Main Entry Point
//!
//! This is the main entry point for the overlay application.
//! The application is platform-agnostic and receives messages
//! via the transport layer (IPC/WebSocket).

// Allow dead code for code that provides APIs for future use
#![allow(dead_code)]

mod config;
mod core;
mod render;
mod transport;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

use anyhow::Result;
use rand::seq::SliceRandom;

#[cfg(unix)]
use gtk::prelude::*;

use crate::config::Config;
use crate::core::{CoreRenderer, RenderEvent};
use crate::render::WindowConfig;
use crate::transport::{
    ipc::IpcConfig, ipc::IpcServer, websocket::WsConfig, websocket::WsServer, TransportBridge,
};

/// Main application state
struct AppState {
    config: Config,
    core_renderer: Arc<RwLock<CoreRenderer>>,
    bridge: Arc<TransportBridge>,
}

impl AppState {
    async fn new() -> Result<Self> {
        println!("[CONFIG] Loading configuration...");
        let config = Config::load_default().unwrap_or_else(|e| {
            eprintln!("[CONFIG] Error loading config: {}, using defaults", e);
            Config::default()
        });

        println!("[CONFIG] ✅ Configuration loaded successfully");

        let core_renderer = CoreRenderer::new();
        let core_renderer_arc = Arc::new(RwLock::new(core_renderer));
        let bridge = Arc::new(TransportBridge::new(core_renderer_arc.read().await.clone()));

        Ok(Self {
            config,
            core_renderer: core_renderer_arc,
            bridge,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Overlay Native...");

    // Initialize application state
    let state = AppState::new().await?;

    // Initialize transport servers
    let (ws_event_tx, mut ws_event_rx) = mpsc::unbounded_channel();

    // Start WebSocket Server
    if state.config.transport.websocket_enabled {
        let ws_config = WsConfig {
            bind_address: state.config.transport.websocket_bind.clone(),
        };
        let ws_server = WsServer::new(ws_config, ws_event_tx.clone());
        tokio::spawn(async move {
            if let Err(e) = ws_server.start().await {
                eprintln!("[TRANSPORT] WebSocket Server error: {}", e);
            }
        });
    }

    // Start IPC Server
    if state.config.transport.ipc_enabled {
        let ipc_config = IpcConfig {
            socket_path: state.config.transport.ipc_socket_path.clone(),
        };
        let ipc_server = IpcServer::new(ipc_config, ws_event_tx.clone());
        tokio::spawn(async move {
            if let Err(e) = ipc_server.start().await {
                eprintln!("[TRANSPORT] IPC Server error: {}", e);
            }
        });
    }

    // Bridge messages to core
    let bridge = state.bridge.clone();
    tokio::spawn(async move {
        while let Some(event) = ws_event_rx.recv().await {
            if let Err(e) = bridge.handle_ws_event(event).await {
                eprintln!("[BRIDGE] Error processing event: {}", e);
            }
        }
    });

    // Initialize GTK for Linux
    #[cfg(unix)]
    {
        gtk::init().unwrap();

        let styles = gtk::CssProvider::new();
        styles
            .load_from_data(include_bytes!("../style.css"))
            .expect("Cannot load styles file");
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().expect("Cannot get main screen"),
            &styles,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Get monitor geometry
    #[cfg(unix)]
    let (monitor_width, monitor_height) = {
        let display = gdk::Display::default();
        let monitor = display
            .as_ref()
            .and_then(|d| d.primary_monitor().or_else(|| d.monitor(0)));

        if let Some(m) = monitor {
            let geom = m.geometry();
            (geom.width(), geom.height())
        } else {
            println!("[WARN] No monitors detected, using default 1920x1080");
            (1920, 1080)
        }
    };

    #[cfg(windows)]
    let (monitor_width, monitor_height) = { crate::render::win32::get_primary_monitor_geometry() };

    println!("Monitor: {}x{}", monitor_width, monitor_height);

    // Calculate window positions
    let positions = {
        let mut p = Vec::new();
        let grid_size = state.config.display.grid_size;
        let margin = state.config.display.monitor_margin;
        let window_size = state.config.display.window_size;

        let cell_width = (monitor_width - margin * 2 - window_size) / grid_size as i32;
        let cell_height = (monitor_height - margin * 2 - window_size) / grid_size as i32;

        for x in 0..grid_size {
            for y in 0..grid_size {
                p.push((margin + x * cell_width, margin + y * cell_height));
            }
        }
        p.shuffle(&mut rand::thread_rng());
        p
    };

    let mut position_idx = 0;
    println!("🚀 Starting main event loop...");

    // Setup Render Event Receiver
    let (render_tx, mut render_rx) = mpsc::unbounded_channel();
    {
        let mut renderer = state.core_renderer.write().await;
        renderer.set_event_channel(render_tx);
    }

    // Main loop
    loop {
        #[cfg(unix)]
        let continue_loop = gtk::main_iteration_do(false);

        #[cfg(windows)]
        let continue_loop = crate::windows::process_messages();

        if !continue_loop {
            break;
        }

        // Process renderer events
        while let Ok(event) = render_rx.try_recv() {
            if let RenderEvent::ElementQueued(element) = event {
                let pos = positions[position_idx];
                position_idx = (position_idx + 1) % positions.len();

                let window_config = WindowConfig {
                    position: pos,
                    size: (state.config.display.window_size, 80),
                    duration: Duration::from_secs(state.config.window.message_duration_seconds),
                    opacity: state.config.display.opacity,
                    border_radius: state.config.display.border_radius,
                    font_family: state.config.display.font_family.clone(),
                    font_size: state.config.display.font_size,
                };

                match *element {
                    crate::core::OverlayElement::ChatMessage(ref message) => {
                        #[cfg(unix)]
                        if let Ok(window) = crate::render::gtk::GtkWindow::from_chat_message(
                            message,
                            &window_config,
                        ) {
                            window.show();
                        }

                        #[cfg(windows)]
                        {
                            // Windows rendering
                        }
                    }
                    crate::core::OverlayElement::Gift(ref gift) => {
                        #[cfg(unix)]
                        if let Ok(window) =
                            crate::render::gtk::GtkWindow::from_gift(gift, &window_config)
                        {
                            window.show();
                        }
                    }
                    crate::core::OverlayElement::Image(ref image) => {
                        #[cfg(unix)]
                        if let Ok(window) =
                            crate::render::gtk::GtkWindow::from_image(image, &window_config)
                        {
                            window.show();
                        }
                    }
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    println!("✅ Shutdown complete");
    Ok(())
}
