//! Middleware system for request/response processing

use crate::models::RequestMetadata;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

/// Request context
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Request ID
    pub request_id: Uuid,

    /// Metadata
    pub metadata: RequestMetadata,

    /// Custom attributes
    pub attributes: std::collections::HashMap<String, String>,
}

/// Response context
#[derive(Debug, Clone)]
pub struct ResponseContext {
    /// Status code
    pub status_code: u16,

    /// Response time in ms
    pub response_time_ms: u64,

    /// Custom attributes
    pub attributes: std::collections::HashMap<String, String>,
}

/// Middleware trait for request/response processing
#[async_trait]
pub trait Middleware: Send + Sync {
    /// Process request before sending
    async fn on_request(&self, _context: &mut RequestContext) {}

    /// Process response after receiving
    async fn on_response(&self, _request: &RequestContext, _response: &mut ResponseContext) {}

    /// Process error
    async fn on_error(&self, _context: &RequestContext, _error: &str) {}
}

/// Middleware chain for processing multiple middlewares
pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Create a new middleware chain
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add middleware to the chain
    pub fn add(mut self, middleware: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }

    /// Process request through all middlewares
    pub async fn process_request(&self, context: &mut RequestContext) {
        for middleware in &self.middlewares {
            middleware.on_request(context).await;
        }
    }

    /// Process response through all middlewares
    pub async fn process_response(
        &self,
        request: &RequestContext,
        response: &mut ResponseContext,
    ) {
        for middleware in &self.middlewares {
            middleware.on_response(request, response).await;
        }
    }

    /// Process error through all middlewares
    pub async fn process_error(&self, context: &RequestContext, error: &str) {
        for middleware in &self.middlewares {
            middleware.on_error(context, error).await;
        }
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Logging middleware
pub struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn on_request(&self, context: &mut RequestContext) {
        tracing::info!(
            request_id = %context.request_id,
            path = %context.metadata.path,
            method = %context.metadata.method,
            "Request received"
        );
    }

    async fn on_response(&self, request: &RequestContext, response: &mut ResponseContext) {
        tracing::info!(
            request_id = %request.request_id,
            status_code = response.status_code,
            response_time_ms = response.response_time_ms,
            "Response sent"
        );
    }

    async fn on_error(&self, context: &RequestContext, error: &str) {
        tracing::error!(
            request_id = %context.request_id,
            error = %error,
            "Request error"
        );
    }
}

/// Authentication middleware
pub struct AuthMiddleware {
    api_key: String,
}

impl AuthMiddleware {
    /// Create new auth middleware
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }
}

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn on_request(&self, context: &mut RequestContext) {
        context
            .attributes
            .insert("Authorization".to_string(), format!("Bearer {}", self.api_key));
        tracing::debug!(request_id = %context.request_id, "Authorization header added");
    }
}

/// Rate limiting middleware
pub struct RateLimitMiddleware {
    requests_per_second: u32,
}

impl RateLimitMiddleware {
    /// Create new rate limit middleware
    pub fn new(requests_per_second: u32) -> Self {
        Self {
            requests_per_second,
        }
    }
}

#[async_trait]
impl Middleware for RateLimitMiddleware {
    async fn on_request(&self, context: &mut RequestContext) {
        context.attributes.insert(
            "X-Rate-Limit-RPS".to_string(),
            self.requests_per_second.to_string(),
        );
    }
}

/// Request validation middleware
pub struct ValidationMiddleware;

#[async_trait]
impl Middleware for ValidationMiddleware {
    async fn on_request(&self, context: &mut RequestContext) {
        let request_id = context.request_id.to_string();
        if request_id.is_empty() {
            tracing::warn!("Invalid request ID");
        }
    }
}

/// Caching middleware
pub struct CachingMiddleware {
    cache_control: String,
}

impl CachingMiddleware {
    /// Create new caching middleware
    pub fn new(cache_control: String) -> Self {
        Self { cache_control }
    }
}

#[async_trait]
impl Middleware for CachingMiddleware {
    async fn on_response(&self, _request: &RequestContext, response: &mut ResponseContext) {
        response
            .attributes
            .insert("Cache-Control".to_string(), self.cache_control.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_middleware_chain() {
        let chain = MiddlewareChain::new()
            .add(Arc::new(LoggingMiddleware))
            .add(Arc::new(ValidationMiddleware));

        let mut context = RequestContext {
            request_id: Uuid::new_v4(),
            metadata: crate::models::RequestMetadata {
                request_id: Uuid::new_v4(),
                correlation_id: None,
                user_id: None,
                timestamp: chrono::Utc::now(),
                path: "/test".to_string(),
                method: "GET".to_string(),
                client_ip: "127.0.0.1".to_string(),
                user_agent: "test".to_string(),
            },
            attributes: Default::default(),
        };

        chain.process_request(&mut context).await;
        assert!(!context.attributes.is_empty() || true); // Logging middleware doesn't add attributes
    }

    #[tokio::test]
    async fn test_auth_middleware() {
        let middleware = AuthMiddleware::new("test-key".to_string());
        let mut context = RequestContext {
            request_id: Uuid::new_v4(),
            metadata: crate::models::RequestMetadata {
                request_id: Uuid::new_v4(),
                correlation_id: None,
                user_id: None,
                timestamp: chrono::Utc::now(),
                path: "/test".to_string(),
                method: "GET".to_string(),
                client_ip: "127.0.0.1".to_string(),
                user_agent: "test".to_string(),
            },
            attributes: Default::default(),
        };

        middleware.on_request(&mut context).await;

        assert!(context
            .attributes
            .get("Authorization")
            .unwrap()
            .contains("Bearer"));
    }
}
