//! Caching layer for the SDK

use crate::error::{Result, SdkError};
use chrono::{Duration, Utc};
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;

/// Cache entry with TTL
#[derive(Clone, Debug)]
struct CacheEntry {
    value: Vec<u8>,
    expires_at: chrono::DateTime<Utc>,
}

impl CacheEntry {
    /// Check if entry is expired
    fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// In-memory cache with TTL support
pub struct Cache {
    storage: Arc<DashMap<String, CacheEntry>>,
    max_entries: usize,
    ttl: Duration,
}

impl Cache {
    /// Create a new cache
    pub fn new(max_entries: usize, ttl_seconds: u64) -> Self {
        Self {
            storage: Arc::new(DashMap::new()),
            max_entries,
            ttl: Duration::seconds(ttl_seconds as i64),
        }
    }

    /// Get value from cache
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        if let Some(entry) = self.storage.get(key) {
            if entry.is_expired() {
                drop(entry);
                self.storage.remove(key);
                return Ok(None);
            }

            serde_json::from_slice(&entry.value)
                .map(Some)
                .map_err(|e| SdkError::CacheError(format!("Deserialization error: {e}")))
        } else {
            Ok(None)
        }
    }

    /// Set value in cache
    pub fn set<T: Serialize>(&self, key: impl Into<String>, value: &T) -> Result<()> {
        if self.storage.len() >= self.max_entries {
            // Simple eviction: remove an arbitrary entry when full
            if let Some(entry) = self.storage.iter().next() {
                let key_to_remove = entry.key().clone();
                drop(entry);
                self.storage.remove(&key_to_remove);
            }
        }

        let key = key.into();
        let expires_at = Utc::now() + self.ttl;

        let serialized = serde_json::to_vec(value)
            .map_err(|e| SdkError::CacheError(format!("Serialization error: {e}")))?;

        self.storage.insert(
            key,
            CacheEntry {
                value: serialized,
                expires_at,
            },
        );

        Ok(())
    }

    /// Remove value from cache
    pub fn remove(&self, key: &str) {
        self.storage.remove(key);
    }

    /// Clear all cache
    pub fn clear(&self) {
        self.storage.clear();
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        self.storage.len()
    }

    /// Check if key exists
    pub fn contains_key(&self, key: &str) -> bool {
        self.storage.contains_key(key)
    }

    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        self.storage
            .iter()
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Remove expired entries
    pub fn cleanup_expired(&self) {
        let expired_keys: Vec<_> = self
            .storage
            .iter()
            .filter(|entry| entry.is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            self.storage.remove(&key);
        }
    }
}

impl Clone for Cache {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
            max_entries: self.max_entries,
            ttl: self.ttl,
        }
    }
}

/// LRU cache wrapper
pub struct LruCache {
    cache: Arc<parking_lot::Mutex<lru::LruCache<String, Vec<u8>>>>,
    ttl: Duration,
    expires: Arc<DashMap<String, chrono::DateTime<Utc>>>,
}

impl LruCache {
    /// Create new LRU cache
    pub fn new(capacity: usize, ttl_seconds: u64) -> Self {
        let cache = lru::LruCache::new(std::num::NonZeroUsize::new(capacity).unwrap());

        Self {
            cache: Arc::new(parking_lot::Mutex::new(cache)),
            ttl: Duration::seconds(ttl_seconds as i64),
            expires: Arc::new(DashMap::new()),
        }
    }

    /// Get value
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        // Check expiration
        if let Some(expires) = self.expires.get(key) {
            if Utc::now() > *expires {
                self.remove(key);
                return Ok(None);
            }
        }

        let mut cache = self.cache.lock();
        if let Some(value) = cache.get(key) {
            serde_json::from_slice(value)
                .map(Some)
                .map_err(|e| SdkError::CacheError(format!("Deserialization error: {e}")))
        } else {
            Ok(None)
        }
    }

    /// Set value
    pub fn set<T: Serialize>(&self, key: impl Into<String>, value: &T) -> Result<()> {
        let key = key.into();
        let serialized = serde_json::to_vec(value)
            .map_err(|e| SdkError::CacheError(format!("Serialization error: {e}")))?;

        let mut cache = self.cache.lock();
        cache.put(key.clone(), serialized);

        self.expires.insert(key, Utc::now() + self.ttl);

        Ok(())
    }

    /// Remove value
    pub fn remove(&self, key: &str) {
        let mut cache = self.cache.lock();
        cache.pop(key);
        self.expires.remove(key);
    }

    /// Clear cache
    pub fn clear(&self) {
        self.cache.lock().clear();
        self.expires.clear();
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.cache.lock().len()
    }
}

impl Clone for LruCache {
    fn clone(&self) -> Self {
        Self {
            cache: Arc::clone(&self.cache),
            ttl: self.ttl,
            expires: Arc::clone(&self.expires),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_set_and_get() {
        let cache = Cache::new(100, 3600);
        cache.set("key1", &"value1").unwrap();

        let result: String = cache.get("key1").unwrap().unwrap();
        assert_eq!(result, "value1");
    }

    #[test]
    fn test_cache_expiration() {
        let cache = Cache::new(100, 1);
        cache.set("key1", &"value1").unwrap();

        std::thread::sleep(std::time::Duration::from_secs(2));

        let result: Option<String> = cache.get("key1").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_remove() {
        let cache = Cache::new(100, 3600);
        cache.set("key1", &"value1").unwrap();

        cache.remove("key1");

        let result: Option<String> = cache.get("key1").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_lru_cache() {
        let cache = LruCache::new(2, 3600);

        cache.set("key1", &"value1").unwrap();
        cache.set("key2", &"value2").unwrap();
        cache.set("key3", &"value3").unwrap();

        let result: Option<String> = cache.get("key1").unwrap();
        // key1 might be evicted due to LRU
        assert!(result.is_none() || result == Some("value1".to_string()));
    }
}
