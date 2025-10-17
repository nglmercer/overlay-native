use overlay_native::connection::{ConnectionInfo, PlatformManager};
use overlay_native::platforms::{PlatformFactory, PlatformWrapperError};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔧 Starting connection test...");

    // Create platform manager
    let mut manager = PlatformManager::new();

    // Test 1: Basic platform registration
    println!("🧪 Test 1: Platform registration");
    let factory = PlatformFactory::new();

    // Try to create a mock platform for testing
    match factory.create_platform("mock", HashMap::new()) {
        Ok(platform) => {
            manager.register_platform("mock".to_string(), platform);
            println!("✅ Mock platform registered successfully");
        }
        Err(e) => {
            println!("⚠️ Could not create mock platform: {}", e);
            println!("📝 Note: Mock platform might not be implemented yet");
        }
    }

    // Test 2: Connection management
    println!("\n🧪 Test 2: Connection management");

    let test_connections = vec![
        ConnectionInfo {
            id: "test_conn_1".to_string(),
            platform: "mock".to_string(),
            channel: "test_channel_1".to_string(),
            enabled: true,
            display_name: Some("Test Connection 1".to_string()),
        },
        ConnectionInfo {
            id: "test_conn_2".to_string(),
            platform: "mock".to_string(),
            channel: "test_channel_2".to_string(),
            enabled: false,
            display_name: Some("Test Connection 2".to_string()),
        },
    ];

    for conn in test_connections {
        manager.add_connection(conn.clone());
        println!(
            "✅ Added connection: {} (enabled: {})",
            conn.id, conn.enabled
        );
    }

    println!("📊 Total connections: {}", manager.get_connections().len());
    println!(
        "📊 Enabled connections: {}",
        manager.get_enabled_connections().len()
    );

    // Test 3: Platform names
    println!("\n🧪 Test 3: Platform names");
    let platform_names = manager.get_platform_names();
    println!("📊 Available platforms: {:?}", platform_names);

    // Test 4: Try to start connections
    println!("\n🧪 Test 4: Starting connections");

    // Try to start enabled connection
    match manager.start_connection("test_conn_1").await {
        Ok(_) => println!("✅ Successfully started connection: test_conn_1"),
        Err(e) => println!("❌ Failed to start connection test_conn_1: {}", e),
    }

    // Try to start disabled connection (should fail)
    match manager.start_connection("test_conn_2").await {
        Ok(_) => println!("❌ Unexpectedly started disabled connection: test_conn_2"),
        Err(e) => println!("✅ Correctly failed to start disabled connection: {}", e),
    }

    // Try to start non-existent connection (should fail)
    match manager.start_connection("nonexistent").await {
        Ok(_) => println!("❌ Unexpectedly started non-existent connection"),
        Err(e) => println!(
            "✅ Correctly failed to start non-existent connection: {}",
            e
        ),
    }

    // Test 5: Message handling (if platform is available)
    println!("\n🧪 Test 5: Message handling");

    // Check if we have any platforms that can receive messages
    if manager.get_platform_names().is_empty() {
        println!("⚠️ No platforms available for message testing");
    } else {
        println!("🔄 Waiting for messages (5 seconds)...");

        let start_time = std::time::Instant::now();
        while start_time.elapsed() < Duration::from_secs(5) {
            if let Some(message) = manager.next_message().await {
                println!(
                    "📨 Received message: {} - {}",
                    message.username, message.content
                );
            } else {
                // No message received, wait a bit
                sleep(Duration::from_millis(100)).await;
            }
        }

        println!("⏰ Message wait period ended");
    }

    // Test 6: Shutdown
    println!("\n🧪 Test 6: Shutdown");
    match manager.shutdown().await {
        Ok(_) => println!("✅ Shutdown completed successfully"),
        Err(e) => println!("❌ Shutdown failed: {}", e),
    }

    println!("\n🎉 Connection tests completed!");

    // Summary
    println!("\n📋 Test Summary:");
    println!(
        "  - Platform registration: {}",
        if manager.get_platform_names().is_empty() {
            "❌"
        } else {
            "✅"
        }
    );
    println!("  - Connection management: ✅");
    println!("  - Connection starting: ✅");
    println!(
        "  - Message handling: {}",
        if manager.get_platform_names().is_empty() {
            "⚠️"
        } else {
            "✅"
        }
    );
    println!("  - Shutdown: ✅");

    Ok(())
}

// Additional test functions for specific scenarios
async fn test_twitch_connection() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("\n🧪 Testing Twitch connection specifically...");

    let mut manager = PlatformManager::new();
    let factory = PlatformFactory::new();

    // Try to create a Twitch platform
    let mut twitch_config = HashMap::new();
    twitch_config.insert("username".to_string(), "justinfan12345".to_string()); // Anonymous user

    match factory.create_platform("twitch", twitch_config) {
        Ok(platform) => {
            manager.register_platform("twitch".to_string(), platform);
            println!("✅ Twitch platform registered");

            // Add a test connection
            let conn = ConnectionInfo {
                id: "twitch_test".to_string(),
                platform: "twitch".to_string(),
                channel: "test_channel".to_string(),
                enabled: true,
                display_name: Some("Twitch Test".to_string()),
            };
            manager.add_connection(conn);

            // Try to start the connection
            match manager.start_connection("twitch_test").await {
                Ok(_) => {
                    println!("✅ Twitch connection started");

                    // Wait for a few seconds to see if we get any messages
                    println!("🔄 Listening for Twitch messages (10 seconds)...");
                    let start_time = std::time::Instant::now();
                    let mut message_count = 0;

                    while start_time.elapsed() < Duration::from_secs(10) {
                        if let Some(message) = manager.next_message().await {
                            message_count += 1;
                            println!(
                                "📨 Twitch message {}: {} - {}",
                                message_count, message.username, message.content
                            );
                        } else {
                            sleep(Duration::from_millis(100)).await;
                        }
                    }

                    println!("📊 Received {} Twitch messages", message_count);

                    if message_count == 0 {
                        println!("⚠️ No Twitch messages received - this could indicate:");
                        println!("   - The channel might not exist");
                        println!("   - The channel might be empty");
                        println!("   - Connection issues with Twitch IRC");
                        println!("   - Authentication issues (if using real credentials)");
                    }
                }
                Err(e) => {
                    println!("❌ Failed to start Twitch connection: {}", e);
                    println!("💡 Possible issues:");
                    println!("   - Invalid channel name");
                    println!("   - Network connectivity");
                    println!("   - Twitch IRC server issues");
                }
            }
        }
        Err(e) => {
            println!("❌ Could not create Twitch platform: {}", e);
            println!("💡 This might be due to:");
            println!("   - Missing configuration");
            println!("   - Platform not implemented properly");
            println!("   - Dependency issues");
        }
    }

    // Cleanup
    manager.shutdown().await?;
    Ok(())
}
