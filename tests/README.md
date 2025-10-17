# Emotes System Test Suite

This directory contains comprehensive tests for the overlay-native emotes system, covering functionality, performance, edge cases, and reliability.

## Test Categories

### 1. Integration Tests (`emotes_integration.rs`)
- **Purpose**: Verify complete emotes workflow from parsing to rendering
- **Coverage**:
  - EmoteSystem basic functionality
  - Cache lifecycle management
  - Parser accuracy
  - Renderer operations
  - Third-party provider integration
  - Configuration updates
  - Concurrent access patterns

### 2. Edge Case Tests (`emotes_edge_cases.rs`)
- **Purpose**: Handle unusual inputs and error conditions gracefully
- **Scenarios**:
  - Empty/null inputs
  - Malformed emote data
  - Unicode and special characters
  - Extremely large inputs
  - Concurrent error conditions
  - Memory corruption scenarios

## Running Tests

### Basic Test Run
```bash
cargo test
```

### Specific Test Categories
```bash
# Integration tests only
cargo test --test emotes_integration

# Edge case tests only
cargo test --test emotes_edge_cases
```

### Benchmark Tests
```bash
# Run criterion benchmarks
cargo bench

# Specific benchmark group
cargo bench -- cache_operations
cargo bench -- parser_operations
cargo bench -- emote_system
```

### Test with Specific Features
```bash
# Run tests with logging
RUST_LOG=debug cargo test

# Run tests with backtrace on failure
RUST_BACKTRACE=1 cargo test

# Run tests in release mode for performance testing
cargo test --release
```

## Test Configuration

### Environment Variables
- `RUST_LOG`: Set logging level (error, warn, info, debug, trace)
- `RUST_BACKTRACE`: Enable backtrace on panic (1 or full)
- `OVERLAY_NATIVE_TEST_TIMEOUT`: Override test timeout (default: 30s)

### Test Data
Tests use mock data and temporary directories to avoid affecting the main system:
- Temporary cache directories via `tempfile`
- Mock providers for simulating API responses
- Generated test data for performance testing

## Performance Benchmarks

### Cache Performance
- **Target**: >1000 insertions/sec, >10000 lookups/sec
- **Memory**: Efficient LRU eviction with <100MB overhead for 10k emotes

### Parser Performance
- **Target**: >1000 messages/sec parsing
- **Accuracy**: >95% emote detection rate
- **Unicode**: Full Unicode support without performance degradation

### System Throughput
- **Target**: >100 messages/sec concurrent processing
- **Latency**: <50ms average processing time
- **Reliability**: >99% success rate under normal load

## Debugging Failed Tests

### Common Issues
1. **Network timeouts**: Check internet connectivity for provider tests
2. **Permission errors**: Ensure write permissions for temp directories
3. **Resource limits**: Increase ulimit for stress tests
4. **Time sensitivity**: Some tests may be flaky on slow systems

### Debug Commands
```bash
# Run single test with output
cargo test test_name -- --nocapture

# Run tests with detailed logging
RUST_LOG=debug cargo test -- --nocapture

# Run tests in single thread (easier debugging)
cargo test -- --test-threads=1

# Ignore specific tests
cargo test --ignore test_that_fails
```

### Test Coverage
```bash
# Install cargo-tarpaulin for coverage
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage
```

## Mock Providers

The test suite includes several mock providers for testing different scenarios:

### FastMockProvider
- High-performance provider for benchmarking
- Zero latency responses
- Configurable emote sets

### FailingProvider
- Simulates various failure conditions
- Configurable failure modes (parse, channel, global)
- Tests error handling and recovery

### ErrorMockProvider
- Simulates network errors, timeouts, invalid data
- Tests resilience and error propagation

## Contributing Tests

When adding new functionality, ensure corresponding tests are added:

1. **Unit Tests**: Test individual functions and methods
2. **Integration Tests**: Test component interactions
3. **Performance Tests**: Verify performance requirements
4. **Edge Case Tests**: Handle unusual inputs and conditions

### Test Naming Convention
- Integration tests: `test_<component>_<functionality>`
- Performance tests: `test_<component>_performance_<scenario>`
- Edge case tests: `test_<component>_edge_case_<scenario>`

### Test Structure
```rust
#[tokio::test]
async fn test_component_functionality() {
    // Arrange
    let config = create_test_config();
    let mut system = Component::new(config);

    // Act
    let result = system.do_something().await;

    // Assert
    assert!(result.is_ok());
    // Additional assertions...
}
```

## Continuous Integration

The test suite is designed to run in CI/CD environments:
- All tests complete within 2 minutes
- No external dependencies required
- Deterministic results across platforms
- Graceful handling of resource constraints

### CI Test Matrix
- **OS**: Windows, Linux, macOS
- **Rust**: Stable, Beta (nightly for advanced features)
- **Features**: Default, minimal, all features enabled

## Troubleshooting

### Performance Test Failures
- Increase timeout: `OVERLAY_NATIVE_TEST_TIMEOUT=60 cargo test`
- Run in release mode: `cargo test --release`
- Check system resources: memory, CPU, disk I/O

### Network-Related Test Failures
- Check internet connectivity
- Verify firewall/proxy settings
- Use mock providers for offline testing

### Memory-Related Test Failures
- Increase available memory
- Check for memory leaks with Valgrind (Linux) or similar tools
- Reduce test data sizes for resource-constrained environments

## Future Improvements

Planned enhancements to the test suite:
- [ ] Property-based testing with proptest
- [ ] Fuzz testing for input validation
- [ ] Load testing with simulated real-world scenarios
- [ ] Cross-platform compatibility testing
- [ ] Automated performance regression detection
