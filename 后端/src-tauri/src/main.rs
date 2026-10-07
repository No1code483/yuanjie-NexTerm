#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;

use tauri::Manager;

use nexterm_lib::plugins::_legacy::commands::{agent_v3_commands, tools_commands};
// 阶段3 批次5b（S7）：engine 命令已随 terminal.yuancode/plugins 归 agent/ 收编，
// 但 EngineState 仍需要在 app.manage() 中注册
use nexterm_lib::plugins::_legacy::commands::engine_commands::EngineState;
use nexterm_lib::plugins::_legacy::commands::lsp_commands::LspState;
use nexterm_lib::db::connection::AppState;
use nexterm_lib::db::migrations;
use nexterm_lib::db::repositories::mcp_repo;
use nexterm_lib::db::repositories::permission_repo;
use nexterm_lib::engine::EngineConfig;
use nexterm_lib::engine::YuanEngine;
use nexterm_lib::lsp::LspManager;
use nexterm_lib::plugins::_legacy::services::agent_service::AgentService;
use nexterm_lib::plugins::_legacy::services::backup_scheduler;
use nexterm_lib::plugins::_legacy::services::compact_service::CompactService;
use nexterm_lib::plugins::_legacy::services::git_service::GitService;
use nexterm_lib::plugins::_legacy::services::goal_service::GoalService;
use nexterm_lib::plugins::_legacy::services::inline_service::InlineService;
use nexterm_lib::plugins::_legacy::services::io_control_service::IoControlService;
use nexterm_lib::plugins::_legacy::services::mcp_service::McpService;
use nexterm_lib::plugins::customs::auth::start_mek_rotation_scheduler;
use nexterm_lib::plugins::_legacy::services::prompt_service::PromptService;
use nexterm_lib::plugins::_legacy::services::safety_service::SafetyService;
use nexterm_lib::plugins::_legacy::services::sandbox_service::SandboxService;
use nexterm_lib::plugins::_legacy::services::skill_service::SkillService;
use nexterm_lib::tools::ApplyPatchTool;
use nexterm_lib::tools::PlanTool;
use nexterm_lib::tools::ToolRegistry;
use std::sync::Arc;
use tokio::sync::Mutex;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // A1 §2.5 启动链路测量：记录 boot 总耗时（含 tracing init + Tauri Builder 构建）
    let boot_start = std::time::Instant::now();

    tauri::Builder::default()
        // 插件化重构：内核最前，板块插件随后登记
        .plugin(kernel::tauri_glue::init())
        .plugin(nexterm_lib::plugins::boards::_hello::init())
        // 阶段3 批次1b-1：L1 boards.home → L2 home.todo / home.journal（注册序=拓扑序）
        .plugin(nexterm_lib::plugins::boards::home::init())
        .plugin(nexterm_lib::plugins::boards::home::features::todo::init())
        .plugin(nexterm_lib::plugins::boards::home::features::journal::init())
        // 阶段3 批次1b-2a：home.timer（拓扑序 L1 → todo → journal → timer）
        .plugin(nexterm_lib::plugins::boards::home::features::timer::init())
        // 阶段3 批次1b-2b-1：home.news（拓扑序 L1 → todo → journal → timer → news）
        .plugin(nexterm_lib::plugins::boards::home::features::news::init())
        // 手稿 20260926：boards.profile 归位为首页必备子插件（L2 feature → 注册序须在 boards.home 之后）
        .plugin(nexterm_lib::plugins::boards::profile::init())
        // 个人中心 4 个 L2 子插件（拓扑序 profile → 子插件）
        .plugin(nexterm_lib::plugins::boards::profile::features::account::init())
        .plugin(nexterm_lib::plugins::boards::profile::features::resume::init())
        .plugin(nexterm_lib::plugins::boards::profile::features::quote::init())
        .plugin(nexterm_lib::plugins::boards::profile::features::settings::init())
        // 批C4：home.focus（专注，可选子插件，纯前端无命令；注册序=拓扑序）
        .plugin(nexterm_lib::plugins::boards::home::features::focus::init())
        // 阶段3 批次2a-1：L1 boards.knowledge（批C3 起拆 4 个 L2 子插件，见下）
        .plugin(nexterm_lib::plugins::boards::knowledge::init())
        // 批C3：知识库 4 个 L2 子插件（手稿 20260926；拓扑序 L1 → 4 子插件；
        // templates 须在 L1 后：归属迁移 UPDATE 依赖 L1 基线已登记的 kb_templates 行）
        .plugin(nexterm_lib::plugins::boards::knowledge::features::material::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::learning::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::templates::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::graph::init())
        // 批C3：知识库 8 个纯前端 L2 子插件（无命令无表，拓扑序 L1 → 子插件）
        .plugin(nexterm_lib::plugins::boards::knowledge::features::browse::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::search::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::tags::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::media::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::editors::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::import::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::history::init())
        .plugin(nexterm_lib::plugins::boards::knowledge::features::ai::init())
        // 阶段3 批次2b-1：L1 boards.ai → L2 ai.models / ai.agent / ai.groupchat（注册序=拓扑序）
        .plugin(nexterm_lib::plugins::boards::ai::init())
        .plugin(nexterm_lib::plugins::boards::ai::features::models::init())
        // batchC1：ai.sessions（AI会话必备子插件，注册序=拓扑序：L1 → models → sessions）
        .plugin(nexterm_lib::plugins::boards::ai::features::sessions::init())
        .plugin(nexterm_lib::plugins::boards::ai::features::agent::init())
        .plugin(nexterm_lib::plugins::boards::ai::features::groupchat::init())
        // 本轮：AI 会话纯前端 L2 子插件（对话/多模型对比/群聊编排/提示词模板）
        .plugin(nexterm_lib::plugins::boards::ai::features::chat::init())
        .plugin(nexterm_lib::plugins::boards::ai::features::multimodel::init())
        .plugin(nexterm_lib::plugins::boards::ai::features::orchestration::init())
        .plugin(nexterm_lib::plugins::boards::ai::features::prompts::init())
        .plugin(nexterm_lib::plugins::customs::auth::init())
        // 认证「一切皆插件」：4 个纯前端视图 L2 子插件（login 必备）
        .plugin(nexterm_lib::plugins::customs::auth::features::login::init())
        .plugin(nexterm_lib::plugins::customs::auth::features::register::init())
        .plugin(nexterm_lib::plugins::customs::auth::features::recovery::init())
        .plugin(nexterm_lib::plugins::customs::auth::features::temp::init())
        // 阶段3 批次3a：boards.terminal（L1 直接持 29 命令 + 6 表；terminal.linux 插槽 3b 挂载）
        .plugin(nexterm_lib::plugins::boards::terminal::init())
        // 阶段3 批次3b：terminal.linux（L2，Linux 子系统/Docker 25 命令，短码 lx）
        .plugin(nexterm_lib::plugins::boards::terminal::features::linux::init())
        // 阶段3 批次3c：terminal.yuancode（Yuan Code 编辑器核心 54 命令，拓扑序 L1 → linux → yuancode）
        .plugin(nexterm_lib::plugins::boards::terminal::features::yuancode::init())
        // YuanCode 三级嵌套子插件（L3，纯前端；拓扑序 L2 yuancode → 6 个子插件）
        .plugin(nexterm_lib::plugins::boards::terminal::features::yuancode::features::editor::init())
        .plugin(nexterm_lib::plugins::boards::terminal::features::yuancode::features::agent::init())
        .plugin(nexterm_lib::plugins::boards::terminal::features::yuancode::features::git::init())
        .plugin(nexterm_lib::plugins::boards::terminal::features::yuancode::features::skills::init())
        .plugin(nexterm_lib::plugins::boards::terminal::features::yuancode::features::sandbox::init())
        .plugin(nexterm_lib::plugins::boards::terminal::features::yuancode::features::settings::init())
        // 批C2：terminal.manual（命令手册 5 路由，纯前端无命令；拓扑序 L1 → linux → yuancode → manual）
        .plugin(nexterm_lib::plugins::boards::terminal::features::manual::init())
        // 批C4：terminal.console / terminal.mux / terminal.tools（纯前端 L2，无命令）
        .plugin(nexterm_lib::plugins::boards::terminal::features::console::init())
        .plugin(nexterm_lib::plugins::boards::terminal::features::mux::init())
        .plugin(nexterm_lib::plugins::boards::terminal::features::tools::init())
        // 阶段3 批次4a-1：boards.xin（L1 基础面 28 命令 + 6 表；拓扑序 L1 → wellness → realtime）
        .plugin(nexterm_lib::plugins::boards::xin::init())
        // 阶段3 批次4a-1：xin.wellness（L2，健康助手 17 命令，短码 xw）
        .plugin(nexterm_lib::plugins::boards::xin::features::wellness::init())
        // 阶段3 批次4a-1：xin.realtime（L2，实时语音对话 4 命令，短码 xr）
        .plugin(nexterm_lib::plugins::boards::xin::features::realtime::init())
        // 阶段3 批次4a-2：xin.orchestration（L2，编排面 87 命令，短码 xo）
        .plugin(nexterm_lib::plugins::boards::xin::features::orchestration::init())
        // 小欣「一切皆插件」：12 个纯前端面板 L2 子插件（chat 必备）
        .plugin(nexterm_lib::plugins::boards::xin::features::chat::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::memory::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::mood::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::briefing::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::compaction::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::dream::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::checkpoint::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::search::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::review::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::skill::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::tool::init())
        .plugin(nexterm_lib::plugins::boards::xin::features::evolution::init())
        // 阶段3 批次4b：boards.game（L1 44 条游戏命令 + 17 表）
        .plugin(nexterm_lib::plugins::boards::game::init())
        // game 纯前端 L2 子插件（无命令无表）
        .plugin(nexterm_lib::plugins::boards::game::features::preview::init())
        .plugin(nexterm_lib::plugins::boards::game::features::play3d::init())
        .plugin(nexterm_lib::plugins::boards::game::features::mapping::init())
        // 阶段3 批次4c：customs.systemtools（L1 10 system 命令 + 1 表；
        // 裁定 T2/T3 删除 extension 2 + adapter 3 零消费命令）
        .plugin(nexterm_lib::plugins::customs::systemtools::init())
        // 阶段4 批次6a：customs.recycle（L1 7 recycle 命令 + 1 表）
        .plugin(nexterm_lib::plugins::customs::recycle::init())
        // 回收站「一切皆插件」：2 个纯前端视图 L2 子插件（list 必备）
        .plugin(nexterm_lib::plugins::customs::recycle::features::list::init())
        .plugin(nexterm_lib::plugins::customs::recycle::features::actions::init())
        // 阶段4 批次6b：customs.sync（21 sync 命令 + 2 表） + customs.search（15 search 命令）
        .plugin(nexterm_lib::plugins::customs::sync::init())
        // 同步「一切皆插件」：2 个纯前端视图 L2 子插件（devices 必备）
        .plugin(nexterm_lib::plugins::customs::sync::features::devices::init())
        .plugin(nexterm_lib::plugins::customs::sync::features::conflicts::init())
        .plugin(nexterm_lib::plugins::customs::search::init())
        // search 纯前端 L2 子插件（无命令无表）
        .plugin(nexterm_lib::plugins::customs::search::features::browser::init())
        .plugin(nexterm_lib::plugins::customs::search::features::global::init())
        .plugin(nexterm_lib::plugins::customs::search::features::bookmarks::init())
        // 阶段4 批次6c：customs.intelligence（74 intelligence 命令 + 4 表）
        .plugin(nexterm_lib::plugins::customs::intelligence::init())
        // intelligence 纯前端 L2 子插件（无命令无表）
        .plugin(nexterm_lib::plugins::customs::intelligence::features::dashboard::init())
        .plugin(nexterm_lib::plugins::customs::intelligence::features::suggestions::init())
        .plugin(nexterm_lib::plugins::customs::intelligence::features::behavior::init())
        .plugin(nexterm_lib::plugins::customs::intelligence::features::activity::init())
        .plugin(nexterm_lib::plugins::customs::intelligence::features::settings::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(move |app| {
            // A1 §2.5 启动链路测量：记录 setup 闭包耗时（不含 Tauri Builder 构建）
            let setup_start = std::time::Instant::now();

            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("无法获取应用数据目录: {}", e))?;

            fs::create_dir_all(&data_dir).map_err(|e| format!("无法创建应用数据目录: {}", e))?;

            let db_path = data_dir.join("nexterm.db");
            let db_path_str = db_path.to_string_lossy().to_string();

            let rt =
                tokio::runtime::Runtime::new().map_err(|e| format!("无法创建异步运行时: {}", e))?;

            // A1 §2.5 阶段 1：数据库初始化（连接池 + 迁移 + 权限种子）
            let app_state = rt.block_on(async {
                let t = std::time::Instant::now();
                let state = AppState::new(&db_path_str, data_dir)
                    .await
                    .map_err(|e| format!("无法初始化数据库连接: {}", e))?;
                migrations::run_migrations(&state.pool)
                    .await
                    .map_err(|e| format!("数据库迁移失败: {}", e))?;
                permission_repo::init_default_permissions(&state.pool)
                    .await
                    .map_err(|e| format!("初始化默认权限数据失败: {}", e))?;
                tracing::info!("[startup] 阶段1 数据库初始化: {:?}", t.elapsed());
                Ok::<_, String>(state)
            })?;

            // A1 §2.5 阶段 2：服务注册（9 个无状态服务，纯内存操作）
            let t = std::time::Instant::now();
            let pool = app_state.pool.clone();
            let mek_manager = app_state.mek_manager.clone();
            app.manage(app_state);
            app.manage(AgentService::new());
            app.manage(SandboxService::new());
            app.manage(SafetyService::new());
            app.manage(McpService::new());
            app.manage(SkillService::new());
            app.manage(CompactService::new());
            app.manage(IoControlService::new());
            app.manage(PromptService::new());
            app.manage(GitService::new());
            // D1 v3.1 Task 3.1: Agent v3 进程内注册表（按 agent_id 索引）
            app.manage(agent_v3_commands::new_agent_registry());
            tracing::info!("[startup] 阶段2 服务注册: {:?}", t.elapsed());

            // A1 §2.5 阶段 2.5 + 阶段 3 并行化（v1.51.3 性能优化 Phase 2 Task 2.1.1）
            // 两者均仅依赖 pool，无相互依赖，可用 tokio::join! 并行执行
            // 原实现为两次 block_on 串行（MCP恢复 → GoalService初始化），现合并为单次 block_on + join!
            let t = std::time::Instant::now();
            let mcp_service = app.state::<McpService>();
            let goal_service = rt.block_on(async {
                // 阶段 2.5：D1.6 MCP 持久化服务器自动恢复
                // 从数据库加载已启用的 MCP 服务器配置，注册到运行时内存
                // 多用户隔离（批次 6）：启动时无登录用户，加载 admin（user_id=1）的配置以保持向后兼容；
                // 用户登录后可通过 yuan_mcp_load_persisted_servers 加载自己的配置
                let mcp_restore = async {
                    match mcp_repo::load_enabled_servers(&pool, 1).await {
                        Ok(servers) => {
                            let total = servers.len();
                            let mut loaded = 0;
                            for server in servers {
                                if mcp_service.register_server(server, true).await.is_ok() {
                                    loaded += 1;
                                }
                            }
                            tracing::info!(
                                "[startup] 阶段2.5 D1.6 恢复 {}/{} 个 MCP 服务器",
                                loaded,
                                total
                            );
                        }
                        Err(e) => {
                            tracing::warn!("[startup] 阶段2.5 D1.6 MCP 服务器恢复失败: {}", e);
                        }
                    }
                };

                // 阶段 3：GoalService 初始化（建表 + 缓存加载 + 事件监听）
                let goal_init = async {
                    let gs = GoalService::new();
                    gs.initialize(pool.clone()).await;
                    gs
                };

                // 并行执行：两个 future 在同一 task 上交替轮询（无数据竞争）
                let (_, gs) = tokio::join!(mcp_restore, goal_init);
                gs
            });
            app.manage(goal_service);
            tracing::info!(
                "[startup] 阶段2.5+3 并行完成（MCP恢复 + GoalService初始化）: {:?}",
                t.elapsed()
            );

            // A1 §2.5 阶段 4：引擎 + LSP + 工具注册
            let t = std::time::Instant::now();
            app.manage(EngineState {
                engine: Arc::new(Mutex::new(YuanEngine::new(EngineConfig::default()))),
            });
            app.manage(LspState {
                manager: Arc::new(Mutex::new(LspManager::new())),
            });

            // 行内补全服务（共享 LSP 管理器 + 统一模型管理 + MEK）
            // D1.1：接入 AiModelService，让 inline 补全真正调用云端 API 模型
            {
                let inline_lsp = Arc::new(Mutex::new(LspManager::new()));
                app.manage(InlineService::new(
                    inline_lsp,
                    pool.clone(),
                    mek_manager.clone(),
                ));
            }

            // 工具注册中心
            app.manage(tools_commands::ToolState {
                registry: Arc::new(Mutex::new(ToolRegistry::new())),
                plan_tool: Arc::new(Mutex::new(PlanTool::new())),
                patch_tool: Arc::new(Mutex::new(ApplyPatchTool::new())),
            });
            tracing::info!("[startup] 阶段4 引擎+LSP+工具: {:?}", t.elapsed());

            // A1 §2.5 阶段 5：后台调度器启动（均已 tokio::spawn 异步化，不阻塞）
            let t = std::time::Instant::now();
            // T2.8 自动备份系统：启动后台调度器（hourly + daily）
            // 紧急备份由 T2.10 health_check 模块按需触发
            let scheduler_pool = pool.clone();
            let scheduler_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("无法获取应用数据目录: {}", e))?;
            backup_scheduler::start_backup_scheduler(scheduler_pool, scheduler_data_dir.clone());

            // T2.10.3 健康检查调度器：启动后 60s 执行首次检查 + 每日定时检查
            // Critical 状态自动触发紧急备份
            let health_check_pool = pool.clone();
            backup_scheduler::start_health_check_scheduler(health_check_pool, scheduler_data_dir);

            // T2.13 MEK 密钥轮换调度器：启动后 5 分钟首次检查 + 每 24 小时定时
            // 自动轮换超过 90 天周期的 MEK
            // 阶段3 批次1a-2a：调度器随 crypto/MEK 一并收编至 customs.auth 插件私有实现。
            let mek_rotation_pool = pool.clone();
            start_mek_rotation_scheduler(mek_rotation_pool);

            // A5 Phase 3 Task 1: 网络连通性检测后台任务
            // 启动后立即探测一次，之后每 30s 探测一次；状态变更时 emit "network-status-changed"
            let app_state_ref = app.state::<AppState>();
            app_state_ref
                .connectivity_checker
                .start_background_task(app.handle().clone());

            // spec ai-chat-enhancement Phase 1 §1.2: 模型健康检测后台任务
            // 启动后立即检测一次，之后每 15 分钟检测一次（可由 set_health_check_interval 调整）；
            // 每个模型检测完成时 emit "ai-model-health-changed"，连续 3 次失败升级为 error。
            // user_id 兜底为系统用户 ID=1（启动时用户可能尚未登录）；
            // 后续每个周期会从 current_user 读取最新值（见 monitor::start_background_task）。
            app_state_ref.model_health_monitor.start_background_task(
                app.handle().clone(),
                pool.clone(),
                mek_manager.clone(),
                nexterm_lib::plugins::_legacy::services::model_health_monitor::SYSTEM_USER_ID,
            );
            tracing::info!("[startup] 阶段5 调度器启动: {:?}", t.elapsed());

            // A1 §2.5 启动总耗时汇总
            let setup_elapsed = setup_start.elapsed();
            let boot_elapsed = boot_start.elapsed();
            tracing::info!(
                "[startup] ✅ setup 总耗时: {:?} | boot 总耗时（含 Tauri Builder）: {:?}",
                setup_elapsed,
                boot_elapsed
            );

            // A1.6 启动并行化优化：将 perf_metrics 写入改为非阻塞 spawn
            // 原实现用 block_on 同步等待 2 个 INSERT 完成（~2ms），改为 spawn 后台执行
            // 安全性：性能监控数据，延迟写入/写入失败均不影响业务（原 block_on 也是静默忽略错误）
            // A1 §2.5 性能基线采集：前端可通过 perf_get_slow_queries 或直接查 perf_metrics 获取历史趋势
            let setup_ms = setup_elapsed.as_millis() as i64;
            let boot_ms = boot_elapsed.as_millis() as i64;
            let perf_pool = pool.clone();
            rt.spawn(async move {
                let recorded_at = chrono::Utc::now().to_rfc3339();
                let metadata = format!(r#"{{"boot_ms":{}}}"#, boot_ms);
                let _ = sqlx::query(
                    "INSERT INTO perf_metrics (metric_name, metric_value_ms, recorded_at, metadata)
                     VALUES (?, ?, ?, ?)",
                )
                .bind("startup_setup_ms")
                .bind(setup_ms)
                .bind(&recorded_at)
                .bind(&metadata)
                .execute(&perf_pool)
                .await;
                let _ = sqlx::query(
                    "INSERT INTO perf_metrics (metric_name, metric_value_ms, recorded_at)
                     VALUES (?, ?, ?)",
                )
                .bind("startup_boot_ms")
                .bind(boot_ms)
                .bind(&recorded_at)
                .execute(&perf_pool)
                .await;
                tracing::info!(
                    "[startup] 启动耗时已写入 perf_metrics: setup={}ms, boot={}ms",
                    setup_ms,
                    boot_ms
                );
            });

            tracing::info!("NexTerm·元界 后端启动成功");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
