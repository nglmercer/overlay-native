use overlay_native::config::{PlatformConfig, PlatformType, Credentials, PlatformSettings};
use overlay_native::platforms::{KickCreator, PlatformCreator};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Kick platform integration with kick_rust library...");

    // Create Kick platform creator
    let kick_creator = KickCreator;

    // Create platform configuration
    let config = PlatformConfig {
        platform_type: PlatformType::Kick,
        enabled: true,
        credentials: Credentials {
            username: Some("test_user".to_string()),
            oauth_token: None,
            api_key: None,
            client_id: None,
            client_secret: None,
        },
        settings: PlatformSettings {
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
            message_buffer_size: 1000,
            enable_emotes: true,
            enable_badges: true,
            custom_settings: std::collections::HashMap::new(),
        },
    };

    // Create the platform
    let mut platform = kick_creator.create(config).await?;

    println!("✓ Kick platform created successfully");
    println!("Platform name: {}", platform.platform_name());

    // Connect to the platform
    platform.connect().await?;
    println!("✓ Connected to Kick platform");

    // Try to join a channel (this may fail if channel doesn't exist, but that's expected)
    match platform.join_channel("test_channel".to_string()).await {
        Ok(()) => println!("✓ Joined test channel"),
        Err(e) => println!("Expected error joining test channel: {}", e),
    }

    println!("✓ Kick integration test completed successfully!");

    Ok(())
}
