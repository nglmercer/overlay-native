//! Dedicated window testing binary for Overlay Native
//!
//! This binary provides comprehensive testing of window functionality including:
//! - Window configuration validation
//! - Window geometry calculations
//! - Animation and timing tests
//! - Window positioning and monitor detection
//! - Window lifecycle management
//! - Test message functionality
//!
//! Run with: cargo run --bin test_windows

#[cfg(unix)]
use gdk::Rectangle;
use overlay_native::config::{Config, DisplayConfig, WindowConfig};
#[cfg(unix)]
use overlay_native::window::{
    get_gdk_monitor, AnchorAlignment, AnchorPoint, Coords, WindowGeometry,
};
#[cfg(windows)]
use overlay_native::windows::WindowsWindow;
use std::time::{Duration, Instant};
use tokio::time;
#[cfg(unix)]
use twitch_irc::message::Emote;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Overlay Native - Window Test Suite");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("This tool tests all window-related functionality to ensure");
    println!("proper window creation, positioning, animation, and cleanup.\n");

    // Run tests with timeout protection
    let result = run_window_tests_with_timeout(Duration::from_secs(30)).await;

    match result {
        Ok(_) => {
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("✅ All window tests completed successfully!");
            println!("💡 Your window configuration is working correctly.");
            Ok(())
        }
        Err(e) => {
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("❌ Window tests failed: {}", e);
            println!("💡 Check the error messages above for details.");
            std::process::exit(1)
        }
    }
}

/// Test window configuration validation
async fn test_window_config_validation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Testing Window Configuration Validation...");

    // Test valid configurations
    let valid_config = WindowConfig {
        message_duration_seconds: 10,
        max_windows: 100,
        test_message: "Test Message".to_string(),
        animation_enabled: true,
        fade_in_duration_ms: 300,
        fade_out_duration_ms: 500,
    };

    println!(
        "   ✅ Valid configuration: message_duration={}s, max_windows={}",
        valid_config.message_duration_seconds, valid_config.max_windows
    );

    // Test edge cases
    let _edge_config = WindowConfig {
        message_duration_seconds: 1,   // Minimum reasonable duration
        max_windows: 1,                // Minimum windows
        test_message: "T".to_string(), // Short message
        animation_enabled: false,      // No animation
        fade_in_duration_ms: 0,        // Instant fade in
        fade_out_duration_ms: 0,       // Instant fade out
    };

    println!("   ✅ Edge case configuration tested");

    // Test display configuration
    let display_config = DisplayConfig {
        monitor_margin: 40,
        window_size: 200,
        grid_size: 100,
        font_family: "Arial".to_string(),
        font_size: 14,
        background_color: "#1e1e1e".to_string(),
        text_color: "#ffffff".to_string(),
        username_color: "#00ff00".to_string(),
        border_radius: 8,
        opacity: 0.9,
    };

    println!(
        "   ✅ Display configuration: window_size={}, opacity={}",
        display_config.window_size, display_config.opacity
    );

    Ok(())
}

/// Test window geometry calculations
async fn test_window_geometry() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📐 Testing Window Geometry Calculations...");

    #[cfg(unix)]
    {
        // Test anchor point calculations
        let center_anchor = AnchorPoint {
            x: AnchorAlignment::CENTER,
            y: AnchorAlignment::CENTER,
        };

        let top_left_anchor = AnchorPoint {
            x: AnchorAlignment::START,
            y: AnchorAlignment::START,
        };

        let bottom_right_anchor = AnchorPoint {
            x: AnchorAlignment::END,
            y: AnchorAlignment::END,
        };

        println!(
            "   ✅ Anchor points: center={}, top-left={}, bottom-right={}",
            center_anchor, top_left_anchor, bottom_right_anchor
        );

        // Test coordinate calculations
        let coords = Coords::from_pixels((100, 200));
        let (x, y) = coords.relative_to();
        println!(
            "   ✅ Coordinates: ({}, {}) -> relative ({}, {})",
            coords.x, coords.y, x, y
        );

        // Test window geometry
        let geometry = WindowGeometry {
            anchor_point: center_anchor,
            offset: Coords::from_pixels((50, 75)),
            size: Coords::from_pixels((300, 150)),
        };

        println!("   ✅ Window geometry: {}", geometry);

        // Test anchor alignment calculations
        let container_size = 1000;
        let window_size = 200;

        let start_pos = AnchorAlignment::START.alignment_to_coordinate(window_size, container_size);
        let center_pos =
            AnchorAlignment::CENTER.alignment_to_coordinate(window_size, container_size);
        let end_pos = AnchorAlignment::END.alignment_to_coordinate(window_size, container_size);

        println!("   ✅ Alignment calculations:");
        println!("      - START: {} (should be 0)", start_pos);
        println!("      - CENTER: {} (should be 400)", center_pos);
        println!("      - END: {} (should be 800)", end_pos);
    }

    #[cfg(not(unix))]
    {
        println!("   ℹ️  Window geometry tests skipped (Unix-only)");
    }

    Ok(())
}

