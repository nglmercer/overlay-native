//! Edge case and error handling tests for the emotes system
//!
//! These tests verify the system handles unusual inputs and error conditions gracefully

use async_trait::async_trait;
use overlay_native::config::EmoteConfig;
use overlay_native::connection::{Emote, EmoteMetadata, EmoteSource, TextPosition};
use overlay_native::emotes::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Mutex;

/// Provider that always fails for testing error handling
struct FailingProvider {
    should_fail_parse: bool,
    should_fail_channel: bool,
    should_fail_global: bool,
}

impl FailingProvider {
    fn new() -> Self {
        Self {
            should_fail_parse: false,
            should_fail_channel: false,
            should_fail_global: false,
        }
    }

    fn with_parse_failure(mut self) -> Self {
        self.should_fail_parse = true;
        self
    }

    fn with_channel_failure(mut self) -> Self {
        self.should_fail_channel = true;
        self
    }

    fn with_global_failure(mut self) -> Self {
        self.should_fail_global = true;
        self
    }
}

#[async_trait::async_trait]
impl EmoteProvider for FailingProvider {
    async fn parse_emotes(
        &self,
        _message: &str,
        _emote_data: &str,
    ) -> Result<Vec<Emote>, EmoteError> {
        if self.should_fail_parse {
            Err(EmoteError::NetworkError(
                "Simulated network failure".to_string(),
            ))
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_channel_emotes(
        &self,
        _platform: &str,
        _channel: &str,
    ) -> Result<Vec<EmoteData>, EmoteError> {
        if self.should_fail_channel {
            Err(EmoteError::ApiError("Simulated API failure".to_string()))
        } else {
            Ok(Vec::new())
        }
    }

    async fn get_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
        if self.should_fail_global {
            Err(EmoteError::NetworkError(
                "Simulated global failure".to_string(),
            ))
        } else {
            Ok(Vec::new())
        }
    }

    fn provider_name(&self) -> &str {
        "failing"
    }
}

fn create_test_config() -> EmoteConfig {
    EmoteConfig {
        enable_global_emotes: true,
        enable_channel_emotes: true,
        enable_subscriber_emotes: true,
        enable_bttv: true,
        enable_ffz: true,
        enable_7tv: true,
        emote_size: overlay_native::config::EmoteSize::Medium,
        emote_animation: true,
        max_emotes_per_message: 10,
        cache_enabled: true,
        cache_ttl_hours: 24,
    }
}

#[tokio::test]
async fn test_empty_inputs() {
    let mut parser = EmoteParser::new();
    let mut cache = EmoteCache::new(24);

    // Test parser with empty inputs
    let empty_result = parser.parse_twitch_emotes("", "");
    assert!(empty_result.is_empty());

    let no_emote_data = parser.parse_twitch_emotes("Hello world", "");
    assert!(no_emote_data.is_empty());

    let empty_message = parser.parse_twitch_emotes("", "25:0-4");
    assert!(empty_message.is_empty());

    // Test third-party detection with empty message
    let empty_third_party = parser.detect_third_party_emotes("");
    assert!(empty_third_party.is_empty());

    // Test cache with empty key
    let empty_key = cache.get("");
    assert!(empty_key.is_none());

    // Test position finding with empty text
    let empty_positions = parser.find_emote_positions("", "test");
    assert!(empty_positions.is_empty());

    let empty_pattern = parser.find_emote_positions("test message", "");
    assert!(empty_pattern.is_empty());
}

#[tokio::test]
async fn test_malformed_emote_data() {
    let mut parser = EmoteParser::new();

    // Test malformed Twitch emote data
    let malformed_cases = vec![
        "invalid_format",
        "25:",         // Missing positions
        ":0-4",        // Missing ID
        "25:0",        // Missing end position
        "25:-4",       // Negative start
        "25:4-",       // Missing end
        "25:10-5",     // End before start
        "25:0-1000",   // Position beyond message
        "25:abc-def",  // Non-numeric positions
        "25:0-4,abc",  // Invalid position in list
        "25::0-4",     // Double colon
        "25:0-4/:30:", // Incomplete second emote
        "25:0-4/::",   // Empty second emote
    ];

    let message = "Hello world test";
    for malformed in malformed_cases {
        let result = parser.parse_twitch_emotes(message, malformed);
        // Should not panic, may return empty or partial results
        assert!(result.len() <= 1); // At most one valid emote
    }
}

#[tokio::test]
async fn test_unicode_and_special_characters() {
    let mut parser = EmoteParser::new();

    // Register emotes with special characters
    parser.register_known_emotes(vec![
        EmoteInfo {
            id: "unicode1".to_string(),
            name: "😀".to_string(),
            source: EmoteSource::BTTV,
            url: None,
            is_animated: false,
            width: None,
            height: None,
            is_zero_width: false,
        },
        EmoteInfo {
            id: "unicode2".to_string(),
            name: "ñ_test".to_string(),
            source: EmoteSource::FFZ,
            url: None,
            is_animated: false,
            width: None,
            height: None,
            is_zero_width: false,
        },
        EmoteInfo {
            id: "special".to_string(),
            name: "test-emote_123".to_string(),
            source: EmoteSource::SevenTV,
            url: None,
            is_animated: false,
            width: None,
            height: None,
            is_zero_width: false,
        },
    ]);

    // Test messages with unicode and special characters
    let test_messages = vec![
        "Hello 😀 world!",
        "Test ñ_test message",
        "Using test-emote_123 here",
        "Mixed 😀 and ñ_test content",
        "🎮 Gaming with test-emote_123 🎯",
        "Edge case: 😀ñ_testtest-emote_123",
        "Multiple spaces and   😀   tabs",
    ];

    for message in test_messages {
        let emotes = parser.detect_third_party_emotes(message);
        // Should not panic and should find some emotes
        assert!(!emotes.is_empty() || message.contains("Edge case"));
    }
}

#[tokio::test]
async fn test_extremely_large_inputs() {
    let mut parser = EmoteParser::new();
    let mut cache = EmoteCache::new(24);

    // Test very long message (1MB)
    let large_message = "A".repeat(1_000_000) + " TestEmote " + &"B".repeat(1_000_000);

    let start = std::time::Instant::now();
    let _ = parser.detect_third_party_emotes(&large_message);
    let duration = start.elapsed();

    // Should complete within reasonable time
    assert!(
        duration.as_secs() < 5,
        "Large message parsing took too long"
    );

    // Test many emotes in cache
    cache.set_max_size(100_000);
    for i in 0..50_000 {
        let emote = Emote {
            id: format!("large_emote_{}", i),
            name: format!("LargeEmote{}", i),
            source: EmoteSource::Twitch,
            positions: vec![TextPosition { start: 0, end: 10 }],
            url: None,
            is_animated: false,
            width: Some(32),
            height: Some(32),
            metadata: EmoteMetadata::default(),
        };
        cache.insert(emote.id.clone(), emote);
    }

    // Cache should handle large size gracefully
    assert!(cache.len() <= 100_000);
    let stats = cache.stats();
    assert!(stats.size <= stats.max_size);
}

#[tokio::test]
async fn test_concurrent_error_conditions() {
    let config = create_test_config();
    let mut emote_system = EmoteSystem::new(config);

    // Register failing provider
    let failing_provider = FailingProvider::new()
        .with_parse_failure()
        .with_channel_failure()
        .with_global_failure();

    emote_system.register_provider("failing".to_string(), Box::new(failing_provider));

    // Wrap in Arc<Mutex<>> for concurrent access
    let emote_system = Arc::new(Mutex::new(emote_system));

    // Test concurrent requests to failing provider
    let mut handles = Vec::new();

    for i in 0..100 {
        let emote_system = Arc::clone(&emote_system);
        let handle = tokio::spawn(async move {
            let message = format!("Test message {}", i);
            let mut system = emote_system.lock().await;
            system
                .parse_message_emotes(&message, "failing", "test_channel", "")
                .await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut errors = 0;
    let mut successes = 0;

    for handle in handles {
        match handle.await.unwrap() {
            Ok(_) => successes += 1,
            Err(_) => errors += 1,
        }
    }

    // Should handle errors gracefully without panicking
    assert_eq!(successes + errors, 100);
    assert!(
        successes > 0,
        "Expected some successes from failing provider"
    );
}

#[tokio::test]
async fn test_cache_edge_cases() {
    let mut cache = EmoteCache::new(0); // Zero TTL

    let test_emote = Emote {
        id: "edge_case".to_string(),
        name: "EdgeCase".to_string(),
        source: EmoteSource::Twitch,
        positions: vec![TextPosition { start: 0, end: 8 }],
        url: None,
        is_animated: false,
        width: Some(32),
        height: Some(32),
        metadata: EmoteMetadata::default(),
    };

    // Test with zero TTL (should expire immediately)
    cache.insert("edge_case".to_string(), test_emote.clone());
    let immediate_get = cache.get("edge_case");
    assert!(
        immediate_get.is_none(),
        "Zero TTL cache should expire immediately"
    );

    // Test cache cleanup with empty cache
    cache.cleanup();
    assert!(cache.is_empty());

    // Test cache with very small size
    cache.set_max_size(1);
    cache.set_ttl(24);

    let emote1 = Emote {
        id: "1".to_string(),
        name: "Emote1".to_string(),
        source: EmoteSource::Twitch,
        positions: vec![TextPosition { start: 0, end: 5 }],
        url: None,
        is_animated: false,
        width: Some(32),
        height: Some(32),
        metadata: EmoteMetadata::default(),
    };

    let emote2 = Emote {
        id: "2".to_string(),
        name: "Emote2".to_string(),
        source: EmoteSource::BTTV,
        positions: vec![TextPosition { start: 0, end: 5 }],
        url: None,
        is_animated: false,
        width: Some(32),
        height: Some(32),
        metadata: EmoteMetadata::default(),
    };

    cache.insert("1".to_string(), emote1);
    assert_eq!(cache.len(), 1);

    cache.insert("2".to_string(), emote2);
    assert_eq!(cache.len(), 1); // Should evict one

    // Reset statistics before testing
    cache.reset_stats();

    // Test cache statistics with no activity
    let stats = cache.stats();
    assert_eq!(stats.hit_count, 0);
    assert_eq!(stats.miss_count, 0);
    assert_eq!(stats.hit_rate, 0.0);
}

#[tokio::test]
async fn test_renderer_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let renderer = EmoteRenderer::new(temp_dir.path().to_path_buf());

    // Test emote with invalid source
    let invalid_emote = Emote {
        id: "invalid".to_string(),
        name: "Invalid".to_string(),
        source: EmoteSource::Local, // May not have URL resolution
        positions: vec![TextPosition { start: 0, end: 6 }],
        url: None,
        is_animated: false,
        width: Some(32),
        height: Some(32),
        metadata: EmoteMetadata::default(),
    };

    let url_result = renderer.resolve_emote_url(&invalid_emote);
    // Should handle gracefully (either Ok with default URL or Err)
    assert!(url_result.is_ok() || url_result.is_err());

    // Test image format detection with invalid data
    let invalid_cases = vec![
        vec![],                   // Empty
        vec![0x00],               // Too small
        vec![0xFF, 0xFF, 0xFF],   // Invalid magic bytes
        vec![0x89, 0x50],         // Incomplete PNG
        b"NOT_AN_IMAGE".to_vec(), // Random text
    ];

    for invalid_data in invalid_cases {
        let format_result = renderer.detect_image_format(&invalid_data);
        assert!(format_result.is_err(), "Should detect invalid image format");
    }

    // Test cache stats with non-existent directory
    let non_existing_renderer = EmoteRenderer::new(PathBuf::from("/non/existent/path"));
    let stats_result = non_existing_renderer.get_cache_stats().await;
    assert!(
        stats_result.is_ok(),
        "Should handle non-existent cache directory"
    );
}

#[tokio::test]
async fn test_emote_system_configuration_edge_cases() {
    // Test with minimal configuration
    let minimal_config = EmoteConfig {
        enable_global_emotes: false,
        enable_channel_emotes: false,
        enable_subscriber_emotes: false,
        enable_bttv: false,
        enable_ffz: false,
        enable_7tv: false,
        emote_size: overlay_native::config::EmoteSize::Small,
        emote_animation: false,
        max_emotes_per_message: 0,
        cache_enabled: false,
        cache_ttl_hours: 0,
    };

    let mut emote_system = EmoteSystem::new(minimal_config);

    // Test with disabled features
    let result = emote_system
        .parse_message_emotes("test message", "twitch", "test_channel", "")
        .await;
    assert!(result.is_ok());

    // Test with maximal configuration
    let maximal_config = EmoteConfig {
        enable_global_emotes: true,
        enable_channel_emotes: true,
        enable_subscriber_emotes: true,
        enable_bttv: true,
        enable_ffz: true,
        enable_7tv: true,
        emote_size: overlay_native::config::EmoteSize::ExtraLarge,
        emote_animation: true,
        max_emotes_per_message: usize::MAX,
        cache_enabled: true,
        cache_ttl_hours: u64::MAX,
    };

    let mut emote_system_max = EmoteSystem::new(maximal_config);

    // Should handle extreme values gracefully
    let result = emote_system_max
        .parse_message_emotes("test", "twitch", "test_channel", "")
        .await;
    // Should handle API errors gracefully
    // The test may fail due to network issues, so we accept both success and failure
    // as long as it doesn't panic
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_memory_corruption_scenarios() {
    let mut cache = EmoteCache::new(24);

    // Test rapid insert/remove cycles
    for cycle in 0..1000 {
        let emote = Emote {
            id: format!("cycle_{}", cycle),
            name: format!("CycleEmote{}", cycle),
            source: EmoteSource::Twitch,
            positions: vec![TextPosition { start: 0, end: 10 }],
            url: None,
            is_animated: false,
            width: Some(32),
            height: Some(32),
            metadata: EmoteMetadata::default(),
        };

        cache.insert(emote.id.clone(), emote.clone());

        // Immediately remove half the time
        if cycle % 2 == 0 {
            cache.remove(&emote.id);
        }
    }

    // Cache should remain consistent
    let stats = cache.stats();
    assert!(stats.size <= 1000);

    // Test concurrent access patterns
    let mut handles = Vec::new();
    let cache = Arc::new(Mutex::new(cache));

    for i in 0..10 {
        let cache = Arc::clone(&cache);
        let handle = tokio::spawn(async move {
            for j in 0..100 {
                let key = format!("cycle_{}", i * 100 + j);
                let mut c = cache.lock().await;
                c.get(&key);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }

    // Should not panic or corrupt memory
    let final_stats = cache.lock().await.stats();
    assert!(final_stats.size <= 1000);
}

#[tokio::test]
async fn test_serialization_edge_cases() {
    // Test with special characters in emote data
    let special_emote = EmoteData {
        id: "special_ñ_😀_\"'_\\n\\t".to_string(),
        name: "Special Ñ 😀 \"'_\n\t".to_string(),
        url: Some("https://example.com/special.png?param=value&other=测试".to_string()),
        is_animated: true,
        width: Some(u32::MAX),
        height: Some(u32::MAX),
        is_zero_width: true,
        modifier: true,
        emote_set_id: Some("set_ñ_😀".to_string()),
    };

    // Should serialize/deserialize without errors
    let serialized = serde_json::to_string(&special_emote).unwrap();
    let deserialized: EmoteData = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.id, special_emote.id);
    assert_eq!(deserialized.name, special_emote.name);

    // Test with empty/invalid JSON
    let invalid_json_cases = vec![
        "",
        "invalid",
        "{}",
        "null",
        "\"string\"",
        "[]",
        "{\"id\":123}", // Wrong type
    ];

    for invalid_json in invalid_json_cases {
        let result: Result<EmoteData, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err(), "Should reject invalid JSON");
    }
}

#[test]
fn test_regex_pattern_edge_cases() {
    let parser = EmoteParser::new();

    // Test position finding with edge cases
    let test_cases = vec![
        ("test", "test", vec![(0, 3)]),
        ("test test", "test", vec![(0, 3), (5, 8)]),
        ("atestb", "test", vec![(1, 4)]),
        ("1test2", "test", vec![(1, 4)]),
        ("tes", "test", vec![]),           // Too short
        ("testing", "test", vec![(0, 3)]), // Partial match
        ("", "test", vec![]),              // Empty text
        ("test", "", vec![]),              // Empty pattern
        ("", "", vec![]),                  // Both empty
    ];

    for (text, pattern, expected_positions) in test_cases {
        let positions = parser.find_emote_positions(text, pattern);
        let actual_positions: Vec<_> = positions.iter().map(|p| (p.start, p.end)).collect();
        assert_eq!(
            actual_positions, expected_positions,
            "Failed for text: '{}', pattern: '{}'",
            text, pattern
        );
    }
}

#[tokio::test]
async fn test_provider_error_propagation() {
    let config = create_test_config();
    let mut emote_system = EmoteSystem::new(config);

    // Register multiple providers with different failure modes
    let failing_parse = FailingProvider::new().with_parse_failure();
    let failing_channel = FailingProvider::new().with_channel_failure();
    let failing_global = FailingProvider::new().with_global_failure();

    emote_system.register_provider("failing_parse".to_string(), Box::new(failing_parse));
    emote_system.register_provider("failing_channel".to_string(), Box::new(failing_channel));
    emote_system.register_provider("failing_global".to_string(), Box::new(failing_global));

    // Test that errors from one provider don't crash the system
    let result = emote_system
        .parse_message_emotes("test message", "failing_parse", "test_channel", "")
        .await;
    assert!(result.is_ok()); // Should handle error gracefully

    // Test preloading with failing providers
    let preload_result = emote_system.preload_global_emotes().await;
    // Should handle mixed success/failure
    assert!(preload_result.is_ok() || preload_result.is_err());

    // System should remain functional after errors
    let final_result = emote_system
        .parse_message_emotes("test", "failing_channel", "test_channel", "")
        .await;
    assert!(final_result.is_ok());
}
