mod config;
mod connection;
mod emotes;
mod mapping;
mod platforms;

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

use rand::seq::SliceRandom;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::connection::{ConnectionInfo, PlatformManager};
use crate::emotes::EmoteSystem;
use crate::mapping::MappingSystem;
use crate::platforms::{CredentialManager, PlatformFactory};

use anyhow::Result;
use tokio::sync::broadcast;

#[cfg(windows)]
use winapi::shared::windef::{HWND, RECT};
#[cfg(windows)]
use winapi::um::winuser::{GetClientRect, GetWindowLongPtrW, InvalidateRect, GWLP_USERDATA};

/// Application events for the emitter system
#[derive(Debug, Clone)]
enum AppEvent {
    MessageReceived(connection::ChatMessage),
    WindowUpdate,
    Shutdown,
}

/// Event emitter for decoupled communication
struct EventEmitter {
    sender: broadcast::Sender<AppEvent>,
}

/// Simple window tracker for basic management
struct WindowTracker {
    #[cfg(unix)]
    windows: Arc<RwLock<Vec<SpawnedWindow>>>,
    #[cfg(windows)]
    windows: Arc<RwLock<Vec<WindowsWindow>>>,
}

impl WindowTracker {
    fn new() -> Self {
        #[cfg(unix)]
        {
            Self {
                windows: Arc::new(RwLock::new(Vec::new())),
            }
        }
        #[cfg(windows)]
        {
            Self {
                windows: Arc::new(RwLock::new(Vec::new())),
            }
        }
    }

    #[cfg(unix)]
    async fn add_window(&self, window: SpawnedWindow) {
        let mut windows = self.windows.write().await;
        windows.push(window);
    }

    #[cfg(windows)]
    async fn add_window(&self, window: WindowsWindow) {
        let mut windows = self.windows.write().await;
        windows.push(window);
    }

    async fn cleanup_expired(&self) {
        let now = tokio::time::Instant::now();
        let max_time = Duration::from_secs(10);

        #[cfg(unix)]
        {
            let mut windows = self.windows.write().await;
            windows.retain(|w| {
                let elapsed = now - w.created;
                if elapsed >= max_time {
                    w.w.close();
                    false
                } else {
                    let progress = elapsed.as_secs_f64() / max_time.as_secs_f64();
                    w.progress.set_fraction(progress);
                    true
                }
            });
        }

        #[cfg(windows)]
        {
            let mut windows = self.windows.write().await;
            let mut windows_to_remove = Vec::new();

            // Update progress for all windows and identify expired ones
            for (i, w) in windows.iter_mut().enumerate() {
                let elapsed = now - w.created;
                if elapsed >= max_time {
                    windows_to_remove.push(i);
                } else {
                    let progress = elapsed.as_secs_f64() / max_time.as_secs_f64();

                    // Only update if progress changed significantly (2% or more)
                    let progress_diff = (w.progress - progress).abs();
                    if progress_diff >= 0.02 {
                        // Update progress
                        w.progress = progress;
                        unsafe {
                            // Update the stored window data
                            let window_data_ptr = GetWindowLongPtrW(w.hwnd, GWLP_USERDATA)
                                as *mut crate::windows::WindowData;
                            if !window_data_ptr.is_null() {
                                (*window_data_ptr).progress = progress;
                            }

                            // Only invalidate the progress bar area to avoid flickering
                            let mut client_rect = RECT {
                                left: 0,
                                top: 0,
                                right: 0,
                                bottom: 0,
                            };
                            GetClientRect(w.hwnd, &mut client_rect);

                            let progress_rect = RECT {
                                left: 10,
                                top: client_rect.bottom - 15,
                                right: client_rect.right - 10,
                                bottom: client_rect.bottom - 5,
                            };
                            InvalidateRect(w.hwnd, &progress_rect, 0); // Don't erase background
                        }
                    }
                }
            }

            // Remove expired windows (in reverse order to maintain indices)
            for &i in windows_to_remove.iter().rev() {
                let w = windows.remove(i);
                w.close();
            }
        }
    }
}

