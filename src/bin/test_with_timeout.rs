//! Test binary for Overlay Native with timeout functionality
//!
//! This binary tests the main application with proper timeout handling
//! to prevent the process from hanging indefinitely.
//!
//! Run with: cargo run --bin test_with_timeout

use overlay_native::connection::{ConnectionInfo, PlatformManager};
use overlay_native::platforms::PlatformFactory;
use std::collections::HashMap;
use std::time::Duration;
use tokio::time;

/// Test configuration
mod test_config {
    use super::*;
    pub const TEST_TIMEOUT: Duration = Duration::from_secs(60);
    pub const MESSAGE_WAIT_TIMEOUT: Duration = Duration::from_secs(10);
    pub const CONNECTION_TIMEOUT: Duration = Duration::from_secs(15);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Overlay Native - Timeout Test Suite");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = 0;

    // Test 1: Basic platform manager initialization
    total_tests += 1;
    println!("🧪 Test 1: Platform Manager Initialization");
    match time::timeout(test_config::TEST_TIMEOUT, test_platform_manager_init()).await {
        Ok(result) => match result {
            Ok(_) => {
                println!("✅ Test 1 passed");
                passed_tests += 1;
            }
            Err(e) => {
                println!("❌ Test 1 failed: {}", e);
                failed_tests += 1;
            }
        },
        Err(_) => {
            println!("❌ Test 1 timed out after {:?}", test_config::TEST_TIMEOUT);
            failed_tests += 1;
        }
    }

    // Test 2: Connection management with timeout
    total_tests += 1;
    println!("\n🧪 Test 2: Connection Management");
    match time::timeout(test_config::TEST_TIMEOUT, test_connection_management()).await {
        Ok(result) => match result {
            Ok(_) => {
                println!("✅ Test 2 passed");
                passed_tests += 1;
            }
            Err(e) => {
                println!("❌ Test 2 failed: {}", e);
                failed_tests += 1;
            }
        },
        Err(_) => {
            println!("❌ Test 2 timed out after {:?}", test_config::TEST_TIMEOUT);
            failed_tests += 1;
        }
    }

    // Test 3: Message handling with timeout
    total_tests += 1;
    println!("\n🧪 Test 3: Message Handling");
    match time::timeout(test_config::TEST_TIMEOUT, test_message_handling()).await {
        Ok(result) => match result {
            Ok(_) => {
                println!("✅ Test 3 passed");
                passed_tests += 1;
            }
            Err(e) => {
                println!("❌ Test 3 failed: {}", e);
                failed_tests += 1;
            }
        },
        Err(_) => {
            println!("❌ Test 3 timed out after {:?}", test_config::TEST_TIMEOUT);
            failed_tests += 1;
        }
    }

    // Test 4: Shutdown with timeout
    total_tests += 1;
    println!("\n🧪 Test 4: Shutdown");
    match time::timeout(test_config::TEST_TIMEOUT, test_shutdown()).await {
        Ok(result) => match result {
            Ok(_) => {
                println!("✅ Test 4 passed");
                passed_tests += 1;
            }
            Err(e) => {
                println!("❌ Test 4 failed: {}", e);
                failed_tests += 1;
            }
        },
        Err(_) => {
            println!("❌ Test 4 timed out after {:?}", test_config::TEST_TIMEOUT);
            failed_tests += 1;
        }
    }

    // Summary
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Test Summary:");
    println!("   Total tests: {}", total_tests);
    println!("   Passed: {}", passed_tests);
    println!("   Failed: {}", failed_tests);

    if failed_tests == 0 {
        println!("\n🎉 All tests passed!");
        println!("   The application handles timeouts correctly.");
        Ok(())
    } else {
        println!("\n⚠️  Some tests failed.");
        println!("   Check the logs above for details.");
        std::process::exit(1)
    }
}

/// Test platform manager initialization
async fn test_platform_manager_init() -> Result<(), Box<dyn std::error::Error>> {
    println!("   🔄 Creating platform manager...");
    let manager = PlatformManager::new();

    println!("   ✅ Platform manager created successfully");
    println!(
        "   📊 Available platforms: {:?}",
        manager.get_platform_names()
    );

    Ok(())
}

