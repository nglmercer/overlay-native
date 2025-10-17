//! Criterion benchmarks for the emotes system
//!
//! These benchmarks measure the performance of critical emote operations
//! under various conditions and workloads.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use overlay_native::connection::{Emote, EmoteMetadata, EmoteSource, TextPosition};
use overlay_native::emotes::*;
use std::collections::HashMap;
use std::time::Duration;

/// Create test emote data for benchmarking
fn create_benchmark_emote_data(count: usize) -> Vec<EmoteData> {
    let mut emotes = Vec::with_capacity(count);
    for i in 0..count {
        emotes.push(EmoteData {
            id: format!("benchmark_emote_{}", i),
            name: format!("BenchmarkEmote{}", i),
            url: Some(format!("https://example.com/emote_{}.png", i)),
            is_animated: i % 10 == 0,
            width: Some(32),
            height: Some(32),
            is_zero_width: i % 100 == 0,
            modifier: i % 50 == 0,
            emote_set_id: Some(format!("set_{}", i / 10)),
        });
    }
    emotes
}

/// Create test emote for benchmarking
fn create_benchmark_emote(id: &str, name: &str) -> Emote {
    Emote {
        id: id.to_string(),
        name: name.to_string(),
        source: EmoteSource::Twitch,
        positions: vec![TextPosition {
            start: 0,
            end: name.len() - 1,
        }],
        url: Some(format!("https://example.com/{}.png", name)),
        is_animated: false,
        width: Some(32),
        height: Some(32),
        metadata: EmoteMetadata::default(),
    }
}

/// Generate test message with emotes
fn generate_benchmark_message(emote_count: usize, total_length: usize) -> String {
    let mut message = String::new();
    let mut current_length = 0;

    while current_length < total_length {
        if current_length > 0 {
            message.push(' ');
            current_length += 1;
        }

        let emote_name = format!("BenchmarkEmote{}", current_length % emote_count);
        message.push_str(&emote_name);
        current_length += emote_name.len();
    }

    message
}

fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");

    // Benchmark cache insertions
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("cache_insert", size), size, |b, &size| {
            b.iter(|| {
                let mut cache = EmoteCache::new(24);
                cache.set_max_size(size);

                for i in 0..size {
                    let emote =
                        create_benchmark_emote(&format!("emote_{}", i), &format!("Emote{}", i));
                    cache.insert(emote.id.clone(), black_box(emote));
                }
            });
        });
    }

    // Benchmark cache lookups
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("cache_lookup", size), size, |b, &size| {
            // Pre-populate cache
            let mut cache = EmoteCache::new(24);
            cache.set_max_size(size);

            for i in 0..size {
                let emote = create_benchmark_emote(&format!("emote_{}", i), &format!("Emote{}", i));
                cache.insert(emote.id.clone(), emote);
            }

            b.iter(|| {
                for i in 0..size.min(1000) {
                    let key = format!("emote_{}", i % size);
                    black_box(cache.get(&key));
                }
            });
        });
    }

    // Benchmark cache eviction
    for size in [100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("cache_eviction", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let mut cache = EmoteCache::new(24);
                    cache.set_max_size(size);

                    // Insert more items than capacity to trigger eviction
                    for i in 0..size * 2 {
                        let emote = create_benchmark_emote(
                            &format!("evict_emote_{}", i),
                            &format!("EvictEmote{}", i),
                        );
                        cache.insert(emote.id.clone(), black_box(emote));
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_parser_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_operations");

    // Benchmark Twitch emote parsing
    let twitch_cases = [
        ("simple", "Hello Kappa world", "25:6-10"),
        (
            "multiple",
            "Kappa PogChamp LUL",
            "25:0-4/305954156:6-13/13:15-17",
        ),
        (
            "complex",
            "This Kappa is PogChamp and LUL",
            "25:5-9/305954156:14-21/13:26-28",
        ),
    ];

    for (name, message, emote_data) in twitch_cases.iter() {
        group.bench_with_input(
            BenchmarkId::new("twitch_parse", name),
            &(message, emote_data),
            |b, (message, emote_data)| {
                let parser = EmoteParser::new();
                b.iter(|| {
                    black_box(parser.parse_twitch_emotes(message, emote_data));
                });
            },
        );
    }

    // Benchmark third-party emote detection
    for emote_count in [100, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::new("third_party_detection", emote_count),
            emote_count,
            |b, &emote_count| {
                let mut parser = EmoteParser::new();

                // Register known emotes
                let known_emotes = create_benchmark_emote_data(emote_count);
                let emote_info: Vec<_> = known_emotes
                    .into_iter()
                    .map(|e| EmoteInfo {
                        id: e.id,
                        name: e.name,
                        source: EmoteSource::BTTV,
                        url: e.url,
                        is_animated: e.is_animated,
                        width: e.width,
                        height: e.height,
                        is_zero_width: e.is_zero_width,
                    })
                    .collect();
                parser.register_known_emotes(emote_info);

                let message = generate_benchmark_message(emote_count.min(100), 500);

                b.iter(|| {
                    black_box(parser.detect_third_party_emotes(&message));
                });
            },
        );
    }

    // Benchmark position finding
    for (name, text, pattern) in [
        ("short", "Hello Kappa world", "Kappa"),
        ("long_repeated", "Kappa ".repeat(100), "Kappa"),
        (
            "complex",
            "This is a complex message with multiple BenchmarkEmote0 and BenchmarkEmote1 patterns",
            "BenchmarkEmote0",
        ),
    ]
    .iter()
    {
        group.bench_with_input(
            BenchmarkId::new("position_finding", name),
            &(text, pattern),
            |b, (text, pattern)| {
                let parser = EmoteParser::new();
                b.iter(|| {
                    black_box(parser.find_emote_positions(text, pattern));
                });
            },
        );
    }

    group.finish();
}

fn bench_emote_system(c: &mut Criterion) {
    let mut group = c.benchmark_group("emote_system");

    // Benchmark message parsing with different emote loads
    for emote_count in [5, 10, 25].iter() {
        group.bench_with_input(
            BenchmarkId::new("parse_message", emote_count),
            emote_count,
            |b, &emote_count| {
                let config = crate::config::EmoteConfig::default();
                let mut emote_system = EmoteSystem::new(config);

                // Register mock provider
                struct FastMockProvider {
                    emotes: Vec<EmoteData>,
                }

                #[async_trait::async_trait]
                impl EmoteProvider for FastMockProvider {
                    async fn parse_emotes(
                        &self,
                        message: &str,
                        _emote_data: &str,
                    ) -> Result<Vec<Emote>, EmoteError> {
                        let mut emotes = Vec::new();
                        let words: Vec<&str> = message.split_whitespace().collect();

                        for (word_index, word) in words.iter().enumerate() {
                            if let Some(emote_data) = self.emotes.iter().find(|e| e.name == *word) {
                                let start_pos = if word_index == 0 {
                                    0
                                } else {
                                    words[..word_index]
                                        .iter()
                                        .map(|w| w.len() + 1)
                                        .sum::<usize>()
                                };
                                let end_pos = start_pos + word.len() - 1;

                                emotes.push(Emote {
                                    id: emote_data.id.clone(),
                                    name: emote_data.name.clone(),
                                    source: EmoteSource::Local,
                                    positions: vec![TextPosition {
                                        start: start_pos,
                                        end: end_pos,
                                    }],
                                    url: emote_data.url.clone(),
                                    is_animated: emote_data.is_animated,
                                    width: emote_data.width,
                                    height: emote_data.height,
                                    metadata: EmoteMetadata {
                                        is_zero_width: emote_data.is_zero_width,
                                        modifier: emote_data.modifier,
                                        emote_set_id: emote_data.emote_set_id.clone(),
                                        tier: None,
                                    },
                                });
                            }
                        }
                        Ok(emotes)
                    }

                    async fn get_channel_emotes(
                        &self,
                        _platform: &str,
                        _channel: &str,
                    ) -> Result<Vec<EmoteData>, EmoteError> {
                        Ok(self.emotes.clone())
                    }

                    async fn get_global_emotes(&self) -> Result<Vec<EmoteData>, EmoteError> {
                        Ok(self.emotes.clone())
                    }

                    fn provider_name(&self) -> &str {
                        "fast_mock"
                    }
                }

                let mock_emotes = create_benchmark_emote_data(emote_count);
                let mock_provider = FastMockProvider {
                    emotes: mock_emotes,
                };
                emote_system.register_provider("mock".to_string(), Box::new(mock_provider));

                let message = generate_benchmark_message(emote_count, 100);

                let rt = tokio::runtime::Runtime::new().unwrap();
                b.iter(|| {
                    rt.block_on(async {
                        black_box(
                            emote_system
                                .parse_message_emotes(&message, "mock", "test_channel", "")
                                .await
                                .unwrap(),
                        )
                    });
                });
            },
        );
    }

    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    // Benchmark emote data serialization
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("emote_data_serialize", size),
            size,
            |b, &size| {
                let emotes = create_benchmark_emote_data(size);
                b.iter(|| {
                    black_box(serde_json::to_string(&emotes).unwrap());
                });
            },
        );
    }

    // Benchmark emote data deserialization
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("emote_data_deserialize", size),
            size,
            |b, &size| {
                let emotes = create_benchmark_emote_data(size);
                let serialized = serde_json::to_string(&emotes).unwrap();
                b.iter(|| {
                    black_box(serde_json::from_str::<Vec<EmoteData>>(&serialized).unwrap());
                });
            },
        );
    }

    group.finish();
}

