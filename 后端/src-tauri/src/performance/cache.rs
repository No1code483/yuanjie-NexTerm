//! 缓存系统 — 多级缓存实现

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// 缓存条目
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    created_at: Instant,
    ttl: Duration,
    access_count: u64,
}

impl<V> CacheEntry<V> {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// 通用 LRU 缓存
#[derive(Debug)]
pub struct LruCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// 缓存数据
    entries: Mutex<HashMap<K, CacheEntry<V>>>,
    /// 最大容量
    max_capacity: usize,
    /// 默认 TTL
    default_ttl: Duration,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new(max_capacity: usize, ttl_secs: u64) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_capacity,
            default_ttl: Duration::from_secs(ttl_secs),
        }
    }

    /// 获取缓存值
    pub fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.get_mut(key) {
            if entry.is_expired() {
                entries.remove(key);
                return None;
            }
            entry.access_count += 1;
            Some(entry.value.clone())
        } else {
            None
        }
    }

    /// 设置缓存值
    pub fn set(&self, key: K, value: V) {
        let mut entries = self.entries.lock().unwrap();

        // 如果已满，移除最旧的条目
        if entries.len() >= self.max_capacity && !entries.contains_key(&key) {
            self.evict_one(&mut entries);
        }

        entries.insert(
            key,
            CacheEntry {
                value,
                created_at: Instant::now(),
                ttl: self.default_ttl,
                access_count: 0,
            },
        );
    }

    /// 设置带 TTL 的缓存值
    pub fn set_with_ttl(&self, key: K, value: V, ttl_secs: u64) {
        let mut entries = self.entries.lock().unwrap();

        if entries.len() >= self.max_capacity && !entries.contains_key(&key) {
            self.evict_one(&mut entries);
        }

        entries.insert(
            key,
            CacheEntry {
                value,
                created_at: Instant::now(),
                ttl: Duration::from_secs(ttl_secs),
                access_count: 0,
            },
        );
    }

    /// 删除缓存
    pub fn remove(&self, key: &K) -> Option<V> {
        self.entries
            .lock()
            .unwrap()
            .remove(key)
            .map(|e| e.value)
    }

    /// 清空缓存
    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
    }

    /// 获取缓存大小
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 驱逐最旧的条目
    fn evict_one(&self, entries: &mut HashMap<K, CacheEntry<V>>) {
        if let Some(oldest_key) = entries
            .iter()
            .min_by_key(|(_, e)| e.created_at)
            .map(|(k, _)| k.clone())
        {
            entries.remove(&oldest_key);
        }
    }

    /// 清理过期条目
    pub fn purge_expired(&self) -> usize {
        let mut entries = self.entries.lock().unwrap();
        let before = entries.len();
        entries.retain(|_, e| !e.is_expired());
        before - entries.len()
    }
}

/// 文件内容缓存
#[derive(Debug)]
pub struct FileCache {
    cache: LruCache<String, CachedFile>,
}

/// 缓存的文件
#[derive(Debug, Clone)]
pub struct CachedFile {
    pub content: String,
    pub size: usize,
    pub modified_at: i64,
}

impl FileCache {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            cache: LruCache::new(max_capacity, 300), // 5 分钟 TTL
        }
    }

    pub fn get(&self, path: &str) -> Option<CachedFile> {
        self.cache.get(&path.to_string())
    }

    pub fn set(&self, path: &str, content: String, size: usize, modified_at: i64) {
        self.cache.set(
            path.to_string(),
            CachedFile {
                content,
                size,
                modified_at,
            },
        );
    }

    pub fn invalidate(&self, path: &str) {
        self.cache.remove(&path.to_string());
    }

    pub fn clear(&self) {
        self.cache.clear();
    }
}

/// 搜索结果缓存
#[derive(Debug)]
pub struct SearchCache {
    cache: LruCache<String, Vec<SearchCacheEntry>>,
}

/// 搜索结果条目
#[derive(Debug, Clone)]
pub struct SearchCacheEntry {
    pub file_path: String,
    pub line_number: u32,
    pub line_content: String,
    pub match_start: usize,
    pub match_end: usize,
}

impl SearchCache {
    pub fn new() -> Self {
        Self {
            cache: LruCache::new(10, 120), // 2 分钟 TTL
        }
    }

    pub fn get(&self, query: &str) -> Option<Vec<SearchCacheEntry>> {
        self.cache.get(&query.to_string())
    }

    pub fn set(&self, query: &str, results: Vec<SearchCacheEntry>) {
        self.cache.set(query.to_string(), results);
    }

    pub fn invalidate_all(&self) {
        self.cache.clear();
    }
}