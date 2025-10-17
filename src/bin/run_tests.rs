//! Test runner binary for Overlay Native
//!
//! This binary runs the comprehensive test suite and provides
//! detailed reporting with timeout protection.
//!
//! Run with: cargo run --bin run_tests

use overlay_native::tests;
use std::process;
use std::time::Duration;
use tokio::time;

#[tokio::main]
async fn main() {
    println!("🧪 Overlay Native - Comprehensive Test Runner");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Running test suite with timeout protection...\n");

    // Run tests with global timeout
    let global_timeout = time::timeout(Duration::from_secs(120), async {
        match tests::run_tests().await {
            Ok(_) => {
                println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("🎉 ALL TESTS PASSED!");
                println!("✅ The application handles timeouts correctly");
                println!("✅ WebSocket connections are properly managed");
                println!("✅ Message flow works as expected");
                println!("✅ Shutdown procedures complete successfully");
                process::exit(0);
            }
            Err(e) => {
                println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("❌ TEST SUITE FAILED: {}", e);
                println!("💡 Check the logs above for specific test failures");
                process::exit(1);
            }
        }
    });

    match global_timeout.await {
        Ok(_) => {
            // Already handled in the timeout future
        }
        Err(_) => {
            println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("⏰ GLOBAL TIMEOUT REACHED!");
            println!("❌ Test suite took longer than 2 minutes");
            println!("💡 This indicates:");
            println!("   - Possible deadlocks in the application");
            println!("   - WebSocket connections not timing out properly");
            println!("   - Infinite loops in message processing");
            println!("   - Network connectivity issues");
            process::exit(1);
        }
    }
}
