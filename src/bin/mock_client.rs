//! Mock Client Example for Overlay Native
//!
//! This example creates a mock client that sends random users and random comments
//! to test the overlay functionality without needing a real streaming platform connection.
//!
//! Run with: cargo run --bin mock_client

use overlay_native::core::ChatMessageElement;
use overlay_native::transport::{ChatMessagePayload, EmotePayload, IncomingMessage};
use rand::seq::SliceRandom;
use rand::Rng;
use std::time::Duration;

/// Random username generator
fn generate_random_username() -> String {
    let prefixes = [
        "Cool", "Epic", "Super", "Mega", "Ultra", "Pro", "Elite", "Ninja", "Shadow", "Dark",
        "Light", "Fire", "Ice", "Storm", "Thunder", "Cyber", "Neon", "Quantum", "Pixel", "Retro",
        "Crypto", "Meta",
    ];

    let suffixes = [
        "Gamer", "Player", "Streamer", "Viewer", "Fan", "User", "Coder", "Hacker", "Wizard",
        "Master", "Lord", "King", "Queen", "Boss", "Champion", "Legend", "Hero", "Warrior",
        "Knight", "Ninja",
    ];

    let numbers = ["", "", "", "69", "420", "123", "2024", "99", "007", "42"];

    let mut rng = rand::thread_rng();
    let prefix = prefixes.choose(&mut rng).unwrap();
    let suffix = suffixes.choose(&mut rng).unwrap();
    let number = numbers.choose(&mut rng).unwrap();

    format!("{}{}{}", prefix, suffix, number)
}

/// Random message/comment generator
fn generate_random_message() -> String {
    let greetings = [
        "Hey everyone!",
        "Hello!",
        "Hi chat!",
        "What's up?",
        "Greetings!",
        "Yo!",
        "Hey guys!",
        "Hi!",
        "Hello chat!",
        "Howdy!",
    ];

    let reactions = [
        "PogChamp",
        "LULW",
        "KEKW",
        "OMEGALUL",
        "monkaS",
        "Sadge",
        "PepeLaugh",
        "FeelsGoodMan",
        "FeelsBadMan",
        "PogU",
    ];

    let phrases = [
        "This stream is awesome!",
        "Great content!",
        "Love the stream!",
        "Keep up the good work!",
        "You're amazing!",
        "Best stream ever!",
        "Let's go!",
        "Pog moment!",
        "That was insane!",
        "CLIP IT!",
        "First time watching, this is great!",
        "Been here for years!",
        "Just donated!",
        "Subscribed today!",
        "Gifted 5 subs!",
        "This game is so fun to watch!",
        "The gameplay is incredible!",
        "When's the next stream?",
        "What time zone?",
        "Love the community!",
        "Chat is so wholesome!",
        "Best community ever!",
        "Pogchamp!",
        "That play was clean!",
        "How did that happen?!",
        "INSANE!",
        "Skill issue",
        "Chat, is this real?",
        "Is this live?",
        "LOL",
        "LMAO",
        "ROFL",
        "haha",
        "nice one!",
        "gg",
        "GG",
        "Gg",
        "cringe",
        "based",
        "W",
        "L",
        "W in the chat",
        "L in the chat",
        "F in chat",
        "press F",
        "rip",
        "oof",
        "big oof",
        "massive W",
        "tru",
        "fr",
        "no cap",
        "sheesh",
        "bussin",
        "no way",
    ];

    let questions = [
        "How long have you been streaming?",
        "What game is this?",
        "What's your favorite game?",
        "Do you play any other games?",
        "Can you say hi?",
        "Can we get some F's in chat?",
        "When is the next subathon?",
        "How many subs do you have?",
        "What's your rank?",
        "Any tips for new players?",
        "What's your streaming setup?",
        "How long have you been playing?",
        "What's your main?",
        "Favorite streamer?",
        "Favorite food?",
    ];

    let emote_only = [
        "Kappa",
        "PogChamp",
        "LUL",
        "KEKW",
        "OMEGALUL",
        "Sadge",
        "PepeHands",
        "monkaS",
        "PepeLaugh",
        "FeelsGoodMan",
        "FeelsBadMan",
        "PogU",
        "Pog",
        "KEKW KEKW",
        "LUL LUL LUL",
        "Kappa Kappa Kappa",
    ];

    let mut rng = rand::thread_rng();
    let message_type = rng.gen_range(0..5);

    let message = match message_type {
        0 => greetings.choose(&mut rng).unwrap().to_string(),
        1 => format!(
            "{} {}",
            reactions.choose(&mut rng).unwrap(),
            reactions.choose(&mut rng).unwrap()
        ),
        2 => phrases.choose(&mut rng).unwrap().to_string(),
        3 => questions.choose(&mut rng).unwrap().to_string(),
        4 => emote_only.choose(&mut rng).unwrap().to_string(),
        _ => phrases.choose(&mut rng).unwrap().to_string(),
    };

    // Sometimes add multiple phrases
    if rng.gen_bool(0.3) {
        let extra = phrases.choose(&mut rng).unwrap();
        return format!("{} {}", message, extra);
    }

    message
}

