//! 注册表与状态机（契约：06_Rust代码契约.md §四）
//!
//! 启用状态持久化表（SQLite）：
//! CREATE TABLE plugin_registry (
//!   id TEXT PRIMARY KEY, version TEXT NOT NULL, level TEXT NOT NULL,
//!   parent TEXT, enabled INTEGER NOT NULL DEFAULT 1,
//!   installed_at TEXT NOT NULL, updated_at TEXT NOT NULL,
//!   state_note TEXT, db_namespace TEXT
//! );

use kernel_api::{Manifest, PluginId, PluginLevel};
use std::collections::HashMap;
use std::sync::Arc;

use crate::db::{DbHandle, MigrationOrchestrator, MigrationSet};
use crate::error::KernelError;
use crate::event_bus::EventBus;
use crate::plugin::{
    KernelCtx, Plugin, PluginHandle, PluginState, ServiceRegistry, StartupReportItem,
};
use crate::security::SecurityGate;

/// 必备插件名单（手稿 20260926：必备插件不可单独停用/删除，只跟随其父插件同步停用）。
/// 与前端 PluginManagerPage 的 REQUIRED 提示同源——前端经 PluginInfo.required 字段读取，勿在此名单外自行维护。
pub const REQUIRED_PLUGINS: &[&str] = &[
    "boards.home",         // 首页根板块（项目唯一必备板块）
    "boards.profile",      // 个人中心（首页必备子插件）
    "profile.account",     // 账号（个人中心必备子插件，默认视图）
    "customs.recycle",     // 回收站（首页必备子插件）
    "recycle.list",        // 回收站列表（回收站必备子插件，核心视图）
    "customs.auth",        // 认证基础（登录体系根基）
    "auth.login",          // 登录（认证必备子插件，核心视图）
    "ai.models",           // 模型管理（AI会话必备子插件）
    "ai.sessions",         // 会话列表（AI会话必备子插件）
    "ai.chat",             // 对话（AI会话必备子插件，核心内容）
    "knowledge.material",  // 资料库（知识库必备子插件，批C3）
    "knowledge.learning",  // 学习库（知识库必备子插件，批C3）
    "knowledge.browse",  // 条目浏览（知识库必备子插件，板块核心内容）
    "xin.chat",            // 对话（小欣必备子插件，核心内容）
    "game.preview",   // 游戏预览（游戏板块必备子插件）
    "game.play3d",    // 3D 游戏（游戏板块必备子插件）
    "terminal.yuancode.editor",  // 编辑器核心（YuanCode 必备子插件）
    "terminal.console",  // 终端命令行（终端板块必备子插件，核心内容）
    "terminal.mux",      // 标签页与分屏（终端板块必备子插件，布局骨架）
    "intelligence.dashboard",  // 仪表盘（底层智能必备子插件）
    "intelligence.settings",   // 设置（底层智能必备子插件）
    "search.browser",          // 浏览器搜索（搜索必备子插件）
    "sync.devices",        // 设备管理（同步必备子插件，核心视图）
];