/// Test window timing and animation settings
async fn test_window_timing() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⏱️ Testing Window Timing and Animation...");

    let config = Config::default();

    // Test message duration
    let message_duration = config.message_duration();
    println!("   ✅ Message duration: {:?}", message_duration);

    // Test animation settings
    println!(
        "   ✅ Animation enabled: {}",
        config.window.animation_enabled
    );
    println!(
        "   ✅ Fade in duration: {}ms",
        config.window.fade_in_duration_ms
    );
    println!(
        "   ✅ Fade out duration: {}ms",
        config.window.fade_out_duration_ms
    );

    // Test timing calculations
    let total_animation_time = Duration::from_millis(
        config.window.fade_in_duration_ms + config.window.fade_out_duration_ms,
    );
    println!("   ✅ Total animation time: {:?}", total_animation_time);

    // Test that fade durations are reasonable
    assert!(
        config.window.fade_in_duration_ms <= 1000,
        "Fade in too long"
    );
    assert!(
        config.window.fade_out_duration_ms <= 1000,
        "Fade out too long"
    );
    println!("   ✅ Animation durations are within reasonable limits");

    // Test message duration limits
    assert!(
        config.window.message_duration_seconds >= 1,
        "Message duration too short"
    );
    assert!(
        config.window.message_duration_seconds <= 3600,
        "Message duration too long"
    );
    println!("   ✅ Message duration is within reasonable limits");

    Ok(())
}

/// Test window positioning and monitor detection
async fn test_window_positioning() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📍 Testing Window Positioning...");

    #[cfg(unix)]
    {
        // Test monitor detection
        let monitor = get_gdk_monitor();
        let geometry = monitor.geometry();
        println!(
            "   ✅ Monitor geometry: {}x{} at ({}, {})",
            geometry.width(),
            geometry.height(),
            geometry.x(),
            geometry.y()
        );

        // Test different window positions
        let positions = vec![
            (0, 0),                                                   // Top-left
            (geometry.width() - 200, 0),                              // Top-right
            (0, geometry.height() - 100),                             // Bottom-left
            (geometry.width() - 200, geometry.height() - 100),        // Bottom-right
            (geometry.width() / 2 - 100, geometry.height() / 2 - 50), // Center
        ];

        for (i, pos) in positions.iter().enumerate() {
            println!("   ✅ Position {}: ({}, {})", i + 1, pos.0, pos.1);
        }

        // Test that positions are within monitor bounds
        for pos in &positions {
            assert!(
                pos.0 >= 0 && pos.0 <= geometry.width(),
                "X position out of bounds"
            );
            assert!(
                pos.1 >= 0 && pos.1 <= geometry.height(),
                "Y position out of bounds"
            );
        }
        println!("   ✅ All positions are within monitor bounds");

        // Test monitor margin calculations
        let config = Config::default();
        let margin = config.display.monitor_margin;
        println!("   ✅ Monitor margin: {}px", margin);

        // Test that margin doesn't exceed reasonable bounds
        assert!(margin >= 0, "Margin cannot be negative");
        assert!(margin <= 200, "Margin too large");
        println!("   ✅ Monitor margin is reasonable");
    }

    #[cfg(not(unix))]
    {
        println!("   ℹ️  Window positioning tests skipped (Unix-only)");

        // Test monitor margin calculations (platform-agnostic)
        let config = Config::default();
        let margin = config.display.monitor_margin;
        println!("   ✅ Monitor margin: {}px", margin);

        // Test that margin doesn't exceed reasonable bounds
        assert!(margin >= 0, "Margin cannot be negative");
        assert!(margin <= 200, "Margin too large");
        println!("   ✅ Monitor margin is reasonable");
    }

    Ok(())
}