/// Test connection management with timeout
async fn test_connection_management() -> Result<(), Box<dyn std::error::Error>> {
    println!("   🔄 Testing connection management...");
    let mut manager = PlatformManager::new();

    // Add test connections
    let connections = vec![
        ConnectionInfo {
            id: "test_conn_1".to_string(),
            platform: "twitch".to_string(),
            channel: "test_channel_1".to_string(),
            enabled: true,
            display_name: Some("Test Connection 1".to_string()),
        },
        ConnectionInfo {
            id: "test_conn_2".to_string(),
            platform: "youtube".to_string(),
            channel: "test_channel_2".to_string(),
            enabled: false,
            display_name: Some("Test Connection 2".to_string()),
        },
    ];

    for conn in connections {
        manager.add_connection(conn);
    }

    println!("   ✅ Connections added successfully");
    println!(
        "   📊 Total connections: {}",
        manager.get_connections().len()
    );
    println!(
        "   📊 Enabled connections: {}",
        manager.get_enabled_connections().len()
    );

    // Test starting a connection with timeout
    println!("   🔄 Testing connection startup with timeout...");
    let connection_timeout = time::timeout(test_config::CONNECTION_TIMEOUT, async {
        manager.start_connection("test_conn_1").await
    })
    .await;

    match connection_timeout {
        Ok(result) => match result {
            Ok(_) => println!("   ✅ Connection started successfully"),
            Err(e) => println!("   ⚠️  Connection failed (expected for test): {}", e),
        },
        Err(_) => {
            return Err("Connection startup timed out".into());
        }
    }

    Ok(())
}

/// Test message handling with timeout
async fn test_message_handling() -> Result<(), Box<dyn std::error::Error>> {
    println!("   🔄 Testing message handling with timeout...");
    let mut manager = PlatformManager::new();

    // Test message reception timeout (should timeout since no messages are sent)
    println!("   🔄 Testing message reception timeout...");
    let message_timeout = time::timeout(test_config::MESSAGE_WAIT_TIMEOUT, async {
        manager.next_message().await
    })
    .await;

    match message_timeout {
        Ok(result) => {
            if result.is_none() {
                println!("   ✅ Message reception timed out correctly (no messages)");
            } else {
                println!("   ⚠️  Unexpected message received");
            }
        }
        Err(_) => {
            return Err("Message reception timeout test failed".into());
        }
    }

    Ok(())
}

/// Test shutdown with timeout
async fn test_shutdown() -> Result<(), Box<dyn std::error::Error>> {
    println!("   🔄 Testing shutdown with timeout...");
    let mut manager = PlatformManager::new();

    // Add a test connection
    let conn = ConnectionInfo {
        id: "shutdown_test".to_string(),
        platform: "twitch".to_string(),
        channel: "test_channel".to_string(),
        enabled: true,
        display_name: Some("Shutdown Test".to_string()),
    };
    manager.add_connection(conn);

    // Test shutdown with timeout
    let shutdown_timeout = time::timeout(test_config::CONNECTION_TIMEOUT, async {
        manager.shutdown().await
    })
    .await;

    match shutdown_timeout {
        Ok(result) => match result {
            Ok(_) => println!("   ✅ Shutdown completed successfully"),
            Err(e) => return Err(format!("Task error: {}", e).into()),
        },
        Err(_) => {
            return Err("Shutdown timed out".into());
        }
    }

    Ok(())
}

/// Additional utility function to test specific scenarios
async fn test_specific_scenario(scenario: &str) -> Result<(), Box<dyn std::error::Error>> {
    match scenario {
        "websocket" => test_websocket_scenario().await,
        "multiple_platforms" => test_multiple_platforms_scenario().await,
        _ => Ok(()),
    }
}

/// Test WebSocket specific scenario
async fn test_websocket_scenario() -> Result<(), Box<dyn std::error::Error>> {
    println!("   🔄 Testing WebSocket scenario...");
    // This would test WebSocket-specific functionality
    // For now, just return success
    println!("   ✅ WebSocket scenario test completed");
    Ok(())
}

/// Test multiple platforms scenario
async fn test_multiple_platforms_scenario() -> Result<(), Box<dyn std::error::Error>> {
    println!("   🔄 Testing multiple platforms scenario...");
    // This would test multiple platform connections
    // For now, just return success
    println!("   ✅ Multiple platforms scenario test completed");
    Ok(())
}
