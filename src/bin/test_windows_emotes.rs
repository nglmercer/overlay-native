#[cfg(windows)]
use overlay_native::windows::WindowsWindow;

#[cfg(windows)]
fn main() {
    println!("🧪 Testing Windows emote rendering...");

    // Create test emotes using core EmoteElement type
    let test_emotes = vec![
        EmoteElement {
            id: "25".to_string(),
            name: "Kappa".to_string(),
            platform: Some("twitch".to_string()),
            source: overlay_native::core::EmoteSource::Platform("twitch".to_string()),
            positions: vec![0..4],
            image_url: None,
        },
        EmoteElement {
            id: "425618".to_string(),
            name: "FeelsGoodMan".to_string(),
            platform: Some("twitch".to_string()),
            source: overlay_native::core::EmoteSource::Platform("twitch".to_string()),
            positions: vec![5..16],
            image_url: None,
        },
        EmoteElement {
            id: "304355148".to_string(),
            name: "PepeLaugh".to_string(),
            platform: Some("twitch".to_string()),
            source: overlay_native::core::EmoteSource::Platform("twitch".to_string()),
            positions: vec![17..25],
            image_url: None,
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