impl Clone for WindowTracker {
    fn clone(&self) -> Self {
        Self {
            windows: self.windows.clone(),
        }
    }
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

#[cfg(unix)]
use gdk::prelude::MonitorExt;
#[cfg(unix)]
use gtk::prelude::{CssProviderExt, GtkWindowExt, ProgressBarExt};
#[cfg(unix)]
use window::{get_gdk_monitor, spawn_window, SpawnedWindow};

#[cfg(windows)]
use windows::{get_monitor_geometry, process_messages, WindowsWindow};

/// Estado principal de la aplicación
struct AppState {
    config: Config,
    platform_manager: Arc<RwLock<PlatformManager>>,
    emote_system: Arc<RwLock<EmoteSystem>>,
    mapping_system: Arc<RwLock<MappingSystem>>,
    platform_factory: Arc<PlatformFactory>,
    credential_manager: Arc<CredentialManager>,
    event_emitter: Arc<EventEmitter>,
    window_tracker: Arc<WindowTracker>,
}

impl AppState {
    async fn new() -> Result<Self> {
        // Cargar configuración
        println!("[CONFIG] Loading configuration...");
        let config = Config::load_default().unwrap_or_else(|e| {
            eprintln!("[CONFIG] Error loading config: {}, using defaults", e);
            Config::default()
        });

        // Mostrar información de configuración cargada
        println!("[CONFIG] ✅ Configuration loaded successfully");
        println!("[CONFIG] Enabled platforms: {:?}", config.get_enabled_platforms());
        println!("[CONFIG] Enabled connections: {}", config.get_enabled_connections().len());
        for conn in config.get_enabled_connections() {
            println!("[CONFIG]   - {} ({} platform, channel: '{}')",
                     conn.id, conn.platform, conn.channel);
        }

        // Crear sistemas
        let platform_manager = Arc::new(RwLock::new(PlatformManager::new()));
        let emote_system = Arc::new(RwLock::new(EmoteSystem::new(config.emotes.clone())));
        let mapping_system = Arc::new(RwLock::new(MappingSystem::default()));
        let platform_factory = Arc::new(PlatformFactory::new());
        let credential_manager = Arc::new(CredentialManager::new());

        let event_emitter = Arc::new(EventEmitter::new());
        let window_tracker = Arc::new(WindowTracker::new());

        Ok(Self {
            config,
            platform_manager,
            emote_system,
            mapping_system,
            platform_factory,
            credential_manager,
            event_emitter,
            window_tracker,
        })
    }

    async fn initialize_platforms(&self) -> Result<()> {
        let mut manager = self.platform_manager.write().await;
        let enabled_platforms = self.config.get_enabled_platforms();
        eprintln!("[DEBUG] Enabled platforms: {:?}", enabled_platforms);

        for platform_name in enabled_platforms {
            if let Some(platform_config) = self.config.get_platform_config(platform_name) {
                // Crear instancia de la plataforma
                let platform = self
                    .platform_factory
                    .create_platform(
                        &platform_config.platform_type.to_string(),
                        platform_config.clone(),
                    )
                    .await?;

                // Registrar plataforma en el manager
                manager.register_platform(platform_name.to_string(), platform);

                // Guardar credenciales
                self.credential_manager
                    .store_credentials(
                        platform_name.to_string(),
                        platform_config.credentials.clone(),
                    )
                    .await;

                println!("✅ Platform {} initialized", platform_name);
            }
        }

        Ok(())
    }

    async fn start_connections(&self) -> Result<()> {
        let mut manager = self.platform_manager.write().await;
        let enabled_connections = self.config.get_enabled_connections();

        println!("[CONNECTIONS] Starting connections. Found {} enabled connections",
                 enabled_connections.len()
        );

        for connection in enabled_connections {
            println!(
                "[CONNECTIONS] 🔄 Processing connection: {} (platform: {}, channel: '{}')",
                connection.id, connection.platform, connection.channel
            );

            // Agregar conexión al manager
            manager.add_connection(ConnectionInfo {
                id: connection.id.clone(),
                platform: connection.platform.clone(),
                channel: connection.channel.clone(),
                enabled: connection.enabled,
                display_name: connection.display_name.clone(),
            });

            // Iniciar conexión
            println!("[CONNECTIONS] 🚀 Attempting to start connection: {}", connection.id);
            match manager.start_connection(&connection.id).await {
                Ok(_) => {
                    println!(
                        "✅ Connected to '{}' on {} ({})",
                        connection.channel, connection.platform, connection.id
                    );
                    println!("[CONNECTIONS] ✅ Successfully started connection: {}", connection.id);
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to connect to '{}' on {}: {}",
                        connection.channel, connection.platform, e
                    );
                    eprintln!(
                        "[CONNECTIONS] ❌ Connection start failed for {}: {}",
                        connection.id, e
                    );
                }
            }
        }

