//! ai.models 插件清单（阶段3 批次2b-1；L2 Feature，parent boards.ai）。
//!
//! 覆盖面：ai_models 表（4 条 CRUD + provider info + 3 条健康检测命令），共 8 条命令。
//! 健康检测命令经 `chat_service::check_model_health` 复用单模型检测逻辑
//! （chat_service 属批次 2b-2，本批暂跨引用，登记 2b-2 收口）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "ai.models".into(),
        name: "模型管理".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.ai".into()),
        slot: Some("ai.models".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // ai_models 为既有旧表（短码 `am` ≠ 旧名前缀 `ai_` → name_prefixed=0），
            // 归属登记见 migrations/0001_baseline.sql。
            db: vec!["ai_models".into()],
            // 本批不发布/订阅领域事件；`ai-model-health-changed` 为 Tauri emit，非内核总线事件。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
            ipc: vec!["am_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "ai.models".into(),
    })
}