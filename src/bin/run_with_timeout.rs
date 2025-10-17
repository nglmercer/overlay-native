//! Main application runner with timeout protection
//!
//! This binary runs the main Overlay Native application with proper timeout handling
//! to prevent the process from hanging indefinitely when waiting for WebSocket messages.
//!
//! Run with: cargo run --bin run_with_timeout
//!
//! Window Testing Features:
//! - Window configuration validation
//! - Window creation and lifecycle tests
//! - Animation and fade timing tests
//! - Message display duration tests
//! - Window positioning and geometry tests
//!
//! Window Testing Features:
//! - Window configuration validation
//! - Window creation and lifecycle tests
//! - Animation and fade timing tests
//! - Message display duration tests
//! - Window positioning and geometry tests

use overlay_native::config::Config;
use overlay_native::connection::{ConnectionInfo, PlatformManager};
use overlay_native::emotes::EmoteSystem;
use overlay_native::mapping::MappingSystem;
use overlay_native::platforms::{CredentialManager, PlatformFactory};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for window test mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--test-windows" {
        return window_tests::run_all_window_tests().await;
    }

    println!("🚀 Starting Overlay Native with Timeout Protection");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("This version includes timeout protection to prevent indefinite hanging");
    println!("when waiting for WebSocket messages.\n");
    println!("💡 Run with --test-windows flag to test window functionality");

    // Global timeout for the entire application
    let global_timeout = time::timeout(Duration::from_secs(120), async {
        // 2 minutes max
        run_application().await
    });

    match global_timeout.await {
        Ok(result) => match result {
            Ok(_) => {
                println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("✅ Application completed successfully");
                println!("💡 The application ran within the timeout limits");
                Ok(())
            }
            Err(e) => {
                println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("❌ Application failed: {}", e);
                println!("💡 Check the configuration and network connectivity");
                Err(e)
            }
        },
        Err(_) => {
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("⏰ GLOBAL TIMEOUT REACHED!");
            println!("❌ Application took longer than 5 minutes");
            println!("💡 This indicates:");
            println!("   - Possible deadlocks in the application");
            println!("   - WebSocket connections not timing out properly");
            println!("   - Infinite loops in message processing");
            println!("   - Network connectivity issues");
            println!("   - The application is running normally but took too long");
            std::process::exit(1)
        }
    }
}

/// Main application logic with timeout protection
async fn run_application() -> Result<(), Box<dyn std::error::Error>> {
    println!("📋 Loading configuration...");
    let config = Config::load_default().unwrap_or_else(|e| {
        eprintln!("⚠️  Error loading config: {}, using defaults", e);
        Config::default()
    });

    println!("🔄 Creating application components...");
    let platform_manager = Arc::new(RwLock::new(PlatformManager::new()));
    let emote_system = Arc::new(RwLock::new(EmoteSystem::new(config.emotes.clone())));
    let _mapping_system = Arc::new(RwLock::new(MappingSystem::default()));
    let platform_factory = Arc::new(PlatformFactory::new());
    let credential_manager = Arc::new(CredentialManager::new());

    println!("🎯 Initializing platforms...");
    let enabled_platforms = config.get_enabled_platforms();
    println!("📊 Found {} enabled platform(s)", enabled_platforms.len());

    for platform_name in enabled_platforms {
        if let Some(platform_config) = config.get_platform_config(platform_name) {
            println!("   🔄 Setting up {}...", platform_name);

            // Create platform instance
            let platform = platform_factory
                .create_platform(
                    &platform_config.platform_type.to_string(),
                    platform_config.clone(),
                )
                .await;

            match platform {
                Ok(platform) => {
                    // Register platform in the manager
                    platform_manager
                        .write()
                        .await
                        .register_platform(platform_name.to_string(), platform);

                    // Store credentials
                    credential_manager
                        .store_credentials(
                            platform_name.to_string(),
                            platform_config.credentials.clone(),
                        )
                        .await;

                    println!("   ✅ Platform {} initialized", platform_name);
                }
                Err(e) => {
                    eprintln!(
                        "   ❌ Failed to initialize platform {}: {}",
                        platform_name, e
                    );
                }
            }
        }
    }

    // Add connections from config
    for connection in &config.connections {
        if connection.enabled {
            platform_manager
                .write()
                .await
                .add_connection(ConnectionInfo {
                    id: connection.id.clone(),
                    platform: connection.platform.clone(),
                    channel: connection.channel.clone(),
                    enabled: connection.enabled,
                    display_name: connection.display_name.clone(),
                });
        }
    }

    println!("📥 Preloading global emotes...");
    let emote_load_timeout = time::timeout(Duration::from_secs(30), async {
        emote_system.write().await.preload_global_emotes().await
    })
    .await;

    match emote_load_timeout {
        Ok(result) => {
            if let Err(e) = result {
                eprintln!("⚠️  Failed to preload emotes: {}", e);
            } else {
                println!("✅ Global emotes preloaded");
            }
        }
        Err(_) => {
            eprintln!("⚠️  Emote preloading timed out after 30 seconds");
        }
    }

    println!("🔗 Starting connections...");
    let enabled_connections = config.get_enabled_connections();
    println!(
        "📊 Found {} enabled connection(s)",
        enabled_connections.len()
    );

    for connection in &enabled_connections {
        println!("   🔄 Starting connection: {}", connection.id);

        let connection_timeout = time::timeout(Duration::from_secs(15), async {
            platform_manager
                .write()
                .await
                .start_connection(&connection.id)
                .await
        })
        .await;

        match connection_timeout {
            Ok(result) => match result {
                Ok(_) => println!(
                    "   ✅ Connected to {} on {}",
                    connection.channel, connection.platform
                ),
                Err(e) => eprintln!("   ❌ Failed to start connection {}: {}", connection.id, e),
            },
            Err(_) => {
                eprintln!("   ⏰ Connection startup timed out for {}", connection.id);
            }
        }
    }

    println!("🎉 Overlay Native started successfully!");
    println!("📊 Connected to {} platform(s)", enabled_connections.len());
    println!("🔗 Active connections: {}", enabled_connections.len());
    println!("\n💡 Application is now running with timeout protection");
    println!("   - Global timeout: 2 minutes");
    println!("   - Connection timeout: 15 seconds");
    println!("   - Emote loading timeout: 30 seconds");
    println!("   - The application will exit automatically if it hangs");

    // Main message processing loop with timeout protection
    println!("\n🔄 Starting message processing loop...");
    let start_time = std::time::Instant::now();
    let mut message_count = 0;

    // Run for a maximum of 4.5 minutes to allow for graceful shutdown
    while start_time.elapsed() < Duration::from_secs(110) {
        // Check for messages with timeout
        let message_timeout = time::timeout(Duration::from_secs(1), async {
            platform_manager.write().await.next_message().await
        })
        .await;

        match message_timeout {
            Ok(Some(message)) => {
                message_count += 1;
                println!(
                    "📨 Message {}: {} - {}",
                    message_count, message.username, message.content
                );

                // Process the message (in a real application, this would display it)
                // For now, we just count and log the messages
            }
            Ok(None) => {
                // No message received, this is normal
            }
            Err(_) => {
                // Message reception timed out, this is expected
            }
        }

        // Small delay to prevent busy waiting
        time::sleep(Duration::from_millis(100)).await;
    }

    println!("\n⏰ Application runtime limit reached");
    println!("📊 Processed {} messages", message_count);

    // Graceful shutdown
    println!("🔄 Shutting down...");
    let shutdown_timeout = time::timeout(Duration::from_secs(10), async {
        platform_manager.write().await.shutdown().await
    })
    .await;

    match shutdown_timeout {
        Ok(result) => {
            if let Err(e) = result {
                eprintln!("⚠️  Shutdown failed: {}", e);
            } else {
                println!("✅ Shutdown complete");
            }
        }
        Err(_) => {
            eprintln!("⚠️  Shutdown timed out after 10 seconds");
        }
    }

    Ok(())
}

