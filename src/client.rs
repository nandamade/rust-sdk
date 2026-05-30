//! Main SDK client

use crate::cache::Cache;
use crate::config::Config;
use crate::error::{Result, SdkError};
use crate::middleware::{
    AuthMiddleware, LoggingMiddleware, MiddlewareChain, RequestContext, ResponseContext,
};
use crate::models::*;
use crate::rate_limit::SlidingWindowRateLimiter;
use crate::utils::{RetryHelper, RetryPolicy};
use crate::validation::{EmailValidator, Validator};
use chrono::Utc;
use reqwest::Client as ReqwestClient;
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Main SDK Client
pub struct Client {
    config: Config,
    http_client: ReqwestClient,
    cache: Cache,
    rate_limiter: SlidingWindowRateLimiter,
    middleware_chain: MiddlewareChain,
    retry_policy: RetryPolicy,
}

impl Client {
    /// Create a new client with base URL
    pub fn new(base_url: impl Into<String>) -> Result<Self> {
        let config = Config::new(base_url);
        Self::with_config(config)
    }

    /// Create a new client with configuration
    pub fn with_config(config: Config) -> Result<Self> {
        let mut builder = ReqwestClient::builder()
            .timeout(config.http_timeout())
            .connect_timeout(Duration::from_secs(config.http.connect_timeout_secs));

        if config.http.force_http2 {
            builder = builder.http2_prior_knowledge();
        }

        let http_client = builder
            .build()
            .map_err(|e| SdkError::http(format!("Failed to create HTTP client: {e}")))?;

        let mut middleware_chain = MiddlewareChain::new().add(Arc::new(LoggingMiddleware));

        if let Some(ref api_key) = config.api_key {
            middleware_chain = middleware_chain.add(Arc::new(AuthMiddleware::new(api_key.clone())));
        }

        let cache = Cache::new(config.cache.max_entries, config.cache.ttl_secs);

        let rate_limiter = SlidingWindowRateLimiter::new(
            config.rate_limit.requests_per_second,
            Duration::from_secs(1),
        );

        let retry_policy = RetryPolicy {
            max_attempts: config.retry.max_attempts,
            initial_backoff_ms: config.retry.initial_backoff_ms,
            max_backoff_ms: config.retry.max_backoff_ms,
            multiplier: config.retry.multiplier,
            jitter: config.retry.jitter,
        };

        Ok(Self {
            config,
            http_client,
            cache,
            rate_limiter,
            middleware_chain,
            retry_policy,
        })
    }

    /// Create client from environment
    pub fn from_env() -> Result<Self> {
        let config = Config::from_env()?;
        Self::with_config(config)
    }

    /// Get configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get HTTP client
    pub fn http_client(&self) -> &ReqwestClient {
        &self.http_client
    }

    /// Get cache
    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    /// Get rate limiter
    pub fn rate_limiter(&self) -> &SlidingWindowRateLimiter {
        &self.rate_limiter
    }

    /// Build a request context for the given path and method
    fn build_context(&self, path: &str, method: &str) -> RequestContext {
        let request_id = Uuid::new_v4();
        RequestContext {
            request_id,
            metadata: RequestMetadata {
                request_id,
                correlation_id: None,
                user_id: None,
                timestamp: Utc::now(),
                path: path.to_string(),
                method: method.to_string(),
                client_ip: "127.0.0.1".to_string(),
                user_agent: "rust-sdk".to_string(),
            },
            attributes: Default::default(),
        }
    }

    /// Apply middleware attributes as headers on a request builder
    fn apply_attributes(
        &self,
        mut builder: reqwest::RequestBuilder,
        context: &RequestContext,
    ) -> reqwest::RequestBuilder {
        for (key, value) in &context.attributes {
            builder = builder.header(key.as_str(), value.as_str());
        }
        // Also apply custom headers from config
        for (key, value) in &self.config.custom_headers {
            builder = builder.header(key.as_str(), value.as_str());
        }
        builder
    }