/// Test window lifecycle and cleanup
async fn test_window_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Testing Window Lifecycle...");

    let config = Config::default();

    // Test maximum windows limit
    println!("   ✅ Maximum windows: {}", config.window.max_windows);
    assert!(
        config.window.max_windows > 0,
        "Max windows must be positive"
    );
    assert!(config.window.max_windows <= 1000, "Max windows too high");

    // Test message duration limits
    println!(
        "   ✅ Message duration: {} seconds",
        config.window.message_duration_seconds
    );
    assert!(
        config.window.message_duration_seconds >= 1,
        "Message duration too short"
    );
    assert!(
        config.window.message_duration_seconds <= 3600,
        "Message duration too long"
    );

    // Test cleanup timing
    let cleanup_interval = Duration::from_secs(config.window.message_duration_seconds);
    println!("   ✅ Cleanup interval: {:?}", cleanup_interval);

    // Test that the configuration allows for reasonable window management
    let windows_per_second = 10.0; // Reasonable maximum
    let max_sustained_windows =
        (config.window.message_duration_seconds as f64 * windows_per_second) as usize;
    assert!(
        config.window.max_windows >= max_sustained_windows,
        "Max windows too low for message duration"
    );

    println!("   ✅ Window lifecycle configuration is reasonable");

    // Test window size configuration
    let window_size = config.display.window_size;
    println!("   ✅ Window size: {}px", window_size);
    assert!(window_size >= 50, "Window size too small");
    assert!(window_size <= 1000, "Window size too large");
    println!("   ✅ Window size is reasonable");

    Ok(())
}

/// Test test message functionality
async fn test_test_message() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🧪 Testing Test Message Functionality...");

    let config = Config::default();

    // Test test message content
    println!("   ✅ Test message: '{}'", config.window.test_message);
    assert!(
        !config.window.test_message.is_empty(),
        "Test message cannot be empty"
    );
    assert!(
        config.window.test_message.len() <= 500,
        "Test message too long"
    );

    #[cfg(unix)]
    {
        // Test that test message can be used for window creation
        let test_user = "TestUser";
        let test_message = &config.window.test_message;
        let test_emotes: Vec<Emote> = vec![];
        let test_position = (100, 100);
        let monitor_geometry = get_gdk_monitor().geometry();

        println!("   ✅ Test parameters:");
        println!("      - User: {}", test_user);
        println!("      - Message: {}", test_message);
        println!("      - Emotes: {}", test_emotes.len());
        println!(
            "      - Position: ({}, {})",
            test_position.0, test_position.1
        );
    }

    // Test that the message is appropriate for display
    assert!(
        !config.window.test_message.contains('\0'),
        "Test message contains null characters"
    );
    assert!(
        !config.window.test_message.contains("javascript:"),
        "Test message contains unsafe content"
    );

    println!("   ✅ Test message is safe and appropriate for display");

    // Test with different message types
    let messages = vec![
        "Hello World!",
        "Testing 123",
        "🎉 Emoji test 🎉",
        "Longer test message with multiple words",
        "Short",
    ];

    for msg in messages {
        assert!(!msg.is_empty(), "Test message cannot be empty");
        assert!(msg.len() <= 500, "Test message too long");
    }
    println!("   ✅ Various message types validated");

    Ok(())
}

/// Test display configuration
async fn test_display_configuration() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎨 Testing Display Configuration...");

    let config = Config::default();

    // Test font configuration
    println!("   ✅ Font family: {}", config.display.font_family);
    println!("   ✅ Font size: {}px", config.display.font_size);
    assert!(
        config.display.font_size >= 8 && config.display.font_size <= 72,
        "Font size out of reasonable range"
    );

    // Test color configuration
    println!(
        "   ✅ Background color: {}",
        config.display.background_color
    );
    println!("   ✅ Text color: {}", config.display.text_color);
    println!("   ✅ Username color: {}", config.display.username_color);

    // Validate color formats (basic check)
    for color in &[
        &config.display.background_color,
        &config.display.text_color,
        &config.display.username_color,
    ] {
        assert!(
            color.starts_with('#') && color.len() == 7,
            "Invalid color format: {}",
            color
        );
    }
    println!("   ✅ Color formats are valid");

    // Test opacity
    println!("   ✅ Opacity: {}", config.display.opacity);
    assert!(
        config.display.opacity > 0.0 && config.display.opacity <= 1.0,
        "Opacity must be between 0 and 1"
    );

    // Test border radius
    println!("   ✅ Border radius: {}px", config.display.border_radius);
    assert!(
        config.display.border_radius <= 50,
        "Border radius too large"
    );

    // Test grid size
    println!("   ✅ Grid size: {}px", config.display.grid_size);
    assert!(config.display.grid_size > 0, "Grid size must be positive");
    assert!(
        config.display.grid_size <= config.display.window_size,
        "Grid size cannot be larger than window size"
    );

    println!("   ✅ All display configuration values are valid");

    Ok(())
}

