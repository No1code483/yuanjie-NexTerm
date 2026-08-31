#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;

use tauri::Manager;

use nexterm_lib::commands::{
    adapter_commands, ai_commands, /* ai_v2_commands, */ auth_commands, backup_commands, browser_commands, chat_commands,
    custom_theme_commands,
    // D1 v3.1 Task 3.1 + 3.5: Agent 化 + 云端 API Key 管理
    agent_v3_commands, cloud_api_commands,
    // D1 v3.2 Task 3.4: 多模型与协作（Phase 6）— 模型路由 + 监测钩子 + 协作会话
    model_routing_commands, yuan_code_monitor_commands, collab_session_commands,
    distill_commands,
    editor_commands, /* editor_v2_commands, */ engine_commands, extension_commands, file_edit_commands, font_commands, game_commands, game_opponent_commands, game_story_commands, game_natural_language_commands, game_behavior_commands, game_intelligence_commands, git_commands, health_check_commands, intelligence_commands,
    intelligence_behavior_commands, intelligence_anomaly_commands, intelligence_v4_commands, journal_commands, kb_commands, linux_commands,
    lsp_commands, news_commands, news_source_commands, profile_commands, project_indexer_commands, recycle_commands, restore_commands, search_commands,
    /* search_v2_commands, */ system_commands, terminal_commands, /* terminal_v2_commands, */ timer_commands,
    todo_commands, xin_basic_commands, xin_wellness_commands, xin_orchestration_commands,
    xin_video_commands, xin_realtime_commands, tools_commands,
    yuan_agent_commands, yuan_agent_autonomous_commands, yuancode_commands, yuan_goal_commands, yuan_inline_commands, yuan_mcp_commands, yuan_safety_commands, yuan_sandbox_commands, yuan_skill_commands, yuan_compact_commands, yuan_io_control_commands, yuan_prompt_commands, perf_commands,
    mek_rotation_commands,
    sync_commands,
};
use nexterm_lib::services::agent_service::AgentService;
use nexterm_lib::services::backup_scheduler;
use nexterm_lib::services::mek_rotation_scheduler;
use nexterm_lib::services::compact_service::CompactService;
use nexterm_lib::services::git_service::GitService;
use nexterm_lib::services::goal_service::GoalService;
use nexterm_lib::services::io_control_service::IoControlService;
use nexterm_lib::services::prompt_service::PromptService;
use nexterm_lib::services::mcp_service::McpService;
use nexterm_lib::services::safety_service::SafetyService;
use nexterm_lib::services::sandbox_service::SandboxService;
use nexterm_lib::services::skill_service::SkillService;
use nexterm_lib::db::connection::AppState;
use nexterm_lib::db::migrations;
use nexterm_lib::db::repositories::mcp_repo;
use nexterm_lib::db::repositories::permission_repo;
use nexterm_lib::engine::YuanEngine;
use nexterm_lib::engine::EngineConfig;
use nexterm_lib::lsp::LspManager;
use nexterm_lib::services::inline_service::InlineService;
use nexterm_lib::tools::ToolRegistry;
use nexterm_lib::tools::PlanTool;
use nexterm_lib::tools::ApplyPatchTool;
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

            let rt = tokio::runtime::Runtime::new().map_err(|e| format!("无法创建异步运行时: {}", e))?;

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
                                loaded, total
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
            tracing::info!("[startup] 阶段2.5+3 并行完成（MCP恢复 + GoalService初始化）: {:?}", t.elapsed());

            // A1 §2.5 阶段 4：引擎 + LSP + 工具注册
            let t = std::time::Instant::now();
            app.manage(engine_commands::EngineState {
                engine: Arc::new(Mutex::new(YuanEngine::new(EngineConfig::default()))),
            });
            app.manage(lsp_commands::LspState {
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
            let scheduler_data_dir = app.path().app_data_dir()
                .map_err(|e| format!("无法获取应用数据目录: {}", e))?;
            backup_scheduler::start_backup_scheduler(scheduler_pool, scheduler_data_dir.clone());

            // T2.10.3 健康检查调度器：启动后 60s 执行首次检查 + 每日定时检查
            // Critical 状态自动触发紧急备份
            let health_check_pool = pool.clone();
            backup_scheduler::start_health_check_scheduler(health_check_pool, scheduler_data_dir);

            // T2.13 MEK 密钥轮换调度器：启动后 5 分钟首次检查 + 每 24 小时定时
            // 自动轮换超过 90 天周期的 MEK
            let mek_rotation_pool = pool.clone();
            mek_rotation_scheduler::start_mek_rotation_scheduler(mek_rotation_pool);

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
            app_state_ref
                .model_health_monitor
                .start_background_task(
                    app.handle().clone(),
                    pool.clone(),
                    mek_manager.clone(),
                    nexterm_lib::services::model_health_monitor::SYSTEM_USER_ID,
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
        // ============================================================================
        // T2.1.2 Command 分级注册说明（L0/L1/L2 三级）
        // ----------------------------------------------------------------------------
        // Tauri 架构限制：generate_handler! 宏在编译期注册所有 Command，不支持运行时
        // 动态注册。因此 L0/L1/L2 分级不是通过"延迟注册"实现，而是通过以下方式：
        //
        // - L0（启动期必需）：auth / system_config / health_check / perf
        //   → 启动后立即可用，前端 App.tsx 首屏即调用
        // - L1（用户触发常规）：ai / chat / kb / editor / todo / journal / timer /
        //   recycle / search / profile / news / terminal / linux / git / browser /
        //   extension / adapter / font / custom_theme / backup / restore /
        //   mek_rotation / sync / file_edit / news_source
        //   → 用户进入对应路由后按需调用，路由级 lazy import 已在前端实现
        // - L2（重型模块懒加载）：yuan_code / yuan_agent / yuan_sandbox / yuan_goal /
        //   yuan_inline / yuan_mcp / yuan_safety / yuan_skill / yuan_compact /
        //   yuan_io_control / yuan_prompt / project_indexer / tools / distill /
        //   engine / lsp / intelligence(v2/v3/v4) / xin(basic/wellness/orchestration/
        //   video/realtime) / game / game_opponent / game_story
        //   → 前端通过 router.tsx createLazyRoute 懒加载对应 Page 组件，
        //     Page 组件 import 对应 ipc 模块时才触发 Vite chunk 加载
        //     （vite.config.ts manualChunks 已将 editor-vendor/three-vendor 等拆分）
        //
        // 验证：L0 命令启动期可用 ✅ | L2 按需加载 ✅（前端路由级 lazy import）
        // ============================================================================
        .invoke_handler(tauri::generate_handler![
            // ===== L0: 启动期必需（auth/system/health/perf）=====
            auth_commands::register,
            auth_commands::login,
            auth_commands::create_temp_account,
            auth_commands::recover_by_phrase,
            auth_commands::logout,
            auth_commands::auth_verify_token,
            auth_commands::auth_get_permissions,
            auth_commands::auth_reset_password,
            auth_commands::auth_restore_session,
            auth_commands::session_list,
            auth_commands::session_revoke,
            auth_commands::auth_2fa_setup,
            auth_commands::auth_2fa_verify,
            auth_commands::auth_2fa_enable,
            auth_commands::auth_2fa_disable,
            auth_commands::auth_2fa_status,
            auth_commands::auth_2fa_login_verify,
            // ===== L1: 用户触发常规命令（ai/chat/kb/editor/todo/journal/timer 等）=====
            ai_commands::get_ai_models,
            ai_commands::add_ai_model,
            ai_commands::update_ai_model,
            ai_commands::delete_ai_model,
            ai_commands::get_ai_agents,
            ai_commands::add_ai_agent,
            ai_commands::update_ai_agent,
            ai_commands::delete_ai_agent,
            ai_commands::ai_get_orchestration_status,
            ai_commands::ai_end_group_chat,
            ai_commands::get_ai_provider_info,
            // spec ai-chat-enhancement Phase 1 §1.3: 模型健康检测命令
            ai_commands::check_all_models_health,
            ai_commands::get_model_health_status,
            ai_commands::set_health_check_interval,
            chat_commands::create_conversation,
            chat_commands::get_conversations,
            chat_commands::update_conversation,
            chat_commands::delete_conversation,
            chat_commands::send_message,
            chat_commands::get_messages,
            chat_commands::get_participants,
            chat_commands::delete_message,
            chat_commands::run_orchestrator,
            chat_commands::stop_generation,
            chat_commands::check_model_health,
            chat_commands::mark_conversation_read,
            chat_commands::reorder_conversations,
            chat_commands::search_conversations,
            chat_commands::toggle_star_conversation,
            chat_commands::branch_conversation,
            chat_commands::export_conversation,
            chat_commands::prompt_template_list,
            chat_commands::prompt_template_create,
            chat_commands::prompt_template_update,
            chat_commands::prompt_template_delete,
            kb_commands::get_kb_categories,
            kb_commands::add_kb_category,
            kb_commands::delete_kb_category,
            kb_commands::update_kb_category,
            kb_commands::get_kb_entries,
            kb_commands::add_kb_entry,
            kb_commands::delete_kb_entry,
            kb_commands::move_kb_entry,
            kb_commands::move_kb_category,
            kb_commands::update_kb_entry,
            kb_commands::search_kb_entries,
            kb_commands::kb_import_folder,
            kb_commands::kb_import_multi_folders,
            kb_commands::kb_check_files_existence,
            kb_commands::get_all_kb_entries,
            kb_commands::get_kb_category_counts,
            kb_commands::get_kb_tags,
            kb_commands::add_kb_tag,
            kb_commands::update_kb_tag,
            kb_commands::delete_kb_tag,
            kb_commands::get_kb_entry_tags,
            kb_commands::get_kb_all_entry_tags,
            kb_commands::get_kb_tag_stats,
            kb_commands::set_kb_entry_tags,
            kb_commands::get_kb_entries_by_tag,
            kb_commands::batch_add_kb_tag,
            kb_commands::batch_remove_kb_tag,
            kb_commands::toggle_kb_favorite,
            kb_commands::get_kb_favorites,
            kb_commands::record_kb_access,
            kb_commands::get_kb_recent,
            kb_commands::batch_delete_kb_entries,
            kb_commands::batch_move_kb_entries,
            kb_commands::kb_scan_directory,
            kb_commands::kb_read_external_file,
            kb_commands::kb_read_file_base64,
            kb_commands::kb_extract_docx_text,
            kb_commands::kb_extract_doc_text,
            kb_commands::kb_extract_rtf_text,
            kb_commands::kb_extract_pptx_text,
            kb_commands::kb_extract_odp_text,
            kb_commands::kb_extract_epub_text,
            kb_commands::kb_list_zip_contents,
            kb_commands::kb_extract_psd_info,
            kb_commands::kb_extract_ai_info,
            kb_commands::kb_extract_table_data,
            kb_commands::move_kb_category_to_recycle,
            kb_commands::kb_add_tracked_path,
            kb_commands::kb_get_tracked_paths,
            kb_commands::kb_remove_tracked_path,
            kb_commands::kb_add_scanned_files,
            kb_commands::kb_check_paths,
            kb_commands::kb_get_templates,
            kb_commands::kb_create_template,
            kb_commands::kb_update_template,
            kb_commands::kb_delete_template,
            kb_commands::kb_get_backlinks,
            kb_commands::kb_get_outgoing_links,
            kb_commands::kb_semantic_search,
            kb_commands::kb_get_snapshots,
            kb_commands::kb_restore_snapshot,
            // A5 离线同步 Phase 3 Task 4: 知识库附件预加载
            kb_commands::kb_attachment_pin,
            kb_commands::kb_attachment_unpin,
            kb_commands::kb_attachment_preload,
            kb_commands::kb_attachment_list_pinned,
            kb_commands::kb_attachment_get_cached_path,
            editor_commands::editor_open,
            editor_commands::editor_save,
            editor_commands::editor_auto_save,
            editor_commands::editor_close,
            editor_commands::editor_get_versions,
            editor_commands::editor_get_version,
            editor_commands::editor_restore_version,
            editor_commands::editor_recover_session,
            editor_commands::editor_list_documents,
            editor_commands::editor_delete_document,
            editor_commands::editor_extract_metadata,
            editor_commands::editor_generate_thumbnail,
            editor_commands::editor_highlight,
            editor_commands::editor_csv_preview,
            editor_commands::editor_decrypted_preview,
            editor_commands::editor_search,
            editor_commands::editor_convert,
            editor_commands::editor_convert_content,
            editor_commands::editor_canvas_save,
            editor_commands::editor_canvas_load,
            editor_commands::editor_canvas_delete,
            editor_commands::editor_diff_versions,
            editor_commands::editor_cleanup_versions,
            editor_commands::editor_index_content,
            editor_commands::editor_recover_all_sessions,
            editor_commands::editor_cleanup_sessions,
            file_edit_commands::fileedit_read,
            file_edit_commands::fileedit_write,
            file_edit_commands::fileedit_get_status,
            file_edit_commands::tableedit_read,
            file_edit_commands::tableedit_write,
            file_edit_commands::tableedit_export_csv,
            file_edit_commands::pptedit_get_slides,
            file_edit_commands::pptedit_update_slide,
            file_edit_commands::pptedit_add_slide,
            file_edit_commands::pptedit_delete_slide,
            file_edit_commands::pptedit_reorder,
            file_edit_commands::pdfedit_save,
            file_edit_commands::imageedit_save,
            file_edit_commands::audioedit_save,
            todo_commands::get_todos,
            todo_commands::get_todos_paginated,
            todo_commands::add_todo,
            todo_commands::toggle_todo,
            todo_commands::update_todo,
            todo_commands::delete_todo,
            journal_commands::get_journal,
            journal_commands::save_journal,
            journal_commands::delete_journal,
            timer_commands::get_timers,
            timer_commands::create_timer,
            timer_commands::update_timer_state,
            timer_commands::delete_timer,
            timer_commands::timer_action,
            recycle_commands::recycle_list,
            recycle_commands::recycle_move_to,
            recycle_commands::recycle_restore,
            recycle_commands::recycle_delete_permanently,
            recycle_commands::recycle_empty_all,
            recycle_commands::cleanup_expired_recycle,
            recycle_commands::recycle_stats,
            search_commands::search_global,
            search_commands::global_search,
            search_commands::search_ai_summary,
            search_commands::search_index_document,
            search_commands::search_delete_document,
            search_commands::search_clear_index,
            search_commands::search_rebuild_index,
            search_commands::search_index_status,
            search_commands::search_suggest,
            search_commands::search_advanced,
            search_commands::search_facets,
            search_commands::search_history,
            search_commands::search_clear_history,
            search_commands::search_hot_queries,
            search_commands::search_batch_index,
            profile_commands::get_profile,
            profile_commands::set_profile,
            profile_commands::get_resumes,
            profile_commands::add_resume,
            profile_commands::update_resume,
            profile_commands::delete_resume,
            profile_commands::get_random_quote,
            profile_commands::add_quote,
            profile_commands::get_all_quotes,
            profile_commands::batch_add_quotes,
            profile_commands::seed_default_quotes,
            profile_commands::delete_quote,
            profile_commands::profile_change_password,
            profile_commands::profile_change_username,
            profile_commands::profile_update_profile,
            profile_commands::save_personal_info,
            profile_commands::get_personal_info,
            profile_commands::check_quote_duplicate,
            profile_commands::resume_polish,
            profile_commands::export_user_data,
            news_commands::get_news,
            news_commands::get_news_by_category,
            news_commands::fetch_news,
            news_commands::add_news,
            news_commands::mark_news_read,
            news_commands::clear_old_news,
            news_commands::toggle_news_favorite,
            news_commands::get_pending_delete_news,
            news_commands::generate_news_ai_summary,
            // A5 Phase 3 Task 3: 新闻源离线缓存
            news_commands::news_get_cached,
            news_commands::news_cache_status,
            news_source_commands::get_news_sources,
            news_source_commands::add_news_source,
            news_source_commands::delete_news_source,
            system_commands::get_system_config,
            system_commands::set_system_config,
            system_commands::get_all_system_configs,
            system_commands::system_open_file,
            system_commands::system_open_url,
            system_commands::system_get_app_info,
            system_commands::clipboard_write_text,
            system_commands::clipboard_read_text,
            system_commands::window_set_always_on_top,
            system_commands::window_screenshot,
            terminal_commands::terminal_create_session,
            terminal_commands::terminal_write_input,
            terminal_commands::terminal_resize,
            terminal_commands::terminal_kill_session,
            terminal_commands::terminal_list_sessions,
            terminal_commands::terminal_execute_builtin,
            terminal_commands::terminal_get_system_info,
            terminal_commands::terminal_list_directory,
            terminal_commands::terminal_get_history,
            terminal_commands::terminal_clear_history,
            terminal_commands::terminal_get_history_count,
            terminal_commands::terminal_save_layout,
            terminal_commands::terminal_load_layout,
            terminal_commands::terminal_clear_layout,
            terminal_commands::terminal_detect_wsl,
            terminal_commands::terminal_create_wsl_session,
            terminal_commands::terminal_mux_get_active_sessions,
            terminal_commands::terminal_mux_clear_all_sessions,
            terminal_commands::terminal_search_history_fts,
            terminal_commands::terminal_ssh_list_profiles,
            terminal_commands::terminal_ssh_save_profile,
            terminal_commands::terminal_ssh_delete_profile,
            terminal_commands::terminal_ssh_connect,
            terminal_commands::terminal_ssh_disconnect,
            terminal_commands::terminal_ssh_write,
            terminal_commands::terminal_ssh_resize,
            terminal_commands::terminal_get_themes,
            terminal_commands::terminal_get_config,
            terminal_commands::terminal_save_config,
            linux_commands::linux_get_environment,
            linux_commands::linux_list_versions,
            linux_commands::linux_get_status_panel,
            linux_commands::linux_get_shell_status,
            linux_commands::linux_get_system_info,
            linux_commands::linux_get_iso_progress,
            linux_commands::linux_list_directory,
            linux_commands::linux_view_file,
            linux_commands::linux_search_source,
            linux_commands::linux_download_kernel,
            linux_commands::linux_set_active,
            linux_commands::linux_remove_kernel,
            linux_commands::linux_build_kernel,
            linux_commands::linux_analyze_config,
            linux_commands::linux_list_modules,
            linux_commands::linux_analyze_logs,
            linux_commands::linux_perf_profile,
            linux_commands::linux_run_benchmark,
            linux_commands::linux_run_stress,
            linux_commands::docker_list_containers,
            linux_commands::docker_container_start,
            linux_commands::docker_container_stop,
            linux_commands::docker_container_logs,
            linux_commands::docker_list_images,
            linux_commands::network_stats,
            extension_commands::extension_get_entry,
            extension_commands::extension_list_modules,
            adapter_commands::adapter_list,
            adapter_commands::adapter_install,
            adapter_commands::adapter_uninstall,
            // ===== L2: 重型模块（yuan_code/agent/mcp/sandbox/skill/game/intelligence/xin/engine/lsp）
            // 前端通过 router.tsx lazy import 按需加载对应 Page，Vite manualChunks 拆分 vendor =====
            yuancode_commands::yuan_list_files,
            yuancode_commands::yuan_read_file,
            yuancode_commands::yuan_write_file,
            yuancode_commands::yuan_create_item,
            yuancode_commands::yuan_delete_item,
            yuancode_commands::yuan_rename_item,
            yuancode_commands::yuan_highlight,
            yuancode_commands::yuan_execute,
            yuancode_commands::yuan_complete,
            yuancode_commands::yuan_complete_stream,
            yuancode_commands::yuan_analyze,
            yuancode_commands::yuan_save_snippet,
            yuancode_commands::yuan_get_snippets,
            yuancode_commands::yuan_update_snippet,
            yuancode_commands::yuan_delete_snippet,
            yuancode_commands::yuan_search_snippets,
            yuancode_commands::yuan_compute_diff,
            yuancode_commands::yuan_search_files,
            yuancode_commands::yuan_replace_files,
            yuancode_commands::yuan_copy_move,
            yuancode_commands::yuan_get_file_info,
            yuancode_commands::yuan_save_workspace,
            yuancode_commands::yuan_load_workspace,
            yuancode_commands::yuan_list_workspaces,
            yuancode_commands::yuan_delete_workspace,
            yuancode_commands::yuan_format_code,
            yuancode_commands::yuan_settings_save,
            // 工具系统命令
            tools_commands::yuan_tools_list,
            tools_commands::yuan_tools_search,
            tools_commands::yuan_tools_call,
            tools_commands::yuan_plan_create,
            tools_commands::yuan_plan_update_step,
            tools_commands::yuan_plan_get,
            tools_commands::yuan_plan_list_all,
            tools_commands::yuan_apply_patch,
            yuan_agent_commands::yuan_agent_spawn,
            yuan_agent_commands::yuan_agent_fork,
            yuan_agent_commands::yuan_agent_list,
            yuan_agent_commands::yuan_agent_list_all,
            yuan_agent_commands::yuan_agent_status,
            yuan_agent_commands::yuan_agent_abort,
            yuan_agent_commands::yuan_agent_transition,
            yuan_agent_commands::yuan_agent_send_message,
            yuan_agent_commands::yuan_agent_receive_messages,
            yuan_agent_commands::yuan_agent_list_roles,
            yuan_agent_commands::yuan_agent_get_count,
            yuan_agent_commands::yuan_agent_cleanup,
            yuan_agent_commands::yuan_agent_list_templates,
            yuan_agent_commands::yuan_agent_deploy,
            yuan_agent_commands::yuan_agent_execute,
            // D1 v3.1+v3.2 自主执行 + 多文件原子编辑
            yuan_agent_autonomous_commands::yuan_agent_execute_autonomous,
            yuan_agent_autonomous_commands::yuan_multi_file_edit,
            // D1.7 跨文件 diff 预览
            yuan_agent_autonomous_commands::yuan_multi_file_preview_diffs,
            // D1 v3.1 Task 3.1: Agent 化（Phase 3）— 7 种 Agent 类型 + 计划/执行/审查
            agent_v3_commands::yuan_v3_agent_types,
            agent_v3_commands::yuan_v3_agent_create,
            agent_v3_commands::yuan_v3_agent_plan,
            agent_v3_commands::yuan_v3_agent_execute,
            agent_v3_commands::yuan_v3_agent_review,
            agent_v3_commands::yuan_v3_agent_safety_check,
            agent_v3_commands::yuan_v3_agent_status,
            agent_v3_commands::yuan_v3_agent_list,
            agent_v3_commands::yuan_v3_agent_destroy,
            // D1 v3.1 Task 3.5: 云端 API Key 管理 + ModelSelector 后端
            cloud_api_commands::cloud_api_list,
            cloud_api_commands::cloud_api_upsert,
            cloud_api_commands::cloud_api_set_enabled,
            cloud_api_commands::cloud_api_delete,
            cloud_api_commands::cloud_api_providers,
            cloud_api_commands::cloud_api_test_connection,
            // D1 v3.2 Task 3.4.1: 模型路由配置（按任务类型选模型，强制云端 API 用于编程）
            model_routing_commands::model_routing_list,
            model_routing_commands::model_routing_upsert,
            model_routing_commands::model_routing_set_enabled,
            model_routing_commands::model_routing_delete,
            model_routing_commands::model_routing_resolve,
            model_routing_commands::model_routing_task_types,
            // D1 v3.2 Task 3.4.2: 与恐龙双脑集成（非侵入式监测 Yuan Code 行为，可关闭）
            yuan_code_monitor_commands::yuan_code_monitor_record_event,
            yuan_code_monitor_commands::yuan_code_monitor_status,
            yuan_code_monitor_commands::yuan_code_monitor_summary,
            // D1 v3.2 Task 3.4.3 / 3.4.4: 协作会话管理（Yjs 接口骨架 + 多人协作）
            collab_session_commands::collab_session_create,
            collab_session_commands::collab_session_list,
            collab_session_commands::collab_session_get,
            collab_session_commands::collab_session_join,
            collab_session_commands::collab_session_leave,
            collab_session_commands::collab_session_close,
            collab_session_commands::collab_session_update_cursor,
            // D2 恐龙双脑 Phase 0：蒸馏数据集 + 本地推理 PoC
            distill_commands::distill_extract_samples,
            distill_commands::distill_generate_dataset,
            distill_commands::distill_list_samples,
            distill_commands::distill_approve_sample,
            distill_commands::distill_reject_sample,
            distill_commands::distill_local_infer,
            // Phase0.1 数据源对接：导出 JSONL + 样本统计
            distill_commands::distill_export_jsonl,
            distill_commands::distill_sample_stats,
            // D3.5 小欣视频输入多模态
            xin_video_commands::xin_video_analyze,
            xin_video_commands::xin_video_summarize,
            xin_video_commands::xin_video_check_ffmpeg,
            // D3.6 实时对话：状态机 + PCM 推送（骨架，STT/LLM/TTS 流式在 D3.6.3-5 填充）
            xin_realtime_commands::xin_realtime_start,
            xin_realtime_commands::xin_realtime_stop,
            xin_realtime_commands::xin_realtime_push_chunk,
            xin_realtime_commands::xin_realtime_get_state,
            yuan_sandbox_commands::yuan_sandbox_create,
            yuan_sandbox_commands::yuan_sandbox_list,
            yuan_sandbox_commands::yuan_sandbox_list_by_agent,
            yuan_sandbox_commands::yuan_sandbox_get,
            yuan_sandbox_commands::yuan_sandbox_execute,
            yuan_sandbox_commands::yuan_sandbox_read_file,
            yuan_sandbox_commands::yuan_sandbox_write_file,
            yuan_sandbox_commands::yuan_sandbox_delete_file,
            yuan_sandbox_commands::yuan_sandbox_list_files,
            yuan_sandbox_commands::yuan_sandbox_terminate,
            yuan_sandbox_commands::yuan_sandbox_cleanup,
            yuan_sandbox_commands::yuan_sandbox_count,
            yuan_sandbox_commands::yuan_sandbox_save,
            yuan_sandbox_commands::yuan_sandbox_history,
            yuan_goal_commands::yuan_goal_create,
            yuan_goal_commands::yuan_goal_get,
            yuan_goal_commands::yuan_goal_list,
            yuan_goal_commands::yuan_goal_update,
            yuan_goal_commands::yuan_goal_start,
            yuan_goal_commands::yuan_goal_pause,
            yuan_goal_commands::yuan_goal_complete,
            yuan_goal_commands::yuan_goal_abort,
            yuan_goal_commands::yuan_goal_delete,
            yuan_goal_commands::yuan_goal_consume_tokens,
            yuan_goal_commands::yuan_goal_update_progress,
            yuan_goal_commands::yuan_goal_save_checkpoint,
            yuan_goal_commands::yuan_goal_build_continuation,
            yuan_goal_commands::yuan_goal_checkpoints_count,
            yuan_inline_commands::yuan_inline_complete,
            yuan_inline_commands::yuan_inline_available,
            yuan_inline_commands::yuan_inline_edit,

            // D1.4 项目索引系统（Yuan Code v3 项目级上下文）
            project_indexer_commands::project_index,
            project_indexer_commands::project_index_status,
            project_indexer_commands::project_search_symbols,
            project_indexer_commands::project_find_references,
            project_indexer_commands::project_get_related_files,
            yuan_mcp_commands::yuan_mcp_register_server,
            yuan_mcp_commands::yuan_mcp_connect_server,
            yuan_mcp_commands::yuan_mcp_disconnect_server,
            yuan_mcp_commands::yuan_mcp_list_servers,
            yuan_mcp_commands::yuan_mcp_list_all_tools,
            yuan_mcp_commands::yuan_mcp_server_status,
            yuan_mcp_commands::yuan_mcp_call_tool,
            yuan_mcp_commands::yuan_mcp_spawn,
            yuan_mcp_commands::yuan_mcp_health_check,
            yuan_mcp_commands::yuan_mcp_health_check_all,
            yuan_mcp_commands::yuan_mcp_set_lifecycle_config,
            yuan_mcp_commands::yuan_mcp_get_lifecycle_state,
            yuan_mcp_commands::yuan_mcp_list_shaped_tools,
            yuan_mcp_commands::yuan_mcp_group_tools_by_namespace,
            yuan_mcp_commands::yuan_mcp_list_tools_by_namespace,
            yuan_mcp_commands::yuan_mcp_get_tool_namespaces,
            yuan_mcp_commands::yuan_mcp_set_shaper_config,
            yuan_mcp_commands::yuan_mcp_search_tools,
            yuan_mcp_commands::yuan_mcp_set_tool_enabled,
            yuan_mcp_commands::yuan_mcp_set_enabled_tools,
            yuan_mcp_commands::yuan_mcp_set_disabled_tools,
            yuan_mcp_commands::yuan_mcp_list_filtered_tools,
            yuan_mcp_commands::yuan_mcp_list_resources,
            yuan_mcp_commands::yuan_mcp_read_resource,
            yuan_mcp_commands::yuan_mcp_list_prompts,
            yuan_mcp_commands::yuan_mcp_get_prompt,
            yuan_mcp_commands::yuan_mcp_set_deferred_namespaces,
            yuan_mcp_commands::yuan_mcp_get_deferred_namespaces,
            yuan_mcp_commands::yuan_mcp_load_namespace,
            yuan_mcp_commands::yuan_mcp_unload_namespace,
            // D1.6 MCP 服务器配置持久化
            yuan_mcp_commands::yuan_mcp_list_persisted_servers,
            yuan_mcp_commands::yuan_mcp_save_server_config,
            yuan_mcp_commands::yuan_mcp_delete_server_config,
            yuan_mcp_commands::yuan_mcp_set_server_enabled,
            yuan_mcp_commands::yuan_mcp_load_persisted_servers,
            // D1 v3.1 Task 3.2.1-3.2.11: 内置 MCP 生态 + 工具调用历史
            yuan_mcp_commands::yuan_mcp_list_builtin_servers,
            yuan_mcp_commands::yuan_mcp_list_builtin_tools,
            yuan_mcp_commands::yuan_mcp_call_builtin_tool,
            yuan_mcp_commands::yuan_mcp_configure_builtin,
            yuan_mcp_commands::yuan_mcp_list_call_history,
            yuan_safety_commands::yuan_safety_check,
            yuan_safety_commands::yuan_safety_quick_check,
            yuan_safety_commands::yuan_safety_set_profile,
            yuan_safety_commands::yuan_safety_get_profile,
            yuan_safety_commands::yuan_safety_list_profiles,
            yuan_skill_commands::yuan_skill_add_root,
            yuan_skill_commands::yuan_skill_set_project_files,
            yuan_skill_commands::yuan_skill_load_all,
            yuan_skill_commands::yuan_skill_get,
            yuan_skill_commands::yuan_skill_list,
            yuan_skill_commands::yuan_skill_detect_implicit,
            yuan_skill_commands::yuan_skill_register,
            yuan_skill_commands::yuan_skill_unregister,
            yuan_skill_commands::yuan_skill_trigger,
            yuan_skill_commands::yuan_skill_auto_discover,
            yuan_skill_commands::yuan_skill_match,
            yuan_skill_commands::yuan_skill_render_context,
            yuan_skill_commands::yuan_skill_stats,
            yuan_skill_commands::yuan_skill_export,
            // D1.8 Skill 市场：浏览/搜索/安装/卸载
            yuan_skill_commands::yuan_skill_market_list,
            yuan_skill_commands::yuan_skill_market_search,
            yuan_skill_commands::yuan_skill_market_get,
            yuan_skill_commands::yuan_skill_market_install,
            yuan_skill_commands::yuan_skill_market_uninstall,
            // D1 v3.2 Task 3.3: Skill 系统升级（安装/卸载/执行/评分）
            yuan_skill_commands::yuan_skill_install,
            yuan_skill_commands::yuan_skill_uninstall,
            yuan_skill_commands::yuan_skill_execute,
            yuan_skill_commands::yuan_skill_builtin_list,
            yuan_skill_commands::yuan_skill_rate,
            yuan_skill_commands::yuan_skill_review,
            yuan_skill_commands::yuan_skill_ratings_get,
            yuan_compact_commands::yuan_compact_config_get,
            yuan_compact_commands::yuan_compact_config_update,
            yuan_compact_commands::yuan_compact_estimate,
            yuan_compact_commands::yuan_compact_check,
            yuan_compact_commands::yuan_compact_execute,
            yuan_compact_commands::yuan_compact_session,
            yuan_compact_commands::yuan_compact_reset,
            yuan_io_control_commands::yuan_io_config_get,
            yuan_io_control_commands::yuan_io_config_update,
            yuan_io_control_commands::yuan_io_stats,
            yuan_io_control_commands::yuan_io_execute,
            yuan_io_control_commands::yuan_io_cancel,
            yuan_io_control_commands::yuan_io_cancel_all,
            yuan_io_control_commands::yuan_io_kill,
            yuan_io_control_commands::yuan_io_stdin,
            yuan_prompt_commands::yuan_prompt_list_templates,
            yuan_prompt_commands::yuan_prompt_get_template,
            yuan_prompt_commands::yuan_prompt_render,
            yuan_prompt_commands::yuan_prompt_set_custom_template,
            yuan_prompt_commands::yuan_prompt_remove_custom_template,
            yuan_prompt_commands::yuan_prompt_set_variable_default,
            yuan_prompt_commands::yuan_agents_discover,
            yuan_prompt_commands::yuan_agents_sources,
            yuan_prompt_commands::yuan_agents_assemble,
            yuan_prompt_commands::yuan_agents_set_max_bytes,
            yuan_prompt_commands::yuan_agents_get_max_bytes,
            yuan_prompt_commands::yuan_prompt_assemble,
            browser_commands::browser_open_window,
            browser_commands::browser_create_view,
            browser_commands::browser_navigate_view,
            browser_commands::browser_resize_view,
            browser_commands::browser_close_view,
            browser_commands::browser_cleanup,
            intelligence_commands::intelligence_get_config,
            intelligence_commands::intelligence_get_enabled,
            intelligence_commands::intelligence_set_enabled,
            intelligence_commands::intelligence_trigger_proactive_actions,
            intelligence_commands::intelligence_set_config,
            intelligence_commands::intelligence_get_context,
            intelligence_commands::intelligence_track_activity,
            intelligence_commands::intelligence_set_active_file,
            intelligence_commands::intelligence_add_terminal_session,
            intelligence_commands::intelligence_remove_terminal_session,
            intelligence_commands::intelligence_set_window_title,
            intelligence_commands::intelligence_get_ollama_status,
            intelligence_commands::intelligence_get_suggestions,
            intelligence_commands::intelligence_take_snapshot,
            intelligence_commands::intelligence_get_snapshots,
            intelligence_commands::intelligence_query_local_llm,
            intelligence_commands::intelligence_terminal_suggest,
            intelligence_commands::intelligence_resume_polish,
            intelligence_commands::intelligence_news_summary,
            intelligence_commands::intelligence_todo_suggest,
            intelligence_commands::intelligence_timer_remind,
            intelligence_commands::intelligence_kb_classify,
            intelligence_commands::intelligence_daily_briefing,
            // D2.3 模块渗透：群聊 + 游戏 智能建议（非侵入式，关闭后返回空）
            intelligence_commands::intelligence_chat_suggest,
            intelligence_commands::intelligence_game_suggest,
            intelligence_behavior_commands::intelligence_v2_analyze_behavior,
            intelligence_behavior_commands::intelligence_v2_recommend_workflows,
            intelligence_behavior_commands::intelligence_v2_detect_tech_stack,
            intelligence_behavior_commands::intelligence_v2_cognitive_load,
            intelligence_behavior_commands::intelligence_v2_notifications,
            intelligence_behavior_commands::intelligence_v2_dashboard,
            intelligence_anomaly_commands::intelligence_v3_detect_anomaly,
            intelligence_anomaly_commands::intelligence_v3_snapshot_resources,
            intelligence_anomaly_commands::intelligence_v3_organize_knowledge,
            intelligence_anomaly_commands::intelligence_v3_list_scheduled_tasks,
            intelligence_anomaly_commands::intelligence_v3_execute_scheduled_tasks,
            intelligence_v4_commands::intelligence_v4_log_activity,
            intelligence_v4_commands::intelligence_v4_batch_log_activity,
            intelligence_v4_commands::intelligence_v4_query_activity_logs,
            intelligence_v4_commands::intelligence_v4_activity_stats,
            intelligence_v4_commands::intelligence_v4_clean_activity_logs,
            intelligence_v4_commands::intelligence_v4_clear_all_activity_logs,
            intelligence_v4_commands::intelligence_v4_dashboard,
            intelligence_v4_commands::intelligence_v4_realtime_stats,
            intelligence_v4_commands::intelligence_v4_generate_suggestions,
            intelligence_v4_commands::intelligence_v4_get_suggestions,
            intelligence_v4_commands::intelligence_v4_mark_suggestion,
            intelligence_v4_commands::intelligence_v4_clean_suggestions,
            intelligence_v4_commands::intelligence_v4_analyze_behavior,
            intelligence_v4_commands::intelligence_v4_behavior_trend,
            intelligence_v4_commands::intelligence_v4_behavior_history,
            intelligence_v4_commands::intelligence_v4_get_settings,
            intelligence_v4_commands::intelligence_v4_save_settings,
            intelligence_v4_commands::intelligence_v4_reset_settings,
            intelligence_v4_commands::intelligence_v4_test_connection,
            intelligence_v4_commands::intelligence_v4_export_activity_logs,
            intelligence_v4_commands::intelligence_v4_get_operation_templates,
            intelligence_v4_commands::intelligence_v4_log_with_template,
            // 跨模块智能
            intelligence_v4_commands::intelligence_v4_resume_spell_check,
            intelligence_v4_commands::intelligence_v4_resume_polish,
            intelligence_v4_commands::intelligence_v4_resume_generate,
            intelligence_v4_commands::intelligence_v4_quote_spell_check,
            intelligence_v4_commands::intelligence_v4_quote_source_verify,
            intelligence_v4_commands::intelligence_v4_quote_smart_complete,
            intelligence_v4_commands::intelligence_v4_news_summarize,
            intelligence_v4_commands::intelligence_v4_todo_enhance,
            intelligence_v4_commands::intelligence_v4_journal_fill,
            intelligence_v4_commands::intelligence_v4_timer_remind,
            intelligence_v4_commands::intelligence_v4_kb_classify,
            intelligence_v4_commands::intelligence_v4_kb_summarize,
            intelligence_v4_commands::intelligence_v4_kb_tags,
            intelligence_v4_commands::intelligence_v4_terminal_complete,
            intelligence_v4_commands::intelligence_v4_game_recommend,
            intelligence_v4_commands::intelligence_v4_search_analyze,

            // xin_basic_commands::xin_get_config,
            // xin_basic_commands::xin_set_config,
            // xin_basic_commands::xin_get_personas,
            // xin_basic_commands::xin_get_active_persona,
            // xin_basic_commands::xin_set_active_persona,
            // xin_basic_commands::xin_save_memory,
            // xin_basic_commands::xin_get_memories,
            // xin_basic_commands::xin_search_memories,
            // xin_basic_commands::xin_delete_memory,
            // xin_basic_commands::xin_analyze_sentiment,
            // xin_basic_commands::xin_get_tts_status,
            // xin_basic_commands::xin_tts_speak,
            // xin_basic_commands::xin_process_multimodal,
            // xin_basic_commands::xin_get_mood,
            // xin_basic_commands::xin_update_mood,
            // xin_basic_commands::xin_add_summary,
            // xin_basic_commands::xin_get_summaries,
            // xin_basic_commands::xin_daily_briefing,
            // xin_basic_commands::xin_personality_insights,
            // xin_wellness_commands::xin_v2_mood_history,
            // xin_wellness_commands::xin_v2_consolidate_memories,
            // xin_wellness_commands::xin_v2_memory_links,
            // xin_wellness_commands::xin_v2_conversation_bridge,
            // xin_wellness_commands::xin_v2_add_reminder,
            // xin_wellness_commands::xin_v2_list_reminders,
            // xin_wellness_commands::xin_v2_dismiss_reminder,
            // xin_wellness_commands::xin_v2_delete_reminder,
            // xin_wellness_commands::xin_v2_register_habit,
            // xin_wellness_commands::xin_v2_checkin_habit,
            // xin_wellness_commands::xin_v2_list_habits,
            // xin_wellness_commands::xin_v2_pomodoro_start,
            // xin_wellness_commands::xin_v2_pomodoro_complete_cycle,
            // xin_wellness_commands::xin_v2_pomodoro_stop,
            // xin_wellness_commands::xin_v2_pomodoro_status,
            // xin_wellness_commands::xin_v2_activity_digest,
            // xin_wellness_commands::xin_v2_personality_evolution,
            // game 3D 重构 - 22 个新 IPC 命令（Task 5.1）
            // 世界与境界
            game_commands::game_init_world,
            game_commands::game_get_world_state,
            game_commands::game_get_realm_info,
            game_commands::game_get_breakthrough_preview,
            game_commands::game_start_breakthrough,
            game_commands::game_submit_breakthrough,
            // 建筑系统
            game_commands::game_get_buildings,
            game_commands::game_get_building_catalog,
            game_commands::game_start_building,
            game_commands::game_upgrade_building,
            game_commands::game_remove_building,
            game_commands::game_move_building,
            game_commands::game_get_build_history,
            // 知识联动
            game_commands::game_get_knowledge_domains,
            game_commands::game_get_knowledge_progress,
            game_commands::game_map_kb_category,
            game_commands::game_get_kb_category_mappings,
            game_commands::game_get_points_trend,
            game_commands::game_sync_knowledge_event,
            // 突破历史 + 世界列表
            game_commands::game_get_breakthrough_history,
            game_commands::game_list_worlds,
            game_commands::game_delete_world,
            // 统计聚合便利
            game_commands::game_get_events_and_tasks,
            game_commands::game_get_build_timeline,
            // 时间轴回放（T1，11_时间轴回放.md §3）
            game_commands::game_get_world_snapshot,
            game_commands::game_exit_replay,
            game_commands::game_npc_list,
            game_commands::game_npc_get,
            game_commands::game_npc_history,
            game_commands::game_npc_chat,
            game_commands::game_npc_clear_history,
            // D4.6 智能 NPC 深化：长期记忆 + 关系网
            game_commands::game_npc_memories,
            game_commands::game_npc_relationship,
            // D4.7 跨 NPC 关系联动：传闻机制
            game_commands::game_npc_rumors,
            // D4.3 自适应难度：玩家能力评估
            game_commands::game_player_skill,
            // D4.3 游戏对手 AI 决策（云端 API，AI 失败降级到启发式 mock）
            game_opponent_commands::game_opponent_decide,
            game_story_commands::game_story_generate,
            game_story_commands::game_story_advance,
            game_story_commands::game_story_get,
            game_story_commands::game_story_list,
            // D4.4 自然语言交互（云端 API 解析玩家命令，AI 失败降级到规则）
            game_natural_language_commands::game_nl_parse,
            // D4.6 数据分析 AI 洞察（云端 API 分析玩家行为，AI 失败降级到规则）
            game_behavior_commands::game_analyze_behavior,
            // D4.6 游戏数据底层智能监测接入（非侵入式、可关闭）
            game_intelligence_commands::game_intelligence_record_event,
            game_intelligence_commands::game_intelligence_status,

            git_commands::git_status,
            git_commands::git_diff_file,
            git_commands::git_diff_unstaged,
            git_commands::git_stage_file,
            git_commands::git_stage_all,
            git_commands::git_unstage_file,
            git_commands::git_commit,
            git_commands::git_push,
            git_commands::git_pull,
            git_commands::git_branches,
            git_commands::git_checkout,
            git_commands::git_log,
            git_commands::git_init,

            xin_basic_commands::xin_get_config,
            xin_basic_commands::xin_set_config,
            xin_basic_commands::xin_get_personas,
            xin_basic_commands::xin_get_active_persona,
            xin_basic_commands::xin_set_active_persona,
            xin_basic_commands::xin_save_memory,
            xin_basic_commands::xin_get_memories,
            xin_basic_commands::xin_search_memories,
            xin_basic_commands::xin_delete_memory,
            xin_basic_commands::xin_analyze_sentiment,
            xin_basic_commands::xin_get_tts_status,
            xin_basic_commands::xin_tts_speak,
            xin_basic_commands::xin_process_multimodal,
            xin_basic_commands::xin_get_mood,
            xin_basic_commands::xin_update_mood,
            xin_basic_commands::xin_emotion_trend,
            // D3.8 人格系统补全：人格记忆 + 切换历史
            xin_basic_commands::xin_persona_memory,
            xin_basic_commands::xin_persona_switch_history,
            xin_basic_commands::xin_memorized_personas,
            // D3.8.3 自生长人格：生长触发 + 状态查询
            xin_basic_commands::xin_grow_persona,
            xin_basic_commands::xin_self_growing_persona,
            xin_basic_commands::xin_add_summary,
            xin_basic_commands::xin_get_summaries,
            xin_basic_commands::xin_daily_briefing,
            xin_basic_commands::xin_personality_insights,
            xin_basic_commands::xin_voice_input,
            xin_basic_commands::xin_tts,
            xin_basic_commands::xin_tts_list_voices,
            xin_wellness_commands::xin_v2_mood_history,
            xin_wellness_commands::xin_v2_consolidate_memories,
            xin_wellness_commands::xin_v2_memory_links,
            xin_wellness_commands::xin_v2_conversation_bridge,
            xin_wellness_commands::xin_v2_add_reminder,
            xin_wellness_commands::xin_v2_list_reminders,
            xin_wellness_commands::xin_v2_dismiss_reminder,
            xin_wellness_commands::xin_v2_delete_reminder,
            xin_wellness_commands::xin_v2_register_habit,
            xin_wellness_commands::xin_v2_checkin_habit,
            xin_wellness_commands::xin_v2_list_habits,
            xin_wellness_commands::xin_v2_pomodoro_start,
            xin_wellness_commands::xin_v2_pomodoro_complete_cycle,
            xin_wellness_commands::xin_v2_pomodoro_stop,
            xin_wellness_commands::xin_v2_pomodoro_status,
            xin_wellness_commands::xin_v2_activity_digest,
            xin_wellness_commands::xin_v2_personality_evolution,
            xin_orchestration_commands::xin_v3_estimate_tokens,
            xin_orchestration_commands::xin_v3_context_new,
            xin_orchestration_commands::xin_v3_context_add_message,
            xin_orchestration_commands::xin_v3_context_trim,
            xin_orchestration_commands::xin_v3_context_stats,
            xin_orchestration_commands::xin_v3_compact,
            xin_orchestration_commands::xin_v3_recovery_briefing,
            xin_orchestration_commands::xin_v3_build_prompt,
            xin_orchestration_commands::xin_v3_build_context_header,
            xin_orchestration_commands::xin_v3_list_skills,
            xin_orchestration_commands::xin_v3_execute_skill,
            xin_orchestration_commands::xin_v3_dialogue_create,
            xin_orchestration_commands::xin_v3_dialogue_get,
            xin_orchestration_commands::xin_v3_dialogue_list,
            xin_orchestration_commands::xin_v3_dialogue_delete,
            xin_orchestration_commands::xin_v3_dialogue_search,
            xin_orchestration_commands::xin_v3_dialogue_send,
            xin_orchestration_commands::xin_v3_dialogue_stop,
            xin_orchestration_commands::xin_v3_analyze_intent,
            xin_orchestration_commands::xin_v3_analyze_sentiment,
            xin_orchestration_commands::xin_v3_extract_topics,
            xin_orchestration_commands::xin_v3_post_process,
            xin_orchestration_commands::xin_v3_knowledge_fusion,
            xin_orchestration_commands::xin_v3_list_tools,
            xin_orchestration_commands::xin_v3_parse_tool_calls,
            xin_orchestration_commands::xin_v3_execute_tool,
            xin_orchestration_commands::xin_v3_get_tools_prompt,
            xin_orchestration_commands::xin_v3_parse_attachment,
            xin_orchestration_commands::xin_v3_enhance_output,
            xin_orchestration_commands::xin_v3_detect_code_languages,
            xin_orchestration_commands::xin_v3_dream_check_due,
            xin_orchestration_commands::xin_v3_dream_run_light,
            xin_orchestration_commands::xin_v3_dream_run_deep,
            xin_orchestration_commands::xin_v3_dream_run_rem,
            xin_orchestration_commands::xin_v3_dream_calc_health,
            xin_orchestration_commands::xin_v3_dream_auto_promote,
            xin_orchestration_commands::xin_v3_dream_default_config,
            xin_orchestration_commands::xin_v3_commit_extract,
            xin_orchestration_commands::xin_v3_commit_create,
            xin_orchestration_commands::xin_v3_commit_list_pending,
            xin_orchestration_commands::xin_v3_commit_check_due,
            xin_orchestration_commands::xin_v3_commit_mark_done,
            xin_orchestration_commands::xin_v3_commit_snooze,
            xin_orchestration_commands::xin_v3_commit_dismiss,
            xin_orchestration_commands::xin_v3_commit_stats,
            xin_orchestration_commands::xin_v3_eval_score,
            xin_orchestration_commands::xin_v3_eval_history,
            xin_orchestration_commands::xin_v3_eval_stats,
            xin_orchestration_commands::xin_v3_eval_report,
            xin_orchestration_commands::xin_v3_evolution_rules,
            xin_orchestration_commands::xin_v3_evolution_analyze,
            xin_orchestration_commands::xin_v3_evolution_apply,
            xin_orchestration_commands::xin_v3_evolution_variant_create,
            xin_orchestration_commands::xin_v3_evolution_variant_compare,
            xin_orchestration_commands::xin_v3_evolution_snapshot,
            xin_orchestration_commands::xin_v3_evolution_timeline,
            xin_orchestration_commands::xin_v3_proactive_pending,
            xin_orchestration_commands::xin_v3_proactive_care_check,
            xin_orchestration_commands::xin_v3_proactive_urgency,
            xin_orchestration_commands::xin_v3_proactive_quiet_hours,
            xin_orchestration_commands::xin_v3_proactive_daily_nudge,
            xin_orchestration_commands::xin_v3_proactive_morning_context,
            xin_orchestration_commands::xin_v3_checkpoint_save,
            xin_orchestration_commands::xin_v3_checkpoint_list,
            xin_orchestration_commands::xin_v3_checkpoint_restore,
            xin_orchestration_commands::xin_v3_checkpoint_delete,
            xin_orchestration_commands::xin_v3_checkpoint_cleanup,
            xin_orchestration_commands::xin_v3_review_generate,
            xin_orchestration_commands::xin_v3_review_topic_trends,
            xin_orchestration_commands::xin_v3_review_growth_trajectory,
            xin_orchestration_commands::xin_v3_review_heatmap,
            xin_orchestration_commands::xin_v3_compaction_get_config,
            xin_orchestration_commands::xin_v3_compaction_update_config,
            xin_orchestration_commands::xin_v3_compaction_get_records,
            xin_orchestration_commands::xin_v3_compaction_needs_check,
            xin_orchestration_commands::xin_v3_compaction_auto,
            xin_orchestration_commands::xin_v3_compaction_manual,
            xin_orchestration_commands::xin_v3_fusion_should_retrieve,
            xin_orchestration_commands::xin_v3_fusion_memory_query,
            xin_orchestration_commands::xin_v3_fusion_multi_source,
            xin_orchestration_commands::xin_v3_fusion_unified_context,
            xin_orchestration_commands::xin_v3_fusion_extract_keywords,
            xin_orchestration_commands::xin_v3_post_score_quality,
            xin_orchestration_commands::xin_v3_post_adjust_tone,
            xin_orchestration_commands::xin_v3_post_check_factuality,
            xin_orchestration_commands::xin_v3_post_safety_filter,
            xin_orchestration_commands::xin_v3_post_enhanced_pipeline,
            // Engine commands
            engine_commands::engine_create_session,
            engine_commands::engine_get_session,
            engine_commands::engine_list_sessions,
            engine_commands::engine_destroy_session,
            engine_commands::engine_start_turn,
            engine_commands::engine_complete_turn,
            engine_commands::engine_abort_turn,
            engine_commands::engine_pause_session,
            engine_commands::engine_resume_session,
            engine_commands::engine_get_turns,
            engine_commands::engine_stats,
            engine_commands::engine_subscribe_events,
            // LSP commands
            lsp_commands::lsp_completions,
            lsp_commands::lsp_hover,
            lsp_commands::lsp_definition,
            lsp_commands::lsp_diagnostics,
            lsp_commands::lsp_detect_language,
            // ai_v2_commands::ai_v2_estimate_tokens,
            // ai_v2_commands::ai_v2_allocate_budget,
            // ai_v2_commands::ai_v2_compress_context,
            // ai_v2_commands::ai_v2_context_stats,
            // ai_v2_commands::ai_v2_analyze_convergence,
            // ai_v2_commands::ai_v2_round_summary,
            // ai_v2_commands::ai_v2_final_summary,
            // ai_v2_commands::ai_v2_should_summarize,
            // ai_v2_commands::ai_v2_emergency_summarize,
            // ai_v2_commands::ai_v2_rebalance_budget,
            // ai_v2_commands::ai_v2_remaining_ratio,
            perf_commands::record_perf_metric,
            perf_commands::perf_get_slow_queries,
            perf_commands::perf_get_metrics_summary,
            perf_commands::perf_get_metric_timeseries,
            // T2.8 自动备份系统命令（§2.3.10）
            backup_commands::backup_create_now,
            backup_commands::backup_list,
            backup_commands::backup_delete,
            backup_commands::backup_stats,
            backup_commands::backup_verify,
            // T2.9 数据恢复与导出命令（§2.4 + §2.5）
            restore_commands::restore_from_backup,
            restore_commands::export_to_zip,
            restore_commands::check_import_compatibility,
            restore_commands::import_from_zip,
            // T2.10 异常检测与自愈命令（§2.6）
            health_check_commands::health_check_run,
            health_check_commands::health_check_repair,
            // T2.13 MEK 密钥轮换命令（§2.3.6）
            mek_rotation_commands::mek_rotation_status,
            mek_rotation_commands::mek_rotation_rotate_now,
            mek_rotation_commands::mek_rotation_history,
            // C1.2 字体管理命令（§2.2）
            font_commands::font_list_custom,
            font_commands::font_upload,
            font_commands::font_delete,
            font_commands::font_get_dir,
            // C1.5 自定义主题持久化命令（§2.5）
            custom_theme_commands::custom_theme_list,
            custom_theme_commands::custom_theme_get,
            custom_theme_commands::custom_theme_upsert,
            custom_theme_commands::custom_theme_delete,
            custom_theme_commands::custom_theme_migrate_local,
            // A5 离线与同步机制命令（§2.2 / §2.6）
            sync_commands::sync_enqueue,
            sync_commands::sync_fetch_pending,
            sync_commands::sync_mark_synced,
            sync_commands::sync_mark_failed,
            sync_commands::sync_get_queue_stats,
            sync_commands::sync_register_device,
            sync_commands::sync_list_devices,
            sync_commands::sync_unregister_device,
            // A5 Phase 2-4 新增命令（传输测试 / 调度 / 冲突解决 / ECDH / E2EE）
            sync_commands::sync_test_transport,
            sync_commands::sync_run_once,
            sync_commands::sync_get_status,
            sync_commands::sync_list_conflicts,
            sync_commands::sync_resolve_conflict,
            sync_commands::sync_ecdh_generate_keypair,
            sync_commands::sync_e2ee_encrypt,
            sync_commands::sync_e2ee_validate,
            // A5 Phase 3 Task 1 新增命令（网络状态检测）
            sync_commands::sync_get_network_status,
            sync_commands::sync_check_network_now,
            // A5 Phase 3 Task 5 新增命令（配置同步降级）
            sync_commands::sync_record_config_change,
            sync_commands::sync_flush_config_queue,
            sync_commands::sync_get_pending_config_count,
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
