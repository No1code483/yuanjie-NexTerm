//! ai.sessions 插件清单（batchC1；L2 Feature，parent boards.ai）。
//!
//! 覆盖面：会话面 10 条命令（会话 CRUD 6 + 参与者 1 + 列表/搜索/导出/标记已读/排序/星标/分支）
//! + 4 张会话表归属登记（conversations / conversation_participants / messages / prompt_templates）。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "ai.sessions".into(),
        name: "会话列表".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.ai".into()),
        slot: Some("ai.sessions".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 4 张会话表归标本插件（BUG-033 修订 2026-09-27：归属经本插件
            // migrations/0002_transfer_ownership.sql 自 boards.ai 转移而来；
            // L1 boards.ai manifest 已不再声明这 4 张表）。
            db: vec![
                "conversations".into(),
                "conversation_participants".into(),
                "messages".into(),
                "prompt_templates".into(),
            ],
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // ACL/handler 键由内核按本插件短码 `ss` 构造。
            ipc: vec!["ss_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "ai.sessions".into(),
    })
}
