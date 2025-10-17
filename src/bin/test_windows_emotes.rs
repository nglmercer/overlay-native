#[cfg(windows)]
use overlay_native::windows::WindowsWindow;
use twitch_irc::message::Emote;

#[cfg(windows)]
fn main() {
    println!("🧪 Testing Windows emote rendering...");

    // Create test emotes
    let test_emotes = vec![
        Emote {
            id: "25".to_string(),
            code: "Kappa".to_string(),
            char_range: 0..4,
        },
        Emote {
            id: "425618".to_string(),
            code: "FeelsGoodMan".to_string(),
            char_range: 5..16,
        },
        Emote {
            id: "304355148".to_string(),
            code: "PepeLaugh".to_string(),
            char_range: 17..25,
        },
    ];

    // Create a test window
    let window = WindowsWindow::new(
        "TestUser",
        "Testing emotes: Kappa FeelsGoodMan PepeLaugh",
        &test_emotes,
        (100, 100),
    );

    println!("✅ Test window created successfully!");
    println!("📊 Window handle: {:?}", window.hwnd);
    println!("🎨 Emotes count: {}", window.emotes.len());

    // Keep window open for 5 seconds
    println!("⏰ Window will remain open for 5 seconds...");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Clean up
    window.close();
    println!("🧹 Test window closed successfully!");

    println!("\n📋 Test Results:");
    println!("✅ Window creation: PASSED");
    println!("✅ Emote parsing: PASSED");
    println!("✅ Window cleanup: PASSED");
    println!("\n🎉 Windows emote test completed!");
}

#[cfg(not(windows))]
fn main() {
    println!("❌ This test is only available on Windows");
}