        Ok(())
    }

    async fn preload_emotes(&self) -> Result<()> {
        let mut emote_system = self.emote_system.write().await;

        println!("🔄 Preloading global emotes...");
        match emote_system.preload_global_emotes().await {
            Ok(_) => println!("✅ Global emotes preloaded"),
            Err(e) => {
                println!("⚠️ Failed to preload global emotes: {}", e);
                println!("📝 Continuing without emote cache...");
            }
        }

        Ok(())
    }

    async fn process_message(
        &self,
        mut message: connection::ChatMessage,
    ) -> Result<connection::ChatMessage> {
        // Apply filters if necessary
        if let Some(connection) = self
            .config
            .connections
            .iter()
            .find(|conn| conn.platform == message.platform && conn.channel == message.channel)
        {
            let mut manager = self.platform_manager.write().await;
            if let Some(platform) = manager.get_platform_mut(&message.platform) {
                if !connection.filters.commands_only
                    || message.content.starts_with('!')
                    || message.content.starts_with('/')
                {
                    // Apply filters
                    if !platform
                        .lock()
                        .await
                        .apply_message_filters(&mut message, &connection.filters)
                    {
                        return Err(anyhow::anyhow!("Message filtered out"));
                    }
                } else {
                    return Err(anyhow::anyhow!("Commands only filter active"));
                }
            }
        }

        // Parse additional emotes if necessary
        let mut emote_system = self.emote_system.write().await;
        if let Ok(additional_emotes) = emote_system
            .parse_message_emotes(
                &message.content,
                &message.platform,
                &message.channel,
                "", // Raw emote data if exists
            )
            .await
        {
            message.emotes.extend(additional_emotes);
        }

        // Apply data mapping
        let mut mapping_system = self.mapping_system.write().await;
        let raw_message = mapping::RawPlatformMessage {
            platform: message.platform.clone(),
            channel: message.channel.clone(),
            raw_data: serde_json::to_value(&message)?,
            timestamp: chrono::Utc::now(),
            message_id: Some(message.id.clone()),
        };

        if let Ok(mapped_message) = mapping_system.map_message(&raw_message).await {
            // Actualizar mensaje con datos mapeados
            message.emotes = mapped_message.emotes;
            message.badges = mapped_message.badges;
            message.user_color = mapped_message
                .metadata
                .custom_data
                .get("user_color")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }

        eprintln!(
            "[DEBUG] Message processing complete: {} - {}",
            message.username, message.content
        );
        Ok(message)
    }

