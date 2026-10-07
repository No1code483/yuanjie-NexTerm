//! 内核 Tauri 插件（契约：06_Rust代码契约.md §八 B7 落地决策；阶段1 B7）
//!
//! 职责：
//! 1. 启动期：连接 nexterm.db（与主应用同库）→ 建内核表 → 归属缓存加载
//!    → EventBus（Domain 事件落库器 + 前端 k://event 全量桥接）
//! 2. IPC：唯一传输命令 `kernel_dispatch`——cmd 承载契约逻辑名
//!    `<正式插件ID或短码>:plugin:<命令名>`；属主解析到正式 ID 后，以登记短码生成
//!    `<短码>_<命令名>` 白名单/handler 键，统一 check_ipc + 停用即拒
//! 3. 懒激活：首个 IPC 前完成 registry.bootstrap + activate_all（OnceCell 闸门，仅跑一次）
//!
//! 内核保留命令（owner == "kernel"，无 manifest，可信直查、免 check_ipc）：
//! - `kernel:plugin:get_enabled` -> Vec<PluginInfo>（仅启用，用于应用装配）
//! - `kernel:plugin:list`        -> Vec<PluginInfo>（全部已注册，用于插件管理）
//! - `kernel:plugin:set_enabled` -> (id, enabled) -> null
//! - `kernel:startup_report`     -> Vec<StartupReportItem>

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value as Json;
use tauri::{Emitter, Manager, Runtime};

use kernel_api::{Event, KernelError, PluginId, PluginLevel};

use crate::db::DbHandle;
use crate::event_bus::EventBus;
use crate::plugin::{PluginState, StartupReportItem};
use crate::registry::PluginRegistry;
use crate::registry::REQUIRED_PLUGINS;
use crate::security::SecurityGate;

/// 统一命令闭包签名：Json 进、异步 Json/String 出（契约逻辑名 → 处理闭包）
pub type CommandHandler =
    Arc<dyn Fn(Json) -> Pin<Box<dyn Future<Output = Result<Json, String>> + Send>> + Send + Sync>;

/// kernel:plugin:get_enabled 返回项
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: PluginId,
    pub name: String,
    pub level: PluginLevel,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<PluginId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    /// "registered"/"enabled"/"error"/...（PluginState::as_str）
    pub state: String,
    /// 必备插件（REQUIRED_PLUGINS 名单，不可停用；手稿 20260926）
    pub required: bool,
}

/// 内核全局状态（main app.manage 持有）
pub struct KernelState {
    pub registry: Arc<tokio::sync::Mutex<PluginRegistry>>,
    pub db: Arc<DbHandle>,
    pub bus: Arc<EventBus>,
    pub security: Arc<SecurityGate>,
    /// 启动报告快照（kernel:startup_report 消费）
    pub startup_report: RwLock<Vec<StartupReportItem>>,
    /// 命令表：规范化键（如 "_hello_hello_ping"）→ 处理闭包
    pub commands: RwLock<HashMap<String, CommandHandler>>,
    /// 懒激活闸门（仅跑一次）
    activated: tokio::sync::OnceCell<()>,
}

impl KernelState {
    /// 停用即拒：插件状态必须是 Enabled 才放行（kernel_dispatch 入口统一调用）
    pub async fn ensure_plugin_enabled(&self, id: &str) -> Result<(), KernelError> {
        let reg = self.registry.lock().await;
        match reg.state_of(id) {
            Some(PluginState::Enabled) => Ok(()),
            _ => Err(KernelError::PluginDisabled(id.to_string())),
        }
    }

