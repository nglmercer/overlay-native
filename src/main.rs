mod connection;

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
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::{PrivmsgMessage, ServerMessage};
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};

use std::time::Duration;

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
    // default configuration is to join chat as anonymous.
    let config: ClientConfig<StaticLoginCredentials> = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

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
            let monitor_width = (monitor_geometry.width() - 40 - 200) / 100;
            let monitor_height = (monitor_geometry.height() - 40 - 200) / 100;
            (monitor_width, monitor_height)
        };
        #[cfg(windows)]
        let (monitor_width, monitor_height) = {
            let monitor_width = (monitor_geometry.width - 40 - 200) / 100;
            let monitor_height = (monitor_geometry.height - 40 - 200) / 100;
            (monitor_width, monitor_height)
        };

        let mut p = Vec::new();

        for x in 0..100 {
            for y in 0..100 {
                p.push((x * monitor_width, y * monitor_height));
            }
        }

        p.shuffle(&mut rand::thread_rng());

        p
    };

    let mut windows_count = 0;
    let total_windows = 100;
    
    #[cfg(unix)]
    let windows: &mut [Option<SpawnedWindow>] = &mut vec![None; total_windows];
    #[cfg(windows)]
    let windows: &mut [Option<WindowsWindow>] = &mut vec![None; total_windows];

    #[cfg(unix)]
    {
        windows[windows_count] = Some(
            spawn_window(
                "USERNAME",
                "TEST",
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
                "USERNAME",
                "TEST",
                &[],
                positions[position_idx],
            )
        );
    }
    
    position_idx += 1;
    position_idx %= positions.len();
    windows_count += 1;
    windows_count %= total_windows;

    client.join("apika_luca".to_owned()).unwrap();

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
        const MAX_TIME: Duration = Duration::from_secs(10);

        for win in windows.iter_mut().filter(|x| x.is_some()) {
            let spawned_win = win.as_ref().unwrap();

            let elapsed = now - spawned_win.created;
            if elapsed >= MAX_TIME {
                #[cfg(unix)]
                spawned_win.w.close();
                #[cfg(windows)]
                spawned_win.close();
                *win = None;
            } else {
                let progress = elapsed.as_secs_f64() / MAX_TIME.as_secs_f64();
                #[cfg(unix)]
                spawned_win.progress.set_fraction(progress);
                #[cfg(windows)]
                {
                    let mut spawned_win_mut = win.as_mut().unwrap();
                    spawned_win_mut.set_progress(progress);
                }
            }
        }

        #[cfg(unix)]
        tokio::select! {
            message = incoming_messages.recv() => {
                if let Some(message) = message {
                    match message {
                        ServerMessage::Privmsg(message) => {
                            if let Some(win) = windows[windows_count].take() {
                                win.w.close();
                            }
                            let win = handle_message(message, positions[position_idx], monitor_geometry).await;
                            windows[windows_count] = Some(win);
                            position_idx += 1;
                            position_idx %= positions.len();
                            windows_count += 1;
                            windows_count %= total_windows;
                        },
                        ServerMessage::Ping(_) | ServerMessage::Pong(_) => {},
                        _ => println!("{message:#?}")
                    };
                }
            },
            _ = gtk_loop.tick() => {}
        }
        
        #[cfg(windows)]
        tokio::select! {
            message = incoming_messages.recv() => {
                if let Some(message) = message {
                    match message {
                        ServerMessage::Privmsg(message) => {
                            if let Some(win) = windows[windows_count].take() {
                                win.close();
                            }
                            let win = handle_message(message, positions[position_idx], monitor_geometry).await;
                            windows[windows_count] = Some(win);
                            position_idx += 1;
                            position_idx %= positions.len();
                            windows_count += 1;
                            windows_count %= total_windows;
                        },
                        ServerMessage::Ping(_) | ServerMessage::Pong(_) => {},
                        _ => println!("{message:#?}")
                    };
                }
            },
            _ = windows_loop.tick() => {}
        }
    }
}

#[cfg(unix)]
async fn handle_message(
    message: PrivmsgMessage,
    position: (i32, i32),
    monitor_geometry: gtk::Rectangle,
) -> SpawnedWindow {
    spawn_window(
        &message.sender.name,
        &message.message_text,
        &message.emotes,
        position,
        monitor_geometry,
    )
    .await
}

#[cfg(windows)]
async fn handle_message(
    message: PrivmsgMessage,
    position: (i32, i32),
    _monitor_geometry: windows::WindowGeometry,
) -> WindowsWindow {
    WindowsWindow::new(
        &message.sender.name,
        &message.message_text,
        &message.emotes,
        position,
    )
}
