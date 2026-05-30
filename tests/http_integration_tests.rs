//! Integration tests using mockito to verify actual HTTP request/response flow.
//!
//! These tests verify:
//! - Successful GET/POST/PUT/DELETE requests
//! - Retry logic on server errors
//! - Rate limiter blocking requests
//! - Caching behavior (hit/miss)
//! - Error handling for various HTTP status codes

use rust_sdk::client::Client;
use rust_sdk::config::Config;
use rust_sdk::error::SdkError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TestPayload {
    id: u32,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateRequest {
    name: String,
}

// ============ Successful Requests ============

#[tokio::test]
async fn test_get_request_success() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/users/1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1, "name": "Alice"}"#)
        .create_async()
        .await;

    let client = Client::new(server.url()).unwrap();
    let result: TestPayload = client.get("/users/1").await.unwrap();

    assert_eq!(
        result,
        TestPayload {
            id: 1,
            name: "Alice".to_string()
        }
    );
    mock.assert_async().await;
}

#[tokio::test]
async fn test_post_request_success() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("POST", "/users")
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 42, "name": "Bob"}"#)
        .create_async()
        .await;

    let client = Client::new(server.url()).unwrap();
    let body = CreateRequest {
        name: "Bob".to_string(),
    };
    let result: TestPayload = client.post("/users", &body).await.unwrap();

    assert_eq!(
        result,
        TestPayload {
            id: 42,
            name: "Bob".to_string()
        }
    );
    mock.assert_async().await;
}

#[tokio::test]
async fn test_put_request_success() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("PUT", "/users/1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1, "name": "Alice Updated"}"#)
        .create_async()
        .await;

    let client = Client::new(server.url()).unwrap();
    let body = CreateRequest {
        name: "Alice Updated".to_string(),
    };
    let result: TestPayload = client.put("/users/1", &body).await.unwrap();

    assert_eq!(
        result,
        TestPayload {
            id: 1,
            name: "Alice Updated".to_string()
        }
    );
    mock.assert_async().await;
}

#[tokio::test]
async fn test_delete_request_success() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("DELETE", "/users/1")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1, "name": "Alice"}"#)
        .create_async()
        .await;

    let client = Client::new(server.url()).unwrap();
    let result: TestPayload = client.delete("/users/1").await.unwrap();

    assert_eq!(
        result,
        TestPayload {
            id: 1,
            name: "Alice".to_string()
        }
    );
    mock.assert_async().await;
}

// ============ Error Handling ============

#[tokio::test]
async fn test_get_request_404() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/not-found")
        .with_status(404)
        .with_body("Not Found")
        .expect_at_least(1)
        .create_async()
        .await;

    let client = Client::new(server.url()).unwrap();
    let result: Result<TestPayload, SdkError> = client.get("/not-found").await;

    assert!(result.is_err());
    mock.assert_async().await;
}

#[tokio::test]
async fn test_get_request_500_triggers_retry() {
    let mut server = mockito::Server::new_async().await;

    // Server always returns 500 — client should retry up to max_attempts
    let mock = server
        .mock("GET", "/unstable")
        .with_status(500)
        .with_body("Internal Server Error")
        .expect_at_least(2) // Should retry at least once
        .create_async()
        .await;

    // Use config with fast retry for testing
    let config = Config::new(server.url());
    let client = Client::with_config(config).unwrap();
    let result: Result<TestPayload, SdkError> = client.get("/unstable").await;

    assert!(result.is_err());
    mock.assert_async().await;
}

// ============ Caching ============

#[tokio::test]
async fn test_get_request_caching() {
    let mut server = mockito::Server::new_async().await;

    // Server should only be hit once — second call should come from cache
    let mock = server
        .mock("GET", "/cached")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1, "name": "Cached"}"#)
        .expect(1) // Exactly 1 hit
        .create_async()
        .await;

    let client = Client::new(server.url()).unwrap();

    // First call — hits server
    let result1: TestPayload = client.get("/cached").await.unwrap();
    assert_eq!(result1.name, "Cached");

    // Second call — should come from cache, not server
    let result2: TestPayload = client.get("/cached").await.unwrap();
    assert_eq!(result2.name, "Cached");

    // Verify server was only hit once
    mock.assert_async().await;
}

#[tokio::test]
async fn test_clear_cache_forces_new_request() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/refresh")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1, "name": "Fresh"}"#)
        .expect(2) // Should be hit twice
        .create_async()
        .await;

    let client = Client::new(server.url()).unwrap();

    // First call
    let _: TestPayload = client.get("/refresh").await.unwrap();

    // Clear cache
    client.clear_cache();

    // Second call — should hit server again
    let _: TestPayload = client.get("/refresh").await.unwrap();

    mock.assert_async().await;
}

// ============ Rate Limiting ============

#[tokio::test]
async fn test_rate_limiter_blocks_excess_requests() {
    let mut server = mockito::Server::new_async().await;

    // Create many mocks — but rate limiter should block before all are used
    let _mock = server
        .mock("GET", "/limited")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 1, "name": "OK"}"#)
        .create_async()
        .await;

    // Config with very low rate limit (2 requests per second)
    let mut config = Config::new(server.url());
    config.rate_limit.requests_per_second = 2;
    let client = Client::with_config(config).unwrap();

    // First 2 should succeed
    let r1: Result<TestPayload, SdkError> = client.get("/limited").await;
    assert!(r1.is_ok());

    // Clear cache so it actually makes a new request
    client.clear_cache();
    let r2: Result<TestPayload, SdkError> = client.get("/limited").await;
    assert!(r2.is_ok());

    // Third should be rate limited
    client.clear_cache();
    let r3: Result<TestPayload, SdkError> = client.get("/limited").await;
    assert!(r3.is_err());

    if let Err(SdkError::RateLimitExceeded(_)) = r3 {
        // Expected
    } else {
        panic!("Expected RateLimitExceeded error, got: {:?}", r3);
    }
}

// ============ JSON Parse Errors ============

#[tokio::test]
async fn test_get_request_invalid_json() {
    let mut server = mockito::Server::new_async().await;

    let mock = server
        .mock("GET", "/bad-json")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("this is not json")
        .create_async()
        .await;

    let client = Client::new(server.url()).unwrap();
    let result: Result<TestPayload, SdkError> = client.get("/bad-json").await;

    assert!(result.is_err());
    mock.assert_async().await;
}
