//! 接口层：跨插件共享的类型与 trait。
//! 规则：本 crate 禁止依赖任何插件；禁止包含业务逻辑。
//!
//! 契约来源：06_Rust代码契约.md §二
//! 阶段1 落地决策（已记入 06 修订记录）：
//! 1. KernelError::Db 采用 `Db(String)` 字符串变体（保持本 crate 零 sqlx 依赖）
//! 2. Manifest/SlotSpec 增加 `#[serde(rename_all = "camelCase")]`
//!    （序列化字段名与 07 TS 契约及阶段1 B8 manifest JSON 对齐：kernelApi/i18nNamespace/routePrefix）

use serde::{Deserialize, Serialize};

pub type PluginId = String;

/// 插件三级分类（L1/L2/L3）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginLevel {
    /// L1 板块级插件
    Board,
    /// L2 板块内功能插件
    Feature,
    /// L3 定制插件
    Custom,
}

/// 插件清单（与前端 manifest.ts 字段严格对齐）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: PluginId,
    pub name: String,
    pub level: PluginLevel,
    /// L2/L3 必填；L1 填 None
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PluginId>,
    /// L2 必填：安装到 parent 的哪个插槽
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    /// semver 三段
    pub version: String,
    /// 兼容的 Kernel API 大版本，如 "1"
    pub kernel_api: String,
    pub permissions: Permissions,
    /// L1 声明的嵌套插槽列表；其他级别为空
    #[serde(default)]
    pub slots: Vec<SlotSpec>,
    pub i18n_namespace: String,
}

/// 插件依赖
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub id: PluginId,
    pub min: String,
}

/// 六类权限声明（未声明即拒绝）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    /// 可读写的表（含登记的旧表）
    #[serde(default)]
    pub db: Vec<String>,
    #[serde(default)]
    pub events: EventPerms,
    /// 可被前端调用的 IPC 命令前缀，如 ["hm_todo_*"]
    #[serde(default)]
    pub ipc: Vec<String>,
    #[serde(default)]
    pub fs: Vec<String>,
    #[serde(default)]
    pub net: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EventPerms {
    /// 可订阅的事件域，支持通配如 ["home:*"]
    #[serde(default)]
    pub subscribe: Vec<String>,
    /// 可发布的事件域（通配只允许出现在自己域内）
    #[serde(default)]
    pub publish: Vec<String>,
}

/// L1 插件声明的插槽规格
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotSpec {
    pub id: String,
    /// ui-route / panel / settings-card / command-palette / service
    pub r#type: String,
    pub description: String,
    pub capacity: u32,
    #[serde(default)]
    pub route_prefix: Option<String>,
}

/// 事件（三域统一载荷信封）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// 完整事件名 "<域名>:<对象>.<动作>"，如 "home:todo.updated"
    pub name: String,
    /// 发布者插件 id（总线校验用）
    pub origin: PluginId,
    /// 事件域：name 冒号前部分
    pub domain: String,
    /// 领域事件 / 实时事件 / 能力事件（字符串避免类型耦合）
    pub scope: EventScope,
    /// JSON 载荷（只放摘要：id/类型/少量字段）
    pub payload: serde_json::Value,
    /// 毫秒时间戳
    pub at: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventScope {
    /// 领域事件（落库）
    Domain,
    /// 实时插件事件（不落库）
    Live,
    /// 能力策略事件（拦截链）
    Capability,
}

/// 跨插件服务 trait 示例：删除/恢复契约（回收站依赖）
/// 各业务插件迁移时实现此 trait 并注册。
#[async_trait::async_trait]
pub trait RecyclableService: Send + Sync {
    /// 恢复一条软删除记录；返回是否成功
    async fn restore(&self, item_type: &str, item_id: i64) -> Result<bool, KernelError>;
    /// 彻底删除一条软删除记录
    async fn purge(&self, item_type: &str, item_id: i64) -> Result<bool, KernelError>;
}

/// 跨插件加密基础设施接口：MEK（用户主密钥）会话能力契约。
///
/// 契约依据：`02_核心机制设计/02_后端插件架构设计.md` §七 改造清单
/// 「`crypto/`（用户密钥）→ auth 插件私有 + kernel 仅留加密基础设施接口」。
///
/// 约束：
/// 1. 本 crate 禁止依赖任何插件，故此处只声明能力；实现由 `customs.auth` 提供，
///    装配期经内核门面（`AppState.mek_manager`）注入。
/// 2. 并发串行化由外层 `Arc<RwLock<..>>` 承担，实现内部不设锁，
///    因此读方法取 `&self`、写方法取 `&mut self`。
pub trait MekProvider: Send + Sync {
    /// 缓存用户 MEK（登录 / 注册 / 助记词找回成功后调用）
    fn cache_mek(&mut self, user_id: i64, mek: [u8; 32]);

    /// 读取缓存中的用户 MEK；该用户无会话密钥时返回 `None`
    fn get_mek(&self, user_id: i64) -> Option<&[u8; 32]>;

    /// 清除单个用户的 MEK 缓存（登出 / 改密时调用）
    fn clear_mek(&mut self, user_id: i64);

    /// 清除全部 MEK 缓存（全局登出时调用）
    fn clear_all(&mut self);

    /// 用新 KEK 重新加密 MEK（密钥轮换时重建密文），返回 `(密文, nonce)`
    fn re_encrypt_mek(
        &self,
        mek: &[u8; 32],
        new_kek: &[u8; 32],
    ) -> Result<(Vec<u8>, [u8; 12]), String>;
}

/// 统一内核错误
#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("插件 {0} 未启用")]
    PluginDisabled(String),
    #[error("权限不足：{0}")]
    PermissionDenied(String),
    #[error("数据库错误：{0}")]
    Db(String),
    #[error("事件总线错误：{0}")]
    EventBus(String),
    #[error("插件初始化失败 {plugin}: {reason}")]
    PluginInit { plugin: String, reason: String },
    #[error("配置错误：{0}")]
    Config(String),
}