    /// 懒激活闸门：首个 kernel_dispatch 前完成 bootstrap + activate_all（幂等，仅一次）。
    /// 失败不阻塞 dispatch（错误记录于日志与 startup_report，各命令按自身状态拒绝）。
    async fn ensure_activated(&self) {
        self.activated
            .get_or_init(|| async {
                let mut reg = self.registry.lock().await;
                if let Err(e) = reg.bootstrap(&self.db).await {
                    eprintln!("[kernel] registry bootstrap 失败: {e}");
                }
                match reg
                    .activate_all(self.db.clone(), self.bus.clone(), self.security.clone())
                    .await
                {
                    Ok(report) => *self.startup_report.write().unwrap() = report,
                    Err(e) => eprintln!("[kernel] activate_all 整体失败: {e}"),
                }
            })
            .await;
    }
}

/// 插件 IPC 命令登记入口（插件 init 的 tauri setup 内调用，契约 §八模板）
///
/// `cmd` 为契约逻辑名 `<plugin_id>:plugin:<命令名>`（如 "_hello:plugin:hello_ping"），
/// 内部规范化为白名单键 `<plugin_id>_<命令名>`（":plugin:" → "_"），
/// 与 kernel_dispatch 的查找键、check_ipc 白名单键一致。
pub fn register_command<F, Fut>(state: &KernelState, cmd: &str, handler: F)
where
    F: Fn(Json) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<Json, String>> + Send + 'static,
{
    let key = cmd.replace(":plugin:", "_");
    let wrapped: CommandHandler = Arc::new(
        move |args: Json| -> Pin<Box<dyn Future<Output = Result<Json, String>> + Send>> {
            Box::pin(handler(args))
        },
    );
    state.commands.write().unwrap().insert(key, wrapped);
}

fn resolve_plugin_command_key(
    security: &SecurityGate,
    manifest: &kernel_api::Manifest,
    incoming_key: String,
    command_name: &str,
) -> Result<String, String> {
    if security.check_ipc(manifest, &incoming_key).is_ok() {
        return Ok(incoming_key);
    }

    let short_key = format!(
        "{}_{}",
        SecurityGate::short_code(&manifest.id),
        command_name
    );
    security
        .check_ipc(manifest, &short_key)
        .map_err(|e| e.to_string())?;
    Ok(short_key)
}

/// 内核唯一 IPC 入口（B7 落地决策：generate_handler! 无法承载逻辑命名，统一分发）
#[tauri::command]
pub async fn kernel_dispatch(
    state: tauri::State<'_, KernelState>,
    cmd: String,
    args: Json,
) -> Result<Json, String> {
    // ① 懒激活（幂等；失败记录于日志与 startup_report）
    state.ensure_activated().await;
    dispatch_activated(&state, cmd, args).await
}

async fn dispatch_activated(
    state: &KernelState,
    cmd: String,
    args: Json,
) -> Result<Json, String> {
    // ② 属主提示解析；插件命令的 ACL/handler 键在 manifest 解析后按正式短码生成
    let (owner_hint, command_name, kernel_key) = match cmd.find(":plugin:") {
        Some(pos) => (
            cmd[..pos].to_string(),
            Some(cmd[pos + ":plugin:".len()..].to_string()),
            Some(cmd.replace(":plugin:", "_")),
        ),
        None => match cmd.split_once(':') {
            // 内核保留命令（如 kernel:startup_report）无 ":plugin:" 段
            Some((o, _)) => (o.to_string(), None, Some(cmd.clone())),
            None => {
                return Err(format!(
                    "cmd 格式非法（应为 <正式插件ID或短码>:plugin:<命令名>）: {cmd}"
                ))
            }
        },
    };

    // ③ 内核保留命令（无 manifest，可信直查、免 check_ipc）
    if owner_hint == "kernel" {
        return match kernel_key.as_deref().unwrap_or_default() {
            "kernel_get_enabled" => kernel_get_enabled(&state).await,
            "kernel_list" => kernel_list_plugins(&state).await,
            "kernel_set_enabled" => kernel_set_enabled(&state, args).await,
            "kernel:startup_report" => {
                let report = state.startup_report.read().unwrap().clone();
                serde_json::to_value(&report).map_err(|e| e.to_string())
            }
            _ => Err(format!("未知内核命令: {cmd}")),
        };
    }

    // ④ 插件命令：短码/正式 ID 均先解析到正式 manifest，再以登记短码构造 ACL 与 handler 键
    let command_name = command_name
        .ok_or_else(|| format!("cmd 格式非法（应为 <正式插件ID或短码>:plugin:<命令名>）: {cmd}"))?;
    if command_name.is_empty() {
        return Err(format!("cmd 命令名不能为空: {cmd}"));
    }
    let manifest = {
        let reg = state.registry.lock().await;
        reg.manifest_of_ipc_owner(&owner_hint)
    }
    .ok_or_else(|| format!("命令 {cmd} 的属主插件 {owner_hint} 未注册"))?;
    let incoming_key = kernel_key.expect("插件命令已生成规范化键");
    let key = resolve_plugin_command_key(&state.security, &manifest, incoming_key, &command_name)?;
    state
        .ensure_plugin_enabled(&manifest.id)
        .await
        .map_err(|e| e.to_string())?;

    // ⑤ 路由到插件登记的命令闭包（不持 registry 锁，防死锁）
    let handler = state
        .commands
        .read()
        .unwrap()
        .get(&key)
        .cloned()
        .ok_or_else(|| format!("命令 {cmd} 未登记（插件 init 未 register_command）"))?;
    handler(args).await
}

