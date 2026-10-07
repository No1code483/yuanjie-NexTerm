//! 事件总线（契约：06_Rust代码契约.md §五）
//! broadcast 分域 + 白名单通配 + 拦截链 + kernel_domain_events 异步落库
//!
//! 实现偏差（已记入 06 修订记录）：契约 §三 KernelCtx.events 为 `&EventBus`（不可变引用），
//! 与 §五 `publish/subscribe(&mut self)` 矛盾；且 tauri command 侧需经 Arc 共享。
//! 故采用内部锁（RwLock/Mutex）实现共享可变，公开签名统一为 `&self`。

use kernel_api::{Event, EventScope, PluginId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;

use crate::error::KernelError;

/// 拦截器：同步执行、错误即 veto
/// TODO(阶段3): 拦截链单条 50ms 超时（阶段1 拦截器均为内核内置同步桩，无外呼风险）
pub type Interceptor = Arc<dyn Fn(&Event) -> Result<(), KernelError> + Send + Sync>;

/// 领域事件落库器（由 tauri_glue 注入后启用；异步批量由实现方自行调度）
pub type EventPersister = Arc<dyn Fn(Event) + Send + Sync>;

type Handler = Arc<dyn Fn(&Event) + Send + Sync>;

struct SubscriptionEntry {
    /// 订阅者插件 id（disable 回滚按插件退订时定位）
    #[allow(dead_code)]
    plugin: PluginId,
    filter: String,
}

/// 共享内核状态：registry + bus + db + security（tauri_glue 注入 Tauri manage）
/// 插件命令经 `state.ensure_plugin_enabled(...)` 实现停用即拒
pub struct EventBus {
    /// 按 domain 分组的广播通道
    domains: RwLock<HashMap<String, broadcast::Sender<Arc<Event>>>>,
    /// 能力事件的拦截链（有序，任一 Err 即 veto）：domain -> chain
    interceptors: RwLock<HashMap<String, Vec<Interceptor>>>,
    /// 订阅表：sub_id -> 订阅条目
    subs: Mutex<HashMap<u64, SubscriptionEntry>>,
    /// 同域进程内 handler 分发表：filter -> [(sub_id, handler)]
    handlers: RwLock<HashMap<String, Vec<(u64, Handler)>>>,
    next_sub_id: AtomicU64,
    /// 领域事件异步落库钩子（kernel_domain_events，总线独占写入）
    persister: RwLock<Option<EventPersister>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            domains: RwLock::new(HashMap::new()),
            interceptors: RwLock::new(HashMap::new()),
            subs: Mutex::new(HashMap::new()),
            handlers: RwLock::new(HashMap::new()),
            next_sub_id: AtomicU64::new(1),
            persister: RwLock::new(None),
        }
    }

    /// 注入领域事件落库器（tauri_glue 启动时调用一次）
    pub fn set_persister(&self, p: EventPersister) {
        *self.persister.write().unwrap() = Some(p);
    }

    /// 通配匹配：filter 支持 "hello:*"；"*" 匹配全部
    fn matches(filter: &str, name: &str) -> bool {
        if filter == "*" {
            return true;
        }
        if let Some(prefix) = filter.strip_suffix(":*") {
            return name.starts_with(prefix) && name[prefix.len()..].starts_with(':');
        }
        filter == name
    }

    /// 发布：校验「domain == 发布者自己的域」；领域事件异步落 kernel_domain_events
    /// （manifest 发布白名单校验由 SecurityGate::check_publish 在内核入口统一执行）
    pub fn publish(&self, origin: &PluginId, ev: Event) -> Result<(), KernelError> {
        // 域一致性：name 冒号前部分 == domain 字段 == 发布者域名
        let name_domain = ev.name.split(':').next().unwrap_or("");
        if name_domain.is_empty() || name_domain != ev.domain {
            return Err(KernelError::EventBus(format!(
                "事件名 {} 与域声明 {} 不一致",
                ev.name, ev.domain
            )));
        }
        if ev.domain != *origin {
            return Err(KernelError::PermissionDenied(format!(
                "插件 {origin} 试图发布非本域事件 {}（域 {}）",
                ev.name, ev.domain
            )));
        }

        let arc: Arc<Event> = Arc::new(ev);

        // 拦截链（仅能力事件）：任一 Err 即 veto
        if arc.scope == EventScope::Capability {
            let chain = self.interceptors.read().unwrap().get(&arc.domain).cloned();
            if let Some(chain) = chain {
                for it in chain {
                    it(&arc)?;
                }
            }
        }

        // 分域广播（无接收者时 send 返回 Err，属正常，忽略）
        let tx = self.domains.read().unwrap().get(&arc.domain).cloned();
        if let Some(tx) = tx {
            let _ = tx.send(Arc::clone(&arc));
        }

        // 进程内 handler 分发（按 filter 匹配）
        let snapshot: Vec<Handler> = {
            let map = self.handlers.read().unwrap();
            map.iter()
                .filter(|(filter, _)| Self::matches(filter, &arc.name))
                .flat_map(|(_, v)| v.iter().map(|(_, h)| h.clone()))
                .collect()
        };
        for h in snapshot {
            h(&arc);
        }

        // 领域事件异步落库（kernel_domain_events）
        if arc.scope == EventScope::Domain {
            let p = self.persister.read().unwrap().clone();
            if let Some(p) = p {
                let ev2 = (*arc).clone();
                tokio::spawn(async move { p(ev2) });
            }
        }
        Ok(())
    }

    /// 订阅：返回订阅 id（记入 PluginHandle.subscriptions）
    /// （manifest 订阅白名单校验由 SecurityGate::check_subscribe 在内核入口统一执行）
    pub fn subscribe<F>(&self, plugin: &PluginId, filter: &str, h: F) -> Result<u64, KernelError>
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        if filter.is_empty() {
            return Err(KernelError::EventBus("订阅 filter 不能为空".into()));
        }
        let id = self.next_sub_id.fetch_add(1, Ordering::SeqCst);
        self.handlers
            .write()
            .unwrap()
            .entry(filter.to_string())
            .or_default()
            .push((id, Arc::new(h)));
        self.subs.lock().unwrap().insert(
            id,
            SubscriptionEntry { plugin: plugin.clone(), filter: filter.to_string() },
        );
        Ok(id)
    }

    /// 拦截链注册（仅能力事件域可用）
    pub fn intercept(&self, seam: &str, h: Interceptor) -> Result<(), KernelError> {
        self.interceptors.write().unwrap().entry(seam.to_string()).or_default().push(h);
        Ok(())
    }

    /// 退订（可逆注册回滚入口）；幂等：不存在的 id 不报错
    pub fn unsubscribe(&self, sub_id: u64) -> Result<(), KernelError> {
        let removed = self.subs.lock().unwrap().remove(&sub_id);
        if let Some(entry) = removed {
            let mut map = self.handlers.write().unwrap();
            if let Some(list) = map.get_mut(&entry.filter) {
                list.retain(|(id, _)| *id != sub_id);
                if list.is_empty() {
                    map.remove(&entry.filter);
                }
            }
        }
        Ok(())
    }

    /// 获取某域的广播接收端（供 Tauri 桥接转发 k://event）
    pub fn domain_receiver(&self, domain: &str) -> broadcast::Receiver<Arc<Event>> {
        let mut map = self.domains.write().unwrap();
        let tx = map.entry(domain.to_string()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(256);
            tx
        });
        tx.subscribe()
    }

    /// 当前订阅数（测试/诊断用）
    pub fn sub_count(&self) -> usize {
        self.subs.lock().unwrap().len()
    }
}

