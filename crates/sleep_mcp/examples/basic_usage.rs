//! Basic usage examples for the Sleep MCP Server
//!
//! This example demonstrates how to use the sleep_mcp server programmatically
//! for testing, automation, and workflow control scenarios.

use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Sleep MCP Server - Basic Usage Examples");
    println!("=====================================\n");

    // Example 1: Simple delay in test automation
    println!("1. Test Automation Delay");
    println!("   Simulating a 2-second delay between test steps...");
    let start = std::time::Instant::now();
    sleep(Duration::from_secs(2)).await;
    println!("   ✓ Completed in {:.2}s\n", start.elapsed().as_secs_f64());

    // Example 2: Rate limiting
    println!("2. Rate Limiting Example");
    println!("   Making 3 API calls with 1-second intervals...");
    for i in 1..=3 {
        println!("   Making API call #{}", i);
        // Simulate API call
        sleep(Duration::from_millis(100)).await;
        println!("   ✓ API call #{} completed", i);

        if i < 3 {
            println!("   Waiting 1 second before next call...");
            sleep(Duration::from_secs(1)).await;
        }
    }
    println!("   ✓ All API calls completed\n");

    // Example 3: Exponential backoff retry pattern
    println!("3. Exponential Backoff Retry");
    println!("   Simulating retry with exponential backoff...");

    let mut retry_count = 0;
    let max_retries = 3;

    loop {
        retry_count += 1;
        println!("   Attempt #{}", retry_count);

        // Simulate operation that might fail
        let success = retry_count >= 3; // Succeed on 3rd attempt

        if success {
            println!("   ✓ Operation succeeded!");
            break;
        }

        if retry_count >= max_retries {
            println!("   ✗ Max retries exceeded");
            break;
        }

        let delay = Duration::from_millis(100 * 2_u64.pow(retry_count - 1));
        println!("   Retrying in {}ms...", delay.as_millis());
        sleep(delay).await;
    }
    println!();

    // Example 4: Coordinated timing
    println!("4. Coordinated Timing");
    println!("   Coordinating multiple operations...");

    let tasks = vec![
        ("Task A", Duration::from_millis(500)),
        ("Task B", Duration::from_millis(300)),
        ("Task C", Duration::from_millis(700)),
    ];

    for (name, duration) in tasks {
        println!("   Starting {}", name);
        let start = std::time::Instant::now();
        sleep(duration).await;
        println!(
            "   ✓ {} completed in {:.2}s",
            name,
            start.elapsed().as_secs_f64()
        );
    }

    println!("\n✓ All examples completed successfully!");

    Ok(())
}