    /// Start background message processor that emits events
    async fn start_message_processor(&self) {
        let event_emitter = self.event_emitter.clone();
        let platform_manager = self.platform_manager.clone();

        tokio::spawn(async move {
            let mut pm = platform_manager.write().await;
            loop {
                if let Some(message) = pm.next_message().await {
                    // Emit event directly without complex processing
                    if let Err(e) = event_emitter.emit(AppEvent::MessageReceived(message)) {
                        eprintln!("⚠️ Failed to emit message event: {}", e);
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
    }

    // Window management is now handled internally by WindowManager
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            platform_manager: self.platform_manager.clone(),
            emote_system: self.emote_system.clone(),
            mapping_system: self.mapping_system.clone(),
            platform_factory: self.platform_factory.clone(),
            credential_manager: self.credential_manager.clone(),
            event_emitter: self.event_emitter.clone(),
            window_tracker: self.window_tracker.clone(),
        }
    }
}

#[tokio::main(flavor = "current_thread")]
#[cfg(unix)]
use gdk::Screen;
#[cfg(unix)]
use gtk::{StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};
use rand::prelude::*;

#[cfg(unix)]
fn get_gdk_monitor() -> gdk::Monitor {
    let display = gdk::Display::default().expect("Cannot get default display");
    display
        .monitor_at_point(0, 0)
        .expect("Cannot get monitor at point")
}

#[cfg(unix)]
fn get_monitor_geometry() -> gdk::Rectangle {
    let monitor = get_gdk_monitor();
    monitor.geometry()
}

#[cfg(unix)]
fn spawn_window(
    username: &str,
    message: &str,
    emotes: &[crate::emotes::RenderedEmote],
    position: (i32, i32),
    monitor_geometry: gdk::Rectangle,
) -> SpawnedWindow {
    // Stub implementation for Unix window spawning
    SpawnedWindow {
        w: gtk::Window::new(gtk::WindowType::Toplevel),
        created: Instant::now(),
        progress: gtk::ProgressBar::new(),
    }
}

#[cfg(unix)]
struct SpawnedWindow {
    w: gtk::Window,
    created: Instant,
    progress: gtk::ProgressBar,
}

#[cfg(windows)]
struct PlatformMessage {
    // Stub struct for Windows platform messages
}

#[cfg(unix)]
fn handle_message(
    message: crate::connection::ChatMessage,
    position: (i32, i32),
    monitor_geometry: gdk::Rectangle,
    _config: &crate::config::Config,
) -> SpawnedWindow {
    // Stub implementation for message handling
    SpawnedWindow {
        w: crate::window::Window::new(gtk::WindowType::Toplevel, position.0, position.1),
        created: Instant::now(),
        progress: gtk::ProgressBar::new(),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Starting Overlay Native...");
    eprintln!("[DEBUG] Main function started");

    // Inicializar estado de la aplicación
    eprintln!("[DEBUG] Creating AppState...");
    let state = AppState::new().await?;
    eprintln!("[DEBUG] AppState created successfully");

    // Inicializar plataformas
    state.initialize_platforms().await?;

    // Precargar emotes
    state.preload_emotes().await?;

    // Iniciar conexiones
    state.start_connections().await?;

    // Configuración de UI
    #[cfg(unix)]
    {
        gtk::init().unwrap();

        let styles = gtk::CssProvider::new();
        styles
            .load_from_data(include_bytes!("../style.css"))
            .expect("Cannot load styles file");
        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().expect("Cannot get main screen for styling"),
            &styles,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Obtener geometría del monitor
    #[cfg(unix)]
    let monitor_geometry = get_monitor_geometry();
    #[cfg(windows)]
    let monitor_geometry = get_monitor_geometry();

    println!("Monitor geometry: {:#?}", monitor_geometry);

    // Calcular posiciones para ventanas
    let mut position_idx = 0;
    let positions = {
        let (monitor_width, monitor_height) = if cfg!(unix) {
            let mw = ((monitor_geometry.width as i32
                - state.config.display.monitor_margin as i32
                - state.config.display.window_size as i32)
                / state.config.display.grid_size as i32)
                .max(0);
            let mh = ((monitor_geometry.height as i32
                - state.config.display.monitor_margin as i32
                - state.config.display.window_size as i32)
                / state.config.display.grid_size as i32)
                .max(0);
            (mw, mh)
        } else {
            let mw = ((monitor_geometry.width as i32
                - state.config.display.monitor_margin as i32
                - state.config.display.window_size as i32)
                / state.config.display.grid_size as i32)
                .max(0);
            let mh = ((monitor_geometry.height as i32
                - state.config.display.monitor_margin as i32
                - state.config.display.window_size as i32)
                / state.config.display.grid_size as i32)
                .max(0);
            (mw, mh)
        };

        let mut p = Vec::new();

        for x in 0..state.config.display.grid_size {
            for y in 0..state.config.display.grid_size {
                p.push((x * monitor_width, y * monitor_height));
            }
        }

        p.shuffle(&mut thread_rng());
        p
    };

    // Window management is now handled by AsyncWindowManager
    // No need for manual window arrays

    // position management handled in event loop

    // Loop principal
    let mut timer = tokio::time::interval(tokio::time::Duration::from_millis(100)); // 10 FPS for progress updates (less flickering)
    let mut cleanup_counter = 0;

    println!("✅ Overlay Native started successfully!");
    println!(
        "📊 Connected to {} platforms",
        state.config.get_enabled_platforms().len()
    );
    println!(
        "🔗 Active connections: {}",
        state.config.get_enabled_connections().len()
    );

    eprintln!("[DEBUG] Initialization completed, about to enter main loop");

    // Reset progress timer at main loop start for proper initial timing
    #[cfg(windows)]
    // Progress updates are now handled by AsyncWindowManager

    // Start background tasks
    state.start_message_processor().await;
    println!("📡 Background services started");

    // Subscribe to events before the loop
    let mut event_rx = state.event_emitter.subscribe();

    // Position management for window placement
    let mut position_idx = 0;

    println!("🚀 Starting main event loop...");
    loop {
        let continue_loop;
        #[cfg(unix)]
        {
            continue_loop = gtk::main_iteration_do(false);
        }
        #[cfg(windows)]
        {
            continue_loop = process_messages();
        }
        if !continue_loop {
            break;
        }

        // Add small delay to prevent CPU hogging and allow Windows to process messages
        #[cfg(windows)]
        {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // ~100 FPS main loop, progress updates at 20 FPS
        }

        // Clean up expired windows every 5 frames (every 500ms)
        cleanup_counter += 1;
        if cleanup_counter >= 5 {
            state.window_tracker.cleanup_expired().await;
            cleanup_counter = 0;
        }

        // Process messages and timer ticks using event system
        #[cfg(unix)]
        tokio::select! {
            event = event_rx.recv() => {
                if let Ok(AppEvent::MessageReceived(processed_message)) = event {
                    // Create window asynchronously and add to window manager
                    let message_clone = processed_message.clone();
                    let pos = positions[position_idx];
                    let monitor_geo = monitor_geometry;
                    let config_clone = state.config.clone();
                    let window_tracker = state.window_tracker.clone();

                    // Create window directly (simpler approach to avoid Send issues)
                    let win = handle_message(message_clone, pos, monitor_geo, &config_clone);
                    window_tracker.add_window(win).await;

                    position_idx = (position_idx + 1) % positions.len();
                }
            },
            _ = timer.tick() => {
                // Timer tick - progress bars are updated in the cleanup loop above
            }
        }

        #[cfg(windows)]
        {
            tokio::select! {
                event = event_rx.recv() => {
                    if let Ok(AppEvent::MessageReceived(processed_message)) = event {
                        // Create window asynchronously and add to window manager
                        let message_clone = processed_message.clone();
                        let pos = positions[position_idx];
                        let monitor_geo = monitor_geometry;
                        let config_clone = state.config.clone();
                        let window_tracker = state.window_tracker.clone();

                        // Create window directly (simpler approach to avoid Send issues)
                        let win = handle_message(message_clone, pos, monitor_geo, &config_clone);
                        window_tracker.add_window(win).await;

                        position_idx = (position_idx + 1) % positions.len();
                    }
                },
                _ = timer.tick() => {
                    // Timer tick for Windows - progress bars are updated in the cleanup loop above
                }
            }
        }
    }

    // Limpieza al salir
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

// Funciones de manejo de mensajes y ventanas
#[cfg(unix)]
fn handle_message(
    message: connection::ChatMessage,
    position: (i32, i32),
    monitor_geometry: gtk::Rectangle,
    config: &Config,
) -> SpawnedWindow {
    // Convertir emotes al formato esperado por spawn_window
    let emotes: Vec<twitch_irc::message::Emote> = message
        .emotes
        .iter()
        .map(|e| {
            let char_range = if let Some(pos) = e.positions.first() {
                pos.start..pos.end
            } else {
                0..0
            };
            twitch_irc::message::Emote {
                id: e.id.clone(),
                code: e.name.clone(),
                char_range,
            }
        })
        .collect();

    crate::windows::WindowsWindow::new(&message.username, &message.content, &emotes, position)
}

#[cfg(windows)]
#[cfg(windows)]
fn handle_message(
    message: crate::connection::ChatMessage,
    position: (i32, i32),
    _monitor_geometry: crate::windows::WindowGeometry,
    _config: &crate::config::Config,
) -> WindowsWindow {
    // Convertir emotes al formato esperado por WindowsWindow
    let emotes: Vec<twitch_irc::message::Emote> = message
        .emotes
        .iter()
        .map(|e| {
            let char_range = if let Some(pos) = e.positions.first() {
                pos.start..pos.end
            } else {
                0..0
            };
            twitch_irc::message::Emote {
                id: e.id.clone(),
                code: e.name.clone(),
                char_range,
            }
        })
        .collect();

    WindowsWindow::new(&message.username, &message.content, &emotes, position)
}
