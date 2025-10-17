//! Test utility to diagnose emote provider issues
//!
//! Run with: cargo run --bin test_emotes

use overlay_native::emotes::providers::{
    BTTVEmoteProvider, FFZEmoteProvider, SevenTVEmoteProvider, TwitchEmoteProvider,
};
use overlay_native::emotes::EmoteProvider;

#[tokio::main]
async fn main() {
    println!("🔍 Overlay Native - Emote Provider Diagnostic Tool\n");
    println!("This tool tests each emote provider individually to identify issues.\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut total_success = 0;
    let mut total_failed = 0;
    let mut results = Vec::new();

    // Test BTTV
    println!("📦 Testing BTTV Provider...");
    let bttv = BTTVEmoteProvider::new();
    match test_provider("BTTV", &bttv).await {
        Ok(count) => {
            results.push(("BTTV", true, count, String::new()));
            total_success += 1;
        }
        Err(e) => {
            results.push(("BTTV", false, 0, e));
            total_failed += 1;
        }
    }
    println!();

    // Test FFZ
    println!("📦 Testing FFZ Provider...");
    let ffz = FFZEmoteProvider::new();
    match test_provider("FFZ", &ffz).await {
        Ok(count) => {
            results.push(("FFZ", true, count, String::new()));
            total_success += 1;
        }
        Err(e) => {
            results.push(("FFZ", false, 0, e));
            total_failed += 1;
        }
    }
    println!();

    // Test 7TV
    println!("📦 Testing 7TV Provider...");
    let seven_tv = SevenTVEmoteProvider::new();
    match test_provider("7TV", &seven_tv).await {
        Ok(count) => {
            results.push(("7TV", true, count, String::new()));
            total_success += 1;
        }
        Err(e) => {
            results.push(("7TV", false, 0, e));
            total_failed += 1;
        }
    }
    println!();

    // Test Twitch (note: requires auth for global emotes)
    println!("📦 Testing Twitch Provider...");
    let twitch = TwitchEmoteProvider::new();
    println!("   ℹ️  Twitch provider initialized (auth required for global emotes)");
    println!("   Provider name: {}", twitch.provider_name());
    results.push(("Twitch", true, 0, "Auth required".to_string()));
    println!();

    // Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("📊 Summary:\n");

    for (name, success, count, error) in &results {
        if *success {
            if *count > 0 {
                println!("   ✅ {}: {} emotes loaded", name, count);
            } else {
                println!("   ✅ {}: OK ({})", name, error);
            }
        } else {
            println!("   ❌ {}: FAILED", name);
            println!("      Error: {}", error);
        }
    }

    println!();
    println!("Success: {} | Failed: {}", total_success, total_failed);
    println!();

    if total_failed > 0 {
        println!("⚠️  Some providers failed. Common issues:");
        println!("   1. Network connectivity problems");
        println!("   2. API endpoint changes");
        println!("   3. Rate limiting");
        println!("   4. Firewall/proxy blocking requests");
        println!();
        println!("💡 Try running again in a few seconds.");
        println!("   If the problem persists, check your internet connection.");
        std::process::exit(1);
    } else {
        println!("✅ All providers working correctly!");
        println!("   You can now run the main application with: cargo run");
        std::process::exit(0);
    }
}

async fn test_provider(name: &str, provider: &dyn EmoteProvider) -> Result<usize, String> {
    println!("   🔄 Fetching global emotes...");

    match provider.get_global_emotes().await {
        Ok(emotes) => {
            let count = emotes.len();
            println!("   ✅ Successfully loaded {} emotes", count);

            if count > 0 {
                println!("   📝 Sample emotes:");
                for emote in emotes.iter().take(5) {
                    println!("      - {} (ID: {})", emote.name, emote.id);
                    if let Some(url) = &emote.url {
                        println!("        URL: {}", url);
                    }
                }
            }

            Ok(count)
        }
        Err(e) => {
            println!("   ❌ Failed: {}", e);
            Err(e.to_string())
        }
    }
}