/// kernel:plugin:get_enabled -> Vec<PluginInfo>
async fn kernel_get_enabled(state: &KernelState) -> Result<Json, String> {
    let reg = state.registry.lock().await;
    let infos: Vec<PluginInfo> = reg
        .plugin_ids()
        .into_iter()
        .filter_map(|id| {
            let m = reg.manifest_of(&id)?;
            let st = reg.state_of(&id)?;
            if st != PluginState::Enabled {
                return None;
            }
            Some(plugin_info(id, m, st))
        })
        .collect();
    serde_json::to_value(infos).map_err(|e| e.to_string())
}

/// kernel:plugin:list -> Vec<PluginInfo>（包含 disabled/error 等全部已注册状态）
async fn kernel_list_plugins(state: &KernelState) -> Result<Json, String> {
    let reg = state.registry.lock().await;
    let infos: Vec<PluginInfo> = reg
        .plugin_ids()
        .into_iter()
        .filter_map(|id| {
            let m = reg.manifest_of(&id)?;
            let st = reg.state_of(&id)?;
            Some(plugin_info(id, m, st))
        })
        .collect();
    serde_json::to_value(infos).map_err(|e| e.to_string())
}

fn plugin_info(id: PluginId, manifest: kernel_api::Manifest, state: PluginState) -> PluginInfo {
    PluginInfo {
        required: REQUIRED_PLUGINS.contains(&id.as_str()),
        id,
        name: manifest.name,
        level: manifest.level,
        version: manifest.version,
        parent: manifest.parent,
        slot: manifest.slot,
        state: state.as_str().to_string(),
    }
}