/// Generate random user color
fn generate_random_color() -> String {
    let colors = [
        "#FF0000", "#00FF00", "#0000FF", "#FF00FF", "#00FFFF", "#FFFF00", "#FF6600", "#FF0066",
        "#6600FF", "#00FF66", "#FF3333", "#33FF33", "#3333FF", "#FF33FF", "#33FFFF", "#FF9900",
        "#9900FF", "#00FF99", "#FF0099", "#0099FF", "#1E90FF", "#FF4500", "#32CD32", "#FFD700",
        "#8A2BE2",
    ];
    let mut rng = rand::thread_rng();
    colors.choose(&mut rng).unwrap().to_string()
}

/// Generate random emotes for a message
fn generate_random_emotes(content: &str) -> Vec<EmotePayload> {
    let twitch_emotes = [
        ("25", "Kappa"),
        ("425618", "FeelsGoodMan"),
        ("304355148", "PepeLaugh"),
        ("120232", "PogChamp"),
        ("425618", "FeelsGoodMan"),
        ("28087", "LUL"),
        ("305954156", "KEKW"),
        ("301682635", "OMEGALUL"),
        ("301508298", "Sadge"),
        ("301508298", "monkaS"),
    ];

    let bttv_emotes = [
        ("566c9fc265dbbdab32ec053b", "PepeHands"),
        ("5834261e3d2f2519c6afd566", "monkaS"),
        ("5e76d338d6581c3724c0f8b2", "PepeLaugh"),
        ("5e9c6c0e24fb3533c6ee6a3e", "POGGERS"),
    ];

    let mut rng = rand::thread_rng();
    let mut emotes = Vec::new();

    // 30% chance to have emotes
    if rng.gen_bool(0.3) {
        let num_emotes = rng.gen_range(1..=3);

        for _ in 0..num_emotes {
            let (id, name) = if rng.gen_bool(0.7) {
                twitch_emotes.choose(&mut rng).unwrap()
            } else {
                bttv_emotes.choose(&mut rng).unwrap()
            };

            // Find emote position in content or add at random position
            let pos = if content.contains(name) {
                let start = content.find(name).unwrap();
                EmotePayload {
                    id: id.to_string(),
                    name: name.to_string(),
                    url: Some(format!(
                        "https://static-cdn.jtvnw.net/emoticons/v2/{}/default/dark/2.0",
                        id
                    )),
                    is_animated: false,
                    positions: vec![overlay_native::transport::EmotePosition {
                        start,
                        end: start + name.len(),
                    }],
                    width: Some(28),
                    height: Some(28),
                }
            } else {
                EmotePayload {
                    id: id.to_string(),
                    name: name.to_string(),
                    url: Some(format!(
                        "https://static-cdn.jtvnw.net/emoticons/v2/{}/default/dark/2.0",
                        id
                    )),
                    is_animated: false,
                    positions: vec![],
                    width: Some(28),
                    height: Some(28),
                }
            };

            emotes.push(pos);
        }
    }

    emotes
}

