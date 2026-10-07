use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "customs.systemtools".into(),
        name: "系统工具".into(),
        level: PluginLevel::Custom,
        parent: None,
        slot: None,
        version: "0.2.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 批次4c S1：system_config 表归属。
            // 批次6a S1：perf_metrics / slow_query_log / custom_themes 归属判定为 kernel
            //（db_归属判定表 §4）。backup_records 归属判定为 customs.systemtools。
            // 2026-09-26：mek_versions / mek_rotation_log 原误判为 customs.systemtools，
            // 实测二表仅由 customs.auth 读写（crypto/mek_manager.rs、mek_rotation_scheduler.rs），
            // 且 customs.auth 的 0002 基线迁移已登记归属，故本插件撤销声明（避免归属冲突）。
            // 注：perf_metrics / slow_query_log / custom_themes 表结构由生产迁移创建，
            // 在 kernel_table_ownership 中归属为 kernel，此处不重复声明。
            // backup_records 表存在于单独的 backup_records.db（非主库），kernel_table_ownership
            // 仅跟踪主数据库 nexterm.db 的表，此处不声明。
            db: vec!["system_config".into()],
            // 本批不发布/订阅领域事件。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 10 条 system IPC 别名 + 24 条 L3 systemtools IPC 别名 + 0 条 extension/adapter（裁定删除）。
            ipc: vec!["st_*".into()],
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "systemtools".into(),
    })
}