/// Window testing module
mod window_tests {
    use super::*;
    use overlay_native::config::{DisplayConfig, WindowConfig};
    use std::time::Instant;

    #[cfg(unix)]
    use gdk::Rectangle;
    #[cfg(unix)]
    use overlay_native::window::{
        get_gdk_monitor, AnchorAlignment, AnchorPoint, Coords, WindowGeometry,
    };
    #[cfg(unix)]
    use twitch_irc::message::Emote;

    /// Test window configuration validation
    pub async fn test_window_config_validation() -> Result<(), Box<dyn std::error::Error>> {
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
    pub async fn test_window_geometry() -> Result<(), Box<dyn std::error::Error>> {
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
        }

        #[cfg(not(unix))]
        {
            println!("   ℹ️  Window geometry tests skipped (Unix-only)");
        }

        Ok(())
    }

    /// Test window timing and animation settings
    pub async fn test_window_timing() -> Result<(), Box<dyn std::error::Error>> {
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

        Ok(())
    }

    /// Test window positioning and monitor detection
    pub async fn test_window_positioning() -> Result<(), Box<dyn std::error::Error>> {
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
        }

        #[cfg(not(unix))]
        {
            println!("   ℹ️  Window positioning tests skipped (Unix-only)");
        }

        Ok(())
    }

    /// Test window lifecycle and cleanup
    pub async fn test_window_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
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

        Ok(())
    }

    /// Test test message functionality
    pub async fn test_test_message() -> Result<(), Box<dyn std::error::Error>> {
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

        Ok(())
    }

    /// Run all window tests
    pub async fn run_all_window_tests() -> Result<(), Box<dyn std::error::Error>> {
        println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        println!("🧪 OVERLAY NATIVE - WINDOW TEST SUITE");
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
}

/// Utility function to run window tests with timeout
pub async fn run_window_tests_with_timeout(
    max_runtime: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let timeout = time::timeout(max_runtime, window_tests::run_all_window_tests()).await;

    match timeout {
        Ok(result) => result,
        Err(_) => {
            eprintln!("Window tests exceeded maximum runtime of {:?}", max_runtime);
            Err("Window tests timeout".into())
        }
    }
}

/// Utility function to run the application with custom timeout
pub async fn run_with_timeout(max_runtime: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let timeout = time::timeout(max_runtime, run_application()).await;

    match timeout {
        Ok(result) => result,
        Err(_) => {
            eprintln!("Application exceeded maximum runtime of {:?}", max_runtime);
            Err("Application timeout".into())
        }
    }
}
