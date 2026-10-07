//! terminal.linux 插件清单（阶段3 批次3b；L2 Feature，parent boards.terminal）。
//!
//! 覆盖面：linux_commands 全量 25 条（linux_* 19 + docker_* 5 + network_stats——
//! 环境/内核源码查询、性能基准、Docker 容器管理、网络统计）。**前端零消费**
//!（唯一消费者 components/Linux.tsx 为死文件，裁定 T9 随批判删；实际 linux 终端
//! 由 Terminal 页 Tab 内 LinuxTerminal.tsx 承载，数据来自前端 linuxShell.ts 模拟层）
//! → client 不提供便捷方法（43-A「有 alias 无前端方法」口径），仅登记命名空间。
//! 无业务表（linux_service 为文件系统/系统查询/docker CLI，不落库）、无领域事件。
//! Manifest 含 String/Vec 字段无法作 const，用 OnceLock 懒初始化产生 &'static Manifest。

use std::sync::OnceLock;

use kernel_api::{EventPerms, Manifest, Permissions, PluginLevel};

static MANIFEST: OnceLock<Manifest> = OnceLock::new();

pub fn manifest() -> &'static Manifest {
    MANIFEST.get_or_init(|| Manifest {
        id: "terminal.linux".into(),
        name: "Linux 子系统".into(),
        level: PluginLevel::Feature,
        parent: Some("boards.terminal".into()),
        slot: Some("terminal.linux".into()),
        version: "0.1.0".into(),
        kernel_api: "1".into(),
        permissions: Permissions {
            // 不持有业务表（linux/docker 命令为系统查询/文件系统/docker CLI，无落库）。
            db: vec![],
            // 不发布/订阅内核领域事件（无 emit 点，S5 零改动实测）。
            events: EventPerms {
                subscribe: vec![],
                publish: vec![],
            },
            // 25 条 IPC；ACL/handler 键由内核按短码 `lx` 构造（<短码>_<命令名>）。
            ipc: vec!["lx_*".into()],
            // linux_service 访问内核源码目录 / docker CLI（fs 面在段级 S1 实测为
            // 进程级系统访问，不走 manifest fs 白名单；登记说明）。
            fs: vec![],
            net: vec![],
        },
        slots: vec![],
        i18n_namespace: "terminal.linux".into(),
    })
}
