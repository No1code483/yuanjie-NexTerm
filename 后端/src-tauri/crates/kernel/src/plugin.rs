//! 插件 trait 与内核上下文（契约：06_Rust代码契约.md §三）
//!
//! 实现偏差（已记入 06 修订记录）：
//! - KernelCtx 改为持有 `Arc` 克隆（EventBus 内部锁化后无需 &mut）；
//! - Registry 需要 per-plugin 的 Registrar 收集，由 registry 自行构造 ctx。

use kernel_api::{Manifest, PluginId};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::db::DbHandle;
use crate::error::KernelError;
use crate::event_bus::EventBus;
use crate::security::SecurityGate;

/// 迁移文件（插件内静态声明）
#[derive(Debug, Clone, Copy)]
pub struct MigrationFile {
    pub version: i64,
    pub sql: &'static str,
}

/// 插件注册物句柄集合（可逆注册的凭据）
#[derive(Default)]
pub struct PluginHandle {
    /// 事件订阅 id
    pub subscriptions: Vec<u64>,
    /// 已注册服务 trait 名
    pub services: Vec<&'static str>,
    pub migrations_ns: Option<String>,
}

/// 插件状态机
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    Registered,
    Installed,
    Enabled,
    Disabled,
    Error,
}

impl PluginState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PluginState::Registered => "registered",
            PluginState::Installed => "installed",
            PluginState::Enabled => "enabled",
            PluginState::Disabled => "disabled",
            PluginState::Error => "error",
        }
    }
}

/// 注册收集器（init 期间插件通过它登记注册物）
#[derive(Default)]
pub struct Registrar {
    pub subscriptions: Vec<u64>,
    pub services: Vec<(&'static str, Box<dyn Any + Send + Sync>)>,
    pub migrations: Option<(String, Vec<MigrationFile>)>,
    /// 统计用
    pub commands_registered: u32,
}

/// 内核向插件暴露的受控能力（对标 dsh 的 ctx）
pub struct KernelCtx {
    pub plugin_id: PluginId,
    pub db: Arc<DbHandle>,
    pub events: Arc<EventBus>,
    pub security: Arc<SecurityGate>,
    pub register: Registrar,
}

/// 插件 trait（每个插件实现一次）
#[async_trait::async_trait]
pub trait Plugin: Send + Sync + 'static {
    /// manifest 静态数据（插件内 const 定义）
    fn manifest(&self) -> &'static Manifest;

    /// 初始化：注册 commands/services/listeners/migrations
    async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError>;

    /// 停用清理（默认空实现；内核会按 handle 逆向回滚）
    async fn shutdown(&self, _ctx: &mut KernelCtx, _handle: PluginHandle) -> Result<(), KernelError> {
        Ok(())
    }
}

/// 启动报告项（kernel:startup_report 返回结构）
#[derive(Debug, Clone, serde::Serialize)]
pub struct StartupReportItem {
    pub plugin: PluginId,
    /// 状态字符串（"enabled"/"error"/...）
    pub state: String,
    pub note: Option<String>,
}

/// 服务注册表（trait 名 -> 实例；跨插件服务发现，阶段1 最小实现）
#[derive(Default)]
pub struct ServiceRegistry {
    services: HashMap<&'static str, Arc<dyn Any + Send + Sync>>,
}

impl ServiceRegistry {
    pub fn register(&mut self, name: &'static str, instance: Arc<dyn Any + Send + Sync>) {
        self.services.insert(name, instance);
    }

    pub fn unregister(&mut self, name: &str) {
        self.services.remove(name);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<dyn Any + Send + Sync>> {
        self.services.get(name)
    }
}
