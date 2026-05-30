# Rust SDK - Production-Grade Comprehensive SDK

A feature-rich, production-grade Rust SDK with async operations, advanced error handling, middleware system, caching, rate limiting, and comprehensive testing.

## Features

### 🚀 Core Features
- **Async HTTP Client** - Built on `tokio` and `reqwest` with full async support
- **Advanced Error Handling** - Comprehensive error types with context information
- **Middleware System** - Pluggable middleware for request/response processing
- **Request Validation** - Email, URL, UUID, length, and custom validators
- **Rate Limiting** - Token bucket and sliding window rate limiters
- **Caching Layer** - In-memory cache with TTL and LRU eviction
- **Database Integration** - Query builder, repository pattern, soft deletes
- **Configuration Management** - Environment variables and file-based configuration
- **Authentication** - API key, HMAC signatures, OAuth support
- **Retry Logic** - Exponential backoff with jitter
- **Logging** - Structured logging with tracing

### 🔒 Security Features
- HMAC-SHA256 signature generation and verification
- Input sanitization (HTML, SQL, path traversal)
- Request validation
- Rate limiting per user/endpoint
- Error context tracking with request IDs

### 📊 Advanced Features
- **Metrics** - Rate limiter metrics and acceptance rates
- **Query Builder** - Fluent SQL query construction
- **Soft Deletes** - Entity soft delete support
- **Pagination** - Pagination support for list responses
- **Request Context** - Request metadata and correlation IDs
- **Middleware Chain** - Sequential middleware processing

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
rust-sdk = { path = "./rust-sdk" }
tokio = { version = "1.35", features = ["full"] }
```

## Quick Start

### Basic Usage

```rust
use rust_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = Client::new("https://api.example.com")?;
    
    // Make requests
    let data: serde_json::Value = client.get("/endpoint").await?;
    
    Ok(())
}
```

### Configuration

```rust
use rust_sdk::config::Config;

let config = Config::new("https://api.example.com")
    .with_api_key("your-api-key")
    .with_header("X-Custom-Header", "value")
    .with_debug(true);

let client = Client::with_config(config)?;
```

### From Environment

```rust
// Load from .env file
let client = Client::from_env()?;
```

Requires environment variables:
- `SDK_BASE_URL` - API base URL (required)
- `SDK_API_KEY` - API key (optional)
- `SDK_DEBUG` - Debug mode (optional)

### Caching

```rust
let cache = Cache::new(1000, 3600); // 1000 entries, 1 hour TTL

// Store
cache.set("key", &value)?;

// Retrieve
let result: Option<T> = cache.get("key")?;

// Clear
cache.clear();
```

### Validation

```rust
use rust_sdk::validation::*;

// Built-in validators
let email_validator = EmailValidator;
email_validator.validate("test@example.com")?;

let url_validator = UrlValidator;
url_validator.validate("https://example.com")?;

// Length validator
let len_validator = LengthValidator::new(3, 20);
len_validator.validate("username")?;

// Sanitization
let sanitized = Sanitizer::sanitize_html("<script>alert(1)</script>");
```

### Rate Limiting

```rust
use rust_sdk::rate_limit::*;
use std::time::Duration;

// Token bucket
let bucket = TokenBucket::new(100, 10); // 100 capacity, 10 refill/sec
bucket.try_acquire(5)?;

// Per-user rate limiter
let limiter = PerUserRateLimiter::new(1000, 100);
limiter.try_acquire("user123", 5)?;

// Sliding window
let limiter = SlidingWindowRateLimiter::new(100, Duration::from_secs(60));
limiter.allow_request("client1")?;
```

### Database

```rust
use rust_sdk::database::*;

// Query builder
let query = QueryBuilder::new()
    .select(vec!["id", "name", "email"])
    .from("users")
    .where_clause("is_active = true")
    .order_by("created_at", "DESC")
    .limit(50)
    .build()?;

// Repository pattern
let db = InMemoryDatabase::new();
let user_repo = UserRepository::new(Arc::new(db));

// Soft deletes
let mut entity = BaseEntity::new();
entity.soft_delete();
```

### Error Handling

```rust
use rust_sdk::error::*;

match some_operation() {
    Ok(result) => println!("Success: {:?}", result),
    Err(SdkError::ValidationError(msg)) => {
        eprintln!("Validation failed: {}", msg);
    }
    Err(SdkError::RateLimitExceeded(msg)) => {
        eprintln!("Rate limit: {}", msg);
    }
    Err(e) => eprintln!("Error: {}", e),
}

