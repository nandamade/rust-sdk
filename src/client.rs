//! Main SDK client

use crate::cache::Cache;
use crate::config::Config;
use crate::error::{Result, SdkError};
use crate::middleware::{AuthMiddleware, LoggingMiddleware, MiddlewareChain, RequestContext};
use crate::models::*;
use crate::rate_limit::SlidingWindowRateLimiter;
use crate::utils::{RetryHelper, RetryPolicy};
use crate::validation::{EmailValidator, Validator};
use chrono::Utc;
use reqwest::Client as ReqwestClient;
use std::sync::Arc;
use std::time::Duration;
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
        let http_client = ReqwestClient::builder()
            .timeout(config.http_timeout())
            .connect_timeout(Duration::from_secs(config.http.connect_timeout_secs))
            .http2_prior_knowledge()
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

    /// Health check
    pub async fn health_check(&self) -> Result<HealthCheckResponse> {
        let url = format!("{}/health", self.config.base_url);

        let mut context = RequestContext {
            request_id: Uuid::new_v4(),
            metadata: RequestMetadata {
                request_id: Uuid::new_v4(),
                correlation_id: None,
                user_id: None,
                timestamp: Utc::now(),
                path: "/health".to_string(),
                method: "GET".to_string(),
                client_ip: "127.0.0.1".to_string(),
                user_agent: "rust-sdk".to_string(),
            },
            attributes: Default::default(),
        };

        self.middleware_chain.process_request(&mut context).await;

        let response = RetryHelper::retry_with_backoff(
            || async {
                self.http_client
                    .get(&url)
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
        let url = format!("{}{}", self.config.base_url, path);

        // Check cache
        if let Ok(Some(cached)) = self.cache.get::<T>(path) {
            tracing::debug!("Cache hit for path: {}", path);
            return Ok(cached);
        }

        // Rate limit
        self.rate_limiter
            .allow_request("default_user")
            .map_err(|e| {
                tracing::warn!("Rate limit exceeded: {}", e);
                e
            })?;

        let response = RetryHelper::retry_with_backoff(
            || async {
                self.http_client
                    .get(&url)
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

        self.rate_limiter
            .allow_request("default_user")
            .map_err(|e| {
                tracing::warn!("Rate limit exceeded: {}", e);
                e
            })?;

        let response = RetryHelper::retry_with_backoff(
            || async {
                self.http_client
                    .post(&url)
                    .json(body)
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

        self.rate_limiter
            .allow_request("default_user")
            .map_err(|e| {
                tracing::warn!("Rate limit exceeded: {}", e);
                e
            })?;

        let response = RetryHelper::retry_with_backoff(
            || async {
                self.http_client
                    .put(&url)
                    .json(body)
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
        .await?;

        response
            .json::<R>()
            .await
            .map_err(|e| SdkError::http(format!("Failed to parse response: {e}")))
    }

    /// DELETE request
    pub async fn delete<R: serde::de::DeserializeOwned>(&self, path: &str) -> Result<R> {
        let url = format!("{}{}", self.config.base_url, path);

        self.rate_limiter
            .allow_request("default_user")
            .map_err(|e| {
                tracing::warn!("Rate limit exceeded: {}", e);
                e
            })?;

        let response = RetryHelper::retry_with_backoff(
            || async {
                self.http_client
                    .delete(&url)
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
}