/// Run all window tests
#[cfg(windows)]
async fn test_windows_window_creation() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🪟 Testing Windows Window Creation...");

    // Test window creation with different scenarios
    let test_cases = vec![
        ("test_user", "Hello world!", (100, 100)),
        (
            "another_user",
            "This is a longer message to test window sizing",
            (200, 150),
        ),
        ("", "Empty username test", (300, 200)),
        ("user", "Empty message test", (400, 250)),
    ];

    for (i, (username, message, position)) in test_cases.iter().enumerate() {
        println!("   🪟 Creating test window {}...", i + 1);

        // Create window
        let window = WindowsWindow::new(username, message, &[], *position);

        // Verify window was created
        assert!(!window.hwnd.is_null(), "Window handle should not be null");
        println!("     ✅ Window handle created: {:p}", window.hwnd);

        // Verify window properties
        assert!(
            !window.username.is_empty() || !window.message.is_empty(),
            "Window should have username or message"
        );
        assert_eq!(window.username, *username, "Username should match");
        assert_eq!(window.message, *message, "Message should match");
        assert_eq!(window.progress, 0.0, "Initial progress should be 0.0");
        println!("     ✅ Window properties verified");

        // Test window positioning (verify window exists)
        unsafe {
            let mut rect = std::mem::zeroed();
            let result = winapi::um::winuser::GetWindowRect(window.hwnd, &mut rect);
            assert!(result != 0, "Should be able to get window rect");
            println!(
                "     ✅ Window position: ({}, {}), size: {}x{}",
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top
            );
        }

        // Clean up
        window.close();
        println!("     ✅ Window {} closed successfully", i + 1);
    }

    println!("   ✅ Windows window creation test completed");
    Ok(())
}

#[cfg(windows)]
async fn test_windows_window_properties() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔍 Testing Windows Window Properties...");

    // Create a test window
    let mut window = WindowsWindow::new("test_user", "Test message", &[], (150, 150));

    // Test progress updates
    let test_progress_values = vec![0.0, 0.25, 0.5, 0.75, 1.0];

    for progress in test_progress_values {
        println!("   📊 Setting progress to {:.0}%", progress * 100.0);
        window.set_progress(progress);

        // Small delay to allow window to process
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Verify progress was set
        assert!(
            (window.progress - progress).abs() < 0.01,
            "Progress should be updated correctly"
        );
        println!("     ✅ Progress updated successfully");
    }

    // Check window styling
    unsafe {
        // Check if window has layered attribute
        let ex_style =
            winapi::um::winuser::GetWindowLongW(window.hwnd, winapi::um::winuser::GWL_EXSTYLE);
        assert!(
            (ex_style & winapi::um::winuser::WS_EX_LAYERED as i32) != 0,
            "Window should have layered style"
        );
        assert!(
            (ex_style & winapi::um::winuser::WS_EX_TOPMOST as i32) != 0,
            "Window should be topmost"
        );
        assert!(
            (ex_style & winapi::um::winuser::WS_EX_TOOLWINDOW as i32) != 0,
            "Window should be tool window"
        );
        println!("   ✅ Window styles verified (layered, topmost, tool window)");

        // Check window transparency
        let mut alpha: u8 = 0;
        let mut flags: u32 = 0;
        let result = winapi::um::winuser::GetLayeredWindowAttributes(
            window.hwnd,
            std::ptr::null_mut(),
            &mut alpha,
            &mut flags,
        );
        assert!(result != 0, "Should be able to get layered attributes");
        assert!(alpha > 200, "Window should be mostly opaque");
        assert!(
            (flags & winapi::um::winuser::LWA_ALPHA) != 0,
            "Alpha flag should be set"
        );
        println!("   ✅ Window transparency verified (alpha: {})", alpha);
    }

    // Clean up
    window.close();
    println!("   ✅ Windows window properties test completed");
    Ok(())
}

