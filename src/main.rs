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
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::connection::{ConnectionInfo, PlatformManager};
use crate::emotes::EmoteSystem;
use crate::mapping::MappingSystem;
use crate::platforms::{CredentialManager, PlatformFactory};

use anyhow::Result;

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
}

impl AppState {
    async fn new() -> Result<Self> {
        // Cargar configuración
        let config = Config::load_default().unwrap_or_else(|e| {
            eprintln!("Error loading config: {}, using defaults", e);
            Config::default()
        });

        // Crear sistemas
        let platform_manager = Arc::new(RwLock::new(PlatformManager::new()));
        let emote_system = Arc::new(RwLock::new(EmoteSystem::new(config.emotes.clone())));
        let mapping_system = Arc::new(RwLock::new(MappingSystem::default()));
        let platform_factory = Arc::new(PlatformFactory::new());
        let credential_manager = Arc::new(CredentialManager::new());

        Ok(Self {
            config,
            platform_manager,
            emote_system,
            mapping_system,
            platform_factory,
            credential_manager,
        })
    }

    async fn initialize_platforms(&self) -> Result<()> {
        let mut manager = self.platform_manager.write().await;
        let enabled_platforms = self.config.get_enabled_platforms();

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

        eprintln!(
            "[DEBUG] Starting connections. Found {} enabled connections",
            enabled_connections.len()
        );

        for connection in enabled_connections {
            eprintln!(
                "[DEBUG] Processing connection: {} (platform: {}, channel: {})",
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
            eprintln!("[DEBUG] Attempting to start connection: {}", connection.id);
            match manager.start_connection(&connection.id).await {
                Ok(_) => {
                    println!(
                        "✅ Connected to {} on {} ({})",
                        connection.channel, connection.platform, connection.id
                    );
                    eprintln!("[DEBUG] Successfully started connection: {}", connection.id);
                }
                Err(e) => {
                    eprintln!(
                        "❌ Failed to connect to {} on {}: {}",
                        connection.channel, connection.platform, e
                    );
                    eprintln!(
                        "[DEBUG] Connection start failed for {}: {}",
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
        emote_system.preload_global_emotes().await?;
        println!("✅ Global emotes preloaded");

        Ok(())
    }

    async fn process_message(
        &self,
        mut message: connection::ChatMessage,
    ) -> Result<connection::ChatMessage> {
        eprintln!(
            "[DEBUG] Processing message from {}: {} - {}",
            message.platform, message.username, message.content
        );

        // Aplicar filtros si es necesario
        if let Some(connection) = self
            .config
            .connections
            .iter()
            .find(|conn| conn.platform == message.platform && conn.channel == message.channel)
        {
            eprintln!("[DEBUG] Found connection config for message");
            let mut manager = self.platform_manager.write().await;
            if let Some(platform) = manager.get_platform_mut(&message.platform) {
                eprintln!("[DEBUG] Found platform for message processing");
                if !connection.filters.commands_only
                    || message.content.starts_with('!')
                    || message.content.starts_with('/')
                {
                    eprintln!("[DEBUG] Applying message filters");
                    // Aplicar filtros
                    if !platform.apply_message_filters(&mut message, &connection.filters) {
                        eprintln!("[DEBUG] Message filtered out");
                        return Err(anyhow::anyhow!("Message filtered out"));
                    }
                    eprintln!("[DEBUG] Message passed filters");
                } else {
                    eprintln!("[DEBUG] Commands only filter active, message rejected");
                    return Err(anyhow::anyhow!("Commands only filter active"));
                }
            } else {
                eprintln!(
                    "[DEBUG] Platform not found for message: {}",
                    message.platform
                );
            }
        } else {
            eprintln!(
                "[DEBUG] No connection config found for platform: {}, channel: {}",
                message.platform, message.channel
            );
        }

        eprintln!("[DEBUG] Parsing additional emotes");
        // Parsear emotes adicionales si es necesario
        let mut emote_system = self.emote_system.write().await;
        if let Ok(additional_emotes) = emote_system
            .parse_message_emotes(
                &message.content,
                &message.platform,
                &message.channel,
                "", // Datos de emotes crudos si existen
            )
            .await
        {
            message.emotes.extend(additional_emotes);
        }

        // Aplicar mapeo de datos
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
}

#[tokio::main(flavor = "current_thread")]
#[cfg(unix)]
use gdk::Screen;
#[cfg(unix)]
use gtk::{StyleContext, STYLE_PROVIDER_PRIORITY_APPLICATION};
use rand::prelude::*;

use tokio::time::{self, Instant};

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
async fn spawn_window(
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

#[cfg(windows)]
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
async fn handle_message(
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

    // Inicializar estado de la aplicación
    let state = AppState::new().await?;

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

    // Inicializar ventanas
    let mut windows_count = 0;
    let total_windows = state.config.window.max_windows;

    #[cfg(unix)]
    let mut windows: Vec<Option<SpawnedWindow>> = vec![None; total_windows];
    #[cfg(windows)]
    let mut windows: Vec<Option<WindowsWindow>> = vec![None; total_windows];

    // Ventana de prueba inicial
    #[cfg(unix)]
    {
        windows[windows_count] = Some(
            spawn_window(
                &state
                    .config
                    .platforms
                    .values()
                    .next()
                    .map(|p| p.credentials.username.clone().unwrap_or_default())
                    .unwrap_or_else(|| "USERNAME".to_string()),
                &state.config.window.test_message,
                &[],
                positions[position_idx],
                monitor_geometry,
            )
            .await,
        );
        windows_count += 1;
    }
    #[cfg(windows)]
    {
        windows[windows_count] = Some(WindowsWindow::new(
            &state
                .config
                .platforms
                .values()
                .next()
                .map(|p| p.credentials.username.clone().unwrap_or_default())
                .unwrap_or_else(|| "USERNAME".to_string()),
            &state.config.window.test_message,
            &[],
            positions[position_idx],
        ));
        windows_count += 1;
    }

    position_idx += 1;
    position_idx %= positions.len();

    // Loop principal
    let mut timer = tokio::time::interval(tokio::time::Duration::from_millis(10));

    println!("✅ Overlay Native started successfully!");
    println!(
        "📊 Connected to {} platforms",
        state.config.get_enabled_platforms().len()
    );
    println!(
        "🔗 Active connections: {}",
        state.config.get_enabled_connections().len()
    );

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

        let mut windows_count = windows_count % total_windows;

        // Limpiar ventanas expiradas
        let now = tokio::time::Instant::now();
        let max_time = state.config.message_duration();

        for win in windows.iter_mut().filter(|x| x.is_some()) {
            let created_time = if cfg!(unix) {
                win.as_ref().unwrap().created
            } else {
                win.as_ref().unwrap().created
            };
            let elapsed = now - created_time;
            if elapsed >= max_time {
                #[cfg(unix)]
                {
                    if let Some(ref mut w) = win {
                        w.w.close();
                        *win = None;
                    }
                }
                #[cfg(windows)]
                if let Some(ref mut w) = win {
                    w.close();
                    *win = None;
                }
            } else {
                let progress = elapsed.as_secs_f64() / max_time.as_secs_f64();
                #[cfg(unix)]
                if let Some(ref mut w) = win {
                    w.progress.set_fraction(progress);
                }
                #[cfg(windows)]
                if let Some(ref mut w) = win {
                    w.set_progress(progress);
                }
            }
        }

        // Ensure windows_count doesn't exceed bounds
        windows_count %= total_windows;

        // Procesar mensajes
        #[cfg(unix)]
        tokio::select! {
            message = state.platform_manager.write().await.next_message() => {
                if let Some(message) = message {
                    match state.process_message(message).await {
                        Ok(processed_message) => {
                            if let Some(win) = windows[windows_count].take() {
                                win.w.close();
                            }
                            let win = handle_message(processed_message, positions[position_idx], monitor_geometry, &state.config).await;
                            windows[windows_count] = Some(win);
                            position_idx += 1;
                            position_idx %= positions.len();
                            windows_count += 1;
                        }
                        Err(e) => {
                            eprintln!("⚠️ Error processing message: {}", e);
                        }
                    }
                }
            },
            _ = timer.tick() => {}
        }

        #[cfg(windows)]
        {
            let mut pm = state.platform_manager.write().await;
            tokio::select! {
                message = pm.next_message() => {
                    if let Some(message) = message {
                        eprintln!("[DEBUG] Main loop received message: {} - {} - {}", message.platform, message.username, message.content);
                        match state.process_message(message).await {
                            Ok(processed_message) => {
                                eprintln!("[DEBUG] Message processed successfully, creating window");
                                if let Some(win) = windows[windows_count].take() {
                                    win.close();
                                }
                                let win = handle_message(processed_message, positions[position_idx], monitor_geometry, &state.config).await;
                                windows[windows_count] = Some(win);
                                position_idx += 1;
                                position_idx %= positions.len();
                                windows_count = (windows_count + 1) % total_windows;
                                eprintln!("[DEBUG] Window created and positioned");
                            }
                            Err(e) => {
                                eprintln!("⚠️ Error processing message: {}", e);
                                eprintln!("[DEBUG] Message processing failed with error: {}", e);
                            }
                        }
                    } else {
                        eprintln!("[DEBUG] No message received from platform manager");
                    }
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
async fn handle_message(
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

    spawn_window(
        &message.username,
        &message.content,
        &emotes,
        position,
        monitor_geometry,
    )
    .await
}

#[cfg(windows)]
#[cfg(windows)]
async fn handle_message(
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
