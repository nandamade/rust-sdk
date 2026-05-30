//! Examples for using the Rust SDK

use rust_sdk::cache::Cache;
use rust_sdk::database::*;
use rust_sdk::prelude::*;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("=== Rust SDK Examples ===\n");

    // Example 1: Basic Client Creation
    example_basic_client().await?;

    // Example 2: Configuration
    example_config().await?;

    // Example 3: Caching
    example_caching().await?;

    // Example 4: Validation
    example_validation()?;

    // Example 5: Database Operations
    example_database().await?;

    // Example 6: Error Handling
    example_error_handling().await?;

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}

async fn example_basic_client() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("1. Basic Client Creation");
    println!("--------------------------");

    let client = Client::new("https://api.example.com")?;
    println!("✓ Client created successfully");
    println!("  Base URL: {}\n", client.config().base_url);

    Ok(())
}

async fn example_config() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("2. Configuration Examples");
    println!("--------------------------");

    // Create config with fluent builder
    let config = Config::new("https://api.example.com")
        .with_api_key("secret-api-key")
        .with_header("X-Custom-Header", "custom-value")
        .with_debug(true);

    println!("✓ Configuration created");
    println!("  Base URL: {}", config.base_url);
    println!("  Debug: {}", config.debug);
    println!("  Custom Headers: {}\n", config.custom_headers.len());

    Ok(())
}

async fn example_caching() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("3. Caching Examples");
    println!("--------------------------");

    let cache = Cache::new(100, 3600);

    // Store data in cache
    cache.set("user:123", &"John Doe")?;
    cache.set("user:456", &"Jane Smith")?;

    println!("✓ Stored 2 items in cache");
    println!("  Cache size: {}", cache.size());

    // Retrieve from cache
    let user: String = cache.get("user:123")?.unwrap();
    println!("  Retrieved: {user} for user:123");

    // Check if key exists
    if cache.contains_key("user:456") {
        println!("  user:456 exists in cache");
    }

    // Clear cache
    cache.clear();
    println!("  Cache cleared\n");

    Ok(())
}

fn example_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("4. Validation Examples");
    println!("--------------------------");

    let client = Client::new("https://api.example.com")?;

    // Email validation
    match client.validate_email("valid@example.com") {
        Ok(()) => println!("✓ valid@example.com is valid"),
        Err(e) => println!("✗ Email validation failed: {e}"),
    }

    match client.validate_email("invalid-email") {
        Ok(()) => println!("✓ invalid-email is valid"),
        Err(_e) => println!("✓ Correctly rejected invalid-email"),
    }

    println!();
    Ok(())
}

async fn example_database() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("5. Database Examples");
    println!("--------------------------");

    // Create in-memory database
    let db = InMemoryDatabase::new();

    // Health check
    db.health_check().await?;
    println!("✓ Database health check passed");

    // Create query
    let query = QueryBuilder::new()
        .select(vec!["id", "email", "username"])
        .from("users")
        .where_clause("is_active = true")
        .order_by("created_at", "DESC")
        .limit(10)
        .build()?;

    println!("✓ Query built successfully");
    println!("  Query: {query}\n");

    Ok(())
}

async fn example_error_handling() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("6. Error Handling Examples");
    println!("--------------------------");

    // Example of handling different error types
    let result: Result<String> = Err(SdkError::validation("Invalid input format"));

    match result {
        Ok(_) => println!("✓ Operation succeeded"),
        Err(SdkError::ValidationError(msg)) => {
            println!("✓ Caught validation error: {msg}");
        }
        Err(e) => println!("✗ Other error: {e}"),
    }

    // Check if error is retryable
    let error = SdkError::http("Connection timeout");
    if error.is_retryable() {
        println!("✓ HTTP error is retryable\n");
    }

    Ok(())
}
