//! Mock Client for Overlay Native
//!
//! Sends random chat messages and events to test the overlay.
//! Run with: cargo run --bin mock_client


use overlay_native::core::{Alert, Badge, OverlayElement};
use overlay_native::render::{
    get_monitor_size, init_platform_backend, PlatformWindow, WindowConfig,
};
use overlay_native::transport::{
    ChatMessagePayload, GiftPayload, GiftType as TransportGiftType, IncomingMessage,
};
use rand::seq::SliceRandom;
use rand::Rng;
use std::time::{Duration, Instant};

#[cfg(unix)]
use overlay_native::render::gtk::GtkWindow;
#[cfg(windows)]
use overlay_native::render::win32::Win32Window;

fn generate_random_username() -> String {
    let prefixes = [
        "Epic", "Mega", "Ultra", "Ninja", "Shadow", "Neon", "Pixel", "Retro",
    ];
    let suffixes = [
        "Gamer", "Player", "Streamer", "Viewer", "Coder", "Master", "Legend",
    ];
    let numbers = ["", "69", "420", "123", "7", "42"];
    let mut rng = rand::thread_rng();
    format!(
        "{}{}{}",
        prefixes.choose(&mut rng).unwrap(),
        suffixes.choose(&mut rng).unwrap(),
        numbers.choose(&mut rng).unwrap()
    )
}

fn generate_random_message() -> String {
    let phrases = [
        "Hey everyone!",
        "This stream is awesome!",
        "Great content!",
        "Let's go!",
        "That was insane!",
        "CLIP IT!",
        "LOL",
        "GG",
        "W in the chat",
        "sheesh",
    ];
    let mut rng = rand::thread_rng();
    phrases.choose(&mut rng).unwrap().to_string()
}

fn generate_random_color() -> String {
    let colors = [
        "#FF0000", "#00FF00", "#0000FF", "#FF00FF", "#00FFFF", "#FFFF00", "#FF6600",
    ];
    let mut rng = rand::thread_rng();
    colors.choose(&mut rng).unwrap().to_string()
}

fn generate_random_badges() -> Vec<Badge> {
    let mut rng = rand::thread_rng();
    let mut badges = Vec::new();
    if rng.gen_bool(0.4) {
        badges.push(Badge {
            id: "subscriber".to_string(),
            name: "Subscriber".to_string(),
            url: Some(
                "https://static-cdn.jtvnw.net/badges/v1/5d9f2208-5dd8-11e7-8513-2ff4adfae661/2"
                    .to_string(),
            ),
            title: Some("Subscriber".to_string()),
        });
    }
    badges
}

fn create_random_chat_payload() -> ChatMessagePayload {
    ChatMessagePayload {
        id: None,
        username: generate_random_username(),
        display_name: None,
        content: generate_random_message(),
        user_color: Some(generate_random_color()),
        badges: vec![], // simplified badges for mock
        platform: Some("mock".to_string()),
        channel: Some("test".to_string()),
        metadata: std::collections::HashMap::new(),
    }
}

fn create_random_message() -> IncomingMessage {
    let mut rng = rand::thread_rng();
    if rng.gen_bool(0.9) {
        IncomingMessage::ChatMessage(create_random_chat_payload())
    } else {
        IncomingMessage::Gift(GiftPayload {
            from_user: generate_random_username(),
            to_user: None,
            gift_type: TransportGiftType::Subscription,
            amount: Some(rng.gen_range(1..12)),
            tier: Some("Tier 1".to_string()),
            message: Some(generate_random_message()),
            platform: Some("mock".to_string()),
        })
    }
}

fn main() {
    println!("🚀 Starting Mock Client...");

    init_platform_backend();

    let (monitor_width, monitor_height) = get_monitor_size();
    println!("Monitor: {}x{}", monitor_width, monitor_height);

    let window_config = WindowConfig {
        position: (50, 50),
        size: (400, 80),
        duration: Duration::from_secs(10),
        opacity: 0.9,
        border_radius: 8,
        font_family: "Arial".to_string(),
        font_size: 14,
    };

    let num_messages = 5;

    #[cfg(unix)]
    let mut active_windows: Vec<(String, GtkWindow, Instant)> = Vec::new();

    for i in 1..=num_messages {
        let message = create_random_message();

        match message {
            IncomingMessage::ChatMessage(payload) => {
                println!(
                    "💬 [{}/{}] {}: {}",
                    i, num_messages, payload.username, payload.content
                );
                
                let id = payload.id.clone().unwrap_or_else(|| i.to_string());
                let alert = Alert::chat(
                    id.clone(),
                    payload.username,
                    payload.content,
                    payload.user_color,
                    generate_random_badges(),
                );

                #[cfg(unix)]
                {
                    if let Ok(window) = GtkWindow::from_alert(&alert, &window_config)
                    {
                        window.show();
                        active_windows.push((alert.id.clone(), window, Instant::now()));
                    }
                }
            }
            IncomingMessage::Gift(payload) => {
                println!(
                    "🎁 [{}/{}] Gift from {}",
                    i, num_messages, payload.from_user
                );
                
                let id = format!("gift_{}", i);
                let gift_desc = format!("a sub");
                let alert = Alert::gift(id.clone(), payload.from_user, gift_desc, payload.message);

                #[cfg(unix)]
                {
                    if let Ok(window) = GtkWindow::from_alert(&alert, &window_config) {
                        window.show();
                        active_windows.push((alert.id.clone(), window, Instant::now()));
                    }
                }
            }
            _ => {}
        }

        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(1000) {
            #[cfg(unix)]
            {
                overlay_native::render::gtk::gtk_iteration();
            }
            #[cfg(windows)]
            {
                overlay_native::render::win32::process_messages();
            }

            for (_, window, created) in active_windows.iter_mut() {
                let progress = (10.0 - created.elapsed().as_secs_f64()) / 10.0;
                window.set_progress(progress.max(0.0));
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }

    println!("\n✅ Done. Closing in 5s...");
    std::thread::sleep(Duration::from_secs(5));

    for (_, window, _) in active_windows.drain(..) {
        window.close();
    }
}