// Check if error is retryable
if error.is_retryable() {
    // Retry operation
}
```

## Architecture

```
rust-sdk/
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── client.rs           # Main SDK client
│   ├── config.rs           # Configuration management
│   ├── error.rs            # Error types and handling
│   ├── models.rs           # Data models
│   ├── middleware.rs       # Middleware system
│   ├── cache.rs            # Caching layer
│   ├── validation.rs       # Input validation
│   ├── rate_limit.rs       # Rate limiting
│   ├── database.rs         # Database integration
│   └── utils.rs            # Utility functions
├── tests/
│   └── integration_tests.rs # Comprehensive tests
├── examples/
│   └── main.rs            # Usage examples
└── Cargo.toml             # Dependencies
```

## Project Structure

### Modules

#### `client.rs` - Main SDK Client
- `Client` - Main entry point
- HTTP methods: GET, POST, PUT, DELETE
- Health checks
- Cache integration
- Rate limiting

#### `config.rs` - Configuration
- `Config` - Main configuration struct
- `HttpConfig` - HTTP client settings
- `RetryConfig` - Retry policy
- `CacheConfig` - Cache settings
- `RateLimitConfig` - Rate limit settings

#### `error.rs` - Error Handling
- `SdkError` - Comprehensive error types
- `ErrorContext` - Error context information
- Error categorization (retryable, auth, etc.)

#### `middleware.rs` - Middleware System
- `Middleware` trait - Async middleware interface
- `MiddlewareChain` - Sequential middleware processing
- Built-in middlewares: Logging, Auth, RateLimit, Validation, Caching

#### `cache.rs` - Caching
- `Cache` - In-memory cache with TTL
- `LruCache` - LRU cache implementation
- Automatic expiration
- Thread-safe operations

#### `validation.rs` - Input Validation
- `Validator` trait
- `EmailValidator`, `UrlValidator`, `UuidValidator`
- `LengthValidator`, `AlphanumericValidator`
- `RegexValidator` - Custom regex patterns
- `Sanitizer` - XSS, SQL injection, path traversal prevention

#### `rate_limit.rs` - Rate Limiting
- `TokenBucket` - Token bucket algorithm
- `SlidingWindowRateLimiter` - Sliding window rate limiting
- `PerUserRateLimiter` - Per-user rate limiting

#### `database.rs` - Database Integration
- `DatabaseConnection` trait
- `InMemoryDatabase` - Test database
- `UserRepository` - Example repository
- `QueryBuilder` - Fluent query construction
- `BaseEntity` - Base entity with soft deletes

#### `utils.rs` - Utilities
- `exponential_backoff()` - Backoff calculation
- `RetryHelper` - Retry logic
- `SignatureGenerator` - HMAC-SHA256
- `TimeUtils` - Time utilities
- `RateLimiterMetrics` - Metrics tracking

## Testing

### Run All Tests

```bash
cargo test
```

### Run Specific Test

```bash
cargo test test_cache_set_get
```

### Run Integration Tests

```bash
cargo test --test integration_tests
```

### Run with Output

```bash
cargo test -- --nocapture
```

### Run Examples

```bash
cargo run --example main
```

## Test Coverage

The SDK includes comprehensive tests covering:

- ✅ Configuration management
- ✅ Client creation and configuration
- ✅ Email validation
- ✅ Caching (set, get, TTL, eviction)
- ✅ Rate limiting (token bucket, sliding window)
- ✅ Database operations (query builder, soft deletes)
- ✅ Error handling and retry logic
- ✅ Signature generation and verification
- ✅ Input sanitization
- ✅ Models serialization/deserialization

## Performance Considerations

### Caching
- Default TTL: 1 hour
- Maximum entries: 1000
- LRU eviction policy

### Rate Limiting
- Token bucket: 100 requests/sec default
- Sliding window: 60-second window default
- Per-user limiting supported

### Retry Policy
- Initial backoff: 100ms
- Maximum backoff: 30 seconds
- Multiplier: 2.0
- Jitter: Enabled

## Security Considerations

1. **Input Validation** - Always validate user input
2. **Sanitization** - Sanitize HTML, SQL, and paths
3. **HTTPS** - Always use HTTPS in production
4. **API Keys** - Store securely in environment variables
5. **Rate Limiting** - Implement per-user and per-endpoint
6. **Error Messages** - Don't expose sensitive data in errors
7. **Logging** - Don't log sensitive information

## Contributing

Contributions are welcome! Please:
1. Add tests for new features
2. Update documentation
3. Follow Rust conventions
4. Run `cargo fmt` and `cargo clippy`

## License

MIT License - See LICENSE file for details

## Examples

See `examples/main.rs` for comprehensive usage examples.

## API Reference

### Client Methods

- `new(base_url)` - Create client
- `with_config(config)` - Create from config
- `from_env()` - Create from environment
- `get::<T>(path)` - GET request
- `post::<T, R>(path, body)` - POST request
- `put::<T, R>(path, body)` - PUT request
- `delete::<R>(path)` - DELETE request
- `health_check()` - Health check
- `validate_email(email)` - Email validation
- `clear_cache()` - Clear cache

### Configuration

- `Config::new(url)` - Create default config
- `Config::from_env()` - Load from environment
- `Config::from_file(path)` - Load from file
- `.with_api_key(key)` - Set API key
- `.with_header(key, value)` - Add custom header
- `.with_debug(bool)` - Enable debug mode

## Troubleshooting

### Connection Timeout
- Increase `http.timeout_secs` in config
- Check network connectivity

### Rate Limit Exceeded
- Implement backoff logic
- Use `RetryHelper::retry_with_backoff()`

### Cache Not Working
- Ensure `cache.enabled` is true in config
- Check cache TTL settings

### Validation Failures
- Use appropriate validators
- Check input format

## Support

For issues and questions, please open an issue in the repository.
