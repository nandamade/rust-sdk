//! Data models for the SDK

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Base response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// Success status
    pub success: bool,

    /// Response data
    pub data: Option<T>,

    /// Error message if failed
    pub error: Option<String>,

    /// Request ID for tracking
    pub request_id: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Response metadata
    pub metadata: ResponseMetadata,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// HTTP status code
    pub status_code: u16,

    /// Response time in ms
    pub response_time_ms: u64,

    /// API version
    pub api_version: String,

    /// Rate limit info
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_info: Option<RateLimitInfo>,
}

/// Rate limit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Limit
    pub limit: u32,

    /// Remaining requests
    pub remaining: u32,

    /// Reset timestamp
    pub reset_at: DateTime<Utc>,
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// Items in this page
    pub items: Vec<T>,

    /// Current page
    pub page: u32,

    /// Items per page
    pub per_page: u32,

    /// Total items
    pub total: u64,

    /// Total pages
    pub total_pages: u32,

    /// Has next page
    pub has_next: bool,

    /// Has previous page
    pub has_prev: bool,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(items: Vec<T>, page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        let has_next = page < total_pages;
        let has_prev = page > 1;

        Self {
            items,
            page,
            per_page,
            total,
            total_pages,
            has_next,
            has_prev,
        }
    }
}

/// User model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    /// User ID
    pub id: Uuid,

    /// Username
    pub username: String,

    /// Email
    pub email: String,

    /// Display name
    pub display_name: String,

    /// Is active
    pub is_active: bool,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// User creation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub display_name: String,
}

/// Auth token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    /// Access token
    pub access_token: String,

    /// Refresh token
    pub refresh_token: String,

    /// Token type
    pub token_type: String,

    /// Expires in seconds
    pub expires_in: u64,

    /// Issued at
    pub issued_at: DateTime<Utc>,

    /// Scopes
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMetadata {
    /// Request ID
    pub request_id: Uuid,

    /// Correlation ID
    pub correlation_id: Option<Uuid>,

    /// User ID
    pub user_id: Option<Uuid>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Request path
    pub path: String,

    /// Request method
    pub method: String,

    /// Client IP
    pub client_ip: String,

    /// User agent
    pub user_agent: String,
}

/// Event model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Event ID
    pub id: Uuid,

    /// Event type
    pub event_type: String,

    /// Associated resource ID
    pub resource_id: Option<Uuid>,

    /// Event data
    pub data: serde_json::Value,

    /// Created by
    pub created_by: Uuid,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Webhook model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    /// Webhook ID
    pub id: Uuid,

    /// Webhook URL
    pub url: String,

    /// Events to subscribe to
    pub events: Vec<String>,

    /// Is active
    pub is_active: bool,

    /// Secret for signing
    pub secret: String,

    /// Retry policy
    pub retry_policy: RetryPolicy,

    /// Created at
    pub created_at: DateTime<Utc>,

    /// Updated at
    pub updated_at: DateTime<Utc>,
}

/// Retry policy for webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// Maximum retries
    pub max_retries: u32,

    /// Backoff in seconds
    pub backoff_seconds: u32,

    /// Maximum backoff
    pub max_backoff_seconds: u32,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Status
    pub status: String,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Version
    pub version: String,

    /// Service checks
    pub checks: std::collections::HashMap<String, ServiceHealth>,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    /// Is healthy
    pub healthy: bool,

    /// Status message
    pub message: String,

    /// Response time in ms
    pub response_time_ms: u64,
}

/// Error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// Error code
    pub code: String,

    /// Error message
    pub message: String,

    /// Error details
    pub details: Option<serde_json::Value>,

    /// Request ID
    pub request_id: Uuid,

    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginated_response_creation() {
        let items = vec![1, 2, 3];
        let response = PaginatedResponse::new(items, 1, 10, 25);

        assert_eq!(response.page, 1);
        assert_eq!(response.total, 25);
        assert_eq!(response.total_pages, 3);
        assert!(response.has_next);
        assert!(!response.has_prev);
    }

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: serde_json::json!({}),
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user, deserialized);
    }
}
