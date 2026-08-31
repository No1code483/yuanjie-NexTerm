//! LRU 查询缓存层（Phase 3 §2.2.5）
//!
//! 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 3
//!
//! 设计目标：
//! - 缓存高频读查询结果（系统配置、用户基础信息、索引元数据、字典数据等）
//! - 默认 TTL 60s，可通过 with_ttl 覆盖；超过 TTL 自动失效
//! - 线程安全：Arc<Mutex<LruCache<...>>>，写入互斥，读取也互斥（LRU 内部需修改顺序）
//! - 容量可配：默认 256 项
//! - 非侵入式：调用方选择是否使用，不影响现有代码
//!
//! 不适用场景（不要缓存）：
//! - 频繁写入的表（如 activity_logs / messages / audit_log）
//! - 用户私有且实时性要求高的数据
//! - 大结果集（> 1KB）
//!
//! 使用示例：
//! ```ignore
//! use crate::db::query_cache::QueryCache;
//!
//! let cache = QueryCache::<String>::new(128);
//! if let Some(c) = cache.get("system_config:theme") {
//!     return Ok(c);
//! }
//! let val = sqlx::query_scalar("SELECT config_value FROM system_config WHERE config_key = ?")
//!     .bind("theme")
//!     .fetch_one(pool).await?;
//! cache.insert("system_config:theme".into(), val.clone());
//! Ok(val)
//! ```

use std::num::NonZeroUsize;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lru::LruCache;

/// 默认缓存容量
pub const DEFAULT_CACHE_CAPACITY: usize = 256;

/// 默认 TTL（60 秒）
pub const DEFAULT_TTL: Duration = Duration::from_secs(60);

/// 缓存项
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
}

/// 容量安全转换：0 视为 1（避免 NonZeroUsize 转换失败）
fn to_nonzero(capacity: usize) -> NonZeroUsize {
    NonZeroUsize::new(capacity.max(1)).unwrap_or_else(|| NonZeroUsize::new(1).unwrap())
}

/// TTL + LRU 缓存
///
/// 包装 `lru::LruCache`，增加 TTL 失效支持。
/// 线程安全：内部使用 `Mutex`（LRU 读取需修改访问顺序，无法用 RwLock）。
pub struct QueryCache<V: Clone> {
    inner: Mutex<LruCache<String, CacheEntry<V>>>,
    ttl: Duration,
}

impl<V: Clone> QueryCache<V> {
    /// 创建指定容量 + 默认 TTL（60s）的缓存
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(LruCache::new(to_nonzero(capacity))),
            ttl: DEFAULT_TTL,
        }
    }

    /// 创建指定容量 + TTL 的缓存
    pub fn with_ttl(capacity: usize, ttl: Duration) -> Self {
        Self {
            inner: Mutex::new(LruCache::new(to_nonzero(capacity))),
            ttl,
        }
    }

    /// 查询缓存（命中且未过期则返回值，并刷新 LRU 顺序）
    pub fn get(&self, key: &str) -> Option<V> {
        let mut guard = self.inner.lock().ok()?;
        let entry = guard.get(key)?;
        if entry.inserted_at.elapsed() > self.ttl {
            // 过期：移除并返回 None
            drop(guard);
            self.invalidate(key);
            return None;
        }
        Some(entry.value.clone())
    }

    /// 写入缓存
    pub fn insert(&self, key: String, value: V) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.put(
                key,
                CacheEntry {
                    value,
                    inserted_at: Instant::now(),
                },
            );
        }
    }

    /// 主动失效单条
    pub fn invalidate(&self, key: &str) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.pop(key);
        }
    }

    /// 清空全部
    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.clear();
        }
    }

    /// 当前缓存项数量（含可能过期但未清理的）
    pub fn len(&self) -> usize {
        self.inner.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_basic_get_insert() {
        let cache: QueryCache<String> = QueryCache::new(8);
        assert_eq!(cache.get("k1"), None);
        cache.insert("k1".into(), "v1".into());
        assert_eq!(cache.get("k1"), Some("v1".to_string()));
    }

    #[test]
    fn test_ttl_expiry() {
        let cache: QueryCache<i32> = QueryCache::with_ttl(8, Duration::from_millis(50));
        cache.insert("k".into(), 42);
        assert_eq!(cache.get("k"), Some(42));
        thread::sleep(Duration::from_millis(80));
        assert_eq!(cache.get("k"), None);
    }

    #[test]
    fn test_invalidate_and_clear() {
        let cache: QueryCache<i32> = QueryCache::new(8);
        cache.insert("a".into(), 1);
        cache.insert("b".into(), 2);
        cache.invalidate("a");
        assert_eq!(cache.get("a"), None);
        assert_eq!(cache.get("b"), Some(2));
        cache.clear();
        assert_eq!(cache.get("b"), None);
    }

    #[test]
    fn test_lru_eviction() {
        // 容量 2：插入 a, b, c 后 a 应被驱逐
        let cache: QueryCache<i32> = QueryCache::new(2);
        cache.insert("a".into(), 1);
        cache.insert("b".into(), 2);
        // 访问 a，使 b 成为最旧
        let _ = cache.get("a");
        cache.insert("c".into(), 3);
        assert_eq!(cache.get("b"), None); // b 被驱逐
        assert_eq!(cache.get("a"), Some(1));
        assert_eq!(cache.get("c"), Some(3));
    }
}
