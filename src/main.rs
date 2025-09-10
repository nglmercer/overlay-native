mod connection;
mod config;
mod twitch;

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
use std::time::Duration;

use crate::config::Config;
use crate::connection::{ChatMessage, PlatformManager};
use crate::twitch::TwitchPlatform;

#[cfg(unix)]
use gdk::prelude::MonitorExt;
#[cfg(unix)]
use window::{get_gdk_monitor, spawn_window, SpawnedWindow};
#[cfg(unix)]
use gtk::prelude::{CssProviderExt, GtkWindowExt, ProgressBarExt};

#[cfg(windows)]
use windows::{WindowsWindow, get_monitor_geometry, process_messages};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Cargar configuración desde JSON
    let config = Config::load_default().unwrap_or_else(|e| {
        eprintln!("Error loading config: {}, using defaults", e);
        Config::default()
    });
    
    // Crear el manager de plataformas
    let mut platform_manager = PlatformManager::new();
    
    // Crear e inicializar la plataforma de Twitch
    let twitch_platform = TwitchPlatform::new();
    platform_manager.run_platform(twitch_platform, config.platform.default_channel.clone())
        .await
        .expect("Failed to start Twitch platform");

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

    #[cfg(unix)]
    let monitor_geometry = get_gdk_monitor().geometry();
    #[cfg(windows)]
    let monitor_geometry = get_monitor_geometry();
    
    println!("{monitor_geometry:#?}");
    let mut position_idx = 0;
    let positions = {
        #[cfg(unix)]
        let (monitor_width, monitor_height) = {
            let monitor_width = (monitor_geometry.width() - config.display.monitor_margin - config.display.window_size) / config.display.grid_size;
            let monitor_height = (monitor_geometry.height() - config.display.monitor_margin - config.display.window_size) / config.display.grid_size;
            (monitor_width, monitor_height)
        };
        #[cfg(windows)]
        let (monitor_width, monitor_height) = {
            let monitor_width = (monitor_geometry.width - config.display.monitor_margin - config.display.window_size) / config.display.grid_size;
            let monitor_height = (monitor_geometry.height - config.display.monitor_margin - config.display.window_size) / config.display.grid_size;
            (monitor_width, monitor_height)
        };

        let mut p = Vec::new();

        for x in 0..config.display.grid_size {
            for y in 0..config.display.grid_size {
                p.push((x * monitor_width, y * monitor_height));
            }
        }

        p.shuffle(&mut rand::thread_rng());

        p
    };

    let mut windows_count = 0;
    let total_windows = config.window.max_windows;
    
    #[cfg(unix)]
    let windows: &mut [Option<SpawnedWindow>] = &mut vec![None; total_windows];
    #[cfg(windows)]
    let windows: &mut [Option<WindowsWindow>] = &mut vec![None; total_windows];

    #[cfg(unix)]
    {
        windows[windows_count] = Some(
            spawn_window(
                &config.platform.username,
                &config.window.test_message,
                &[],
                positions[position_idx],
                monitor_geometry,
            )
            .await,
        );
    }
    #[cfg(windows)]
    {
        windows[windows_count] = Some(
            WindowsWindow::new(
                &config.platform.username,
                &config.window.test_message,
                &[],
                positions[position_idx],
            )
        );
    }
    
    position_idx += 1;
    position_idx %= positions.len();
    windows_count += 1;
    windows_count %= total_windows;

    #[cfg(unix)]
    let mut gtk_loop = tokio::time::interval(Duration::from_millis(10));
    #[cfg(windows)]
    let mut windows_loop = tokio::time::interval(Duration::from_millis(10));

    loop {
        #[cfg(unix)]
        {
            let b = gtk::main_iteration_do(false);
            if !b {
                break;
            }
        }
        #[cfg(windows)]
        {
            if !process_messages() {
                break;
            }
        }

        let now = tokio::time::Instant::now();
        let max_time = config.message_duration();

        for win in windows.iter_mut().filter(|x| x.is_some()) {
            let spawned_win = win.as_ref().unwrap();

            let elapsed = now - spawned_win.created;
            if elapsed >= max_time {
                #[cfg(unix)]
                spawned_win.w.close();
                #[cfg(windows)]
                spawned_win.close();
                *win = None;
            } else {
                let progress = elapsed.as_secs_f64() / max_time.as_secs_f64();
                #[cfg(unix)]
                spawned_win.progress.set_fraction(progress);
                #[cfg(windows)]
                {
                    let spawned_win_mut = win.as_mut().unwrap();
                    spawned_win_mut.set_progress(progress);
                }
            }
        }

        #[cfg(unix)]
        tokio::select! {
            message = platform_manager.next_message() => {
                if let Some(message) = message {
                    if let Some(win) = windows[windows_count].take() {
                        win.w.close();
                    }
                    let win = handle_message(message, positions[position_idx], monitor_geometry).await;
                    windows[windows_count] = Some(win);
                    position_idx += 1;
                    position_idx %= positions.len();
                    windows_count += 1;
                    windows_count %= total_windows;
                }
            },
            _ = gtk_loop.tick() => {}
        }
        
        #[cfg(windows)]
        tokio::select! {
            message = platform_manager.next_message() => {
                if let Some(message) = message {
                    if let Some(win) = windows[windows_count].take() {
                        win.close();
                    }
                    let win = handle_message(message, positions[position_idx], monitor_geometry).await;
                    windows[windows_count] = Some(win);
                    position_idx += 1;
                    position_idx %= positions.len();
                    windows_count += 1;
                    windows_count %= total_windows;
                }
            },
            _ = windows_loop.tick() => {}
        }
    }
}

#[cfg(unix)]
async fn handle_message(
    message: ChatMessage,
    position: (i32, i32),
    monitor_geometry: gtk::Rectangle,
) -> SpawnedWindow {
    // Convertir ChatEmote a los emotes esperados por spawn_window
    let emotes: Vec<twitch_irc::message::Emote> = message.emotes.iter().map(|e| {
        let char_range = if let Some((start, end)) = e.positions.first() {
            *start..*end
        } else {
            0..0
        };
        twitch_irc::message::Emote {
            id: e.id.clone(),
            code: e.name.clone(),
            char_range,
        }
    }).collect();
    
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
async fn handle_message(
    message: ChatMessage,
    position: (i32, i32),
    _monitor_geometry: windows::WindowGeometry,
) -> WindowsWindow {
    // Convertir ChatEmote a los emotes esperados por WindowsWindow
    let emotes: Vec<twitch_irc::message::Emote> = message.emotes.iter().map(|e| {
        let char_range = if let Some((start, end)) = e.positions.first() {
            *start..*end
        } else {
            0..0
        };
        twitch_irc::message::Emote {
            id: e.id.clone(),
            code: e.name.clone(),
            char_range,
        }
    }).collect();
    
    WindowsWindow::new(
        &message.username,
        &message.content,
        &emotes,
        position,
    )
}