/// 通配匹配（桥接层复用）
pub fn match_pattern(filter: &str, name: &str) -> bool {
    EventBus::matches(filter, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::AtomicUsize;

    fn ev(origin: &str, name: &str, scope: EventScope) -> Event {
        let domain = name.split(':').next().unwrap().to_string();
        Event {
            name: name.to_string(),
            origin: origin.to_string(),
            domain,
            scope,
            payload: json!({"k": 1}),
            at: 1730000000000,
        }
    }

    // 必测 1：域校验拒绝（跨域发布）
    #[test]
    fn cross_domain_publish_rejected() {
        let bus = EventBus::new();
        // 插件 "hello" 发布域名 "home" 的事件 → 拒绝
        let err = bus
            .publish(&"hello".into(), ev("hello", "home:todo.updated", EventScope::Domain))
            .unwrap_err();
        assert!(matches!(err, KernelError::PermissionDenied(_)), "实际: {err:?}");

        // 事件名与 domain 字段不一致 → 拒绝
        let mut bad = ev("hello", "hello:ping", EventScope::Live);
        bad.domain = "other".into();
        let err = bus.publish(&"hello".into(), bad).unwrap_err();
        assert!(matches!(err, KernelError::EventBus(_)), "实际: {err:?}");
    }

    // 必测 2：白名单通配（hello:* 收 hello:ping）
    #[test]
    fn wildcard_subscription() {
        let bus = EventBus::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let h2 = Arc::clone(&hits);
        bus.subscribe(&"front".into(), "hello:*", move |_e| {
            h2.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();

        bus.publish(&"hello".into(), ev("hello", "hello:ping", EventScope::Live)).unwrap();
        bus.publish(&"hello".into(), ev("hello", "hello:pong", EventScope::Live)).unwrap();
        // 非本域不触发
        bus.publish(&"home".into(), ev("home", "home:todo.updated", EventScope::Live)).unwrap();

        assert_eq!(hits.load(Ordering::SeqCst), 2);
    }

    // 必测 3：退订幂等
    #[test]
    fn unsubscribe_idempotent() {
        let bus = EventBus::new();
        let hits = Arc::new(AtomicUsize::new(0));
        let h2 = Arc::clone(&hits);
        let id = bus
            .subscribe(&"front".into(), "hello:*", move |_e| {
                h2.fetch_add(1, Ordering::SeqCst);
            })
            .unwrap();

        bus.unsubscribe(id).unwrap();
        // 二次退订 → 不 Err（幂等）
        bus.unsubscribe(id).unwrap();

        bus.publish(&"hello".into(), ev("hello", "hello:ping", EventScope::Live)).unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 0);
        assert_eq!(bus.sub_count(), 0);
    }

    // 必测 4：拦截 veto（能力事件）
    #[test]
    fn interceptor_veto() {
        let bus = EventBus::new();
        let called = Arc::new(AtomicUsize::new(0));
        let c2 = Arc::clone(&called);

        bus.intercept(
            "hello",
            Arc::new(move |_e| {
                c2.fetch_add(1, Ordering::SeqCst);
                Err(KernelError::PermissionDenied("veto".into()))
            }),
        )
        .unwrap();

        let err = bus
            .publish(&"hello".into(), ev("hello", "hello:cap", EventScope::Capability))
            .unwrap_err();
        assert!(matches!(err, KernelError::PermissionDenied(_)), "{err:?}");
        assert_eq!(called.load(Ordering::SeqCst), 1, "拦截器应已执行");
    }

    // 必测 5：match_pattern 边界（防前缀误匹配）
    #[test]
    fn pattern_boundaries() {
        assert!(EventBus::matches("hello:*", "hello:ping"));
        assert!(!EventBus::matches("hello:*", "hellox:ping"));
        assert!(EventBus::matches("*", "any:thing"));
        assert!(EventBus::matches("hello:ping", "hello:ping"));
        assert!(!EventBus::matches("hello:ping", "hello:pong"));
    }
}
