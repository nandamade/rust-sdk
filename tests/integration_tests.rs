//! Comprehensive integration and unit tests for the SDK

#[cfg(test)]
mod tests {
    use rust_sdk::cache::Cache;
    use rust_sdk::database::*;
    use rust_sdk::prelude::*;
    use rust_sdk::rate_limit::{PerUserRateLimiter, TokenBucket};
    use rust_sdk::utils::*;
    use rust_sdk::validation::*;

    use uuid::Uuid;

    // ============ Configuration Tests ============

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.base_url, "https://api.example.com");
        assert!(config.api_key.is_none());
        assert!(!config.debug);
    }

    #[test]
    fn test_config_builder() {
        let config = Config::new("https://api.test.com")
            .with_api_key("test-key")
            .with_debug(true);

        assert_eq!(config.base_url, "https://api.test.com");
        assert_eq!(config.api_key, Some("test-key".to_string()));
        assert!(config.debug);
    }

    #[test]
    fn test_config_with_header() {
        let config =
            Config::new("https://api.example.com").with_header("X-Custom-Header", "custom-value");

        assert_eq!(
            config.custom_headers.get("X-Custom-Header"),
            Some(&"custom-value".to_string())
        );
    }

    // ============ Client Tests ============

    #[test]
    fn test_client_creation() {
        let client = Client::new("https://api.example.com");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_from_config() {
        let config = Config::new("https://api.example.com");
        let client = Client::with_config(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_validate_email() {
        let client = Client::new("https://api.example.com").unwrap();

        assert!(client.validate_email("valid@example.com").is_ok());
        assert!(client.validate_email("invalid.email").is_err());
        assert!(client.validate_email("test+tag@example.co.uk").is_ok());
    }

    // ============ Cache Tests ============

    #[test]
    fn test_cache_set_get() {
        let cache = Cache::new(100, 3600);

        cache.set("key1", &"value1").unwrap();
        let result: String = cache.get("key1").unwrap().unwrap();

        assert_eq!(result, "value1");
    }

    #[test]
    fn test_cache_multiple_entries() {
        let cache = Cache::new(100, 3600);

        cache.set("key1", &"value1").unwrap();
        cache.set("key2", &"value2").unwrap();
        cache.set("key3", &"value3").unwrap();

        assert_eq!(cache.size(), 3);

        let result: String = cache.get("key2").unwrap().unwrap();
        assert_eq!(result, "value2");
    }

    #[test]
    fn test_cache_remove() {
        let cache = Cache::new(100, 3600);

        cache.set("key1", &"value1").unwrap();
        assert_eq!(cache.size(), 1);

        cache.remove("key1");
        assert_eq!(cache.size(), 0);
    }

    #[test]
    fn test_cache_contains_key() {
        let cache = Cache::new(100, 3600);

        cache.set("key1", &"value1").unwrap();
        assert!(cache.contains_key("key1"));
        assert!(!cache.contains_key("key2"));
    }

    #[test]
    fn test_cache_clear() {
        let cache = Cache::new(100, 3600);

        cache.set("key1", &"value1").unwrap();
        cache.set("key2", &"value2").unwrap();
        assert_eq!(cache.size(), 2);

        cache.clear();
        assert_eq!(cache.size(), 0);
    }

    // ============ Validation Tests ============

    #[test]
    fn test_email_validator() {
        let validator = EmailValidator;

        assert!(validator.validate("test@example.com").is_ok());
        assert!(validator.validate("user.name+tag@example.co.uk").is_ok());
        assert!(validator.validate("invalid").is_err());
        assert!(validator.validate("@example.com").is_err());
    }

    #[test]
    fn test_length_validator() {
        let validator = LengthValidator::new(3, 10);

        assert!(validator.validate("hello").is_ok());
        assert!(validator.validate("ab").is_err());
        assert!(validator.validate("this is too long").is_err());
    }

    #[test]
    fn test_alphanumeric_validator() {
        let validator = AlphanumericValidator;

        assert!(validator.validate("hello123").is_ok());
        assert!(validator.validate("hello_world").is_ok());
        assert!(validator.validate("hello-world").is_ok());
        assert!(validator.validate("hello@world").is_err());
        assert!(validator.validate("hello world").is_err());
    }

    #[test]
    fn test_regex_validator() {
        let validator = RegexValidator::new(r"^[0-9]{3}-[0-9]{4}$").unwrap();

        assert!(validator.validate("123-4567").is_ok());
        assert!(validator.validate("1234567").is_err());
    }

    #[test]
    fn test_sanitizer_html() {
        let input = "<script>alert('xss')</script>";
        let sanitized = Sanitizer::sanitize_html(input);

        assert!(!sanitized.contains("<script"));
        assert!(sanitized.contains("&lt;script"));
    }

    #[test]
    fn test_sanitizer_sql() {
        let input = "'; DROP TABLE users; --";
        let sanitized = Sanitizer::sanitize_sql(input);

        assert!(!sanitized.contains(";"));
        assert!(!sanitized.contains("--"));
    }

    #[test]
    fn test_sanitizer_path() {
        let input = "../../etc/passwd";
        let sanitized = Sanitizer::sanitize_path(input);

        assert!(!sanitized.contains("../"));
    }

    // ============ Rate Limiting Tests ============

    #[test]
    fn test_token_bucket_acquire() {
        let bucket = TokenBucket::new(10, 1);

        assert!(bucket.try_acquire(5).is_ok());
        assert_eq!(bucket.current_tokens(), 5);

        assert!(bucket.try_acquire(10).is_err());
    }

    #[test]
    fn test_token_bucket_multiple_acquire() {
        let bucket = TokenBucket::new(10, 1);

        assert!(bucket.try_acquire(3).is_ok());
        assert!(bucket.try_acquire(4).is_ok());
        assert!(bucket.try_acquire(3).is_ok());
        assert!(bucket.try_acquire(1).is_err());
    }

    #[test]
    fn test_per_user_rate_limiter() {
        let limiter = PerUserRateLimiter::new(10, 1);

        assert!(limiter.try_acquire("user1", 5).is_ok());
        assert_eq!(limiter.current_tokens("user1"), 5);

        assert!(limiter.try_acquire("user2", 5).is_ok());
        assert_eq!(limiter.current_tokens("user2"), 5);

        // Different users have separate limits
        assert!(limiter.try_acquire("user1", 6).is_err());
        assert!(limiter.try_acquire("user2", 5).is_ok());
    }

    // ============ Utility Tests ============

    #[test]
    fn test_exponential_backoff() {
        let backoff1 = exponential_backoff(0, 100, 30000, 2.0);
        let backoff2 = exponential_backoff(1, 100, 30000, 2.0);

        assert!(backoff2 > backoff1);
        assert!(backoff2.as_millis() <= 30000);
    }

    #[test]
    fn test_signature_generation() {
        let secret = "my-secret";
        let data = "test-data";

        let signature = SignatureGenerator::generate_signature(secret, data);
        assert!(!signature.is_empty());

        assert!(SignatureGenerator::verify_signature(secret, data, &signature).is_ok());
        assert!(SignatureGenerator::verify_signature(secret, data, "invalid").is_err());
    }

    #[test]
    fn test_rate_limiter_metrics() {
        let metrics = RateLimiterMetrics::new();

        assert_eq!(metrics.total_requests(), 0);
        assert_eq!(metrics.rejected_requests(), 0);

        metrics.record_request();
        metrics.record_request();

        assert_eq!(metrics.total_requests(), 2);
        assert_eq!(metrics.rejected_requests(), 0);
        assert_eq!(metrics.acceptance_rate(), 100.0);

        metrics.record_rejection();
        assert_eq!(metrics.acceptance_rate(), 50.0);
    }

    #[test]
    fn test_time_utils() {
        let now = TimeUtils::now_utc_secs();
        assert!(now > 0);

        assert_eq!(TimeUtils::secs_to_millis(1), 1000);
        assert_eq!(TimeUtils::secs_to_millis(60), 60000);

        assert_eq!(TimeUtils::millis_to_secs(1000), 1);
        assert_eq!(TimeUtils::millis_to_secs(60000), 60);
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicyBuilder::new()
            .max_attempts(5)
            .initial_backoff_ms(50)
            .max_backoff_ms(10000)
            .build();

        assert_eq!(policy.max_attempts, 5);
        assert_eq!(policy.initial_backoff_ms, 50);
        assert_eq!(policy.max_backoff_ms, 10000);
    }

    // ============ Database Tests ============

    #[test]
    fn test_base_entity_creation() {
        let entity = BaseEntity::new();

        assert!(entity.id != Uuid::nil());
        assert!(!entity.is_deleted());
        assert!(entity.deleted_at.is_none());
    }

    #[test]
    fn test_base_entity_soft_delete() {
        let mut entity = BaseEntity::new();
        assert!(!entity.is_deleted());

        entity.soft_delete();
        assert!(entity.is_deleted());
        assert!(entity.deleted_at.is_some());
    }

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new()
            .select(vec!["id", "username", "email"])
            .from("users")
            .where_clause("is_active = true")
            .where_clause("role = 'admin'")
            .order_by("created_at", "DESC")
            .limit(50)
            .offset(10)
            .build()
            .unwrap();

        assert!(query.contains("SELECT id, username, email"));
        assert!(query.contains("FROM users"));
        assert!(query.contains("WHERE is_active = true AND role = 'admin'"));
        assert!(query.contains("ORDER BY created_at DESC"));
        assert!(query.contains("LIMIT 50"));
        assert!(query.contains("OFFSET 10"));
    }

    #[test]
    fn test_query_builder_minimal() {
        let query = QueryBuilder::new().from("products").build().unwrap();

        assert!(query.contains("SELECT *"));
        assert!(query.contains("FROM products"));
    }

    #[tokio::test]
    async fn test_in_memory_database() {
        let db = InMemoryDatabase::new();

        assert!(db.health_check().await.is_ok());
        assert!(db.execute("INSERT INTO test VALUES (1)").await.is_ok());

        let result: Option<String> = db.query_one("SELECT * FROM test").await.unwrap();
        assert!(result.is_none()); // Not implemented in demo
    }

    // ============ Models Tests ============

    #[test]
    fn test_paginated_response_creation() {
        let items = vec![1, 2, 3, 4, 5];
        let response = PaginatedResponse::new(items, 1, 5, 23);

        assert_eq!(response.page, 1);
        assert_eq!(response.per_page, 5);
        assert_eq!(response.total, 23);
        assert_eq!(response.total_pages, 5);
        assert!(response.has_next);
        assert!(!response.has_prev);
    }

    #[test]
    fn test_paginated_response_last_page() {
        let items = vec![21, 22, 23];
        let response = PaginatedResponse::new(items, 5, 5, 23);

        assert_eq!(response.page, 5);
        assert!(!response.has_next);
        assert!(response.has_prev);
    }

    #[test]
    fn test_user_model_serialization() {
        let user = User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            display_name: "Test User".to_string(),
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: serde_json::json!({"role": "admin"}),
        };

        let json = serde_json::to_string(&user).unwrap();
        let deserialized: User = serde_json::from_str(&json).unwrap();

        assert_eq!(user.username, deserialized.username);
        assert_eq!(user.email, deserialized.email);
    }

    #[test]
    fn test_auth_token_model() {
        let token = AuthToken {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            token_type: "Bearer".to_string(),
            expires_in: 3600,
            issued_at: chrono::Utc::now(),
            scopes: vec!["read".to_string(), "write".to_string()],
        };

        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("Bearer"));
        assert!(json.contains("3600"));
    }
}