    /// Execute request with full middleware pipeline
    async fn execute_request(
        &self,
        method: &str,
        path: &str,
        request_builder_fn: impl Fn() -> reqwest::RequestBuilder,
    ) -> Result<reqwest::Response> {
        // Rate limit
        self.rate_limiter
            .allow_request("default_user")
            .map_err(|e| {
                tracing::warn!("Rate limit exceeded: {e}");
                e
            })?;

        // Build context and run request middleware
        let mut context = self.build_context(path, method);
        self.middleware_chain.process_request(&mut context).await;

        let start = Instant::now();

        // Execute with retry
        let result = RetryHelper::retry_with_backoff(
            || async {
                let builder = request_builder_fn();
                let builder = self.apply_attributes(builder, &context);

                builder
                    .send()
                    .await
                    .map_err(|e| SdkError::http(e.to_string()))
                    .and_then(|resp| {
                        if resp.status().is_success() {
                            Ok(resp)
                        } else {
                            Err(SdkError::http(format!("HTTP {}", resp.status())))
                        }
                    })
            },
            &self.retry_policy,
        )
        .await;

        let elapsed = start.elapsed();

        // Run response/error middleware
        match &result {
            Ok(resp) => {
                let mut response_context = ResponseContext {
                    status_code: resp.status().as_u16(),
                    response_time_ms: elapsed.as_millis() as u64,
                    attributes: Default::default(),
                };
                self.middleware_chain
                    .process_response(&context, &mut response_context)
                    .await;
            }
            Err(e) => {
                self.middleware_chain
                    .process_error(&context, &e.to_string())
                    .await;
            }
        }

        result
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthCheckResponse> {
        let url = format!("{}/health", self.config.base_url);

        let http_client = self.http_client.clone();
        let response = self
            .execute_request("GET", "/health", || http_client.get(&url))
            .await?;

        response
            .json::<HealthCheckResponse>()
            .await
            .map_err(|e| SdkError::http(format!("Failed to parse response: {e}")))
    }

    /// GET request
    pub async fn get<T: serde::de::DeserializeOwned + serde::Serialize>(
        &self,
        path: &str,
    ) -> Result<T> {
        // Check cache first
        if let Ok(Some(cached)) = self.cache.get::<T>(path) {
            tracing::debug!("Cache hit for path: {path}");
            return Ok(cached);
        }

        let url = format!("{}{}", self.config.base_url, path);
        let http_client = self.http_client.clone();
        let response = self
            .execute_request("GET", path, || http_client.get(&url))
            .await?;

        let data = response
            .json::<T>()
            .await
            .map_err(|e| SdkError::http(format!("Failed to parse response: {e}")))?;

        // Cache result
        let _ = self.cache.set(path, &data);

        Ok(data)
    }

    /// POST request
    pub async fn post<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let url = format!("{}{}", self.config.base_url, path);
        let http_client = self.http_client.clone();
        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| SdkError::http(format!("Failed to serialize body: {e}")))?;

        let response = self
            .execute_request("POST", path, || {
                http_client
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .body(body_bytes.clone())
            })
            .await?;

        response
            .json::<R>()
            .await
            .map_err(|e| SdkError::http(format!("Failed to parse response: {e}")))
    }

    /// PUT request
    pub async fn put<T: serde::Serialize, R: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R> {
        let url = format!("{}{}", self.config.base_url, path);
        let http_client = self.http_client.clone();
        let body_bytes = serde_json::to_vec(body)
            .map_err(|e| SdkError::http(format!("Failed to serialize body: {e}")))?;

        let response = self
            .execute_request("PUT", path, || {
                http_client
                    .put(&url)
                    .header("Content-Type", "application/json")
                    .body(body_bytes.clone())
            })
            .await?;

        response
            .json::<R>()
            .await
            .map_err(|e| SdkError::http(format!("Failed to parse response: {e}")))
    }

    /// DELETE request
    pub async fn delete<R: serde::de::DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = format!("{}{}", self.config.base_url, path);
        let http_client = self.http_client.clone();

        let response = self
            .execute_request("DELETE", path, || http_client.delete(&url))
            .await?;

        response
            .json::<R>()
            .await
            .map_err(|e| SdkError::http(format!("Failed to parse response: {e}")))
    }

    /// Validate email
    pub fn validate_email(&self, email: &str) -> Result<()> {
        EmailValidator.validate(email)
    }

    /// Clear cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = Client::new("https://api.example.com");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_with_config() {
        let config = Config::new("https://api.example.com");
        let client = Client::with_config(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_validate_email() {
        let client = Client::new("https://api.example.com").unwrap();

        assert!(client.validate_email("test@example.com").is_ok());
        assert!(client.validate_email("invalid-email").is_err());
    }

    #[test]
    fn test_middleware_attributes_applied() {
        // Client with API key should have auth middleware
        let config = Config::new("https://api.example.com").with_api_key("test-key");
        let client = Client::with_config(config).unwrap();

        // Build context and process middleware
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut context = client.build_context("/test", "GET");
            client.middleware_chain.process_request(&mut context).await;

            // AuthMiddleware should have added Authorization attribute
            assert!(context.attributes.contains_key("Authorization"));
            assert!(context
                .attributes
                .get("Authorization")
                .unwrap()
                .contains("Bearer test-key"));
        });
    }

    #[test]
    fn test_apply_attributes_adds_headers() {
        let config = Config::new("https://api.example.com").with_header("X-Custom", "custom-value");
        let client = Client::with_config(config).unwrap();

        let mut context = client.build_context("/test", "GET");
        context
            .attributes
            .insert("Authorization".to_string(), "Bearer token".to_string());

        // Verify apply_attributes works by building a request
        let builder = client.http_client.get("https://api.example.com/test");
        let builder = client.apply_attributes(builder, &context);
        let request = builder.build().unwrap();

        assert_eq!(
            request.headers().get("Authorization").unwrap(),
            "Bearer token"
        );
        assert_eq!(request.headers().get("X-Custom").unwrap(), "custom-value");
    }
}