/// Generate random badges for a user
fn generate_random_badges() -> Vec<overlay_native::transport::BadgePayload> {
    let mut rng = rand::thread_rng();
    let mut badges = Vec::new();

    // 40% chance of having subscriber badge
    if rng.gen_bool(0.4) {
        let months = rng.gen_range(1..60);
        badges.push(overlay_native::transport::BadgePayload {
            id: "subscriber".to_string(),
            name: format!("{} Month Subscriber", months),
            url: Some(
                "https://static-cdn.jtvnw.net/badges/v1/5d9f2208-5dd8-11e7-8513-2ff4adfae661/2".to_string(),
            ),
            title: Some(format!("{} month subscriber", months)),
        });
    }

    // 10% chance of moderator
    if rng.gen_bool(0.1) {
        badges.push(overlay_native::transport::BadgePayload {
            id: "moderator".to_string(),
            name: "Moderator".to_string(),
            url: Some(
                "https://static-cdn.jtvnw.net/badges/v1/3267646d-33f0-4b17-b3df-f923a41db1d0/2"
                    .to_string(),
            ),
            title: Some("Moderator".to_string()),
        });
    }

    // 5% chance of VIP
    if rng.gen_bool(0.05) {
        badges.push(overlay_native::transport::BadgePayload {
            id: "vip".to_string(),
            name: "VIP".to_string(),
            url: Some(
                "https://static-cdn.jtvnw.net/badges/v1/b817aba4-fad8-49e2-b88a-7cc744f6efff/2"
                    .to_string(),
            ),
            title: Some("VIP".to_string()),
        });
    }

    // 2% chance of founder
    if rng.gen_bool(0.02) {
        badges.push(overlay_native::transport::BadgePayload {
            id: "founder".to_string(),
            name: "Founder".to_string(),
            url: Some(
                "https://static-cdn.jtvnw.net/badges/v1/511b78a9-ab37-472f-9569-457753bbe7d3/2"
                    .to_string(),
            ),
            title: Some("Founder".to_string()),
        });
    }

    badges
}

/// Create a random chat message payload
fn create_random_chat_payload() -> ChatMessagePayload {
    let username = generate_random_username();
    let content = generate_random_message();
    let emotes = generate_random_emotes(&content);
    let badges = generate_random_badges();
    let color = generate_random_color();

    ChatMessagePayload {
        id: None, // Will be auto-generated
        username,
        display_name: None,
        content,
        user_color: Some(color),
        emotes,
        badges,
        platform: Some("mock".to_string()),
        channel: Some("test_channel".to_string()),
        metadata: std::collections::HashMap::new(),
    }
}