/// kernel:plugin:set_enabled -> (id, enabled) -> null
async fn kernel_set_enabled(state: &KernelState, args: Json) -> Result<Json, String> {
    let id = args
        .get("id")
        .and_then(Json::as_str)
        .ok_or("缺少参数 id")?
        .to_string();
    let enabled = args
        .get("enabled")
        .and_then(Json::as_bool)
        .ok_or("缺少参数 enabled")?;
    let mut reg = state.registry.lock().await;
    reg.set_enabled(
        state.db.clone(),
        state.bus.clone(),
        state.security.clone(),
        &id,
        enabled,
    )
    .await
    .map_err(|e| e.to_string())?;
    drop(reg);
    Ok(Json::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{KernelCtx, Plugin, PluginHandle};
    use kernel_api::{EventPerms, Manifest, Permissions};

    struct ListingPlugin {
        manifest: &'static Manifest,
    }

    #[async_trait::async_trait]
    impl Plugin for ListingPlugin {
        fn manifest(&self) -> &'static Manifest {
            self.manifest
        }

        async fn init(&self, _ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
            Ok(PluginHandle::default())
        }
    }

    fn manifest(id: &str, ipc: &str) -> Manifest {
        Manifest {
            id: id.into(),
            name: "测试插件".into(),
            level: PluginLevel::Board,
            parent: None,
            slot: None,
            version: "0.1.0".into(),
            kernel_api: "1".into(),
            permissions: Permissions {
                events: EventPerms::default(),
                ipc: vec![ipc.into()],
                ..Default::default()
            },
            slots: vec![],
            i18n_namespace: id.into(),
        }
    }

    #[test]
    fn formal_profile_id_normalizes_to_short_code_acl_key() {
        let key = resolve_plugin_command_key(
            &SecurityGate::new(),
            &manifest("boards.profile", "pf_*"),
            "boards.profile_get_profile".into(),
            "get_profile",
        )
        .unwrap();

        assert_eq!(key, "pf_get_profile");
    }

    #[test]
    fn registered_owner_key_remains_unchanged() {
        let key = resolve_plugin_command_key(
            &SecurityGate::new(),
            &manifest("_hello", "_hello_*"),
            "_hello_hello_ping".into(),
            "hello_ping",
        )
        .unwrap();

        assert_eq!(key, "_hello_hello_ping");
    }

    #[tokio::test]
    async fn management_list_includes_disabled_plugins_but_enabled_list_does_not() {
        let mut registry = PluginRegistry::new();
        registry.register(Box::new(ListingPlugin {
            manifest: Box::leak(Box::new(manifest("_hello", "_hello_*"))),
        }));
        let pool = sqlx::sqlite::SqlitePool::connect("sqlite::memory:")
            .await
            .unwrap();
        let db = Arc::new(DbHandle::new(pool));
        let bus = Arc::new(EventBus::new());
        let security = Arc::new(SecurityGate::new());
        registry.bootstrap(&db).await.unwrap();
        registry
            .activate_all(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&security))
            .await
            .unwrap();
        registry
            .set_enabled(
                Arc::clone(&db),
                Arc::clone(&bus),
                Arc::clone(&security),
                "_hello",
                false,
            )
            .await
            .unwrap();
        let state = KernelState {
            registry: Arc::new(tokio::sync::Mutex::new(registry)),
            db,
            bus,
            security,
            startup_report: RwLock::new(vec![]),
            commands: RwLock::new(HashMap::new()),
            activated: tokio::sync::OnceCell::new(),
        };

        assert_eq!(kernel_get_enabled(&state).await.unwrap(), serde_json::json!([]));
        assert_eq!(
            kernel_list_plugins(&state).await.unwrap(),
            serde_json::json!([{
                "id": "_hello",
                "name": "测试插件",
                "level": "board",
                "version": "0.1.0",
                "state": "disabled",
                "required": false
            }])
        );
        assert_eq!("kernel:plugin:list".replace(":plugin:", "_"), "kernel_list");

        let handler_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called = Arc::clone(&handler_called);
        state.commands.write().unwrap().insert(
            "_hello_hello_ping".into(),
            Arc::new(move |_| {
                called.store(true, std::sync::atomic::Ordering::SeqCst);
                Box::pin(async { Ok(serde_json::json!("pong:_hello")) })
            }),
        );

        let error = dispatch_activated(
            &state,
            "_hello:plugin:hello_ping".into(),
            serde_json::json!({}),
        )
        .await
        .unwrap_err();
        assert_eq!(error, "插件 _hello 未启用");
        assert!(!handler_called.load(std::sync::atomic::Ordering::SeqCst));
    }
}

