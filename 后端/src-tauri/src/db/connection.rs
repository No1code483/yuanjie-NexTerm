use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::HashMap;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::crypto::mek_manager::MekManager;
use crate::error::app_error::AppError;
use crate::services::collab::CollabSessionManager;
use crate::services::intelligence_service::IntelligenceService;
use crate::services::model_health_monitor::ModelHealthMonitor;
use crate::services::ssh_service::SshService;
use crate::services::sync::connectivity::ConnectivityChecker;
use crate::services::terminal_mux::TerminalMux;
use crate::services::terminal_service::TerminalService;
use crate::services::xin_dialogue_service::XinDialogueService;
use crate::services::xin_basic_service::XiaoxinService;
use crate::services::xin_tts_service::XinTtsService;

pub struct AppState {
    pub pool: SqlitePool,
    pub mek_manager: Arc<RwLock<MekManager>>,
    pub terminal_service: Arc<TerminalService>,
    pub terminal_mux: Arc<TerminalMux>,
    pub ssh_service: Arc<SshService>,
    pub intelligence_service: Arc<IntelligenceService>,
    pub xiaoxin_service: Arc<XiaoxinService>,
    pub xin_dialogue_service: Arc<XinDialogueService>,
    pub xin_tts_service: Arc<XinTtsService>,
    pub current_user: Arc<RwLock<Option<i64>>>,
    pub current_token: Arc<RwLock<Option<String>>>,
    pub data_dir: PathBuf,
    pub force_stop_flags: Arc<RwLock<HashMap<i64, Arc<AtomicBool>>>>,
    /// D1 v3.2 Task 3.4.4: 协作会话管理器（内存态，懒初始化）
    ///
    /// 首次 collab_session_* 命令调用时通过 get_manager 双检锁初始化。
    /// 重启丢失（符合 PoC 定位）；未来若需持久化，可迁移至 DB。
    pub collab_session_manager: Arc<RwLock<Option<Arc<CollabSessionManager>>>>,
    /// A5 Phase 3 Task 1: 网络连通性检测器（周期探测 + 事件广播）
    ///
    /// 在 main.rs setup 中初始化后调用 `start_background_task(app_handle)` 启动后台周期检测。
    /// 状态变更时通过 Tauri 事件 `network-status-changed` 广播。
    pub connectivity_checker: ConnectivityChecker,
    /// spec ai-chat-enhancement Phase 1 §1.2: 模型健康检测后台服务
    ///
    /// 在 main.rs setup 中初始化后调用 `start_background_task(app_handle, pool, mek, user_id)`
    /// 启动后台周期检测。状态变更时通过 Tauri 事件 `ai-model-health-changed` 广播。
    /// 命令 `set_health_check_interval` / `check_all_models_health` 通过此字段访问 monitor。
    pub model_health_monitor: ModelHealthMonitor,
}

impl AppState {
    pub async fn new(db_path: &str, data_dir: PathBuf) -> Result<Self, AppError> {
        if let Some(parent) = std::path::Path::new(db_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(5));

        let pool = SqlitePoolOptions::new()
            .max_connections(20)
            .connect_with(options)
            .await
            .map_err(|e| AppError::Database(e))?;

        let mek_manager = Arc::new(RwLock::new(MekManager::new()));

        // D3.2 TTS 缓存目录：data_dir/tts_cache
        let tts_cache_dir = data_dir.join("tts_cache");

        Ok(Self {
            xiaoxin_service: Arc::new(XiaoxinService::new(pool.clone())),
            xin_dialogue_service: Arc::new(XinDialogueService::new(pool.clone(), mek_manager.clone())),
            xin_tts_service: Arc::new(XinTtsService::new(tts_cache_dir)),
            intelligence_service: Arc::new(IntelligenceService::new()),
            pool: pool.clone(),
            mek_manager,
            terminal_service: Arc::new(TerminalService::new()),
            terminal_mux: Arc::new(TerminalMux::new(pool.clone())),
            ssh_service: Arc::new(SshService::new()),
            current_user: Arc::new(RwLock::new(None)),
            current_token: Arc::new(RwLock::new(None)),
            data_dir,
            force_stop_flags: Arc::new(RwLock::new(HashMap::new())),
            collab_session_manager: Arc::new(RwLock::new(None)),
            // Phase 3 Task 5：传入 pool，使网络恢复时自动 flush 配置同步队列
            connectivity_checker: ConnectivityChecker::with_pool(pool.clone()),
            // spec ai-chat-enhancement Phase 1 §1.2: 模型健康检测后台服务
            model_health_monitor: ModelHealthMonitor::new(),
        })
    }
}