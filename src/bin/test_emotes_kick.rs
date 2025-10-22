use overlay_native::config::PlatformConfig;
use overlay_native::connection::{Emote, EmoteSource, EmoteMetadata, TextPosition, StreamingPlatform};
use overlay_native::platforms::kick::KickPlatform;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Kick emote parsing...");

    // Create Kick platform with default config
    let config = PlatformConfig::default();
    let platform = KickPlatform::new(config)?;

    // Test messages with different emote patterns
    let test_cases = vec![
        ("Simple emote", "[emote:4096237:rodiksamaChokitoHype]", 1),
        ("Multiple emotes", "[emote:4096237:rodiksamaChokitoHype][emote:37225:KEKLEO]", 2),
        ("Text with emotes", "Hello [emote:4096237:rodiksamaChokitoHype] world!", 1),
        ("No emotes", "Just regular text", 0),
        ("Mixed content", "[emote:37225:KEKLEO] Hello [emote:4096237:rodiksamaChokitoHype]!", 2),
    ];

    for (test_name, message, expected_emote_count) in test_cases {
        println!("\n📝 Test: {}", test_name);
        println!("🔤 Original message: {}", message);

        // Parse emotes
        let emotes = platform.parse_emotes(message, "");

        println!("🎯 Expected emotes: {}", expected_emote_count);
        println!("✅ Actual emotes: {}", emotes.len());

        if emotes.len() == expected_emote_count {
            println!("✅ Test passed!");
        } else {
            println!("❌ Test failed!");
        }

        // Display parsed emotes
        for (i, emote) in emotes.iter().enumerate() {
            println!("   🎭 Emote {}: {} (ID: {})", i + 1, emote.name, emote.id);
            println!("      📊 Positions: {:?}-{:?}",
                emote.positions.get(0).map(|p| p.start),
                emote.positions.get(0).map(|p| p.end));
            println!("      🔗 URL: {:?}", emote.url);
            println!("      🏷️  Source: {:?}", emote.source);
        }
    }

    println!("\n🎉 Emote parsing test completed!");
    Ok(())
}
