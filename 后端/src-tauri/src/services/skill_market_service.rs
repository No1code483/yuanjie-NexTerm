//! D1.8 Skill 市场（本地预置技能目录）
//!
//! 设计依据：
//!   - 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §v3.2 Skill 市场
//!   - v2_下一阶段开发计划 D1.8：可浏览/安装/卸载/调用 Skill
//!
//! 提供应用内置的技能市场目录：用户可浏览、搜索、安装（注册到运行时内存）。
//! 市场技能安装后会带完整元数据（description + policy + trigger_patterns），
//! 可被 trigger_skill 调用、被 detect_implicit 隐式触发。
//!
//! v3.2 升级（Phase 5 Task 3.3.1 / 3.3.2）：
//!   - 市场目录改为从 `BuiltinSkillRegistry::list_metadata()` 动态生成，
//!     与 `skills/builtin/` 下 10 个可执行 Skill 一一对应；
//!   - 每个 market 条目背后都有真实 `execute()` 实现（调用云端 API）。
//!
//! 注：市场为本地预置目录（无远程网络请求），安装 = 注册到内存 SkillsManager。

use serde::{Deserialize, Serialize};

use crate::models::skill::{SkillMetadata, SkillPolicy, SkillScope};
use crate::skills::builtin::BuiltinSkillRegistry;

/// 市场技能条目（含市场展示元数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMarketEntry {
    pub name: String,
    pub description: String,
    pub short_description: String,
    /// 市场分类：编程 / 测试 / 安全 / 文档 / 性能 / 架构 / 调试 / 国际化
    pub category: String,
    pub author: String,
    pub version: String,
    /// 来源：builtin（内置）/ community
    pub source: String,
    pub trigger_patterns: Vec<String>,
    pub file_patterns: Vec<String>,
    pub priority: i32,
    /// 是否已安装（由调用方根据已加载技能名列表填充）
    #[serde(default)]
    pub installed: bool,
}

/// 市场目录 — 从 BuiltinSkillRegistry 动态生成（10 个可执行内置 Skill）
///
/// 与 `skills/builtin/mod.rs` 的 `BuiltinSkillRegistry::new()` 保持同步：
/// 每个市场条目背后都有对应的 `execute()` 实现（调用云端 API）。
fn catalog() -> Vec<SkillMarketEntry> {
    let registry = BuiltinSkillRegistry::new();
    registry
        .list_metadata()
        .into_iter()
        .map(|info| SkillMarketEntry {
            name: info.name,
            description: info.description,
            short_description: info.short_description,
            category: info.category,
            author: "NexTerm".into(),
            version: "1.0.0".into(),
            source: "builtin".into(),
            trigger_patterns: info.trigger_patterns,
            file_patterns: info.file_patterns,
            priority: info.priority,
            installed: false,
        })
        .collect()
}

/// 列出市场技能（标记已安装状态）
pub fn list_market(installed_names: &[String]) -> Vec<SkillMarketEntry> {
    catalog()
        .into_iter()
        .map(|mut e| {
            e.installed = installed_names.iter().any(|n| n == &e.name);
            e
        })
        .collect()
}

/// 搜索市场技能（匹配名称/描述/分类/触发词）
pub fn search_market(query: &str, installed_names: &[String]) -> Vec<SkillMarketEntry> {
    let q = query.to_lowercase();
    catalog()
        .into_iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&q)
                || e.description.to_lowercase().contains(&q)
                || e.short_description.to_lowercase().contains(&q)
                || e.category.to_lowercase().contains(&q)
                || e.trigger_patterns.iter().any(|p| p.to_lowercase().contains(&q))
        })
        .map(|mut e| {
            e.installed = installed_names.iter().any(|n| n == &e.name);
            e
        })
        .collect()
}

/// 获取单个市场技能
pub fn get_entry(name: &str, installed_names: &[String]) -> Option<SkillMarketEntry> {
    catalog()
        .into_iter()
        .find(|e| e.name == name)
        .map(|mut e| {
            e.installed = installed_names.iter().any(|n| n == &e.name);
            e
        })
}

/// 市场技能转运行时元数据（安装时使用，含完整 policy）
pub fn to_metadata(entry: &SkillMarketEntry) -> SkillMetadata {
    SkillMetadata {
        name: entry.name.clone(),
        description: entry.description.clone(),
        short_description: Some(entry.short_description.clone()),
        interface: None,
        dependencies: None,
        policy: Some(SkillPolicy {
            allow_implicit_invocation: Some(true),
            auto_discover: Some(true),
            trigger_patterns: Some(entry.trigger_patterns.clone()),
            file_patterns: if entry.file_patterns.is_empty() {
                None
            } else {
                Some(entry.file_patterns.clone())
            },
            priority: Some(entry.priority),
        }),
        path: format!("[market]/{}", entry.name),
        scope: SkillScope::System,
        plugin_id: None,
        enabled: true,
        file_size: 0,
        created_at: None,
    }
}
