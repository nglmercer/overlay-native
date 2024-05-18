mod connection;
mod window;

#[cfg(target_os = "linux")]
pub mod x11;

#[cfg(target_os = "linux")]
extern crate gdkx11;
#[cfg(target_os = "linux")]
extern crate x11rb;

use gdk::prelude::MonitorExt;
use rand::seq::SliceRandom;
use twitch_irc::login::StaticLoginCredentials;
use twitch_irc::message::{PrivmsgMessage, ServerMessage};
use twitch_irc::TwitchIRCClient;
use twitch_irc::{ClientConfig, SecureTCPTransport};
use window::{get_gdk_monitor, spawn_window, SpawnedWindow};

use std::time::Duration;

use gtk::prelude::{CssProviderExt, GtkWindowExt, ProgressBarExt};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // default configuration is to join chat as anonymous.
    let config = ClientConfig::default();
    let (mut incoming_messages, client) =
        TwitchIRCClient::<SecureTCPTransport, StaticLoginCredentials>::new(config);

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

    let monitor_geometry = get_gdk_monitor().geometry();
    let mut position_idx = 0;
    let positions = {
        let monitor_width = (monitor_geometry.width() - 40 - 200) / 100;
        let monitor_height = (monitor_geometry.height() - 40 - 200) / 100;

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
    let windows: &mut [Option<SpawnedWindow>] = &mut vec![None; total_windows];

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
    position_idx += 1;
    position_idx %= positions.len();
    windows_count += 1;
    windows_count %= total_windows;

    client.join("thegrefg".to_owned()).unwrap();

    let mut gtk_loop = tokio::time::interval(Duration::from_millis(10));

    loop {
        let b = gtk::main_iteration_do(false);
        if !b {
            break;
        }

        let now = tokio::time::Instant::now();
        const MAX_TIME: Duration = Duration::from_secs(10);

        for win in windows.iter_mut().filter(|x| x.is_some()) {
            let spawned_win = win.as_ref().unwrap();

            let elapsed = now - spawned_win.created;
            if elapsed >= MAX_TIME {
                spawned_win.w.close();
                *win = None;
            } else {
                let progress = elapsed.as_secs_f64() / MAX_TIME.as_secs_f64();
                spawned_win.progress.set_fraction(progress);
            }
        }

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
    }
}

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
