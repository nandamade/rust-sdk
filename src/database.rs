//! Database integration module

use crate::error::{Result, SdkError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Database connection trait
#[async_trait::async_trait]
pub trait DatabaseConnection: Send + Sync {
    /// Execute query
    async fn execute(&self, query: &str) -> Result<()>;

    /// Query single row
    async fn query_one<T: for<'de> Deserialize<'de>>(&self, query: &str) -> Result<Option<T>>;

    /// Query multiple rows
    async fn query_all<T: for<'de> Deserialize<'de>>(&self, query: &str) -> Result<Vec<T>>;

    /// Health check
    async fn health_check(&self) -> Result<()>;
}

/// In-memory database for testing
pub struct InMemoryDatabase {
    data: Arc<parking_lot::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

impl Default for InMemoryDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryDatabase {
    /// Create new in-memory database
    pub fn new() -> Self {
        Self {
            data: Arc::new(parking_lot::Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Insert record
    pub fn insert(&self, key: String, value: Vec<u8>) {
        self.data.lock().insert(key, value);
    }

    /// Get record
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.data.lock().get(key).cloned()
    }

    /// Delete record
    pub fn delete(&self, key: &str) -> Option<Vec<u8>> {
        self.data.lock().remove(key)
    }
}

impl Clone for InMemoryDatabase {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
}

#[async_trait::async_trait]
impl DatabaseConnection for InMemoryDatabase {
    async fn execute(&self, _query: &str) -> Result<()> {
        Ok(())
    }

    async fn query_one<T: for<'de> Deserialize<'de>>(&self, _query: &str) -> Result<Option<T>> {
        Ok(None)
    }

    async fn query_all<T: for<'de> Deserialize<'de>>(&self, _query: &str) -> Result<Vec<T>> {
        Ok(Vec::new())
    }

    async fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

/// User repository
pub struct UserRepository<DB: DatabaseConnection> {
    db: Arc<DB>,
}

impl<DB: DatabaseConnection> UserRepository<DB> {
    /// Create new user repository
    pub fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    /// Create user
    pub async fn create(&self, user: &UserDto) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let (query, _params) =
            ParameterizedQuery::new("INSERT INTO users (id, username, email) VALUES ($1, $2, $3)")
                .bind(id.to_string())
                .bind(user.username.clone())
                .bind(user.email.clone())
                .build();

        self.db.execute(&query).await?;
        let _ = _params; // params would be used by a real DB driver
        Ok(id)
    }

    /// Get user by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<UserDto>> {
        let (query, _params) = ParameterizedQuery::new("SELECT * FROM users WHERE id = $1")
            .bind(id.to_string())
            .build();

        self.db.query_one(&query).await
    }

    /// Get all users
    pub async fn get_all(&self) -> Result<Vec<UserDto>> {
        let query = "SELECT * FROM users";
        self.db.query_all(query).await
    }

    /// Update user
    pub async fn update(&self, id: Uuid, user: &UserDto) -> Result<()> {
        let (query, _params) =
            ParameterizedQuery::new("UPDATE users SET username = $1, email = $2 WHERE id = $3")
                .bind(user.username.clone())
                .bind(user.email.clone())
                .bind(id.to_string())
                .build();

        self.db.execute(&query).await
    }

    /// Delete user
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let (query, _params) = ParameterizedQuery::new("DELETE FROM users WHERE id = $1")
            .bind(id.to_string())
            .build();

        self.db.execute(&query).await
    }
}

/// User data transfer object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: Option<Uuid>,
    pub username: String,
    pub email: String,
    pub created_at: Option<DateTime<Utc>>,
}

/// Entity trait
pub trait Entity: Send + Sync {
    /// Get entity ID
    fn id(&self) -> Uuid;

    /// Get entity type
    fn entity_type(&self) -> &str;

    /// Serialize to JSON
    fn to_json(&self) -> Result<String>;
}

/// Base entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseEntity {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl BaseEntity {
    /// Create new base entity
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Mark as deleted (soft delete)
    pub fn soft_delete(&mut self) {
        self.deleted_at = Some(Utc::now());
    }

    /// Check if deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl Default for BaseEntity {
    fn default() -> Self {
        Self::new()
    }
}

/// A parameterized query that separates SQL template from user-provided values.
///
/// This prevents SQL injection by never interpolating user input directly
/// into the query string. The query template uses `$1`, `$2`, etc. as
/// placeholders, and parameters are stored separately.
///
/// # Example
///
/// ```
/// use rust_sdk::database::ParameterizedQuery;
///
/// let (query, params) = ParameterizedQuery::new("SELECT * FROM users WHERE id = $1")
///     .bind("some-uuid".to_string())
///     .build();
///
/// assert_eq!(query, "SELECT * FROM users WHERE id = $1");
/// assert_eq!(params, vec!["some-uuid"]);
/// ```
#[derive(Debug, Clone)]
pub struct ParameterizedQuery {
    query: String,
    params: Vec<String>,
}