pub struct PluginRegistry {
    /// 编译期全集（装配顺序=拓扑序：L1 先于其 L2/L3 注册）
    plugins: Vec<Box<dyn Plugin>>,
    states: HashMap<PluginId, PluginState>,
    handles: HashMap<PluginId, PluginHandle>,
    /// 跨插件服务实例（可逆：disable 时按 handle.services 回滚）
    pub services: ServiceRegistry,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            states: HashMap::new(),
            handles: HashMap::new(),
            services: ServiceRegistry::default(),
        }
    }

    /// 注册插件（装配期调用；顺序即拓扑序）
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let id = plugin.manifest().id.clone();
        self.states.insert(id, PluginState::Registered);
        self.plugins.push(plugin);
    }

    pub fn state_of(&self, id: &str) -> Option<PluginState> {
        self.states.get(id).copied()
    }

    pub fn plugin_ids(&self) -> Vec<PluginId> {
        self.plugins
            .iter()
            .map(|p| p.manifest().id.clone())
            .collect()
    }

    /// 按插件 id 取 manifest 快照（tauri_glue 的 check_ipc / get_enabled 用）
    pub fn manifest_of(&self, id: &str) -> Option<Manifest> {
        self.plugins
            .iter()
            .find(|p| p.manifest().id == id)
            .map(|p| p.manifest().clone())
    }

    /// IPC 逻辑命令允许使用正式插件 ID 或内核登记短码定位属主。
    pub fn manifest_of_ipc_owner(&self, owner: &str) -> Option<Manifest> {
        self.manifest_of(owner).or_else(|| {
            self.plugins
                .iter()
                .find(|p| SecurityGate::short_code(&p.manifest().id) == owner)
                .map(|p| p.manifest().clone())
        })
    }

    fn plugin_mut(&mut self, id: &str) -> Option<&mut Box<dyn Plugin>> {
        self.plugins.iter_mut().find(|p| p.manifest().id == id)
    }

    /// 按 manifest 完成表归属登记：新表必须有短码前缀；旧表必须已由基线迁移登记。
    async fn register_manifest_tables(
        db: &DbHandle,
        manifest: &Manifest,
    ) -> Result<(), KernelError> {
        let short = SecurityGate::short_code(&manifest.id);
        let prefix = format!("{short}_");
        for table in &manifest.permissions.db {
            if table.starts_with("kernel_") {
                continue;
            }
            let name_prefixed = table.starts_with(&prefix);
            if !name_prefixed && db.ownership_record(table).await?.is_none() {
                return Err(KernelError::Config(format!(
                    "插件 {} 的旧表 {table} 未由基线迁移登记到 kernel_table_ownership",
                    manifest.id
                )));
            }
            db.register_table(table, &manifest.id, name_prefixed)
                .await?;
        }
        Ok(())
    }

    /// 首次安装引导：无记录时按「默认启用清单」写入全量 Enabled
    /// （plugin_registry 表由此创建——kernel 拥有）
    pub async fn bootstrap(&self, db: &DbHandle) -> Result<(), KernelError> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS plugin_registry (\
             id TEXT PRIMARY KEY, version TEXT NOT NULL, level TEXT NOT NULL,\
             parent TEXT, enabled INTEGER NOT NULL DEFAULT 1,\
             installed_at TEXT NOT NULL, updated_at TEXT NOT NULL,\
             state_note TEXT, db_namespace TEXT)",
        )
        .execute(db.pool())
        .await
        .map_err(|e| KernelError::Db(e.to_string()))?;

        let existing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry")
            .fetch_one(db.pool())
            .await
            .map_err(|e| KernelError::Db(e.to_string()))?;

        if existing == 0 {
            // 首装引导：全量 Enabled
            for p in &self.plugins {
                let m = p.manifest();
                sqlx::query(
                    "INSERT INTO plugin_registry (id, version, level, parent, enabled, installed_at, updated_at, state_note, db_namespace) \
                     VALUES (?, ?, ?, ?, 1, ?, ?, NULL, ?)",
                )
                .bind(&m.id)
                .bind(&m.version)
                .bind(match m.level {
                    PluginLevel::Board => "board",
                    PluginLevel::Feature => "feature",
                    PluginLevel::Custom => "custom",
                })
                .bind(m.parent.as_deref())
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(&m.i18n_namespace)
                .execute(db.pool())
                .await
                .map_err(|e| KernelError::Db(e.to_string()))?;
            }
        } else {
            // 增量：新出现的插件补登记（默认启用）
            for p in &self.plugins {
                let m = p.manifest();
                let known: Option<String> =
                    sqlx::query_scalar("SELECT id FROM plugin_registry WHERE id = ?")
                        .bind(&m.id)
                        .fetch_optional(db.pool())
                        .await
                        .map_err(|e| KernelError::Db(e.to_string()))?;
                if known.is_none() {
                    sqlx::query(
                        "INSERT INTO plugin_registry (id, version, level, parent, enabled, installed_at, updated_at, state_note, db_namespace) \
                         VALUES (?, ?, ?, ?, 1, ?, ?, NULL, ?)",
                    )
                    .bind(&m.id)
                    .bind(&m.version)
                    .bind(match m.level {
                        PluginLevel::Board => "board",
                        PluginLevel::Feature => "feature",
                        PluginLevel::Custom => "custom",
                    })
                    .bind(m.parent.as_deref())
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(&m.i18n_namespace)
                    .execute(db.pool())
                    .await
                    .map_err(|e| KernelError::Db(e.to_string()))?;
                }
            }
        }
        Ok(())
    }

    /// 分类路由安装校验（嵌套插槽核心）
    pub fn validate_mount(&self, m: &Manifest) -> Result<(), KernelError> {
        match m.level {
            PluginLevel::Board => Ok(()), // 挂内核导航主干
            PluginLevel::Feature => {
                // 1) parent 必须存在且 Enabled
                let parent_id = m.parent.as_deref().ok_or_else(|| {
                    KernelError::Config(format!("L2 插件 {} 缺少 parent 声明", m.id))
                })?;
                let parent_state = self.state_of(parent_id).ok_or_else(|| {
                    KernelError::Config(format!("插件 {} 的 parent {parent_id} 未注册", m.id))
                })?;
                if parent_state != PluginState::Enabled {
                    return Err(KernelError::Config(format!(
                        "插件 {} 的 parent {parent_id} 未启用（状态 {parent_state:?}）",
                        m.id
                    )));
                }
                // 2) parent.manifest().slots 中必须存在 m.slot 指向的插槽
                let slot = m.slot.as_deref().ok_or_else(|| {
                    KernelError::Config(format!("L2 插件 {} 缺少 slot 声明", m.id))
                })?;
                let parent = self
                    .plugins
                    .iter()
                    .find(|p| p.manifest().id == parent_id)
                    .ok_or_else(|| KernelError::Config(format!("parent {parent_id} 不存在")))?;
                let spec = parent
                    .manifest()
                    .slots
                    .iter()
                    .find(|s| s.id == slot)
                    .ok_or_else(|| {
                        KernelError::Config(format!("parent {parent_id} 未声明插槽 {slot}"))
                    })?;
                // 3) 该插槽现有安装数 < capacity（按已启用 L2 中同 parent 同 slot 计数）
                let installed = self
                    .plugins
                    .iter()
                    .filter(|p| {
                        let pm = p.manifest();
                        pm.level == PluginLevel::Feature
                            && pm.parent.as_deref() == Some(parent_id)
                            && pm.slot.as_deref() == Some(slot)
                            && self.states.get(&pm.id) == Some(&PluginState::Enabled)
                    })
                    .count();
                if installed >= spec.capacity as usize {
                    return Err(KernelError::Config(format!(
                        "slot {slot} 容量已满({installed}/{})",
                        spec.capacity
                    )));
                }
                Ok(())
            }
            PluginLevel::Custom => Ok(()), // 权限清单由 SecurityGate.validate_manifest 校验
        }
    }

    /// 启动装配：按拓扑序（注册序）对 enabled 插件调 init；
    /// 单插件失败 → 状态 Error + 记入启动报告，不阻塞
    pub async fn activate_all(
        &mut self,
        db: Arc<DbHandle>,
        events: Arc<EventBus>,
        security: Arc<SecurityGate>,
    ) -> Result<Vec<StartupReportItem>, KernelError> {
        let mut report = Vec::new();
        let ids = self.plugin_ids();
        for id in ids {
            let m = self
                .plugins
                .iter()
                .find(|p| p.manifest().id == id)
                .map(|p| p.manifest().clone())
                .expect("插件已注册");

            // manifest 六类权限总校验
            if let Err(e) = security.validate_manifest(&m) {
                self.states.insert(id.clone(), PluginState::Error);
                report.push(StartupReportItem {
                    plugin: id,
                    state: PluginState::Error.as_str().into(),
                    note: Some(e.to_string()),
                });
                continue;
            }

            // DB 启用状态读取（bootstrap 已保证行存在）
            let enabled_row: Option<i64> =
                sqlx::query_scalar("SELECT enabled FROM plugin_registry WHERE id = ?")
                    .bind(&id)
                    .fetch_optional(db.pool())
                    .await
                    .map_err(|e| KernelError::Db(e.to_string()))?;
            if enabled_row == Some(0) {
                self.states.insert(id.clone(), PluginState::Disabled);
                report.push(StartupReportItem {
                    plugin: id,
                    state: PluginState::Disabled.as_str().into(),
                    note: None,
                });
                continue;
            }

            // 父链过滤（手稿 20260926：子插件随父插件同步停用）——
            // 父插件未启用时子插件不激活（与前端 buildApp 过滤口径一致，避免悬空 IPC 面）
            if let Some(parent) = &m.parent {
                let parent_enabled: Option<i64> =
                    sqlx::query_scalar("SELECT enabled FROM plugin_registry WHERE id = ?")
                        .bind(parent)
                        .fetch_optional(db.pool())
                        .await
                        .map_err(|e| KernelError::Db(e.to_string()))?;
                if parent_enabled != Some(1) {
                    self.states.insert(id.clone(), PluginState::Disabled);
                    report.push(StartupReportItem {
                        plugin: id,
                        state: PluginState::Disabled.as_str().into(),
                        note: Some(format!("随父插件 {parent} 停用")),
                    });
                    continue;
                }
            }

            // 分类路由安装校验
            if let Err(e) = self.validate_mount(&m) {
                self.states.insert(id.clone(), PluginState::Error);
                report.push(StartupReportItem {
                    plugin: id,
                    state: PluginState::Error.as_str().into(),
                    note: Some(e.to_string()),
                });
                continue;
            }

            // init（单插件隔离：失败 → Error，不阻塞其余）
            let mut ctx = KernelCtx {
                plugin_id: id.clone(),
                db: Arc::clone(&db),
                events: Arc::clone(&events),
                security: Arc::clone(&security),
                register: Default::default(),
            };
            let handle = match self.plugin_mut(&id).unwrap().init(&mut ctx).await {
                Ok(h) => h,
                Err(e) => {
                    self.states.insert(id.clone(), PluginState::Error);
                    report.push(StartupReportItem {
                        plugin: id,
                        state: PluginState::Error.as_str().into(),
                        note: Some(e.to_string()),
                    });
                    continue;
                }
            };

            // 迁移应用（init 登记的 migration 集）+ 表归属登记
            if let Some((ns, files)) = &ctx.register.migrations {
                let orch = MigrationOrchestrator;
                if let Err(e) = orch.apply_all(
                    &db,
                    vec![MigrationSet {
                        namespace: ns.clone(),
                        files: files.clone(),
                    }],
                )
                .await
                {
                    self.states.insert(id.clone(), PluginState::Error);
                    report.push(StartupReportItem {
                        plugin: id,
                        state: PluginState::Error.as_str().into(),
                        note: Some(e.to_string()),
                    });
                    continue;
                }
                if let Err(e) = Self::register_manifest_tables(&db, &m).await {
                    self.states.insert(id.clone(), PluginState::Error);
                    report.push(StartupReportItem {
                        plugin: id,
                        state: PluginState::Error.as_str().into(),
                        note: Some(e.to_string()),
                    });
                    continue;
                }
            }

            // 注册物落位（handle 提交）
            for (name, inst) in ctx.register.services {
                let name: &'static str = name;
                self.services.register(name, Arc::new(inst));
            }
            self.handles.insert(id.clone(), handle);
            self.states.insert(id.clone(), PluginState::Enabled);
            report.push(StartupReportItem {
                plugin: id,
                state: PluginState::Enabled.as_str().into(),
                note: None,
            });
        }
        Ok(report)
    }

    /// 运行时启停（插件管理页调用）
    /// enabled=false：DB 写 0 + 回滚 handle（退订/撤销服务）
    /// enabled=true ：DB 写 1 + 重新 init（校验 mount 通过才生效）
    pub async fn set_enabled(
        &mut self,
        db: Arc<DbHandle>,
        events: Arc<EventBus>,
        security: Arc<SecurityGate>,
        id: &str,
        enabled: bool,
    ) -> Result<(), KernelError> {
        if !self.states.contains_key(id) {
            return Err(KernelError::Config(format!("插件 {id} 未注册")));
        }
        // 必备守卫（手稿 20260926）：必备插件不可单独停用，只随父插件同步
        if !enabled && REQUIRED_PLUGINS.contains(&id) {
            return Err(KernelError::Config(format!(
                "插件 {id} 为必备插件，不可停用（只随其父插件同步停用）"
            )));
        }
        let m = self
            .plugins
            .iter()
            .find(|p| p.manifest().id == id)
            .map(|p| p.manifest().clone())
            .expect("插件已注册");

        if enabled {
            // 重新装配（校验不通过 → 显式失败）
            self.validate_mount(&m)?;
            security.validate_manifest(&m)?;

            let mut ctx = KernelCtx {
                plugin_id: id.to_string(),
                db: Arc::clone(&db),
                events: Arc::clone(&events),
                security: Arc::clone(&security),
                register: Default::default(),
            };
            let handle = self.plugin_mut(id).unwrap().init(&mut ctx).await?;

            if let Some((ns, files)) = &ctx.register.migrations {
                let orch = MigrationOrchestrator;
                orch.apply_all(
                    &db,
                    vec![MigrationSet {
                        namespace: ns.clone(),
                        files: files.clone(),
                    }],
                )
                .await?;
                Self::register_manifest_tables(&db, &m).await?;
            }
            for (name, inst) in ctx.register.services {
                let name: &'static str = name;
                self.services.register(name, Arc::new(inst));
            }
            self.handles.insert(id.to_string(), handle);
            self.states.insert(id.to_string(), PluginState::Enabled);
        } else {
            // 逆向回滚（可逆注册）：退订 + 撤销服务
            if let Some(handle) = self.handles.remove(id) {
                for sub in handle.subscriptions {
                    events.unsubscribe(sub)?;
                }
                for name in handle.services {
                    self.services.unregister(name);
                }
            }
            self.states.insert(id.to_string(), PluginState::Disabled);
        }

        sqlx::query("UPDATE plugin_registry SET enabled = ?, updated_at = ? WHERE id = ?")
            .bind(if enabled { 1 } else { 0 })
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(id)
            .execute(db.pool())
            .await
            .map_err(|e| KernelError::Db(e.to_string()))?;

        // 事件契约（04_事件总线 §六首批）：启停落盘成功后发布 state-changed，
        // 前端必订并据此重建路由/导航（07 §九）。域校验三要素一致：origin/domain/name 前缀均 "kernel"
        let _ = events.publish(
            &"kernel".to_string(),
            kernel_api::Event {
                name: "kernel:plugin.state-changed".into(),
                origin: "kernel".into(),
                domain: "kernel".into(),
                scope: kernel_api::EventScope::Domain,
                payload: serde_json::json!({
                    "id": id,
                    "state": if enabled { "enabled" } else { "disabled" },
                    "reason": None::<String>,
                }),
                at: chrono::Utc::now().timestamp_millis(),
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DbHandle;
    use crate::event_bus::EventBus;
    use crate::plugin::{MigrationFile, Registrar};
    use kernel_api::{EventPerms, Permissions, SlotSpec};
    use sqlx::sqlite::SqlitePool;
    use std::sync::atomic::{AtomicUsize, Ordering};

    async fn mem_db() -> Arc<DbHandle> {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect");
        let db = DbHandle::new(pool);
        sqlx::query("CREATE TABLE IF NOT EXISTS plugin_migrations (namespace TEXT NOT NULL, version INTEGER NOT NULL, applied_at TEXT NOT NULL, PRIMARY KEY (namespace, version))")
            .execute(db.pool()).await.unwrap();
        sqlx::query("CREATE TABLE IF NOT EXISTS kernel_table_ownership (table_name TEXT PRIMARY KEY, plugin_id TEXT NOT NULL, name_prefixed INTEGER NOT NULL, created_at TEXT NOT NULL)")
            .execute(db.pool()).await.unwrap();
        Arc::new(db)
    }

    fn base_manifest(id: &str, level: PluginLevel) -> Manifest {
        Manifest {
            id: id.into(),
            name: "测试插件".into(),
            level,
            parent: None,
            slot: None,
            version: "0.1.0".into(),
            kernel_api: "1".into(),
            permissions: Permissions {
                db: vec![format!("{}_t", id)],
                events: EventPerms {
                    subscribe: vec![],
                    publish: vec![format!("{id}:*")],
                },
                ipc: vec![format!("{id}_*")],
                fs: vec![],
                net: vec![],
            },
            slots: vec![],
            i18n_namespace: id.into(),
        }
    }

    /// 可配置测试插件：init 是否失败 / init 顺序记录 / 是否带插槽声明
    struct TestPlugin {
        m: Manifest,
        fail_init: bool,
        counter: Arc<AtomicUsize>,
        order: Arc<std::sync::Mutex<Vec<String>>>,
        migrations: Option<(String, Vec<MigrationFile>)>,
    }

    impl TestPlugin {
        fn board(
            id: &str,
            counter: Arc<AtomicUsize>,
            order: Arc<std::sync::Mutex<Vec<String>>>,
        ) -> Self {
            let mut m = base_manifest(id, PluginLevel::Board);
            m.slots = vec![SlotSpec {
                id: format!("{id}.panel"),
                r#type: "panel".into(),
                description: "测试插槽".into(),
                capacity: 2,
                route_prefix: None,
            }];
            Self {
                m,
                fail_init: false,
                counter,
                order,
                migrations: None,
            }
        }

        fn feature(
            id: &str,
            parent: &str,
            slot: &str,
            counter: Arc<AtomicUsize>,
            order: Arc<std::sync::Mutex<Vec<String>>>,
        ) -> Self {
            let mut m = base_manifest(id, PluginLevel::Feature);
            m.parent = Some(parent.into());
            m.slot = Some(slot.into());
            Self {
                m,
                fail_init: false,
                counter,
                order,
                migrations: None,
            }
        }

        fn with_fail_init(mut self) -> Self {
            self.fail_init = true;
            self
        }

        fn with_migrations(mut self, ns: &str) -> Self {
            self.migrations = Some((
                ns.to_string(),
                vec![MigrationFile {
                    version: 1,
                    sql: "CREATE TABLE IF NOT EXISTS test_tbl (id INTEGER PRIMARY KEY)",
                }],
            ));
            self
        }

        fn with_legacy_table(mut self, table: &str) -> Self {
            self.m.permissions.db = vec![table.to_string()];
            self.migrations = Some((
                self.m.id.clone(),
                vec![MigrationFile {
                    version: 1,
                    sql: Box::leak(
                        format!(
                            "INSERT OR IGNORE INTO kernel_table_ownership \
                         (table_name, plugin_id, name_prefixed, created_at) \
                         VALUES ('{table}', '{}', 0, 'test')",
                            self.m.id
                        )
                        .into_boxed_str(),
                    ),
                }],
            ));
            self
        }
    }

    #[async_trait::async_trait]
    impl Plugin for TestPlugin {
        fn manifest(&self) -> &'static Manifest {
            // 测试插件 manifest 存活期与测试进程一致（泄漏换取 &'static）
            Box::leak(Box::new(self.m.clone()))
        }

        async fn init(&self, ctx: &mut KernelCtx) -> Result<PluginHandle, KernelError> {
            self.order.lock().unwrap().push(ctx.plugin_id.clone());
            self.counter.fetch_add(1, Ordering::SeqCst);
            if self.fail_init {
                return Err(KernelError::PluginInit {
                    plugin: ctx.plugin_id.clone(),
                    reason: "故意失败".into(),
                });
            }
            let mut handle = PluginHandle::default();
            // 订阅自己的域（验证 handle 记账）
            if let Ok(sub) =
                ctx.events
                    .subscribe(&ctx.plugin_id, &format!("{}:ping", ctx.plugin_id), |_e| {})
            {
                ctx.register.subscriptions.push(sub);
                handle.subscriptions.push(sub);
            }
            if let Some((ns, files)) = &self.migrations {
                ctx.register.migrations = Some((ns.to_string(), files.clone()));
                handle.migrations_ns = Some(ns.to_string());
            }
            Ok(handle)
        }
    }

    // 必测 1：bootstrap 首装（空库 → 全量 Enabled 记录）
    #[tokio::test]
    async fn bootstrap_first_install() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(TestPlugin::board(
            "_a",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));
        reg.bootstrap(&db).await.unwrap();

        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(n, 1);
        let en: i64 = sqlx::query_scalar("SELECT enabled FROM plugin_registry WHERE id='_a'")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(en, 1, "首装默认启用");

        // 重复 bootstrap → 幂等（不重复插入）
        reg.bootstrap(&db).await.unwrap();
        let n2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plugin_registry")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(n2, 1);
    }

    // 必测 2：拓扑排序（装配顺序即 init 顺序：L1 先于 L2）
    #[tokio::test]
    async fn activation_order_is_registration_order() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(TestPlugin::board(
            "_b1",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));
        reg.register(Box::new(TestPlugin::feature(
            "_b2",
            "_b1",
            "_b1.panel",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));

        reg.bootstrap(&db).await.unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());
        let report = reg.activate_all(Arc::clone(&db), bus, sec).await.unwrap();

        let o = order.lock().unwrap();
        assert_eq!(
            *o,
            vec!["_b1".to_string(), "_b2".to_string()],
            "L1 必须先于 L2 init: {o:?}"
        );
        assert_eq!(report.len(), 2);
        assert!(report.iter().all(|r| r.state == "enabled"), "{report:?}");
    }

    // 必测 3：Error 隔离（单插件 init 失败不阻塞其余）
    #[tokio::test]
    async fn init_failure_isolated() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(
            TestPlugin::board("_c1", Arc::clone(&counter), Arc::clone(&order)).with_fail_init(),
        ));
        reg.register(Box::new(TestPlugin::board(
            "_c2",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));

        reg.bootstrap(&db).await.unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());
        let report = reg.activate_all(Arc::clone(&db), bus, sec).await.unwrap();

        assert_eq!(reg.state_of("_c1"), Some(PluginState::Error));
        assert_eq!(
            reg.state_of("_c2"),
            Some(PluginState::Enabled),
            "失败插件不得阻塞后续"
        );
        let err_item = report.iter().find(|r| r.plugin == "_c1").unwrap();
        assert_eq!(err_item.state, "error");
        assert!(
            err_item.note.as_deref().unwrap_or("").contains("故意失败"),
            "{err_item:?}"
        );
        let ok_item = report.iter().find(|r| r.plugin == "_c2").unwrap();
        assert_eq!(ok_item.state, "enabled");
        assert!(ok_item.note.is_none());
        assert_eq!(counter.load(Ordering::SeqCst), 2, "两个 init 都被调用");
    }

    // 必测 4：启停幂等（disable 二次不 Err；enable 后订阅无重复）
    #[tokio::test]
    async fn disable_enable_idempotent() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(TestPlugin::board(
            "_d",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));

        reg.bootstrap(&db).await.unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());
        reg.activate_all(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec))
            .await
            .unwrap();
        let subs_after_first = bus.sub_count();

        // disable
        reg.set_enabled(
            Arc::clone(&db),
            Arc::clone(&bus),
            Arc::clone(&sec),
            "_d",
            false,
        )
        .await
        .unwrap();
        assert_eq!(reg.state_of("_d"), Some(PluginState::Disabled));
        assert_eq!(bus.sub_count(), 0, "退订应清空");
        // 二次 disable → 不 Err（幂等）
        reg.set_enabled(
            Arc::clone(&db),
            Arc::clone(&bus),
            Arc::clone(&sec),
            "_d",
            false,
        )
        .await
        .unwrap();

        // enable（重新 init）
        reg.set_enabled(
            Arc::clone(&db),
            Arc::clone(&bus),
            Arc::clone(&sec),
            "_d",
            true,
        )
        .await
        .unwrap();
        assert_eq!(reg.state_of("_d"), Some(PluginState::Enabled));
        assert_eq!(
            bus.sub_count(),
            subs_after_first,
            "重新启用后订阅数与首次一致（无重复）"
        );

        // DB 状态落盘
        let en: i64 = sqlx::query_scalar("SELECT enabled FROM plugin_registry WHERE id='_d'")
            .fetch_one(db.pool())
            .await
            .unwrap();
        assert_eq!(en, 1);
    }

    // 附加：Feature 挂载校验（slot 不存在 / 超容 / parent 未启用）
    #[tokio::test]
    async fn feature_mount_validation() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(TestPlugin::board(
            "_e1",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));
        reg.bootstrap(&db).await.unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());
        reg.activate_all(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec))
            .await
            .unwrap();

        // slot 不存在 → 拒绝
        let orphan = TestPlugin::feature(
            "_e2",
            "_e1",
            "no_such_slot",
            Arc::clone(&counter),
            Arc::clone(&order),
        );
        let err = reg.validate_mount(orphan.manifest()).unwrap_err();
        assert!(err.to_string().contains("no_such_slot"), "{err}");

        // parent 未启用 → 拒绝
        reg.set_enabled(
            Arc::clone(&db),
            Arc::clone(&bus),
            Arc::clone(&sec),
            "_e1",
            false,
        )
        .await
        .unwrap();
        let orphan2 = TestPlugin::feature(
            "_e3",
            "_e1",
            "_e1.panel",
            Arc::clone(&counter),
            Arc::clone(&order),
        );
        let err = reg.validate_mount(orphan2.manifest()).unwrap_err();
        assert!(err.to_string().contains("未启用"), "{err}");
    }

    // 附加：迁移应用 + 表归属登记
    #[tokio::test]
    async fn migrations_applied_and_ownership_registered() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(
            TestPlugin::board("_f", Arc::clone(&counter), Arc::clone(&order))
                .with_migrations("ns_f"),
        ));
        reg.bootstrap(&db).await.unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());
        reg.activate_all(Arc::clone(&db), bus, sec).await.unwrap();

        let v: Option<i64> = sqlx::query_scalar(
            "SELECT version FROM plugin_migrations WHERE namespace='ns_f' AND version=1",
        )
        .fetch_optional(db.pool())
        .await
        .unwrap();
        assert_eq!(v, Some(1), "迁移应已应用并记录版本");

        let owner: Option<String> = sqlx::query_scalar(
            "SELECT plugin_id FROM kernel_table_ownership WHERE table_name='_f_t'",
        )
        .fetch_optional(db.pool())
        .await
        .unwrap();
        assert_eq!(owner.as_deref(), Some("_f"), "manifest db 表应登记归属");
    }

    #[tokio::test]
    async fn legacy_ownership_preserved_on_activation_and_reenable() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        // 非必备 id（boards.legacytest）：本测停用/复启用例不可撞 REQUIRED_PLUGINS 守卫
        reg.register(Box::new(
            TestPlugin::board("boards.legacytest", Arc::clone(&counter), Arc::clone(&order))
                .with_legacy_table("resumes"),
        ));
        reg.bootstrap(&db).await.unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());

        let report = reg
            .activate_all(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec))
            .await
            .unwrap();
        assert_eq!(report[0].state, "enabled", "{report:?}");
        assert_eq!(
            db.ownership_record("resumes").await.unwrap(),
            Some(("boards.legacytest".into(), false))
        );

        reg.set_enabled(
            Arc::clone(&db),
            Arc::clone(&bus),
            Arc::clone(&sec),
            "boards.legacytest",
            false,
        )
        .await
        .unwrap();
        reg.set_enabled(
            Arc::clone(&db),
            Arc::clone(&bus),
            Arc::clone(&sec),
            "boards.legacytest",
            true,
        )
        .await
        .unwrap();
        assert_eq!(
            db.ownership_record("resumes").await.unwrap(),
            Some(("boards.legacytest".into(), false))
        );
    }

    // 必测：启停成功后发布 kernel:plugin.state-changed（04 §六事件契约，F6 前端依赖）
    #[tokio::test]
    async fn set_enabled_publishes_state_changed() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(TestPlugin::board(
            "_g",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));
        reg.bootstrap(&db).await.unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());
        reg.activate_all(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec))
            .await
            .unwrap();

        // 订阅内核域，收集 state-changed
        let hits: Arc<std::sync::Mutex<Vec<kernel_api::Event>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let h2 = Arc::clone(&hits);
        bus.subscribe(&"kernel".to_string(), "kernel:*", move |e| {
            h2.lock().unwrap().push(e.clone());
        })
        .unwrap();

        // disable → 1 次，state=disabled；enable → 1 次，state=enabled
        reg.set_enabled(
            Arc::clone(&db),
            Arc::clone(&bus),
            Arc::clone(&sec),
            "_g",
            false,
        )
        .await
        .unwrap();
        reg.set_enabled(
            Arc::clone(&db),
            Arc::clone(&bus),
            Arc::clone(&sec),
            "_g",
            true,
        )
        .await
        .unwrap();

        let got = hits.lock().unwrap();
        assert_eq!(got.len(), 2, "启停应各发布一次事件: {got:?}");
        for ev in got.iter() {
            assert_eq!(ev.name, "kernel:plugin.state-changed");
            assert_eq!(ev.origin, "kernel");
            assert_eq!(ev.domain, "kernel");
            assert_eq!(ev.payload["id"], "_g");
        }
        assert_eq!(got[0].payload["state"], "disabled");
        assert_eq!(got[1].payload["state"], "enabled");
    }

    // 必测：必备守卫——REQUIRED_PLUGINS 拒绝停用，非必备正常停用（手稿 20260926）
    #[tokio::test]
    async fn set_enabled_rejects_required_plugins() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        // boards.home 在 REQUIRED_PLUGINS 名单内（db 表名用合法标识，避开 manifest 校验）
        let mut required = TestPlugin::board("boards.home", Arc::clone(&counter), Arc::clone(&order));
        required.m.permissions.db = vec!["boards_home_t".into()];
        reg.register(Box::new(required));
        reg.register(Box::new(TestPlugin::board(
            "_g",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));
        reg.bootstrap(&db).await.unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());
        reg.activate_all(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec))
            .await
            .unwrap();

        // 必备插件停用 → Err
        let err = reg
            .set_enabled(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec), "boards.home", false)
            .await
            .expect_err("必备插件停用应被拒绝");
        assert!(err.to_string().contains("必备插件"), "报错应含必备提示: {err}");
        // 必备插件启用（恢复性操作）→ Ok
        reg.set_enabled(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec), "boards.home", true)
            .await
            .unwrap();
        // 非必备插件停用 → Ok
        reg.set_enabled(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec), "_g", false)
            .await
            .unwrap();
    }

    // 必测：父链过滤——父板块停用时其子插件随父停用（手稿 20260926，与前端 buildApp 口径一致）
    #[tokio::test]
    async fn activate_all_skips_children_of_disabled_parent() {
        let db = mem_db().await;
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(TestPlugin::board(
            "_p",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));
        reg.register(Box::new(TestPlugin::feature(
            "_c",
            "_p",
            "_p.panel",
            Arc::clone(&counter),
            Arc::clone(&order),
        )));
        reg.bootstrap(&db).await.unwrap();
        // 父板块直接置停用（DB 层），子插件 DB 仍为启用
        sqlx::query("UPDATE plugin_registry SET enabled = 0 WHERE id = '_p'")
            .execute(db.pool())
            .await
            .unwrap();
        let bus = Arc::new(EventBus::new());
        let sec = Arc::new(SecurityGate::new());
        let report = reg
            .activate_all(Arc::clone(&db), Arc::clone(&bus), Arc::clone(&sec))
            .await
            .unwrap();

        let child = report.iter().find(|r| r.plugin == "_c").expect("报告应含子插件");
        assert_eq!(child.state, "disabled", "子插件应随父停用: {child:?}");
        assert_eq!(
            child.note.as_deref(),
            Some("随父插件 _p 停用"),
            "报告应标注随父停用: {child:?}"
        );
        assert_eq!(reg.state_of("_c"), Some(PluginState::Disabled));
    }

    #[test]
    fn ipc_owner_resolves_formal_id_and_short_code() {
        let mut reg = PluginRegistry::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        reg.register(Box::new(TestPlugin::board(
            "boards.profile",
            counter,
            order,
        )));

        for owner in ["boards.profile", "pf"] {
            let manifest = reg
                .manifest_of_ipc_owner(owner)
                .expect("应解析 profile manifest");
            assert_eq!(manifest.id, "boards.profile");
        }
        assert!(reg.manifest_of_ipc_owner("profile").is_none());
    }

    // Registrar import 保持（供 ctx 构造引用）
    #[allow(dead_code)]
    fn _r(_x: Registrar) {}
}