/// 内核 Tauri 插件（main.rs 装配最前：`.plugin(kernel::tauri_glue::init())`）
pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("kernel")
        .setup(|app, _api| {
            // 1. 数据目录（与主应用共用 nexterm.db）
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db_path = dir.join("nexterm.db");

            // 2. 建池（WAL + busy_timeout；setup 为同步闭包，block_on 完成异步建池）
            let opts = sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&db_path)
                .create_if_missing(true)
                .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
                .busy_timeout(Duration::from_secs(5));
            let pool = tauri::async_runtime::block_on(async {
                sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(20)
                    .connect_with(opts)
                    .await
            })?;

            // 3. DbHandle + 内核表（plugin_registry 兜底同建；plugin_migrations 为
            //    B5 迁移编排器写入目标；kernel_domain_events 按 06 §五 独占写入）
            let db = Arc::new(DbHandle::with_path(pool, db_path));
            tauri::async_runtime::block_on(async {
                for ddl in [
                    "CREATE TABLE IF NOT EXISTS plugin_registry (\
                         id TEXT PRIMARY KEY, version TEXT NOT NULL, level TEXT NOT NULL,\
                         parent TEXT, enabled INTEGER NOT NULL DEFAULT 1,\
                         installed_at TEXT NOT NULL, updated_at TEXT NOT NULL,\
                         state_note TEXT, db_namespace TEXT)",
                    "CREATE TABLE IF NOT EXISTS kernel_table_ownership (\
                         table_name TEXT PRIMARY KEY, plugin_id TEXT NOT NULL,\
                         name_prefixed INTEGER NOT NULL, created_at TEXT NOT NULL)",
                    "CREATE TABLE IF NOT EXISTS plugin_migrations (\
                         namespace TEXT NOT NULL, version INTEGER NOT NULL,\
                         applied_at TEXT NOT NULL, PRIMARY KEY (namespace, version))",
                    "CREATE TABLE IF NOT EXISTS kernel_domain_events (\
                         id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL,\
                         origin TEXT NOT NULL, payload TEXT NOT NULL, at INTEGER NOT NULL)",
                    "CREATE INDEX IF NOT EXISTS idx_kde_name ON kernel_domain_events(name, at)",
                ] {
                    sqlx::query(ddl)
                        .execute(db.pool())
                        .await
                        .map_err(|e| e.to_string())?;
                }
                db.load_ownership().await.map_err(|e| e.to_string())?;
                Ok::<(), String>(())
            })?;

            // 4. 事件总线 + Domain 事件落库器（总线独占写 kernel_domain_events）
            let bus = Arc::new(EventBus::new());
            let db_for_events = db.clone();
            bus.set_persister(Arc::new(move |ev: Event| {
                let db = db_for_events.clone();
                tauri::async_runtime::spawn(async move {
                    let payload =
                        serde_json::to_string(&ev.payload).unwrap_or_else(|_| "{}".into());
                    let _ = sqlx::query(
                        "INSERT INTO kernel_domain_events (name, origin, payload, at) \
                         VALUES (?,?,?,?)",
                    )
                    .bind(&ev.name)
                    .bind(&ev.origin)
                    .bind(&payload)
                    .bind(ev.at)
                    .execute(db.pool())
                    .await;
                });
            }));

            // 5. 安全门 + 注册表
            let security = Arc::new(SecurityGate::new());
            let registry = Arc::new(tokio::sync::Mutex::new(PluginRegistry::new()));

            // 6. 前端事件桥接：全量转发内核事件 → k://event（F4 event-bus 消费）
            //    Tauri 2 插件 setup 闭包参数即为 &mut AppHandle，直接 clone
            let handle = app.clone();
            bus.subscribe(&"kernel".into(), "*", move |ev| {
                let _ = handle.emit("k://event", ev);
            })?;

            // 7. 组装 KernelState（激活延后至首个 kernel_dispatch）
            app.manage(KernelState {
                registry,
                db,
                bus,
                security,
                startup_report: RwLock::new(Vec::new()),
                commands: RwLock::new(HashMap::new()),
                activated: tokio::sync::OnceCell::new(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![kernel_dispatch])
        .build()
}