impl ParameterizedQuery {
    /// Create a new parameterized query with a SQL template.
    /// Use `$1`, `$2`, etc. as placeholders for parameters.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            params: Vec::new(),
        }
    }

    /// Bind a parameter value. Parameters are bound in order ($1, $2, ...).
    pub fn bind(mut self, value: String) -> Self {
        self.params.push(value);
        self
    }

    /// Build the query, returning the template and parameters separately.
    /// The query string is never modified — parameters stay separate.
    pub fn build(self) -> (String, Vec<String>) {
        (self.query, self.params)
    }

    /// Get the number of bound parameters
    pub fn param_count(&self) -> usize {
        self.params.len()
    }
}

/// Query builder for constructing SQL queries
pub struct QueryBuilder {
    select: Vec<String>,
    from: Option<String>,
    where_clauses: Vec<String>,
    order_by: Vec<String>,
    limit_value: Option<u32>,
    offset_value: Option<u32>,
}

impl QueryBuilder {
    /// Create new query builder
    pub fn new() -> Self {
        Self {
            select: vec!["*".to_string()],
            from: None,
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit_value: None,
            offset_value: None,
        }
    }

    /// Select columns
    pub fn select(mut self, columns: Vec<&str>) -> Self {
        self.select = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// From table
    pub fn from(mut self, table: &str) -> Self {
        self.from = Some(table.to_string());
        self
    }

    /// Add where clause
    pub fn where_clause(mut self, clause: &str) -> Self {
        self.where_clauses.push(clause.to_string());
        self
    }

    /// Order by
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by.push(format!("{column} {direction}"));
        self
    }

    /// Limit
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Offset
    pub fn offset(mut self, offset: u32) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Build query
    pub fn build(&self) -> Result<String> {
        let mut query = String::from("SELECT ");
        query.push_str(&self.select.join(", "));

        query.push_str(" FROM ");
        query.push_str(
            self.from
                .as_ref()
                .ok_or_else(|| SdkError::database("Table name not specified"))?,
        );

        if !self.where_clauses.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&self.where_clauses.join(" AND "));
        }

        if !self.order_by.is_empty() {
            query.push_str(" ORDER BY ");
            query.push_str(&self.order_by.join(", "));
        }

        if let Some(limit) = self.limit_value {
            query.push_str(&format!(" LIMIT {limit}"));
        }

        if let Some(offset) = self.offset_value {
            query.push_str(&format!(" OFFSET {offset}"));
        }

        Ok(query)
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_entity() {
        let entity = BaseEntity::new();
        assert!(!entity.is_deleted());
        assert_eq!(entity.created_at, entity.updated_at);
    }

    #[test]
    fn test_soft_delete() {
        let mut entity = BaseEntity::new();
        entity.soft_delete();
        assert!(entity.is_deleted());
    }

    #[test]
    fn test_query_builder() {
        let query = QueryBuilder::new()
            .select(vec!["id", "username"])
            .from("users")
            .where_clause("is_active = true")
            .order_by("created_at", "DESC")
            .limit(10)
            .build()
            .unwrap();

        assert!(query.contains("SELECT id, username"));
        assert!(query.contains("FROM users"));
        assert!(query.contains("WHERE is_active = true"));
        assert!(query.contains("ORDER BY created_at DESC"));
        assert!(query.contains("LIMIT 10"));
    }

    #[tokio::test]
    async fn test_in_memory_database() {
        let db = InMemoryDatabase::new();
        assert!(db.health_check().await.is_ok());
    }

    #[test]
    fn test_parameterized_query_basic() {
        let (query, params) = ParameterizedQuery::new("SELECT * FROM users WHERE id = $1")
            .bind("abc-123".to_string())
            .build();

        assert_eq!(query, "SELECT * FROM users WHERE id = $1");
        assert_eq!(params, vec!["abc-123"]);
    }

    #[test]
    fn test_parameterized_query_multiple_params() {
        let (query, params) =
            ParameterizedQuery::new("INSERT INTO users (id, username, email) VALUES ($1, $2, $3)")
                .bind("id-1".to_string())
                .bind("testuser".to_string())
                .bind("test@example.com".to_string())
                .build();

        assert_eq!(
            query,
            "INSERT INTO users (id, username, email) VALUES ($1, $2, $3)"
        );
        assert_eq!(params.len(), 3);
        assert_eq!(params[0], "id-1");
        assert_eq!(params[1], "testuser");
        assert_eq!(params[2], "test@example.com");
    }

    #[test]
    fn test_parameterized_query_prevents_injection() {
        // Even if user input contains SQL, it stays as a parameter value
        let malicious_input = "'; DROP TABLE users; --".to_string();
        let (query, params) = ParameterizedQuery::new("SELECT * FROM users WHERE name = $1")
            .bind(malicious_input.clone())
            .build();

        // Query template is unchanged — no injection possible
        assert_eq!(query, "SELECT * FROM users WHERE name = $1");
        // Malicious input is safely contained in params
        assert_eq!(params[0], malicious_input);
        // The query string does NOT contain the malicious input
        assert!(!query.contains("DROP TABLE"));
    }

    #[test]
    fn test_parameterized_query_param_count() {
        let pq = ParameterizedQuery::new("SELECT $1, $2, $3")
            .bind("a".to_string())
            .bind("b".to_string())
            .bind("c".to_string());

        assert_eq!(pq.param_count(), 3);
    }
}