fn bench_renderer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("renderer_operations");

    // Benchmark URL resolution for different sources
    let sources = [
        (EmoteSource::Twitch, "25"),
        (EmoteSource::BTTV, "5e7c3560b4d743c5830f0ae4"),
        (EmoteSource::FFZ, "300376284"),
        (EmoteSource::SevenTV, "6123530e941b9435be2a3a4e"),
    ];

    for (source, id) in sources.iter() {
        group.bench_with_input(
            BenchmarkId::new("url_resolution", format!("{:?}", source)),
            &(source, id),
            |b, (source, id)| {
                let renderer = EmoteRenderer::new(std::env::temp_dir().join("benchmark"));
                let emote = Emote {
                    id: id.to_string(),
                    name: format!("Benchmark{}", id),
                    source: source.clone(),
                    positions: vec![TextPosition { start: 0, end: 8 }],
                    url: None,
                    is_animated: false,
                    width: Some(32),
                    height: Some(32),
                    metadata: EmoteMetadata::default(),
                };

                b.iter(|| {
                    black_box(renderer.resolve_emote_url(&emote).unwrap());
                });
            },
        );
    }

    // Benchmark image format detection
    let test_cases = [
        ("png", vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        ("gif", b"GIF89a".to_vec()),
        (
            "webp",
            vec![
                0x52, 0x49, 0x46, 0x46, 0x00, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50,
            ],
        ),
        ("jpg", vec![0xFF, 0xD8, 0xFF, 0xE0]),
    ];

    for (name, data) in test_cases.iter() {
        group.bench_with_input(
            BenchmarkId::new("format_detection", name),
            data,
            |b, data| {
                let renderer = EmoteRenderer::new(std::env::temp_dir().join("benchmark"));
                b.iter(|| {
                    black_box(renderer.detect_image_format(data).unwrap());
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    // Benchmark concurrent cache access
    for thread_count in [2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_cache_access", thread_count),
            thread_count,
            |b, &thread_count| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                b.to_async(&rt).iter(|| async {
                    let mut cache = EmoteCache::new(24);
                    cache.set_max_size(1000);

                    // Pre-populate cache
                    for i in 0..500 {
                        let emote = create_benchmark_emote(
                            &format!("concurrent_{}", i),
                            &format!("Concurrent{}", i),
                        );
                        cache.insert(emote.id.clone(), emote);
                    }

                    let mut handles = Vec::new();

                    for thread in 0..thread_count {
                        let cache = &cache;
                        let handle = tokio::spawn(async move {
                            for i in 0..100 {
                                let key = format!("concurrent_{}", (i + thread * 100) % 500);
                                black_box(cache.get(&key));
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_operations,
    bench_parser_operations,
    bench_emote_system,
    bench_serialization,
    bench_renderer_operations,
    bench_concurrent_operations
);

criterion_main!(benches);