/// Create a random IncomingMessage
fn create_random_message() -> IncomingMessage {
    let mut rng = rand::thread_rng();

    // 90% chat messages, 10% gifts
    if rng.gen_bool(0.9) {
        IncomingMessage::ChatMessage(create_random_chat_payload())
    } else {
        IncomingMessage::Gift(overlay_native::transport::GiftPayload {
            from_user: generate_random_username(),
            to_user: if rng.gen_bool(0.5) {
                Some(generate_random_username())
            } else {
                None
            },
            gift_type: overlay_native::transport::GiftType::Subscription,
            amount: Some(rng.gen_range(1..12)),
            tier: Some("Tier 1".to_string()),
            message: Some(generate_random_message()),
            platform: Some("mock".to_string()),
        })
    }
}

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           OVERLAY NATIVE - MOCK CLIENT EXAMPLE               ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("This example generates random chat messages to test the overlay.");
    println!("It demonstrates the message format expected by the transport layer.");
    println!();

    let mut rng = rand::thread_rng();
    let num_messages = 10;

    println!("📢 Generating {} random messages:\n", num_messages);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    for i in 1..=num_messages {
        let message = create_random_message();

        // Validate the message
        if let Err(e) = message.validate() {
            println!("❌ Message {} validation failed: {}", i, e);
            continue;
        }

        match message {
            IncomingMessage::ChatMessage(payload) => {
                println!("\n💬 Message #{}: Chat Message", i);
                println!("   👤 Username: {}", payload.username);
                println!("   📝 Content: {}", payload.content);

                if let Some(color) = &payload.user_color {
                    println!("   🎨 Color: {}", color);
                }

                if !payload.emotes.is_empty() {
                    println!(
                        "   😀 Emotes: {}",
                        payload
                            .emotes
                            .iter()
                            .map(|e| e.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }

                if !payload.badges.is_empty() {
                    println!(
                        "   🏅 Badges: {}",
                        payload
                            .badges
                            .iter()
                            .map(|b| b.name.clone())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }

                // Convert to core type
                let core_message: ChatMessageElement = payload.into();
                println!(
                    "   ✅ Converted to core message with ID: {}",
                    core_message.id
                );
            }
            IncomingMessage::Gift(payload) => {
                println!("\n🎁 Message #{}: Gift Event", i);
                println!("   👤 From: {}", payload.from_user);
                if let Some(to) = &payload.to_user {
                    println!("   👤 To: {}", to);
                }
                println!("   🎁 Type: {:?}", payload.gift_type);
                if let Some(amount) = payload.amount {
                    println!("   📊 Amount: {}", amount);
                }
            }
            _ => {}
        }

        // Simulate delay between messages
        let delay = rng.gen_range(100..500);
        std::thread::sleep(Duration::from_millis(delay));
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("\n✅ Mock client example completed!");
    println!("\n💡 Tips:");
    println!("   - Use the ChatMessagePayload format to send messages via WebSocket");
    println!("   - The overlay expects JSON messages following the IncomingMessage schema");
    println!("   - Each message is validated before being displayed");
    println!("\n📚 Example JSON message format:");

    let example_json = serde_json::json!({
        "type": "chat_message",
        "data": {
            "username": "TestUser",
            "content": "Hello chat! Kappa",
            "user_color": "#FF0000",
            "emotes": [{
                "id": "25",
                "name": "Kappa",
                "url": "https://static-cdn.jtvnw.net/emoticons/v2/25/default/dark/2.0",
                "positions": [{"start": 12, "end": 17}]
            }],
            "badges": [{
                "id": "subscriber",
                "name": "Subscriber",
                "title": "12 month subscriber"
            }]
        }
    });

    println!(
        "\n{}\n",
        serde_json::to_string_pretty(&example_json).unwrap()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_username() {
        let username = generate_random_username();
        assert!(!username.is_empty());
        assert!(username.len() > 3);
    }

    #[test]
    fn test_generate_message() {
        let message = generate_random_message();
        assert!(!message.is_empty());
    }

    #[test]
    fn test_generate_color() {
        let color = generate_random_color();
        assert!(color.starts_with('#'));
        assert_eq!(color.len(), 7);
    }

    #[test]
    fn test_create_chat_payload() {
        let payload = create_random_chat_payload();
        assert!(!payload.username.is_empty());
        assert!(!payload.content.is_empty());
        assert!(payload.validate().is_ok());
    }

    #[test]
    fn test_message_validation() {
        let message = create_random_message();
        assert!(message.validate().is_ok());
    }

    #[test]
    fn test_json_serialization() {
        let message = create_random_message();
        let json = serde_json::to_string(&message).unwrap();
        let parsed: IncomingMessage = serde_json::from_str(&json).unwrap();
        assert!(parsed.validate().is_ok());
    }
}