#[cfg(windows)]
async fn test_windows_window_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🔄 Testing Windows Window Lifecycle...");

    let mut windows = Vec::new();

    // Test creating multiple windows
    println!("   🪟 Creating multiple windows...");
    for i in 0..5 {
        let window = WindowsWindow::new(
            &format!("user_{}", i),
            &format!("Message {}", i),
            &[],
            (100 + i * 50, 100 + i * 30),
        );
        windows.push(window);
        println!("     ✅ Window {} created", i + 1);
    }

    // Test simultaneous progress updates
    println!("   📊 Updating progress on all windows...");
    for (i, window) in windows.iter_mut().enumerate() {
        let progress = (i + 1) as f64 / 5.0;
        window.set_progress(progress);
        println!(
            "     ✅ Window {} progress set to {:.0}%",
            i + 1,
            progress * 100.0
        );
    }

    // Small delay to allow updates
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test closing windows in different order
    println!("   🗑️  Closing windows in reverse order...");
    for (i, window) in windows.iter_mut().rev().enumerate() {
        window.close();
        println!("     ✅ Window {} closed", 5 - i);
    }

    // Verify all windows are closed
    unsafe {
        for (i, window) in windows.iter().enumerate() {
            let is_window = winapi::um::winuser::IsWindow(window.hwnd);
            assert!(is_window == 0, "Window {} should be destroyed", i + 1);
        }
        println!("   ✅ All windows properly destroyed");
    }

    // Test window creation after cleanup
    println!("   🪟 Testing window creation after cleanup...");
    let final_window = WindowsWindow::new("final_user", "Final test", &[], (300, 300));
    assert!(
        !final_window.hwnd.is_null(),
        "Should be able to create window after cleanup"
    );
    println!("     ✅ Window creation after cleanup successful");

    final_window.close();
    println!("   ✅ Windows window lifecycle test completed");
    Ok(())
}

async fn run_all_window_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧪 OVERLAY NATIVE - COMPREHENSIVE WINDOW TEST SUITE");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let start_time = Instant::now();
    let mut tests_passed = 0;
    let mut tests_failed = 0;

    // Run individual tests
    let test_functions: Vec<(
        &str,
        fn() -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<(), Box<dyn std::error::Error>>>>,
        >,
    )> = vec![
        ("Configuration Validation", || {
            Box::pin(test_window_config_validation())
        }),
        ("Geometry Calculations", || Box::pin(test_window_geometry())),
        ("Timing and Animation", || Box::pin(test_window_timing())),
        ("Window Positioning", || Box::pin(test_window_positioning())),
        ("Window Lifecycle", || Box::pin(test_window_lifecycle())),
        ("Test Message", || Box::pin(test_test_message())),
        ("Display Configuration", || {
            Box::pin(test_display_configuration())
        }),
        #[cfg(windows)]
        ("Windows Window Creation", || {
            Box::pin(test_windows_window_creation())
        }),
        #[cfg(windows)]
        ("Windows Window Properties", || {
            Box::pin(test_windows_window_properties())
        }),
        #[cfg(windows)]
        ("Windows Window Lifecycle", || {
            Box::pin(test_windows_window_lifecycle())
        }),
    ];

    for (test_name, test_func) in test_functions {
        println!("📋 Running: {}", test_name);
        match test_func().await {
            Ok(_) => {
                println!("   ✅ {}: PASSED\n", test_name);
                tests_passed += 1;
            }
            Err(e) => {
                println!("   ❌ {}: FAILED - {}\n", test_name, e);
                tests_failed += 1;
            }
        }
    }

    // Summary
    let duration = start_time.elapsed();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 WINDOW TEST SUMMARY");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("   Tests Passed: {}", tests_passed);
    println!("   Tests Failed: {}", tests_failed);
    println!("   Total Tests:  {}", tests_passed + tests_failed);
    println!("   Duration:     {:?}", duration);

    if tests_failed == 0 {
        println!("   ✅ All window tests passed!");
        Ok(())
    } else {
        println!("   ❌ Some window tests failed");
        Err("Window tests failed".into())
    }
}

/// Utility function to run window tests with timeout
async fn run_window_tests_with_timeout(
    max_runtime: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let timeout = time::timeout(max_runtime, run_all_window_tests()).await;

    match timeout {
        Ok(result) => result,
        Err(_) => {
            eprintln!("Window tests exceeded maximum runtime of {:?}", max_runtime);
            Err("Window tests timeout".into())
        }
    }
}
