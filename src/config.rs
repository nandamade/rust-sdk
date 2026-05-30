//! Configuration management for the SDK

use crate::error::{Result, SdkError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

/// Main SDK configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Base URL for API endpoints
    pub base_url: String,

    /// API key for authentication
    pub api_key: Option<String>,

    /// HTTP client configuration
    pub http: HttpConfig,

    /// Retry configuration
    pub retry: RetryConfig,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Rate limiting configuration
    pub rate_limit: RateLimitConfig,

    /// Database configuration
    pub database: Option<DatabaseConfig>,

    /// Custom headers
    #[serde(default)]
    pub custom_headers: HashMap<String, String>,

    /// Environment
    #[serde(default)]
    pub environment: String,

    /// Debug mode
    #[serde(default)]
    pub debug: bool,
}

/// HTTP client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connect_timeout_secs: u64,

    /// Maximum redirects
    #[serde(default = "default_max_redirects")]
    pub max_redirects: usize,

    /// Keep-alive timeout in seconds
    #[serde(default = "default_keepalive")]
    pub keepalive_secs: u64,

    /// TCP nodelay
    #[serde(default = "default_true")]
    pub tcp_nodelay: bool,

    /// Use HTTP/2
    #[serde(default = "default_true")]
    pub http2: bool,
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_attempts: u32,

    /// Initial backoff in milliseconds
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,

    /// Maximum backoff in milliseconds
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,

    /// Backoff multiplier
    #[serde(default = "default_multiplier")]
    pub multiplier: f64,

    /// Add jitter to backoff
    #[serde(default = "default_true")]
    pub jitter: bool,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Maximum cache entries
    #[serde(default = "default_cache_size")]
    pub max_entries: usize,

    /// TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub ttl_secs: u64,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Requests per second
    #[serde(default = "default_rps")]
    pub requests_per_second: u32,

    /// Burst size
    #[serde(default = "default_burst")]
    pub burst_size: u32,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,

    /// Maximum connections
    #[serde(default = "default_pool_size")]
    pub max_connections: u32,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub connection_timeout_secs: u64,

    /// Idle timeout in seconds
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_secs: u64,
}

// Default functions for serde
fn default_timeout() -> u64 { 30 }
fn default_connect_timeout() -> u64 { 10 }
fn default_max_redirects() -> usize { 5 }
fn default_keepalive() -> u64 { 60 }
fn default_true() -> bool { true }
fn default_max_retries() -> u32 { 3 }
fn default_initial_backoff() -> u64 { 100 }
fn default_max_backoff() -> u64 { 30000 }
fn default_multiplier() -> f64 { 2.0 }
fn default_cache_size() -> usize { 1000 }
fn default_cache_ttl() -> u64 { 3600 }
fn default_rps() -> u32 { 100 }
fn default_burst() -> u32 { 200 }
fn default_pool_size() -> u32 { 10 }
fn default_idle_timeout() -> u64 { 300 }

impl Config {
    /// Create a new configuration from base URL
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
            http: HttpConfig::default(),
            retry: RetryConfig::default(),
            cache: CacheConfig::default(),
            rate_limit: RateLimitConfig::default(),
            database: None,
            custom_headers: HashMap::new(),
            environment: "production".to_string(),
            debug: false,
        }
    }

    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let base_url = std::env::var("SDK_BASE_URL")
            .map_err(|_| SdkError::config("SDK_BASE_URL environment variable not set"))?;

        let api_key = std::env::var("SDK_API_KEY").ok();
        let debug = std::env::var("SDK_DEBUG").ok().map(|v| v == "true").unwrap_or(false);

        Ok(Self {
            base_url,
            api_key,
            debug,
            environment: std::env::var("SDK_ENV").unwrap_or_else(|_| "production".to_string()),
            ..Default::default()
        })
    }

    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| SdkError::config(format!("Failed to read config file: {}", e)))?;

        serde_json::from_str(&content)
            .map_err(|e| SdkError::config(format!("Invalid config file: {}", e)))
    }

    /// Set API key
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    /// Set custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_headers.insert(key.into(), value.into());
        self
    }

    /// Enable debug mode
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Get HTTP timeout as Duration
    pub fn http_timeout(&self) -> Duration {
        Duration::from_secs(self.http.timeout_secs)
    }

    /// Get retry initial backoff as Duration
    pub fn retry_initial_backoff(&self) -> Duration {
        Duration::from_millis(self.retry.initial_backoff_ms)
    }

    /// Get cache TTL as Duration
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache.ttl_secs)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            base_url: "https://api.example.com".to_string(),
            api_key: None,
            http: HttpConfig::default(),
            retry: RetryConfig::default(),
            cache: CacheConfig::default(),
            rate_limit: RateLimitConfig::default(),
            database: None,
            custom_headers: HashMap::new(),
            environment: "production".to_string(),
            debug: false,
        }
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            connect_timeout_secs: 10,
            max_redirects: 5,
            keepalive_secs: 60,
            tcp_nodelay: true,
            http2: true,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 30000,
            multiplier: 2.0,
            jitter: true,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: 1000,
            ttl_secs: 3600,
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second: 100,
            burst_size: 200,
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite://database.db".to_string(),
            max_connections: 10,
            connection_timeout_secs: 30,
            idle_timeout_secs: 300,
        }
    }
}
