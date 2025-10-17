use overlay_native::config::{Credentials, PlatformConfig, PlatformSettings, PlatformType};

use overlay_native::platforms::PlatformFactory;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Kick platform connection...");

    // Test 1: Platform registration
    println!("\n🧪 Test 1: Platform registration");
    let factory = PlatformFactory::new();
    let platforms = factory.list_supported_platforms();
    println!("📊 Available platforms: {:?}", platforms);

    if !platforms.contains(&"kick".to_string()) {
        println!("❌ Kick platform not registered!");
        return Ok(());
    }
    println!("✅ Kick platform registered successfully");

    // Test 2: Create Kick platform instance
    println!("\n🧪 Test 2: Creating Kick platform instance");
    let config = PlatformConfig {
        platform_type: PlatformType::Kick,
        enabled: true,
        credentials: Credentials::default(),
        settings: PlatformSettings::default(),
    };

    let mut platform = factory.create_platform("kick", config).await?;
    println!("✅ Kick platform instance created successfully");

    // Test 3: Connect to WebSocket
    println!("\n🧪 Test 3: Connecting to Kick WebSocket");
    match platform.connect().await {
        Ok(()) => println!("✅ WebSocket connection established"),
        Err(e) => {
            println!("❌ Failed to connect: {}", e);
            return Ok(());
        }
    }

    // Test 4: Join channel
    println!("\n🧪 Test 4: Joining channel");
    match platform.join_channel("rodiksama".to_string()).await {
        Ok(()) => println!("✅ Successfully joined channel"),
        Err(e) => {
            println!("❌ Failed to join channel: {}", e);
            return Ok(());
        }
    }

    // Test 5: Listen for messages
    println!("\n🧪 Test 5: Listening for messages (10 seconds)");
    let message_count = 0;
    let max_messages = 5;

    let listen_task = tokio::spawn(async move {
        let mut platform = platform;
        let mut msg_count = 0;
        while msg_count < max_messages {
            match timeout(Duration::from_secs(2), platform.next_message()).await {
                Ok(Some(msg)) => {
                    println!("📨 Received message: {} - {}", msg.username, msg.content);
                    msg_count += 1;
                }
                Ok(None) => {
                    println!("⏳ No message received, continuing...");
                }
                Err(_) => {
                    println!("⏳ Timeout waiting for message, continuing...");
                }
            }
        }
        platform.disconnect().await.ok();
    });

    // Wait for 10 seconds total
    match timeout(Duration::from_secs(10), listen_task).await {
        Ok(Ok(())) => println!("✅ Message listening completed"),
        Ok(Err(e)) => println!("❌ Task error: {}", e),
        Err(_) => {
            println!("⏰ Test timeout reached");
            // The task is still running, but that's ok for this test
        }
    }

    println!("\n🎯 Test completed!");
    Ok(())
}
