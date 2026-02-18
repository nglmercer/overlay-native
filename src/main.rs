//! Overlay Native - Main Entry Point
//!
//! This is the main entry point for the overlay application.
//! The application is now platform-agnostic and receives messages
//! via the transport layer (IPC/WebSocket).

mod config;
mod connection;
mod core;
mod emotes;
mod mapping;
mod platforms;
mod render;
mod transport;

#[cfg(unix)]
mod window;

#[cfg(windows)]
mod windows;

#[cfg(target_os = "linux")]
pub mod x11;

#[cfg(target_os = "linux")]
extern crate gdkx11;

#[cfg(target_os = "linux")]
extern crate x11rb;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use anyhow::Result;
use rand::seq::SliceRandom;
use tokio::sync::broadcast;

#[cfg(unix)]
use gtk::prelude::*;

#[cfg(windows)]
use crate::windows::process_messages;

use crate::config::Config;
use crate::connection::{ConnectionInfo, PlatformManager};
use crate::core::CoreRenderer;
use crate::emotes::EmoteSystem;
use crate::mapping::MappingSystem;
use crate::platforms::CredentialManager;
use crate::render::WindowConfig;

/// Application events for the emitter system
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
enum AppEvent {
    MessageReceived(Box<connection::ChatMessage>),
    WindowUpdate,
    Shutdown,
}

/// Event emitter for decoupled communication
struct EventEmitter {
    sender: broadcast::Sender<AppEvent>,
}

impl EventEmitter {
    fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self { sender }
    }

    fn emit(&self, event: AppEvent) -> Result<()> {
        self.sender.send(event)?;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }
}

/// Main application state
struct AppState {
    config: Config,
    platform_manager: Arc<RwLock<PlatformManager>>,
    emote_system: Arc<RwLock<EmoteSystem>>,
    mapping_system: Arc<RwLock<MappingSystem>>,
    credential_manager: Arc<CredentialManager>,
    event_emitter: Arc<EventEmitter>,
    core_renderer: Arc<RwLock<CoreRenderer>>,
}

impl AppState {
    async fn new() -> Result<Self> {
        println!("[CONFIG] Loading configuration...");
        let config = Config::load_default().unwrap_or_else(|e| {
            eprintln!("[CONFIG] Error loading config: {}, using defaults", e);
            Config::default()
        });

        println!("[CONFIG] ✅ Configuration loaded successfully");
        println!(
            "[CONFIG] Enabled platforms: {:?}",
            config.get_enabled_platforms()
        );

        let platform_manager = Arc::new(RwLock::new(PlatformManager::new()));
        let emote_system = Arc::new(RwLock::new(EmoteSystem::new(config.emotes.clone())));
        let mapping_system = Arc::new(RwLock::new(MappingSystem::default()));
        let credential_manager = Arc::new(CredentialManager::new());
        let event_emitter = Arc::new(EventEmitter::new());
        let core_renderer = Arc::new(RwLock::new(CoreRenderer::new()));

        Ok(Self {
            config,
            platform_manager,
            emote_system,
            mapping_system,
            credential_manager,
            event_emitter,
            core_renderer,
        })
    }

    async fn start_connections(&self) -> Result<()> {
        let mut manager = self.platform_manager.write().await;
        let enabled_connections = self.config.get_enabled_connections();

        println!(
            "[CONNECTIONS] Starting {} connections",
            enabled_connections.len()
        );

        for connection in enabled_connections {
            println!(
                "[CONNECTIONS] 🔄 Starting: {} ({})",
                connection.id, connection.platform
            );

            manager.add_connection(ConnectionInfo {
                id: connection.id.clone(),
                platform: connection.platform.clone(),
                channel: connection.channel.clone(),
                enabled: connection.enabled,
                display_name: connection.display_name.clone(),
            });

            match manager.start_connection(&connection.id).await {
                Ok(_) => println!(
                    "✅ Connected to '{}' on {}",
                    connection.channel, connection.platform
                ),
                Err(e) => eprintln!("❌ Failed to connect to '{}': {}", connection.channel, e),
            }
        }

        Ok(())
    }

    async fn preload_emotes(&self) -> Result<()> {
        let mut emote_system = self.emote_system.write().await;

        println!("[EMOTES] Preloading global emotes...");
        match emote_system.preload_global_emotes().await {
            Ok(_) => println!("[EMOTES] ✅ Global emotes preloaded"),
            Err(e) => println!("[EMOTES] ⚠️ Failed to preload: {}", e),
        }

        Ok(())
    }

    async fn start_message_processor(&self) {
        let event_emitter = self.event_emitter.clone();
        let platform_manager = self.platform_manager.clone();

        tokio::spawn(async move {
            let mut pm = platform_manager.write().await;
            loop {
                if let Some(message) = pm.next_message().await {
                    if let Err(e) = event_emitter.emit(AppEvent::MessageReceived(Box::new(message))) {
                        eprintln!("⚠️ Failed to emit message event: {}", e);
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            platform_manager: self.platform_manager.clone(),
            emote_system: self.emote_system.clone(),
            mapping_system: self.mapping_system.clone(),
            credential_manager: self.credential_manager.clone(),
            event_emitter: self.event_emitter.clone(),
            core_renderer: self.core_renderer.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Overlay Native...");

    // Initialize application state
    let state = AppState::new().await?;

    // Initialize transport layer
    println!("📡 Initializing transport layer...");

    // Preload emotes
    state.preload_emotes().await?;

    // Start platform connections
    state.start_connections().await?;

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
        let display = gdk::Display::default().expect("No default display");
        let monitor = display.primary_monitor().expect("No primary monitor");
        let geom = monitor.geometry();
        (geom.width(), geom.height())
    };

    #[cfg(windows)]
    let (monitor_width, monitor_height) = {
        let geom = crate::windows::get_monitor_geometry();
        (geom.width, geom.height)
    };

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

    // Start message processor
    state.start_message_processor().await;
    println!("📡 Background services started");

    // Subscribe to events
    let mut event_rx = state.event_emitter.subscribe();
    let mut position_idx = 0;

    println!("🚀 Starting main event loop...");

    // Main loop
    loop {
        #[cfg(unix)]
        let continue_loop = gtk::main_iteration_do(false);

        #[cfg(windows)]
        let continue_loop = process_messages();

        if !continue_loop {
            break;
        }

        #[cfg(windows)]
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Process events
        tokio::select! {
            event = event_rx.recv() => {
                if let Ok(AppEvent::MessageReceived(message)) = event {
                    let pos = positions[position_idx];
                    position_idx = (position_idx + 1) % positions.len();

                    // Create window for the message using the new render module
                    #[cfg(unix)]
                    {
                        let window_config = WindowConfig {
                            position: pos,
                            size: (state.config.display.window_size, 80),
                            duration: Duration::from_secs(state.config.window.message_duration_seconds),
                            opacity: state.config.display.opacity,
                            border_radius: state.config.display.border_radius,
                            font_family: state.config.display.font_family.clone(),
                            font_size: state.config.display.font_size,
                        };

                        // Convert to core message type
                        let core_message = crate::core::ChatMessageElement::new(
                            message.id,
                            message.username,
                            message.content,
                        );

                        if let Ok(window) = crate::render::gtk::GtkWindow::from_chat_message(&core_message, &window_config) {
                            window.show();
                        }
                    }

                    #[cfg(windows)]
                    {
                        // Convert to core message type - emotes are already in core format
                        let core_message = crate::core::ChatMessageElement::new(
                            message.id,
                            message.username,
                            message.content,
                        );

                        let _win = crate::windows::WindowsWindow::new(
                            &message.username,
                            &message.content,
                            &message.emotes,
                            pos,
                        );
                    }
                }
            },
            _ = tokio::time::sleep(Duration::from_millis(100)) => {
                // Timer tick
            }
        }
    }

    // Cleanup
    println!("🔄 Shutting down...");
    state
        .platform_manager
        .write()
        .await
        .shutdown()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;
    println!("✅ Shutdown complete");

    Ok(())
}
