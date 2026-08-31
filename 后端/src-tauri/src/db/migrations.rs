use sqlx::SqlitePool;
use sqlx::Row;

use crate::apply_migration;
use crate::error::app_error::AppError;

/// T2.6 数据完整性 - 迁移系统升级入口
///
/// 实现规范：`功能展望/平台级增强/02_数据完整性_迁移与备份.md` §2.1
///
/// 流程：
/// 1. 启用 WAL + 外键（保留原有行为）
/// 2. 确保升级版 `schema_migrations` 表存在（含 description/applied_by/migration_hash/rollback_script/checksum_verified）
/// 3. 查询已应用版本集合 `applied`
/// 4. 按版本号顺序调用 `apply_migration!` 宏：
///    - 命中 `applied` → 跳过（短路机制生效）
///    - 未命中 → 执行迁移函数 + 记录到 `schema_migrations` 表（含 SHA256 hash）
///
/// 兼容性：
/// - 全新数据库：`applied` 为空，所有迁移顺序执行
/// - 旧数据库（无 `schema_migrations` 表）：第一次启动会执行所有迁移（幂等，不破坏数据），
///   之后启动会命中短路
///
/// 版本号分配规则：
/// - 1-80：现有 inline 迁移函数（保持原有顺序）
/// - 81+：未来新增迁移（T2.6.2+ 阶段逐步将 inline 函数提取为 .sql 文件，版本号不变）
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await?;

    sqlx::query("PRAGMA foreign_keys=ON;")
        .execute(pool)
        .await?;

    // T2.6.1 升级版迁移系统：schema_migrations 表 + 短路机制 + hash 跟踪
    crate::db::migration_loader::ensure_schema_migrations_table(pool).await?;
    let applied = crate::db::migration_loader::get_applied_versions(pool).await?;

    match crate::db::migration_loader::get_current_version(pool).await? {
        Some(current) => {
            tracing::info!(
                "Schema 当前版本 v{}, 开始应用未完成的迁移（已应用 {} 个）",
                current,
                applied.len()
            );
        }
        None => {
            tracing::info!("全新数据库，应用所有迁移");
        }
    }

    // ===== 迁移列表（版本号严格单调递增，顺序与原 run_migrations 一致）=====

    // ----- 核心表（users / permissions / ai / conversations / messages）-----
    apply_migration!(pool, applied, 1, create_users_table);
    apply_migration!(pool, applied, 2, migrate_users_profile_fields);
    apply_migration!(pool, applied, 3, create_permissions_table);
    apply_migration!(pool, applied, 4, create_ai_models_table);
    apply_migration!(pool, applied, 5, create_ai_agents_table);
    apply_migration!(pool, applied, 6, create_conversations_table);
    apply_migration!(pool, applied, 7, create_conversation_participants_table);
    apply_migration!(pool, applied, 8, create_messages_table);

    // ----- 知识库核心表（kb_*）-----
    apply_migration!(pool, applied, 9, create_kb_categories_table);
    apply_migration!(pool, applied, 10, create_kb_entries_table);
    apply_migration!(pool, applied, 11, create_kb_tags_table);
    apply_migration!(pool, applied, 12, create_kb_entry_tags_table);
    apply_migration!(pool, applied, 13, create_kb_recent_access_table);
    apply_migration!(pool, applied, 14, create_kb_tracked_paths_table);
    apply_migration!(pool, applied, 15, migrate_kb_entries_favorite);
    apply_migration!(pool, applied, 16, migrate_kb_entries_content);
    apply_migration!(pool, applied, 17, migrate_kb_entries_source_path);
    apply_migration!(pool, applied, 18, migrate_kb_tags_color);
    apply_migration!(pool, applied, 19, create_kb_templates_table);
    apply_migration!(pool, applied, 20, create_kb_references_table);
    apply_migration!(pool, applied, 21, create_kb_snapshots_table);

    // ----- 知识库扩展表（todos / journals / timers / recycle_bin / user_profiles 等）-----
    apply_migration!(pool, applied, 22, create_todos_table);
    apply_migration!(pool, applied, 23, create_journals_table);
    apply_migration!(pool, applied, 24, create_timers_table);
    apply_migration!(pool, applied, 25, create_recycle_bin_table);
    apply_migration!(pool, applied, 26, create_user_profiles_table);
    apply_migration!(pool, applied, 27, create_resumes_table);
    apply_migration!(pool, applied, 28, create_timeline_table);
    apply_migration!(pool, applied, 29, create_quotes_table);

    // ----- 新闻与系统配置 -----
    apply_migration!(pool, applied, 30, create_news_cache_table);
    apply_migration!(pool, applied, 31, create_news_pending_delete_table);
    apply_migration!(pool, applied, 32, create_news_sources_table);
    apply_migration!(pool, applied, 33, create_system_config_table);

    // ----- 终端相关表 -----
    apply_migration!(pool, applied, 34, create_terminal_history_table);
    apply_migration!(pool, applied, 35, migrate_terminal_history_duration);
    apply_migration!(pool, applied, 36, create_terminal_tab_layout_table);
    apply_migration!(pool, applied, 37, migrate_terminal_tab_layout_pane_data);
    apply_migration!(pool, applied, 38, create_terminal_sessions_table);
    apply_migration!(pool, applied, 39, migrate_terminal_sessions_columns);
    apply_migration!(pool, applied, 40, create_terminal_history_fts);
    apply_migration!(pool, applied, 41, create_ssh_profiles_table);
    apply_migration!(pool, applied, 42, create_terminal_config_table);

    // ----- 编辑器相关表 -----
    apply_migration!(pool, applied, 43, create_editor_documents_table);
    apply_migration!(pool, applied, 44, migrate_editor_documents_table);
    apply_migration!(pool, applied, 45, create_editor_versions_table);
    apply_migration!(pool, applied, 46, create_editor_sessions_table);
    apply_migration!(pool, applied, 47, create_editor_fts_table);

    // ----- Yuan Code -----
    apply_migration!(pool, applied, 48, create_yuan_code_snippets_table);
    apply_migration!(pool, applied, 49, create_yuan_code_workspaces_table);
    apply_migration!(pool, applied, 50, create_yuan_goals_table);

    // ----- 小欣（Xin）相关表 -----
    apply_migration!(pool, applied, 51, create_xin_config_table);
    apply_migration!(pool, applied, 52, create_xin_memories_table);
    apply_migration!(pool, applied, 53, create_xin_summaries_table);
    apply_migration!(pool, applied, 54, create_xin_moods_table);
    apply_migration!(pool, applied, 55, create_xin_reminders_table);
    apply_migration!(pool, applied, 56, create_xin_habits_table);
    apply_migration!(pool, applied, 57, create_xin_conversations_table);
    apply_migration!(pool, applied, 58, create_xin_checkpoints_table);
    apply_migration!(pool, applied, 59, create_xin_compaction_records_table);
    apply_migration!(pool, applied, 60, create_xin_compaction_config_table);

    // ----- 行为与智能 -----
    apply_migration!(pool, applied, 61, create_activity_logs_table);
    apply_migration!(pool, applied, 62, create_suggestions_table);
    apply_migration!(pool, applied, 63, create_behavior_patterns_table);
    apply_migration!(pool, applied, 64, create_intelligence_settings_table);
    apply_migration!(pool, applied, 65, create_auth_sessions_table);

    // ----- 索引 + 补丁迁移 -----
    apply_migration!(pool, applied, 66, create_indexes);
    apply_migration!(pool, applied, 67, migrate_news_github_category);
    apply_migration!(pool, applied, 68, migrate_conversations_starred);
    apply_migration!(pool, applied, 69, create_prompt_templates_table);

    // ----- 游戏 3D 重构相关表（共 10 张，对应 07_数据库设计.md v2.0）-----
    // 顺序约束：归档旧表 → game_worlds → game_knowledge_domains → 其余从表
    apply_migration!(pool, applied, 70, archive_legacy_game_tables);
    apply_migration!(pool, applied, 71, create_game_worlds_table);
    apply_migration!(pool, applied, 72, create_game_knowledge_domains_table);
    apply_migration!(pool, applied, 73, create_game_buildings_table);
    apply_migration!(pool, applied, 74, create_game_knowledge_progress_table);
    apply_migration!(pool, applied, 75, create_game_breakthrough_records_table);
    apply_migration!(pool, applied, 76, create_game_build_history_table);
    apply_migration!(pool, applied, 77, create_game_kb_category_mapping_table);
    apply_migration!(pool, applied, 78, create_game_points_log_table);
    apply_migration!(pool, applied, 79, create_game_daily_limit_counter_table);
    apply_migration!(pool, applied, 80, create_game_points_source_config_table);

    // ----- v1.52 性能基线：性能指标采集表（01_性能优化_首屏200ms计划.md §2.1.3）-----
    apply_migration!(pool, applied, 81, create_perf_metrics_table);

    // ----- v1.52.4 T2.7.1: audit_log 表 + 10 张关键表审计触发器 -----
    // 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.4
    apply_migration!(pool, applied, 82, create_audit_log_table);
    apply_migration!(pool, applied, 83, create_audit_triggers);

    // ----- v1.52.4 T2.7.4: 外键补建 - 表重建（方案 A：仅类型匹配的 7 条 FK）-----
    // 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.2
    // 说明: SQLite 不支持 ALTER TABLE ADD FOREIGN KEY，必须通过表重建实现。
    //       每个迁移创建 _new 表（带 FK）→ INSERT 数据 → DROP 旧表 → RENAME → 重建索引。
    //       类型不匹配的 9 条 FK（user_id TEXT vs users.id INTEGER 等）推迟到 v1.52.5+ 解决。
    apply_migration!(pool, applied, 84, rebuild_kb_references_with_fk);
    apply_migration!(pool, applied, 85, rebuild_kb_snapshots_with_fk);
    apply_migration!(pool, applied, 86, rebuild_terminal_sessions_with_fk);
    apply_migration!(pool, applied, 87, rebuild_xin_checkpoints_with_fk);
    apply_migration!(pool, applied, 88, rebuild_xin_compaction_records_with_fk);
    apply_migration!(pool, applied, 89, rebuild_yuan_goals_with_fk);

    // ----- v1.52.4 T2.13: MEK 密钥轮换机制 -----
    // 规范: 功能展望/平台级增强/05_安全加固_A4.md §2.3
    // 说明: 新增 mek_versions 表（多版本共存 + 渐进迁移）+ mek_rotation_log 表（轮换历史审计）
    //       MekManager 扩展支持版本管理 + 轮换 + 密文迁移
    apply_migration!(pool, applied, 90, create_mek_versions_table);
    apply_migration!(pool, applied, 91, create_mek_rotation_log_table);

    // ----- D1.4 项目索引系统（Yuan Code v3 项目级上下文） -----
    // 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §2.2.2
    // 说明: 5 张表 — projects / files / symbols / imports / dependencies / changes
    apply_migration!(pool, applied, 92, create_project_indexer_tables);

    // ----- D4.2 智能 NPC 系统 -----
    // 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §2.1
    // 说明: 2 张表 — game_npcs / game_npc_conversations
    apply_migration!(pool, applied, 93, create_game_npcs_tables);

    // ----- A1 §2.4.1 慢查询日志表（v1.52.5.x 性能优化深度）-----
    // 规范: 功能展望/平台级增强/01_性能优化_首屏200ms计划.md §2.4.1
    // 说明: slow_query_log 表 + 2 个索引（time DESC + duration DESC）
    apply_migration!(pool, applied, 94, create_slow_query_log_table);

    // ----- C1.5 自定义主题持久化表（替代 localStorage） -----
    // 规范: 功能展望/体验深化/01_主题自定义系统_未来展望.md §2.5
    // 说明: custom_themes 表 — 用户自定义主题跨设备同步基础
    apply_migration!(pool, applied, 95, create_custom_themes_table);

    // ----- 恐龙双脑 Phase 0：蒸馏数据集表 -----
    // 规范: 功能展望/v2_核心战略/01_恐龙双脑智能架构_实施路线图_补充.md §2.1 Phase 0
    // 说明: distill_dataset 表 — 存放从用户活动数据抽取并由教师模型改写的训练样本
    apply_migration!(pool, applied, 96, create_distill_dataset_table);

    // v97-v98: A5 离线与同步机制基础架构
    // 规范: 功能展望/平台级增强/04_离线与同步机制.md §2.2 / §2.6
    // 说明: sync_queue 表 — 变更捕获与同步队列；sync_devices 表 — 已注册设备管理
    apply_migration!(pool, applied, 97, create_sync_queue_table);
    apply_migration!(pool, applied, 98, create_sync_devices_table);

    // v1.52.6 D1.6: MCP 服务器配置持久化（含 3 个预置模板）
    apply_migration!(pool, applied, 99, create_mcp_servers_table);

    // ----- D3.8 人格系统补全：人格记忆 + 切换历史 -----
    // 规范: 功能展望/模块深化/03_小欣_多模态融合_深度.md §2.7
    // 说明: xin_persona_memories 表 — 每个人格独立的交互摘要 + 用户偏好积累
    //       xin_persona_switch_log 表 — 人格切换历史（前端时间线 + 自生长学习数据）
    apply_migration!(pool, applied, 100, create_xin_persona_memories_table);
    apply_migration!(pool, applied, 101, create_xin_persona_switch_log_table);

    // ----- D4.4b 动态剧情 DB 持久化：剧情会话 + 节点 -----
    // 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.4
    // 说明: game_stories 表 — 剧情会话主表（替代 OnceCell 内存存储，支持跨会话恢复）
    //       game_story_nodes 表 — 剧情节点表（分支树持久化，玩家行为影响剧情走向的持久性验收）
    apply_migration!(pool, applied, 102, create_game_stories_table);
    apply_migration!(pool, applied, 103, create_game_story_nodes_table);

    // ----- D4.6 智能 NPC 深化：长期记忆 + 关系网 -----
    // 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §2.1.1 + §D4.6
    // 说明: game_npc_memories 表 — NPC 对玩家的长期记忆（混合方案：规则匹配即时 + AI 每 5 轮批量提取）
    //       game_npc_relationships 表 — NPC↔玩家关系值（-100~100，AI 每轮评估增量，单玩家世界 world+npc UNIQUE）
    apply_migration!(pool, applied, 104, create_game_npc_memories_table);
    apply_migration!(pool, applied, 105, create_game_npc_relationships_table);

    // ----- D4.3 自适应难度：玩家能力评估表 -----
    // 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §2.3 自适应难度
    // 说明: game_player_skill 表 — 记录玩家突破历史表现，用于动态调整难度系数（心流理论）
    apply_migration!(pool, applied, 106, create_game_player_skill_table);

    // ----- D4.7 跨 NPC 关系联动：传闻机制 -----
    // 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §2.1 + 09_游戏/游戏.md §10.3 D4.6 剩余 5%
    // 说明: game_npc_rumors 表 — 跨 NPC 传闻传播（重要记忆自动传播到同世界其他 NPC，形成"听说"效果）
    apply_migration!(pool, applied, 107, create_game_npc_rumors_table);

    // ----- Phase 3 §2.2.2 索引补建 -----
    // 规范: 功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 3
    // 说明: 健康检查 CRITICAL_INDEXES 期望的 6 个索引补建（users.email / conversations.user_id
    //       因列不存在已从 CRITICAL_INDEXES 移除；详见 health_check_service::CRITICAL_INDEXES 注释）
    apply_migration!(pool, applied, 108, supplement_missing_indexes);

    // ----- D1 v3.1 Task 3.5.1: 云端 API Key 专用存储表 -----
    // 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.5.1 + 项目核心设计意图 §三
    // 说明: api_keys 表 — 按 provider 维度存储云端编程 API Key（AES-GCM 加密），
    //       专为 Yuan Code 编程 AI 强制走云端 API 设计，与 ai_models 表（含本地 ollama）解耦
    apply_migration!(pool, applied, 109, create_api_keys_table);

    // ----- D1 v3.2 Task 3.4.1: 模型路由配置表（Phase 6 多模型与协作）-----
    // 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.1 + 项目核心设计意图 §三/§八
    // 说明: model_routing_rules 表 — 按 task_type（编程/分析/审查/文档）路由到不同云端 API 模型；
    //       is_cloud_only = 1 强制约束：编程任务规则只能选云端 API provider，禁止本地底层智能模型
    apply_migration!(pool, applied, 110, create_model_routing_rules_table);

    // ----- D1 v3.2 Task 3.3.4: Skill 评分/评论系统 -----
    // 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §Phase 5（Skill 评分/评论）
    // 说明: skill_ratings 表 — 每个 user 对每个 skill_name 一条评分（1-5 星）+ 可选评论，
    //       UNIQUE(skill_name, user_id) 约束保证一人一评，更新走 UPSERT
    apply_migration!(pool, applied, 111, create_skill_ratings_table);

    // ----- A5 Phase 3 Task 3: 新闻源离线缓存 -----
    // 规范: 功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 3
    // 说明: news_offline_cache 表 — 离线场景下展示的 RSS 新闻快照（与 news_cache 主存储解耦）。
    //       在线拉取成功后同步写入本表；离线时从本表读取，UI 展示「上次更新 N 分钟前」。
    //       表名避开已占用的 news_cache（v30 主存储），使用 news_offline_cache 避免冲突。
    apply_migration!(pool, applied, 112, create_news_offline_cache_table);

    // ----- A5 Phase 3 Task 4: 知识库附件预加载 -----
    // 规范: 功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 4
    // 说明: kb_attachment_cache 表 — 标记常用 KB 附件并预加载到 app_data_dir/attachment_cache/
    //       LRU 淘汰（默认 500MB + 30 天）。离线打开附件时从 local_cache_path 读取。
    //       FK 引用 kb_entries(id) ON DELETE CASCADE（条目删除时自动级联清理缓存记录）。
    apply_migration!(pool, applied, 113, create_kb_attachment_cache_table);

    // ----- spec ai-chat-enhancement Phase 1 §1.1: ai_models 健康检测字段 -----
    // 说明: 给 ai_models 表增加 status / last_health_check / latency_ms 三个字段，
    //       支持模型状态真实检测与持久化（取代 to_response() 中的硬编码 "active"）
    apply_migration!(pool, applied, 114, add_model_health_fields);

    // ----- spec ai-chat-enhancement Phase 2 §2.1: 会话列表字段补全 -----
    // 说明: 给 conversations 表增加 unread_count / sort_order 字段，
    //       支持未读数标记与拖拽自定义排序
    apply_migration!(pool, applied, 115, add_conversation_list_fields);

    // ----- 安全审计修复：多用户数据隔离批次 1（AI 配置类） -----
    // 说明: 给 ai_models / ai_agents / api_keys 三张用户私有数据表添加 user_id 字段，
    //       实现多用户数据隔离，防止 guest 账号读取/修改/删除 admin 的 API Key 等敏感配置。
    // 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
    apply_migration!(pool, applied, 116, add_user_id_to_ai_tables);

    // v117: 多用户数据隔离批次 2 — 会话与消息表
    // 说明: 给 conversations / messages / conversation_participants 三张表添加 user_id 字段，
    //       实现多用户数据隔离，防止跨用户读取/修改/删除会话历史。
    // 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
    apply_migration!(pool, applied, 117, add_user_id_to_conversations_tables);

    // v118: 多用户数据隔离批次 3 — 知识库表（kb_categories / kb_entries / kb_tags 等 10 张表）
    //       给所有用户私有 KB 表添加 user_id 字段，防止跨用户读取/修改/删除知识库数据。
    // 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
    apply_migration!(pool, applied, 118, add_user_id_to_kb_tables);

    // ----- 多用户数据隔离批次 4：小欣（Xin）模块 12 张表添加 user_id 字段 -----
    // 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
    // 说明: 给 xin_config / xin_memories / xin_summaries / xin_moods / xin_reminders /
    //       xin_habits / xin_conversations / xin_checkpoints / xin_compaction_records /
    //       xin_compaction_config / xin_persona_memories / xin_persona_switch_log
    //       添加 user_id 字段（DEFAULT 1 保证现有 admin 数据自动归属 user_id=1）
    apply_migration!(pool, applied, 119, add_user_id_to_xin_tables);

    // ----- 多用户数据隔离批次 5：日程类 2 张表（journals / timers）添加 user_id 字段 -----
    // 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
    // 说明: journals 表需重建以改 UNIQUE(date) → UNIQUE(user_id, date)（支持多用户同一天写日记）；
    //       timers 表无 UNIQUE 约束，直接 ALTER ADD COLUMN。
    apply_migration!(pool, applied, 120, add_user_id_to_journals_timers);

    // ----- 多用户数据隔离批次 6：剩余 14 张用户私有表添加 user_id 字段 -----
    // 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
    // 说明: 11 张表直接 ALTER ADD COLUMN；3 张有 UNIQUE 约束的表（news_sources/custom_themes/model_routing_rules）
    //       需重建表以改 UNIQUE 约束为 (user_id, 原列)。
    //       涉及表：ssh_profiles / prompt_templates / resumes / quotes / custom_themes /
    //       yuan_code_snippets / yuan_code_workspaces / terminal_sessions / recycle_bin /
    //       game_worlds / news_cache / news_sources / mcp_servers / model_routing_rules
    apply_migration!(pool, applied, 121, add_user_id_to_remaining_tables);

    // ----- 多用户数据隔离补充修复：terminal_history / terminal_tab_layout 添加 user_id 字段 -----
    // 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
    // 说明: 批次 6 遗漏的两张终端用户私有表。terminal_history 存储命令历史（高敏感，
    //       可能含密码/密钥），terminal_tab_layout 存储标签布局。两表均无 UNIQUE 约束，
    //       直接 ALTER ADD COLUMN。
    apply_migration!(pool, applied, 122, add_user_id_to_terminal_history_tab_layout);

    tracing::info!(
        "数据库迁移完成（已应用 {} 个迁移，当前版本 v122）",
        applied.len()
    );
    Ok(())
}

/// v95: 创建 custom_themes 表（用户自定义主题持久化）
///
/// 替代 localStorage 的 nexterm-custom-themes 键，支持：
/// - 列表查询：列出所有已保存的自定义主题
/// - 新增/更新：UPSERT 语义（按 name 唯一约束）
/// - 删除：按 id 或 name
/// - 详情：按 id 查询单个主题
async fn create_custom_themes_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0095_create_custom_themes_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

/// v96: 创建 distill_dataset 表（恐龙双脑 Phase 0 蒸馏数据集）
///
/// 规范: 功能展望/v2_核心战略/01_恐龙双脑智能架构_实施路线图_补充.md §2.1 Phase 0
///
/// 存放从用户活动数据（activity_logs / chat_messages / kb_entries）抽取并由
/// 教师模型（云端 API，按项目核心设计意图 §七，走云端 API 模型）改写得到的
/// 学生模型训练样本。
///
/// 字段：source_type / source_id / input_text / teacher_output / metadata
///       / quality_score / status / created_at / reviewed_at
/// 索引：3 个（status / source_type / quality_score DESC）
async fn create_distill_dataset_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0096_create_distill_dataset_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🧠 [D2-Phase0] distill_dataset 表已创建（含 3 个索引）");
    Ok(())
}

/// v97: 创建 sync_queue 表（A5 离线与同步机制 - 变更捕获队列）
///
/// 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.2
/// 字段：table_name / record_id / operation / payload / vector_clock / device_id
///       / created_at / synced_at / sync_status / retry_count / last_error
/// 索引：3 个（status+created_at / table+record / device_id）
async fn create_sync_queue_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0097_create_sync_queue_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔄 [A5] sync_queue 表已创建（含 3 个索引）");
    Ok(())
}

/// v98: 创建 sync_devices 表（A5 离线与同步机制 - 设备管理）
///
/// 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.6
/// 字段：id(UUID) / device_name / device_type / public_key / registered_at
///       / last_seen_at / last_sync_at / is_current_device
/// 索引：1 个（is_current_device）
async fn create_sync_devices_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0098_create_sync_devices_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("📱 [A5] sync_devices 表已创建（含 1 个索引）");
    Ok(())
}

/// v99: 创建 mcp_servers 表（D1.6 MCP 服务器配置持久化）
///
/// 存储用户注册的 MCP 服务器配置，应用重启后可自动恢复。
/// 预置 3 个常用 MCP server 模板（filesystem/git/fetch）。
/// 表字段：id/name/command/args/env/working_dir/auto_connect/lifecycle_config/enabled/is_builtin
/// 索引：2 个（enabled, is_builtin）
async fn create_mcp_servers_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0099_create_mcp_servers_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔌 [D1.6] mcp_servers 表已创建（含 3 个预置模板 + 2 个索引）");
    Ok(())
}

/// v100: 创建 xin_persona_memories 表（D3.8 人格记忆）
///
/// 每个 persona 积累独立的交互摘要 + 用户偏好，切换回该人格时恢复上下文。
/// 自生长人格（self_growing）也用此表积累"用户画像"原料。
/// 字段：persona_id (UNIQUE) / interaction_summary / user_preferences (JSON)
///       / topic_tags (JSON array) / interaction_count / last_interaction_at
async fn create_xin_persona_memories_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0100_create_xin_persona_memories_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🧠 [D3.8] xin_persona_memories 表已创建（人格记忆 + 2 个索引）");
    Ok(())
}

/// v101: 创建 xin_persona_switch_log 表（D3.8 人格切换历史）
///
/// 记录每次人格切换，用于：
/// 1) 前端人格切换时间线展示
/// 2) 自生长人格的学习数据源（分析用户切换习惯）
/// 字段：from_persona_id / to_persona_id / switched_at / trigger
async fn create_xin_persona_switch_log_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0101_create_xin_persona_switch_log_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔄 [D3.8] xin_persona_switch_log 表已创建（切换历史 + 2 个索引）");
    Ok(())
}

/// v102: 创建 game_stories 表（D4.4b 动态剧情会话主表）
///
/// 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.4
///
/// 替代 MVP 阶段的 OnceCell 内存存储，支持：
/// - 跨会话恢复剧情（玩家行为影响剧情走向的持久性验收）
/// - 历史剧情列表查询（前端时间线展示）
/// - 节点表 game_story_nodes 通过 story_id 关联
///
/// 字段：id(UUID) / world_id / user_id / theme / current_node_id / used_ai
///       / is_finished / node_count / created_at / updated_at
/// 索引：2 个（user_id+updated_at / world_id+updated_at）
async fn create_game_stories_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0102_create_game_stories_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("📖 [D4.4b] game_stories 表已创建（剧情会话 + 2 个索引）");
    Ok(())
}

/// v103: 创建 game_story_nodes 表（D4.4b 剧情节点表）
///
/// 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.4
///
/// 存储每次剧情会话的所有节点（分支树持久化）：
/// - 每个节点 = 剧情推进的一个分支点
/// - 包含场景描述 + 旁白 + 选择项 JSON + 是否结局
/// - sequence 字段记录节点顺序（0=start, 1,2,3...）
///
/// 字段：story_id / node_id / title / description / narration / choices(JSON)
///       / is_ending / sequence / created_at
/// 索引：2 个（story_id+sequence / story_id+node_id）
async fn create_game_story_nodes_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0103_create_game_story_nodes_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("📖 [D4.4b] game_story_nodes 表已创建（剧情节点 + 2 个索引）");
    Ok(())
}

/// v104: 创建 game_npc_memories 表（D4.6 NPC 长期记忆）
///
/// 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §2.1.1 + §D4.6
///
/// 混合方案存储 NPC 对玩家的长期记忆：
/// - 来源 source='rule'：规则匹配即时抽取（姓名/喜好/承诺等高频模式，零 AI 成本）
/// - 来源 source='ai'：AI 每 5 轮批量提取一次（重要事件/隐性事实）
///
/// 字段：world_id / npc_id / memory_type(fact/preference/commitment/event) / content / importance
///       / source / recall_count / last_recalled_at / 时间戳
/// 索引：2 个（world+npc+importance / world+npc+memory_type）
async fn create_game_npc_memories_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0104_create_game_npc_memories_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🧠 [D4.6] game_npc_memories 表已创建（NPC 长期记忆 + 2 个索引）");
    Ok(())
}

/// v105: 创建 game_npc_relationships 表（D4.6 NPC↔玩家关系值）
///
/// 规范: 功能展望/模块深化/04_游戏_真实AI接入_深度.md §D4.6
///
/// 单玩家世界约束：每对 (world_id, npc_id) UNIQUE 一行
/// - relationship_value：-100（仇恨）~ +100（挚友），0 为中立
/// - AI 每轮评估增量（-5~+5），CHECK 约束防止越界
/// - relationship_history：JSON 数组保留最近 50 条变化记录（turn/delta/reason/ts）
/// - relationship_label：派生等级标签（hostile/cold/neutral/warm/close/sworn）
///
/// 字段：world_id / npc_id / relationship_value / interaction_count / first/last_interaction_at
///       / relationship_history(JSON) / relationship_label / 时间戳
/// 索引：1 个（world_id）
/// 约束：UNIQUE(world_id, npc_id) + CHECK(relationship_value BETWEEN -100 AND 100)
async fn create_game_npc_relationships_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0105_create_game_npc_relationships_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🤝 [D4.6] game_npc_relationships 表已创建（NPC 关系值 + UNIQUE 约束 + 1 个索引）");
    Ok(())
}

/// v106: 创建 game_player_skill 表（D4.3 自适应难度 — 玩家能力评估）
///
/// 记录玩家在突破考验中的历史表现，用于动态调整难度系数。
/// 设计依据：04_游戏_真实AI接入_深度.md §2.3 自适应难度（心流理论：挑战略高于能力）
///
/// 字段：
/// - skill_score: 综合能力评分 0.0-100.0
/// - attempt_count / success_count / fail_count / total_score / avg_score
/// - streak: 连胜（正数）/连败（负数）
/// - difficulty_multiplier: 难度乘数 0.6-1.5（应用到 difficulty_coefficient 上）
async fn create_game_player_skill_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0106_create_game_player_skill_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🎯 [D4.3] game_player_skill 表已创建（玩家能力评估 + 难度乘数 + 1 个索引）");
    Ok(())
}

/// v107: 创建 game_npc_rumors 表（D4.7 跨 NPC 传闻传播）
///
/// 存储从其他 NPC 传播来的传闻记忆。当 NPC 与玩家互动产生重要记忆时，
/// 该记忆会传播为传闻到同世界其他 NPC，让其他 NPC 在对话中能"听说"玩家的事。
/// UNIQUE(world_id, target_npc_id, origin_memory_id) 保证同一记忆不重复传播给同一 NPC。
async fn create_game_npc_rumors_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0107_create_game_npc_rumors_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🎯 [D4.7] game_npc_rumors 表已创建（跨 NPC 传闻传播 + 1 个索引）");
    Ok(())
}

// ----- v1.52.4 T2.13: MEK 密钥轮换机制 -----

/// v90: 创建 mek_versions 表（MEK 版本管理）
///
/// 存储每个用户的所有 MEK 版本（含历史版本），支持：
/// - 多版本共存：旧版本密文在迁移完成前仍可解密
/// - 渐进迁移：后台批量重新加密密文，避免阻塞用户操作
/// - 版本审计：rotated_from + rotated_at 记录轮换链
async fn create_mek_versions_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0090_create_mek_versions_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

/// v91: 创建 mek_rotation_log 表（MEK 轮换历史）
///
/// 记录每次 MEK 轮换的元数据：
/// - 触发原因（scheduled/manual/emergency）
/// - 迁移记录数
/// - 耗时 + 状态（completed/failed）
async fn create_mek_rotation_log_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0091_create_mek_rotation_log_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

/// v92: 创建项目索引系统表（D1.4 项目级上下文）
///
/// 6 张表：
/// - project_index_projects: 项目记录（root_path 唯一）
/// - project_index_files: 文件记录（path 唯一 + content_hash 增量索引）
/// - project_index_symbols: 符号（function/class/type/interface/variable/constant/enum/module）
/// - project_index_imports: 导入关系（use/import/require/from）
/// - project_index_dependencies: 依赖图（from_file → to_file + 类型 + 强度）
/// - project_index_changes: 文件变更历史（用于行为预测）
async fn create_project_indexer_tables(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0092_create_project_indexer_tables/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

/// v93: 创建智能 NPC 系统表（D4.2）
async fn create_game_npcs_tables(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0093_create_game_npcs_tables/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

/// v94: 创建慢查询日志表（A1 §2.4.1）
///
/// 表结构：slow_query_log（sql_text / duration_ms / command_name / params_json / rows_affected / recorded_at）
/// 索引：2 个（recorded_at DESC + duration_ms DESC）
/// 调用方：`db::query_logger::record_slow_query` 在 SQL 执行后判断耗时是否超阈值
async fn create_slow_query_log_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0094_create_slow_query_log_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🐌 [A1] slow_query_log 表已创建（含 2 个索引）");
    Ok(())
}

async fn create_users_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0001_create_users_table/up.sql
    let sql = include_str!("../../migrations/0001_create_users_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

// 为旧版本数据库补充 user 表的扩展字段（avatar_url / bio / display_name）
async fn migrate_users_profile_fields(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(users);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let names: Vec<String> = columns.iter().map(|r| r.get::<String, _>("name")).collect();
    if !names.contains(&"avatar_url".to_string()) {
        sqlx::query("ALTER TABLE users ADD COLUMN avatar_url TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    if !names.contains(&"bio".to_string()) {
        sqlx::query("ALTER TABLE users ADD COLUMN bio TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    if !names.contains(&"display_name".to_string()) {
        sqlx::query("ALTER TABLE users ADD COLUMN display_name TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn create_permissions_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0003_create_permissions_table/up.sql
    let sql = include_str!("../../migrations/0003_create_permissions_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_ai_models_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: 静态部分 SQL 提取至 migrations/0004_create_ai_models_table/up.sql
    // 动态部分（migrate_ai_models_table）保留 inline：PRAGMA + ALTER TABLE 补 6 个字段
    let sql = include_str!("../../migrations/0004_create_ai_models_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    migrate_ai_models_table(pool).await?;

    Ok(())
}

async fn migrate_ai_models_table(pool: &SqlitePool) -> Result<(), AppError> {
    let columns_result = sqlx::query("PRAGMA table_info(ai_models);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

    let column_names: Vec<String> = columns_result
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    let new_columns = vec![
        ("api_format", "TEXT"),
        ("display_name", "TEXT"),
        ("multimodal", "INTEGER NOT NULL DEFAULT 0"),
        ("system_prompt", "TEXT"),
        ("context_window", "INTEGER NOT NULL DEFAULT 8192"),
        ("temperature", "REAL"),
    ];

    for (col_name, col_type) in new_columns {
        if !column_names.contains(&col_name.to_string()) {
            sqlx::query(&format!("ALTER TABLE ai_models ADD COLUMN {} {};", col_name, col_type))
                .execute(pool)
                .await
                .map_err(AppError::Database)?;
        }
    }

    Ok(())
}

async fn create_ai_agents_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0005_create_ai_agents_table/up.sql
    let sql = include_str!("../../migrations/0005_create_ai_agents_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_conversations_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0006_create_conversations_table/up.sql
    let sql = include_str!("../../migrations/0006_create_conversations_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_conversation_participants_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0007_create_conversation_participants_table/up.sql
    let sql = include_str!("../../migrations/0007_create_conversation_participants_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_messages_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0008_create_messages_table/up.sql
    let sql = include_str!("../../migrations/0008_create_messages_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_kb_categories_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: 静态部分 SQL 提取至 migrations/0009_create_kb_categories_table/up.sql
    // 动态部分（migrate_kb_categories_library）保留 inline：PRAGMA + ALTER TABLE 补 library 字段
    let sql = include_str!("../../migrations/0009_create_kb_categories_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    migrate_kb_categories_library(pool).await?;

    Ok(())
}

async fn migrate_kb_categories_library(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(kb_categories);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let names: Vec<String> = columns.iter().map(|r| r.get::<String, _>("name")).collect();
    if !names.contains(&"library".to_string()) {
        sqlx::query("ALTER TABLE kb_categories ADD COLUMN library TEXT NOT NULL DEFAULT 'material';")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn create_kb_references_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0020_create_kb_references_table/up.sql
    let sql = include_str!("../../migrations/0020_create_kb_references_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_kb_snapshots_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0021_create_kb_snapshots_table/up.sql
    let sql = include_str!("../../migrations/0021_create_kb_snapshots_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_kb_entries_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0010_create_kb_entries_table/up.sql
    let sql = include_str!("../../migrations/0010_create_kb_entries_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_kb_tags_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0011_create_kb_tags_table/up.sql
    let sql = include_str!("../../migrations/0011_create_kb_tags_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_kb_entry_tags_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0012_create_kb_entry_tags_table/up.sql
    let sql = include_str!("../../migrations/0012_create_kb_entry_tags_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_kb_recent_access_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0013_create_kb_recent_access_table/up.sql
    let sql = include_str!("../../migrations/0013_create_kb_recent_access_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_kb_tracked_paths_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: SQL 提取至 migrations/0014_create_kb_tracked_paths_table/up.sql
    let sql = include_str!("../../migrations/0014_create_kb_tracked_paths_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_kb_templates_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.2: 静态部分 SQL 提取至 migrations/0019_create_kb_templates_table/up.sql
    // 动态部分（seed_default_templates）保留 inline：插入 6 个默认模板种子数据
    let sql = include_str!("../../migrations/0019_create_kb_templates_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    seed_default_templates(pool).await?;

    Ok(())
}

async fn seed_default_templates(pool: &SqlitePool) -> Result<(), AppError> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM kb_templates")
        .fetch_one(pool)
        .await
        .map_err(AppError::Database)?;
    if count.0 > 0 {
        return Ok(());
    }
    let now = chrono::Utc::now().timestamp_millis();
    let defaults = vec![
        ("会议纪要", "📝", "结构化会议记录模板", "text", "# 会议纪要\n\n**日期**：\n**参会人**：\n**主题**：\n\n## 讨论内容\n\n\n## 决议事项\n\n\n## 待办跟进\n- [ ] \n"),
        ("研究笔记", "🔬", "研究资料整理模板", "text", "# 研究笔记\n\n**课题**：\n**关键词**：\n**来源**：\n\n## 核心观点\n\n\n## 关键论据\n\n\n## 个人思考\n\n"),
        ("周报总结", "📊", "周报/工作总结模板", "text", "# 周报\n\n**周期**：\n\n## 本周完成\n- \n\n## 进行中\n- \n\n## 下周计划\n- \n\n## 问题与风险\n- \n"),
        ("读书笔记", "📖", "阅读摘录与心得模板", "text", "# 读书笔记\n\n**书名**：\n**作者**：\n**阅读时间**：\n\n## 摘录\n> \n\n## 心得\n\n\n## 行动清单\n- [ ] \n"),
        ("项目计划", "🚀", "项目规划与跟踪模板", "text", "# 项目计划\n\n**项目名**：\n**负责人**：\n**截止日**：\n\n## 目标\n\n\n## 里程碑\n1. \n\n## 资源需求\n\n\n## 风险预案\n\n"),
        ("链接收藏", "🔗", "快捷创建链接条目", "link", ""),
    ];
    for (name, icon, description, entry_type, content) in defaults {
        sqlx::query(
            "INSERT INTO kb_templates (name, icon, description, entry_type, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(name)
        .bind(icon)
        .bind(description)
        .bind(entry_type)
        .bind(content)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn migrate_kb_entries_favorite(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(kb_entries);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let names: Vec<String> = columns.iter().map(|r| r.get::<String, _>("name")).collect();
    if !names.contains(&"is_favorited".to_string()) {
        sqlx::query("ALTER TABLE kb_entries ADD COLUMN is_favorited INTEGER NOT NULL DEFAULT 0;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn migrate_kb_entries_content(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(kb_entries);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let names: Vec<String> = columns.iter().map(|r| r.get::<String, _>("name")).collect();
    if !names.contains(&"content".to_string()) {
        sqlx::query("ALTER TABLE kb_entries ADD COLUMN content TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn migrate_kb_entries_source_path(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(kb_entries);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let names: Vec<String> = columns.iter().map(|r| r.get::<String, _>("name")).collect();
    if !names.contains(&"source_path".to_string()) {
        sqlx::query("ALTER TABLE kb_entries ADD COLUMN source_path TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn migrate_kb_tags_color(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(kb_tags);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let names: Vec<String> = columns.iter().map(|r| r.get::<String, _>("name")).collect();
    if !names.contains(&"color".to_string()) {
        sqlx::query("ALTER TABLE kb_tags ADD COLUMN color TEXT NOT NULL DEFAULT '#00F0FF';")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

async fn create_todos_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: 静态部分 SQL 提取至 migrations/0022_create_todos_table/up.sql
    // 动态部分（migrate_todos_table）保留 inline：PRAGMA + ALTER TABLE 补 5 字段
    let sql = include_str!("../../migrations/0022_create_todos_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    migrate_todos_table(pool).await?;

    Ok(())
}

async fn migrate_todos_table(pool: &SqlitePool) -> Result<(), AppError> {
    let columns_result = sqlx::query("PRAGMA table_info(todos);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

    let column_names: Vec<String> = columns_result
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    if !column_names.contains(&"description".to_string()) {
        sqlx::query("ALTER TABLE todos ADD COLUMN description TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"priority".to_string()) {
        sqlx::query("ALTER TABLE todos ADD COLUMN priority TEXT NOT NULL DEFAULT 'medium';")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"due_date".to_string()) {
        sqlx::query("ALTER TABLE todos ADD COLUMN due_date TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"user_id".to_string()) {
        sqlx::query("ALTER TABLE todos ADD COLUMN user_id INTEGER;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"version".to_string()) {
        sqlx::query("ALTER TABLE todos ADD COLUMN version INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    Ok(())
}

async fn create_journals_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: SQL 提取至 migrations/0023_create_journals_table/up.sql
    let sql = include_str!("../../migrations/0023_create_journals_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_timers_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: SQL 提取至 migrations/0024_create_timers_table/up.sql
    let sql = include_str!("../../migrations/0024_create_timers_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_recycle_bin_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: 静态部分 SQL 提取至 migrations/0025_create_recycle_bin_table/up.sql
    // 动态部分（migrate_recycle_bin_table）保留 inline：PRAGMA + ALTER TABLE 补 2 字段
    let sql = include_str!("../../migrations/0025_create_recycle_bin_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    migrate_recycle_bin_table(pool).await?;

    Ok(())
}

async fn migrate_recycle_bin_table(pool: &SqlitePool) -> Result<(), AppError> {
    let columns_result = sqlx::query("PRAGMA table_info(recycle_bin);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

    let column_names: Vec<String> = columns_result
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    if !column_names.contains(&"title".to_string()) {
        sqlx::query("ALTER TABLE recycle_bin ADD COLUMN title TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"deleted_by".to_string()) {
        sqlx::query("ALTER TABLE recycle_bin ADD COLUMN deleted_by INTEGER;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    Ok(())
}

async fn create_user_profiles_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: SQL 提取至 migrations/0026_create_user_profiles_table/up.sql
    let sql = include_str!("../../migrations/0026_create_user_profiles_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_resumes_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: SQL 提取至 migrations/0027_create_resumes_table/up.sql
    let sql = include_str!("../../migrations/0027_create_resumes_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_timeline_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: SQL 提取至 migrations/0028_create_timeline_table/up.sql
    let sql = include_str!("../../migrations/0028_create_timeline_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_quotes_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: SQL 提取至 migrations/0029_create_quotes_table/up.sql
    let sql = include_str!("../../migrations/0029_create_quotes_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_news_cache_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: 静态部分 SQL 提取至 migrations/0030_create_news_cache_table/up.sql
    // 动态部分（migrate_news_cache_table）保留 inline：PRAGMA + ALTER TABLE 补 5 字段
    let sql = include_str!("../../migrations/0030_create_news_cache_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    migrate_news_cache_table(pool).await?;

    Ok(())
}

async fn migrate_news_cache_table(pool: &SqlitePool) -> Result<(), AppError> {
    let columns_result = sqlx::query("PRAGMA table_info(news_cache);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

    let column_names: Vec<String> = columns_result
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    if !column_names.contains(&"content".to_string()) {
        sqlx::query("ALTER TABLE news_cache ADD COLUMN content TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"category".to_string()) {
        sqlx::query("ALTER TABLE news_cache ADD COLUMN category TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"published_at".to_string()) {
        sqlx::query("ALTER TABLE news_cache ADD COLUMN published_at TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"is_favorite".to_string()) {
        sqlx::query("ALTER TABLE news_cache ADD COLUMN is_favorite INTEGER NOT NULL DEFAULT 0;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    if !column_names.contains(&"ai_summary".to_string()) {
        sqlx::query("ALTER TABLE news_cache ADD COLUMN ai_summary TEXT;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    Ok(())
}

async fn create_news_pending_delete_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: SQL 提取至 migrations/0031_create_news_pending_delete_table/up.sql
    let sql = include_str!("../../migrations/0031_create_news_pending_delete_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    Ok(())
}

async fn create_news_sources_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: 静态部分 SQL 提取至 migrations/0032_create_news_sources_table/up.sql
    // 动态部分（seed_default_news_sources）保留 inline：INSERT OR IGNORE 26 个默认新闻源
    let sql = include_str!("../../migrations/0032_create_news_sources_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    seed_default_news_sources(pool).await?;

    Ok(())
}

async fn seed_default_news_sources(pool: &SqlitePool) -> Result<(), AppError> {
    // 增量同步：逐条 INSERT OR IGNORE，已有旧数据的库也能补入新源
    let defaults = vec![
        // ===== 网安 (security) =====
        ("FreeBuf", "https://www.freebuf.com/feed", "security", "rss"),
        ("安全客", "https://www.anquanke.com/feed", "security", "rss"),
        ("安全内参", "https://www.secrss.com/feed", "security", "rss"),
        ("The Hacker News", "https://feeds.feedburner.com/TheHackersNews", "security", "rss"),
        ("BleepingComputer", "https://www.bleepingcomputer.com/feed/", "security", "rss"),
        ("Security Affairs", "https://securityaffairs.com/feed", "security", "rss"),

        // ===== 漏洞发现与解决 (vulnerability → 网安) =====
        ("NVD News", "https://nvd.nist.gov/feeds/xml/cve/misc/nvd-rss.xml", "vulnerability", "rss"),
        ("Seebug", "https://www.seebug.org/rss/new", "vulnerability", "rss"),
        ("Exploit DB", "https://feeds.exploit-db.com/exploitdb", "vulnerability", "rss"),
        ("Google 安全博客", "https://googleonlinesecurity.blogspot.com/feeds/posts/default", "vulnerability", "atom"),
        ("CISA 已知利用漏洞", "https://www.cisa.gov/known-exploited-vulnerabilities-catalog.xml", "vulnerability", "rss"),

        // ===== 攻防方案 (attack_defense → 网安) =====
        ("奇安信威胁情报", "https://ti.qianxin.com/feed", "attack_defense", "rss"),
        ("火线Zone", "https://www.huoxian.cn/feed", "attack_defense", "rss"),
        ("Dark Reading", "https://www.darkreading.com/rss.xml", "attack_defense", "rss"),

        // ===== 工具应用 (tool_application → 编程) =====
        ("GitHub Trending (全语言)", "https://mshibanami.github.io/GitHubTrendingRSS/daily/all.xml", "tool_application", "rss"),
        ("GitHub Trending (Rust)", "https://mshibanami.github.io/GitHubTrendingRSS/daily/rust.xml", "tool_application", "rss"),
        ("GitHub Trending (Python)", "https://mshibanami.github.io/GitHubTrendingRSS/daily/python.xml", "tool_application", "rss"),
        ("GitHub Trending (Go)", "https://mshibanami.github.io/GitHubTrendingRSS/daily/go.xml", "tool_application", "rss"),
        ("Dev.to", "https://dev.to/feed", "tool_application", "rss"),

        // ===== 技术创新 (tech_innovation → AI) =====
        ("机器之心", "https://www.jiqizhixin.com/rss", "tech_innovation", "rss"),
        ("量子位", "https://www.qbitai.com/feed", "tech_innovation", "rss"),
        ("Hacker News (首页)", "https://hnrss.org/frontpage", "tech_innovation", "rss"),

        // ===== 云原生 (cloud_native → 编程) =====
        ("CNCF Blog", "https://www.cncf.io/blog/feed/", "cloud_native", "rss"),

        // ===== 开源 (open_source → 编程) =====
        ("阮一峰", "https://www.ruanyifeng.com/blog/atom.xml", "open_source", "atom"),
        ("Solidot", "https://www.solidot.org/index.rss", "open_source", "rss"),

        // ===== HN 频道聚合 =====
        ("Hacker News (安全)", "https://hnrss.org/security", "security", "rss"),
        ("Hacker News (编程)", "https://hnrss.org/programming", "programming", "rss"),
    ];

    for (name, url, category, feed_type) in defaults {
        sqlx::query(
            "INSERT OR IGNORE INTO news_sources (name, url, category, feed_type) VALUES (?, ?, ?, ?)",
        )
        .bind(name)
        .bind(url)
        .bind(category)
        .bind(feed_type)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn create_system_config_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.3: SQL 提取至 migrations/0033_create_system_config_table/up.sql
    let sql = include_str!("../../migrations/0033_create_system_config_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_terminal_history_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0034_create_terminal_history_table/up.sql
    let sql = include_str!("../../migrations/0034_create_terminal_history_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn migrate_terminal_history_duration(pool: &SqlitePool) -> Result<(), AppError> {
    let has_column = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('terminal_history') WHERE name = 'duration_ms'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if has_column == 0 {
        sqlx::query("ALTER TABLE terminal_history ADD COLUMN duration_ms INTEGER")
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn create_terminal_tab_layout_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0036_create_terminal_tab_layout_table/up.sql
    let sql = include_str!("../../migrations/0036_create_terminal_tab_layout_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn migrate_terminal_tab_layout_pane_data(pool: &SqlitePool) -> Result<(), AppError> {
    let has_column = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('terminal_tab_layout') WHERE name = 'pane_data'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if has_column == 0 {
        sqlx::query("ALTER TABLE terminal_tab_layout ADD COLUMN pane_data TEXT")
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn create_terminal_sessions_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0038_create_terminal_sessions_table/up.sql
    let sql = include_str!("../../migrations/0038_create_terminal_sessions_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn migrate_terminal_sessions_columns(pool: &SqlitePool) -> Result<(), AppError> {
    // 为旧数据库添加 tab_id 列
    let has_tab_id = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('terminal_sessions') WHERE name = 'tab_id'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if has_tab_id == 0 {
        sqlx::query("ALTER TABLE terminal_sessions ADD COLUMN tab_id TEXT")
            .execute(pool)
            .await?;
    }

    // 为旧数据库添加 pane_id 列
    let has_pane_id = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pragma_table_info('terminal_sessions') WHERE name = 'pane_id'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if has_pane_id == 0 {
        sqlx::query("ALTER TABLE terminal_sessions ADD COLUMN pane_id TEXT")
            .execute(pool)
            .await?;
    }

    Ok(())
}

// T2.6.4: v40 保留 inline —— 多语句迁移（CREATE VIRTUAL TABLE + 3 个 CREATE TRIGGER）
// 原因：sqlx::query() 仅支持单语句，无法简单提取到单个 .sql 文件。
//       与动态迁移处理方式一致；如需文档化可后续单独创建 .sql 作为参考。
async fn create_terminal_history_fts(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query(
        "CREATE VIRTUAL TABLE IF NOT EXISTS terminal_history_fts USING fts5(
            command, output,
            content='terminal_history',
            content_rowid='id'
        );",
    )
    .execute(pool)
    .await?;

    // 触发器：INSERT 时同步
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS terminal_history_fts_insert AFTER INSERT ON terminal_history BEGIN
            INSERT INTO terminal_history_fts(rowid, command, output)
            VALUES (new.id, new.command, new.output);
        END;",
    )
    .execute(pool)
    .await?;

    // 触发器：DELETE 时同步
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS terminal_history_fts_delete AFTER DELETE ON terminal_history BEGIN
            INSERT INTO terminal_history_fts(terminal_history_fts, rowid, command, output)
            VALUES ('delete', old.id, old.command, old.output);
        END;",
    )
    .execute(pool)
    .await?;

    // 触发器：UPDATE 时同步
    sqlx::query(
        "CREATE TRIGGER IF NOT EXISTS terminal_history_fts_update AFTER UPDATE ON terminal_history BEGIN
            INSERT INTO terminal_history_fts(terminal_history_fts, rowid, command, output)
            VALUES ('delete', old.id, old.command, old.output);
            INSERT INTO terminal_history_fts(rowid, command, output)
            VALUES (new.id, new.command, new.output);
        END;",
    )
    .execute(pool)
    .await?;

    tracing::info!("FTS5 全文搜索索引已创建: terminal_history_fts");
    Ok(())
}

async fn create_ssh_profiles_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0041_create_ssh_profiles_table/up.sql
    let sql = include_str!("../../migrations/0041_create_ssh_profiles_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_terminal_config_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0042_create_terminal_config_table/up.sql
    let sql = include_str!("../../migrations/0042_create_terminal_config_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_editor_documents_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0043_create_editor_documents_table/up.sql
    let sql = include_str!("../../migrations/0043_create_editor_documents_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn migrate_editor_documents_table(pool: &SqlitePool) -> Result<(), AppError> {
    let columns_result = sqlx::query("PRAGMA table_info(editor_documents);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

    let column_names: Vec<String> = columns_result
        .iter()
        .map(|row| row.get::<String, _>("name"))
        .collect();

    if !column_names.contains(&"version".to_string()) {
        sqlx::query("ALTER TABLE editor_documents ADD COLUMN version INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    Ok(())
}

async fn create_editor_versions_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0045_create_editor_versions_table/up.sql
    let sql = include_str!("../../migrations/0045_create_editor_versions_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_editor_sessions_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0046_create_editor_sessions_table/up.sql
    let sql = include_str!("../../migrations/0046_create_editor_sessions_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_editor_fts_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0047_create_editor_fts_table/up.sql
    // 注意：与 v40 不同，本表仅单条 CREATE VIRTUAL TABLE（无触发器），可单语句提取
    let sql = include_str!("../../migrations/0047_create_editor_fts_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_yuan_code_snippets_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0048_create_yuan_code_snippets_table/up.sql
    let sql = include_str!("../../migrations/0048_create_yuan_code_snippets_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_yuan_code_workspaces_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0049_create_yuan_code_workspaces_table/up.sql
    let sql = include_str!("../../migrations/0049_create_yuan_code_workspaces_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_yuan_goals_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.4: SQL 提取至 migrations/0050_create_yuan_goals_table/up.sql
    let sql = include_str!("../../migrations/0050_create_yuan_goals_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_config_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0051_create_xin_config_table/up.sql
    let sql = include_str!("../../migrations/0051_create_xin_config_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_memories_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0052_create_xin_memories_table/up.sql
    let sql = include_str!("../../migrations/0052_create_xin_memories_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_summaries_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0053_create_xin_summaries_table/up.sql
    let sql = include_str!("../../migrations/0053_create_xin_summaries_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_moods_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0054_create_xin_moods_table/up.sql
    let sql = include_str!("../../migrations/0054_create_xin_moods_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_reminders_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0055_create_xin_reminders_table/up.sql
    let sql = include_str!("../../migrations/0055_create_xin_reminders_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_habits_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0056_create_xin_habits_table/up.sql
    let sql = include_str!("../../migrations/0056_create_xin_habits_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_conversations_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0057_create_xin_conversations_table/up.sql
    let sql = include_str!("../../migrations/0057_create_xin_conversations_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_checkpoints_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0058_create_xin_checkpoints_table/up.sql
    let sql = include_str!("../../migrations/0058_create_xin_checkpoints_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_compaction_records_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0059_create_xin_compaction_records_table/up.sql
    let sql = include_str!("../../migrations/0059_create_xin_compaction_records_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_xin_compaction_config_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0060_create_xin_compaction_config_table/up.sql
    let sql = include_str!("../../migrations/0060_create_xin_compaction_config_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_activity_logs_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0061_create_activity_logs_table/up.sql
    // 动态部分（create_activity_logs_view）保留 inline：CREATE VIEW 日汇总视图
    let sql = include_str!("../../migrations/0061_create_activity_logs_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    create_activity_logs_view(pool).await?;

    Ok(())
}

async fn create_activity_logs_view(pool: &SqlitePool) -> Result<(), AppError> {
    sqlx::query(
        "CREATE VIEW IF NOT EXISTS v_daily_activity_summary AS
         SELECT
             user_id,
             date(timestamp) AS activity_date,
             module,
             COUNT(*) AS operation_count,
             SUM(duration_secs) AS total_duration_secs,
             COUNT(DISTINCT detail) AS unique_operations
         FROM activity_logs
         GROUP BY user_id, date(timestamp), module;",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn create_suggestions_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0062_create_suggestions_table/up.sql
    let sql = include_str!("../../migrations/0062_create_suggestions_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_behavior_patterns_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0063_create_behavior_patterns_table/up.sql
    let sql = include_str!("../../migrations/0063_create_behavior_patterns_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_intelligence_settings_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0064_create_intelligence_settings_table/up.sql
    let sql = include_str!("../../migrations/0064_create_intelligence_settings_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

async fn create_auth_sessions_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0065_create_auth_sessions_table/up.sql
    let sql = include_str!("../../migrations/0065_create_auth_sessions_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

// T2.6.5: v66 保留 inline —— 50+ 索引批量创建（vec 迭代）
// 原因：每个索引是独立 sqlx::query 调用，无法合并到单个 .sql 文件（sqlx::query 单语句限制）。
//       索引本质上是 CREATE INDEX 语句的集合，提取到 .sql 也只是文档化，无运行时价值。
async fn create_indexes(pool: &SqlitePool) -> Result<(), AppError> {
    let indexes = vec![
        "CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);",
        "CREATE INDEX IF NOT EXISTS idx_users_role ON users(role);",
        "CREATE INDEX IF NOT EXISTS idx_permissions_role ON permissions(role);",
        "CREATE INDEX IF NOT EXISTS idx_ai_models_provider ON ai_models(provider);",
        "CREATE INDEX IF NOT EXISTS idx_ai_agents_model_id ON ai_agents(model_id);",
        "CREATE INDEX IF NOT EXISTS idx_conversations_type ON conversations(type);",
        "CREATE INDEX IF NOT EXISTS idx_conversation_participants_conv ON conversation_participants(conversation_id);",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_conversation_participants_unique ON conversation_participants(conversation_id, model_id, agent_id);",
        "CREATE INDEX IF NOT EXISTS idx_messages_conversation ON messages(conversation_id);",
        "CREATE INDEX IF NOT EXISTS idx_messages_round ON messages(conversation_id, round);",
        "CREATE INDEX IF NOT EXISTS idx_kb_categories_parent ON kb_categories(parent_id);",
        "CREATE INDEX IF NOT EXISTS idx_kb_categories_library ON kb_categories(library);",
        "CREATE INDEX IF NOT EXISTS idx_kb_entries_category ON kb_entries(category_id);",
        "CREATE INDEX IF NOT EXISTS idx_kb_entry_tags_entry ON kb_entry_tags(entry_id);",
        "CREATE INDEX IF NOT EXISTS idx_kb_entry_tags_tag ON kb_entry_tags(tag_id);",
        "CREATE INDEX IF NOT EXISTS idx_kb_recent_access_entry ON kb_recent_access(entry_id);",
        "CREATE INDEX IF NOT EXISTS idx_kb_recent_access_time ON kb_recent_access(accessed_at DESC);",
        "CREATE INDEX IF NOT EXISTS idx_todos_date ON todos(date);",
        "CREATE INDEX IF NOT EXISTS idx_journals_date ON journals(date);",
        "CREATE INDEX IF NOT EXISTS idx_recycle_bin_type ON recycle_bin(item_type);",
        "CREATE INDEX IF NOT EXISTS idx_recycle_bin_auto_delete ON recycle_bin(auto_delete_at);",
        "CREATE INDEX IF NOT EXISTS idx_timeline_date ON timeline(date);",
        "CREATE INDEX IF NOT EXISTS idx_news_cache_fetched ON news_cache(fetched_at);",
        "CREATE INDEX IF NOT EXISTS idx_news_cache_favorite ON news_cache(is_favorite);",
        "CREATE INDEX IF NOT EXISTS idx_news_pending_delete_moved ON news_pending_delete(moved_at);",
        "CREATE INDEX IF NOT EXISTS idx_system_config_key ON system_config(config_key);",
        "CREATE INDEX IF NOT EXISTS idx_terminal_history_created ON terminal_history(created_at);",
        "CREATE INDEX IF NOT EXISTS idx_terminal_history_session ON terminal_history(session_type);",
        "CREATE INDEX IF NOT EXISTS idx_terminal_sessions_tab ON terminal_sessions(tab_id);",
        "CREATE INDEX IF NOT EXISTS idx_terminal_sessions_pane ON terminal_sessions(pane_id);",
        "CREATE INDEX IF NOT EXISTS idx_ssh_profiles_name ON ssh_profiles(name);",
        "CREATE INDEX IF NOT EXISTS idx_editor_documents_source ON editor_documents(source_type, source_id);",
        "CREATE INDEX IF NOT EXISTS idx_editor_documents_uuid ON editor_documents(doc_uuid);",
        "CREATE INDEX IF NOT EXISTS idx_editor_versions_doc ON editor_versions(doc_uuid, version_num);",
        "CREATE INDEX IF NOT EXISTS idx_editor_sessions_doc ON editor_sessions(doc_uuid);",
        "CREATE INDEX IF NOT EXISTS idx_yuan_code_snippets_lang ON yuan_code_snippets(language);",
        "CREATE INDEX IF NOT EXISTS idx_yuan_code_snippets_name ON yuan_code_snippets(name);",
        "CREATE INDEX IF NOT EXISTS idx_yuan_code_workspaces_path ON yuan_code_workspaces(workspace_path);",
        "CREATE INDEX IF NOT EXISTS idx_xin_memories_category ON xin_memories(category);",
        "CREATE INDEX IF NOT EXISTS idx_xin_memories_key ON xin_memories(key);",
        "CREATE INDEX IF NOT EXISTS idx_xin_summaries_conv_id ON xin_summaries(conversation_id);",
        "CREATE INDEX IF NOT EXISTS idx_xin_moods_category ON xin_moods(category);",
        "CREATE INDEX IF NOT EXISTS idx_xin_reminders_active ON xin_reminders(is_active);",
        "CREATE INDEX IF NOT EXISTS idx_xin_habits_category ON xin_habits(category);",
        "CREATE INDEX IF NOT EXISTS idx_xin_conversations_persona ON xin_conversations(persona_id);",
        "CREATE INDEX IF NOT EXISTS idx_xin_conversations_title ON xin_conversations(title);",
        "CREATE INDEX IF NOT EXISTS idx_xin_checkpoints_conv ON xin_checkpoints(conversation_id, created_at DESC);",
        "CREATE INDEX IF NOT EXISTS idx_xin_compaction_records_conv ON xin_compaction_records(conversation_id, created_at DESC);",
        "CREATE INDEX IF NOT EXISTS idx_activity_logs_user_time ON activity_logs(user_id, timestamp);",
        "CREATE INDEX IF NOT EXISTS idx_activity_logs_module ON activity_logs(user_id, module, timestamp);",
        "CREATE INDEX IF NOT EXISTS idx_suggestions_user_status ON suggestions(user_id, status);",
        "CREATE INDEX IF NOT EXISTS idx_suggestions_category ON suggestions(user_id, category);",
        "CREATE INDEX IF NOT EXISTS idx_behavior_patterns_user_date ON behavior_patterns(user_id, date);",
        "CREATE INDEX IF NOT EXISTS idx_intelligence_settings_user ON intelligence_settings(user_id);",
        "CREATE INDEX IF NOT EXISTS idx_auth_sessions_token ON auth_sessions(token);",
        "CREATE INDEX IF NOT EXISTS idx_auth_sessions_user ON auth_sessions(user_id, is_active);",
    ];

    for idx in indexes {
        sqlx::query(idx).execute(pool).await?;
    }

    Ok(())
}

// T2.6.5: v67 保留 inline —— 数据补丁迁移（UPDATE 语句 + 条件逻辑 + rows_affected 统计）
async fn migrate_news_github_category(pool: &SqlitePool) -> Result<(), AppError> {
    // 1. 更新 news_sources 表：GitHub Trending 源 → github 分类
    let sources_updated = sqlx::query(
        "UPDATE news_sources SET category = 'github' \
         WHERE url LIKE '%GitHubTrendingRSS%' AND category != 'github'"
    )
    .execute(pool)
    .await?
    .rows_affected();

    // 2. 更新 news_cache 表：来源为 GitHub Trending 的条目 → github 分类
    let news_updated = sqlx::query(
        "UPDATE news_cache SET category = 'github' \
         WHERE source IN (SELECT name FROM news_sources WHERE url LIKE '%GitHubTrendingRSS%') \
         AND category != 'github'"
    )
    .execute(pool)
    .await?
    .rows_affected();

    if sources_updated > 0 || news_updated > 0 {
        tracing::info!(
            sources = sources_updated,
            news = news_updated,
            "🔄 [迁移] GitHub 新闻分类已更新"
        );
    }

    Ok(())
}

// T2.6.5: v68 保留 inline —— ALTER TABLE + 错误吞咽（列已存在时忽略）
async fn migrate_conversations_starred(pool: &SqlitePool) -> Result<(), AppError> {
    let result = sqlx::query(
        "ALTER TABLE conversations ADD COLUMN starred INTEGER NOT NULL DEFAULT 0",
    )
    .execute(pool)
    .await;
    // 如果列已存在则忽略错误
    if let Err(_) = result {
        // 列可能已存在，忽略
    }
    Ok(())
}

async fn create_prompt_templates_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: SQL 提取至 migrations/0069_create_prompt_templates_table/up.sql
    let sql = include_str!("../../migrations/0069_create_prompt_templates_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    Ok(())
}

// T2.6.5: v70 保留 inline —— 条件 RENAME 归档（PRAGMA 检测 + sqlite_master 查询 + 多分支逻辑）
async fn archive_legacy_game_tables(pool: &SqlitePool) -> Result<(), AppError> {
    // 检测旧 game_worlds 是否存在
    let has_legacy_worlds: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type='table' AND name='game_worlds';",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if has_legacy_worlds > 0 {
        // 检查是否为旧结构（含 world_json 列）
        let columns = sqlx::query("PRAGMA table_info(game_worlds);")
            .fetch_all(pool)
            .await
            .map_err(AppError::Database)?;
        let names: Vec<String> = columns
            .iter()
            .map(|r| r.get::<String, _>("name"))
            .collect();

        if names.contains(&"world_json".to_string()) {
            // 旧表存在且为 JSON Blob 结构 → 重命名归档
            sqlx::query("ALTER TABLE game_worlds RENAME TO game_worlds_legacy;")
                .execute(pool)
                .await
                .map_err(AppError::Database)?;
            tracing::info!("🔄 [迁移] 旧 game_worlds 表已归档为 game_worlds_legacy");
        }
        // 若已存在新结构表，则跳过（幂等）
    }

    // 旧 game_reward_logs 表归档（若存在）
    let has_legacy_reward_logs: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master
         WHERE type='table' AND name='game_reward_logs';",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if has_legacy_reward_logs > 0 {
        sqlx::query("ALTER TABLE game_reward_logs RENAME TO game_reward_logs_legacy;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        tracing::info!("🔄 [迁移] 旧 game_reward_logs 表已归档为 game_reward_logs_legacy");
    }

    Ok(())
}

async fn create_game_worlds_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0071_create_game_worlds_table/up.sql
    // 索引 idx_game_worlds_updated 保留 inline（sqlx::query 单语句限制）
    let sql = include_str!("../../migrations/0071_create_game_worlds_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_worlds_updated ON game_worlds(updated_at DESC);",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_game_knowledge_domains_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0072_create_game_knowledge_domains_table/up.sql
    // 索引 + 种子数据（12 个知识领域）保留 inline
    let sql = include_str!("../../migrations/0072_create_game_knowledge_domains_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_knowledge_domains_sort ON game_knowledge_domains(sort_order);",
    )
    .execute(pool)
    .await?;

    seed_game_knowledge_domains(pool).await?;

    Ok(())
}

async fn create_game_buildings_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0073_create_game_buildings_table/up.sql
    // 2 个索引保留 inline
    let sql = include_str!("../../migrations/0073_create_game_buildings_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_buildings_world ON game_buildings(world_id);",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_buildings_status ON game_buildings(status);",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_game_knowledge_progress_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0074_create_game_knowledge_progress_table/up.sql
    // 索引保留 inline
    let sql = include_str!("../../migrations/0074_create_game_knowledge_progress_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_knowledge_progress_world ON game_knowledge_progress(world_id);",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_game_breakthrough_records_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0075_create_game_breakthrough_records_table/up.sql
    // 索引保留 inline
    let sql = include_str!("../../migrations/0075_create_game_breakthrough_records_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_breakthrough_records_world ON game_breakthrough_records(world_id, created_at DESC);",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_game_build_history_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0076_create_game_build_history_table/up.sql
    // 2 个索引保留 inline
    let sql = include_str!("../../migrations/0076_create_game_build_history_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_build_history_world ON game_build_history(world_id, created_at DESC);",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_build_history_building ON game_build_history(building_id);",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_game_kb_category_mapping_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0077_create_game_kb_category_mapping_table/up.sql
    // 索引保留 inline
    let sql = include_str!("../../migrations/0077_create_game_kb_category_mapping_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_kb_category_mapping_domain ON game_kb_category_mapping(domain_id);",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_game_points_log_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0078_create_game_points_log_table/up.sql
    // 2 个索引保留 inline
    let sql = include_str!("../../migrations/0078_create_game_points_log_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_points_log_world ON game_points_log(world_id, created_at DESC);",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_points_log_domain ON game_points_log(world_id, domain_id, created_at DESC);",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_game_daily_limit_counter_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0079_create_game_daily_limit_counter_table/up.sql
    // 索引保留 inline
    let sql = include_str!("../../migrations/0079_create_game_daily_limit_counter_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_game_daily_limit_world ON game_daily_limit_counter(world_id, counter_date);",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_game_points_source_config_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0080_create_game_points_source_config_table/up.sql
    // 种子数据（11 条积分来源配置）保留 inline
    let sql = include_str!("../../migrations/0080_create_game_points_source_config_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    seed_game_points_source_config(pool).await?;

    Ok(())
}

/// 种子数据：12 个学术知识领域定义。
/// 使用 INSERT OR IGNORE 保证幂等（id 主键唯一约束）。
/// 领域与建筑大类对应关系见 04_建筑系统设计.md / 03_积分系统设计.md。
async fn seed_game_knowledge_domains(pool: &SqlitePool) -> Result<(), AppError> {
    // (id, name, name_en, description, building_category, sort_order)
    let domains: Vec<(&str, &str, &str, &str, &str, i64)> = vec![
        ("cs",          "计算机科学", "Computer Science", "涵盖编程、算法、系统、网络、数据库、AI 等所有与计算相关的内容", "technology", 1),
        ("math",        "数学",       "Mathematics",      "纯数学与应用数学，包括代数、几何、分析、概率统计、离散数学",     "technology", 2),
        ("physics",     "物理",       "Physics",          "经典物理与现代物理，理论与实验",                               "technology", 3),
        ("literature",  "文学",       "Literature",       "中外文学创作与文学研究",                                       "palace",     4),
        ("history",     "历史",       "History",          "历史事件、史学研究、考古",                                     "kingdom",    5),
        ("art",         "艺术",       "Art",              "视觉艺术、表演艺术、设计",                                     "palace",     6),
        ("engineering", "工程",       "Engineering",      "工作执行类行为的默认领域，也涵盖工程学科",                     "city",       7),
        ("medicine",    "医学",       "Medicine",         "医学、药学、健康科学",                                         "sect",       8),
        ("philosophy",  "哲学",       "Philosophy",       "哲学思考与冥想类行为的默认领域，也涵盖哲学学科",               "sect",       9),
        ("economics",   "经济",       "Economics",        "经济学、金融学、管理学",                                       "city",       10),
        ("language",    "语言",       "Language",         "语言学与外语学习",                                             "house",      11),
        ("other",       "其它",       "Other",            "兜底领域，未映射分类的条目默认归入此领域",                     "house",      12),
    ];

    for (id, name, name_en, description, building_category, sort_order) in domains {
        sqlx::query(
            "INSERT OR IGNORE INTO game_knowledge_domains
             (id, name, name_en, description, building_category, sort_order)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(name_en)
        .bind(description)
        .bind(building_category)
        .bind(sort_order)
        .execute(pool)
        .await?;
    }

    tracing::info!("🌱 [种子] game_knowledge_domains 已初始化 12 个知识领域");
    Ok(())
}

/// 种子数据：11 条积分来源配置。
/// 使用 INSERT OR IGNORE 保证幂等（source_type 主键唯一约束）。
/// daily_limit 为 NULL 表示无上限；enabled 默认 1。
async fn seed_game_points_source_config(pool: &SqlitePool) -> Result<(), AppError> {
    let now = chrono::Utc::now().timestamp_millis();
    // (source_type, default_domain_id, points_per_event, daily_limit, description)
    let configs: Vec<(&str, Option<&str>, i64, Option<i64>, &str)> = vec![
        ("kb_entry_create",       None,                 10, None,      "知识库条目新增"),
        ("kb_category_create",    None,                 50, None,      "知识库分类新增"),
        ("kb_entry_update",       None,                  2, Some(20),  "知识库条目更新"),
        ("todo_complete",         Some("engineering"),   5, Some(50),  "待办完成"),
        ("timer_short_complete",  Some("engineering"),   3, None,      "计时器完成（短，<25min）"),
        ("timer_long_complete",   Some("engineering"),   5, None,      "计时器完成（长，≥25min）"),
        ("ai_chat_turn",          Some("cs"),            5, Some(50),  "AI 会话单轮"),
        ("yuancode_use",          Some("cs"),            8, Some(80),  "YuanCode 使用"),
        ("journal_record",        None,                  3, Some(30),  "日志记录"),
        ("building_refund",       None,                  0, None,      "拆除建筑返还（points 在调用方计算）"),
        ("manual_adjust",         None,                  0, None,      "手动调整（管理员/调试）"),
    ];

    for (source_type, default_domain_id, points_per_event, daily_limit, description) in configs {
        sqlx::query(
            "INSERT OR IGNORE INTO game_points_source_config
             (source_type, default_domain_id, points_per_event, daily_limit, description, enabled, updated_at)
             VALUES (?, ?, ?, ?, ?, 1, ?)",
        )
        .bind(source_type)
        .bind(default_domain_id)
        .bind(points_per_event)
        .bind(daily_limit)
        .bind(description)
        .bind(now)
        .execute(pool)
        .await?;
    }

    tracing::info!("🌱 [种子] game_points_source_config 已初始化 11 条积分来源配置");
    Ok(())
}

// ============================================================================
// v1.52 性能基线：性能指标采集表（01_性能优化_首屏200ms计划.md §2.1.3）
// ============================================================================

/// 性能指标采集表：记录启动耗时、FCP/LCP/TTI、路由切换、IPC 调用延迟等。
/// 前端 App.tsx 启动后通过 IPC 上报，后端可定期生成性能仪表盘。
async fn create_perf_metrics_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.6.5: 静态部分 SQL 提取至 migrations/0081_create_perf_metrics_table/up.sql
    // 索引保留 inline
    let sql = include_str!("../../migrations/0081_create_perf_metrics_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_perf_metrics_name_time
         ON perf_metrics(metric_name, recorded_at DESC);",
    )
    .execute(pool)
    .await?;

    tracing::info!("📊 [v1.52] perf_metrics 表已创建");
    Ok(())
}

// ============================================================================
// v1.52.4 T2.7.1：审计日志基础设施（功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.4）
// ============================================================================

/// 审计日志主表：记录 10 张关键表（users/permissions/ai_models/ai_agents/
/// kb_categories/kb_entries/system_config/conversations/messages/todos）的
/// INSERT/UPDATE/DELETE 操作。old_data/new_data 以 JSON 格式存储关键字段快照。
/// 索引（2 个）已包含在 .sql 文件内：(table_name, record_id) + (changed_at)。
async fn create_audit_log_table(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.7.1: 静态 SQL（CREATE TABLE + 2 个索引）提取至 migrations/0082_create_audit_log_table/up.sql
    let sql = include_str!("../../migrations/0082_create_audit_log_table/up.sql");
    sqlx::query(sql).execute(pool).await?;

    tracing::info!("📋 [v1.52.4] audit_log 表已创建（含 2 个索引）");
    Ok(())
}

/// 审计触发器：为 10 张关键表各创建 3 个触发器（INSERT/UPDATE/DELETE），
/// 共 30 个触发器。所有触发器将操作记录写入 audit_log 表。
/// 设计要点：
/// - 只记录业务关键字段（非全部字段），避免日志过大
/// - messages.content 截断到 500 字符（substr(content, 1, 500)）
/// - changed_by 默认 'system'（SQLite 触发器无法访问应用层用户上下文，
///   后续 A4 安全加固阶段可通过 session 上下文细化）
async fn create_audit_triggers(pool: &SqlitePool) -> Result<(), AppError> {
    // T2.7.1: 静态 SQL（30 个 CREATE TRIGGER IF NOT EXISTS）提取至 migrations/0083_create_audit_triggers/up.sql
    let sql = include_str!("../../migrations/0083_create_audit_triggers/up.sql");
    sqlx::query(sql).execute(pool).await?;

    tracing::info!("⚡ [v1.52.4] 30 个审计触发器已创建（10 表 × INSERT/UPDATE/DELETE）");
    Ok(())
}

// =============================================================================
// T2.7.4: 外键补建 - 表重建迁移（方案 A：仅类型匹配的 7 条 FK）
// 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.2.2
// 说明: SQLite 不支持 ALTER TABLE ADD FOREIGN KEY，必须通过表重建实现。
//       每个迁移在 SQL 文件中完成: CREATE _new → INSERT → DROP old → RENAME → 重建索引。
//       数据清理: 删除引用了不存在目标记录的孤儿数据（防御性）。
// =============================================================================

/// v84: 重建 kb_references 表，添加 2 条 FK（source/target_entry_id → kb_entries.id ON DELETE CASCADE）
async fn rebuild_kb_references_with_fk(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0084_rebuild_kb_references_with_fk/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔗 [v1.52.4] kb_references 表已重建（添加 2 条 FK → kb_entries.id）");
    Ok(())
}

/// v85: 重建 kb_snapshots 表，添加 1 条 FK（entry_id → kb_entries.id ON DELETE CASCADE）
async fn rebuild_kb_snapshots_with_fk(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0085_rebuild_kb_snapshots_with_fk/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔗 [v1.52.4] kb_snapshots 表已重建（添加 1 条 FK → kb_entries.id）");
    Ok(())
}

/// v86: 重建 terminal_sessions 表，添加 1 条 FK（tab_id → terminal_tab_layout.tab_id ON DELETE SET NULL）
///
/// 兼容性修复（v1.53）：部分旧库（v1.51 之前）的 terminal_sessions 表可能缺少 status 列。
/// 原因：v38 使用 `CREATE TABLE IF NOT EXISTS`，对已存在的表无效；
///       v39 `migrate_terminal_sessions_columns` 只补了 tab_id/pane_id，遗漏了 status。
/// 修复策略：执行表重建 SQL 前，先用 PRAGMA table_info 检测并补加缺失列。
async fn rebuild_terminal_sessions_with_fk(pool: &SqlitePool) -> Result<(), AppError> {
    // 检测并补加可能缺失的列（v1.51 之前旧库）
    let missing_cols: Vec<(&str, &str)> = vec![
        ("session_type", "TEXT NOT NULL DEFAULT 'cmd'"),
        ("cols", "INTEGER NOT NULL DEFAULT 80"),
        ("rows", "INTEGER NOT NULL DEFAULT 24"),
        ("status", "TEXT NOT NULL DEFAULT 'active'"),
        ("created_at", "INTEGER NOT NULL DEFAULT 0"),
        ("killed_at", "INTEGER"),
    ];

    for (col_name, col_def) in missing_cols {
        let has_col = sqlx::query_scalar::<_, i64>(
            &format!(
                "SELECT COUNT(*) FROM pragma_table_info('terminal_sessions') WHERE name = '{}'",
                col_name
            ),
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        if has_col == 0 {
            tracing::warn!(
                "⚠️ [v1.52.4] 旧库 terminal_sessions 缺少 {} 列，正在补加...",
                col_name
            );
            sqlx::query(&format!(
                "ALTER TABLE terminal_sessions ADD COLUMN {} {}",
                col_name, col_def
            ))
            .execute(pool)
            .await?;
        }
    }

    let sql = include_str!("../../migrations/0086_rebuild_terminal_sessions_with_fk/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔗 [v1.52.4] terminal_sessions 表已重建（添加 1 条 FK → terminal_tab_layout.tab_id）");
    Ok(())
}

/// v87: 重建 xin_checkpoints 表，添加 1 条 FK（conversation_id → xin_conversations.id ON DELETE CASCADE）
async fn rebuild_xin_checkpoints_with_fk(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0087_rebuild_xin_checkpoints_with_fk/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔗 [v1.52.4] xin_checkpoints 表已重建（添加 1 条 FK → xin_conversations.id）");
    Ok(())
}

/// v88: 重建 xin_compaction_records 表，添加 1 条 FK（conversation_id → xin_conversations.id ON DELETE CASCADE）
async fn rebuild_xin_compaction_records_with_fk(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0088_rebuild_xin_compaction_records_with_fk/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔗 [v1.52.4] xin_compaction_records 表已重建（添加 1 条 FK → xin_conversations.id）");
    Ok(())
}

/// v89: 重建 yuan_goals 表，添加 1 条自引用 FK（parent_goal_id → yuan_goals.id ON DELETE SET NULL）
///
/// 特殊处理：自引用 FK 需要在事务中执行 + PRAGMA defer_foreign_keys=ON，
/// 以避免 INSERT 时父节点还未插入导致 FK 检查失败。
/// SQL 文件中的 INSERT 已用 ORDER BY id ASC，但 defer_foreign_keys 提供双重保障。
async fn rebuild_yuan_goals_with_fk(pool: &SqlitePool) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    // 延迟 FK 检查到 COMMIT（处理自引用 FK 的父节点顺序问题）
    sqlx::query("PRAGMA defer_foreign_keys = ON")
        .execute(&mut *tx)
        .await?;

    let sql = include_str!("../../migrations/0089_rebuild_yuan_goals_with_fk/up.sql");
    sqlx::query(sql).execute(&mut *tx).await?;

    tx.commit().await?;

    tracing::info!("🔗 [v1.52.4] yuan_goals 表已重建（添加自引用 FK → yuan_goals.id）");
    Ok(())
}

/// v108: 补建健康检查期望的缺失索引（Phase 3 §2.2.2）
///
/// 规范: 功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 3
///
/// 补建 6 个索引：
/// - idx_messages_conversation_id / idx_kb_entries_category_id / idx_kb_categories_parent_id
/// - idx_audit_log_table_row / idx_audit_log_created_at（与 v82 现有索引同列不同名，按期望名补建）
/// - idx_backup_records_type_created（防御性创建 backup_records 表后再建索引）
///
/// 跳过项：
/// - idx_users_email — users 表无 email 列
/// - idx_conversations_user_id — conversations 表无 user_id 列（多 AI 群聊设计）
/// 已在 health_check_service::CRITICAL_INDEXES 中移除对应期望。
async fn supplement_missing_indexes(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0108_supplement_missing_indexes/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🐌 [Phase3-2.2.2] v108 索引补建完成（6 个索引，2 项因列不存在跳过）");
    Ok(())
}

/// v109: 创建 api_keys 表（Yuan Code v3.1 云端 API Key 专用存储）
///
/// 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.5.1 + 项目核心设计意图 §三
///
/// 与 ai_models 表的区别：
/// - ai_models 混合存储本地 ollama 模型 + 云端模型，每条记录对应一个模型实例
/// - api_keys 按 provider 维度集中存储云端编程 API Key（一个 provider 一条），
///   专为 Yuan Code 编程 AI 强制走云端 API 设计，禁止本地底层智能模型用于编程生成
///
/// 加密：api_key_enc + api_key_nonce 通过 crypto::aes_gcm 加密（复用 MEK）
async fn create_api_keys_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0109_create_api_keys_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🔑 [D1-v3.1-3.5.1] v109 api_keys 表创建完成（云端 API Key 专用存储）");
    Ok(())
}

/// v110: 创建 model_routing_rules 表（Yuan Code v3.2 Task 3.4.1 模型路由配置）
///
/// 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §3.4.1 + 项目核心设计意图 §三/§八
///
/// 强制约束（is_cloud_only = 1）：
/// - 编程任务路由规则只能选云端 API provider（openai/anthropic/deepseek/...）
/// - 禁止选本地底层智能模型（ollama qwen3:8b 等）
/// - 守卫由 model_routing_service 层（CLOUD_API_PROVIDERS 白名单）强制
///
/// 与 api_keys 表的关系：
/// - api_keys 表：按 provider 维度存储云端 API Key（密文）
/// - model_routing_rules 表：按 task_type 维度存储"使用哪个 provider+model"
/// - 路由解析：task_type → rule.provider → api_keys.provider → 取密钥 → 调云端 API
async fn create_model_routing_rules_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0110_create_model_routing_rules_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("🛰️ [D1-v3.2-3.4.1] v110 model_routing_rules 表创建完成（按任务类型路由到云端 API 模型）");
    Ok(())
}

/// v111: 创建 skill_ratings 表（Yuan Code v3.2 Task 3.3.4 Skill 评分/评论系统）
///
/// 规范: 功能展望/模块深化/01_Yuan_Code_对标Codex升级_v3.md §Phase 5（Skill 评分/评论）
///
/// 表结构：
/// - skill_name + user_id 联合唯一（一人一评，更新走 UPSERT）
/// - rating: 1-5 星（CHECK 约束）
/// - review: 可选评论文本
async fn create_skill_ratings_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0111_create_skill_ratings_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("⭐ [D1-v3.2-3.3.4] v111 skill_ratings 表创建完成（Skill 市场评分/评论系统）");
    Ok(())
}

/// v112: 创建 news_offline_cache 表（A5 离线同步 Phase 3 Task 3 新闻源离线缓存）
///
/// 规范: 功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 3
///
/// 设计说明：
/// - 与现有 news_cache（v30）解耦：news_cache 是 RSS 拉取后的主存储（含评分/分类/收藏），
///   news_offline_cache 是离线展示用的快照副本，仅保留离线展示所需的最小字段集。
/// - id 采用 TEXT（基于 source + url + title 哈希），避免与 news_cache 的 INTEGER 主键冲突。
/// - cached_at 记录写入本表的时间，用于 UI 展示「上次更新 N 分钟前」。
/// - 在线时 fetch_and_cache_news 成功后由 news_cache_service::cache_news_items 同步写入本表；
///   离线时由 news_cache_service::get_cached_news 读取展示。
async fn create_news_offline_cache_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0112_create_news_offline_cache_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("📰 [A5-Phase3-Task3] v112 news_offline_cache 表创建完成（新闻源离线缓存）");
    Ok(())
}

/// v113: 创建 kb_attachment_cache 表（A5 离线同步 Phase 3 Task 4 知识库附件预加载）
///
/// 规范: 功能展望/平台级增强/04_离线与同步机制.md §Phase 3 Task 4
///
/// 设计说明：
/// - 标记常用 KB 附件（pin）+ 预加载文件副本到 app_data_dir/attachment_cache/ 目录
/// - last_accessed_at 实现 LRU 淘汰（默认 500MB + 30 天阈值）
/// - 离线打开附件时从 local_cache_path 读取，无需访问原始文件路径
/// - FK 引用 kb_entries(id) ON DELETE CASCADE：条目删除时自动级联清理缓存记录
///   （但本地缓存文件需由 service 层显式删除，FK 仅清理 DB 记录）
async fn create_kb_attachment_cache_table(pool: &SqlitePool) -> Result<(), AppError> {
    let sql = include_str!("../../migrations/0113_create_kb_attachment_cache_table/up.sql");
    sqlx::query(sql).execute(pool).await?;
    tracing::info!("📌 [A5-Phase3-Task4] v113 kb_attachment_cache 表创建完成（知识库附件预加载）");
    Ok(())
}

/// v114: 给 ai_models 表增加 status / last_health_check / latency_ms 字段
///
/// 规范: spec ai-chat-enhancement Phase 1 §1.1
///
/// 说明:
/// - status: online/offline/checking/error/unknown（默认 unknown，首次后台检测后填充真实值）
/// - last_health_check: unix timestamp ms，NULL 表示从未检测
/// - latency_ms: 最近一次成功检测的延迟，NULL 表示未检测或失败
///
/// 兼容性: 用 PRAGMA table_info 检查 status 字段是否存在，避免重复 ALTER TABLE。
///         up.sql 包含 3 条 ALTER TABLE 语句，sqlx::query 只能执行单条，故逐条执行。
async fn add_model_health_fields(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(ai_models);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let names: Vec<String> = columns.iter().map(|r| r.get::<String, _>("name")).collect();

    if !names.contains(&"status".to_string()) {
        sqlx::query("ALTER TABLE ai_models ADD COLUMN status TEXT NOT NULL DEFAULT 'unknown';")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("ALTER TABLE ai_models ADD COLUMN last_health_check INTEGER;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("ALTER TABLE ai_models ADD COLUMN latency_ms INTEGER;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

/// v115: 给 conversations 表增加 unread_count / sort_order 字段
///
/// 规范: spec ai-chat-enhancement Phase 2 §2.1
///
/// 说明:
/// - unread_count: 未读消息数（默认 0），用于会话列表角标显示
/// - sort_order: 拖拽自定义排序值（默认 0），sort_order ASC + updated_at DESC 排序
///
/// 兼容性: 用 PRAGMA table_info 检查 unread_count 字段是否存在，避免重复 ALTER TABLE。
async fn add_conversation_list_fields(pool: &SqlitePool) -> Result<(), AppError> {
    let columns = sqlx::query("PRAGMA table_info(conversations);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let names: Vec<String> = columns.iter().map(|r| r.get::<String, _>("name")).collect();

    if !names.contains(&"unread_count".to_string()) {
        sqlx::query("ALTER TABLE conversations ADD COLUMN unread_count INTEGER NOT NULL DEFAULT 0;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("ALTER TABLE conversations ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }
    Ok(())
}

/// v116: 给 ai_models / ai_agents / api_keys 三张表添加 user_id 字段（多用户数据隔离批次 1）
///
/// 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七（多用户数据隔离缺失修复）
///
/// 说明:
/// - 上述三张表原本无 user_id 字段，任何登录用户可读取/修改/删除他人配置
/// - 本次迁移添加 user_id INTEGER NOT NULL DEFAULT 1（现有数据归 admin，users.id=1）
/// - 后续 repo 层所有查询/更新/删除操作必须按 user_id 过滤
///
/// 兼容性: 用 PRAGMA table_info 检查 user_id 字段是否存在，避免重复 ALTER TABLE。
///         up.sql 包含多条 ALTER TABLE 语句，sqlx::query 只能执行单条，故逐条执行。
async fn add_user_id_to_ai_tables(pool: &SqlitePool) -> Result<(), AppError> {
    // ===== 1. ai_models 表 =====
    let ai_models_cols = sqlx::query("PRAGMA table_info(ai_models);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let ai_models_names: Vec<String> = ai_models_cols.iter().map(|r| r.get::<String, _>("name")).collect();
    if !ai_models_names.contains(&"user_id".to_string()) {
        sqlx::query("ALTER TABLE ai_models ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ai_models_user_id ON ai_models(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ai_models_user_provider ON ai_models(user_id, provider);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    // ===== 2. ai_agents 表 =====
    let ai_agents_cols = sqlx::query("PRAGMA table_info(ai_agents);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let ai_agents_names: Vec<String> = ai_agents_cols.iter().map(|r| r.get::<String, _>("name")).collect();
    if !ai_agents_names.contains(&"user_id".to_string()) {
        sqlx::query("ALTER TABLE ai_agents ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_ai_agents_user_id ON ai_agents(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    // ===== 3. api_keys 表 =====
    // 设计说明：api_keys 保留 provider 全局 UNIQUE 约束（migration 0109 内联定义），
    // 不重建表以降低风险。添加 user_id 用于所有权标识，所有读写按 user_id 过滤。
    // 语义：每个 provider 全局仅允许一个 Key，归创建者所有；其他用户无法配置同 provider 的 Key。
    // 这与 api_keys 表"按 provider 集中存储"的原始设计意图一致（migration 0109 注释）。
    let api_keys_cols = sqlx::query("PRAGMA table_info(api_keys);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let api_keys_names: Vec<String> = api_keys_cols.iter().map(|r| r.get::<String, _>("name")).collect();
    if !api_keys_names.contains(&"user_id".to_string()) {
        sqlx::query("ALTER TABLE api_keys ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    Ok(())
}

/// v117: 多用户数据隔离批次 2 — 给 conversations / messages / conversation_participants 添加 user_id
///
/// 兼容性: 用 PRAGMA table_info 检查 user_id 字段是否存在，避免重复 ALTER TABLE。
///         up.sql 包含多条 ALTER TABLE 语句，sqlx::query 只能执行单条，故逐条执行。
/// 说明: messages 表通过 conversation_id 间接关联 conversations，但加冗余 user_id 字段
///       简化查询（避免每次 JOIN）+ 防御深度（即使 conversation_id 被伪造，messages 仍受 user_id 过滤）。
async fn add_user_id_to_conversations_tables(pool: &SqlitePool) -> Result<(), AppError> {
    // ===== 1. conversations 表 =====
    let conv_cols = sqlx::query("PRAGMA table_info(conversations);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let conv_names: Vec<String> = conv_cols.iter().map(|r| r.get::<String, _>("name")).collect();
    if !conv_names.contains(&"user_id".to_string()) {
        sqlx::query("ALTER TABLE conversations ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_conversations_user_id ON conversations(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_conversations_user_updated ON conversations(user_id, updated_at);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    // ===== 2. messages 表 =====
    let msg_cols = sqlx::query("PRAGMA table_info(messages);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let msg_names: Vec<String> = msg_cols.iter().map(|r| r.get::<String, _>("name")).collect();
    if !msg_names.contains(&"user_id".to_string()) {
        sqlx::query("ALTER TABLE messages ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_user_id ON messages(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_user_conv ON messages(user_id, conversation_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    // ===== 3. conversation_participants 表 =====
    let cp_cols = sqlx::query("PRAGMA table_info(conversation_participants);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let cp_names: Vec<String> = cp_cols.iter().map(|r| r.get::<String, _>("name")).collect();
    if !cp_names.contains(&"user_id".to_string()) {
        sqlx::query("ALTER TABLE conversation_participants ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_conversation_participants_user_id ON conversation_participants(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
    }

    Ok(())
}

/// v118: 多用户数据隔离批次 3 — 知识库表添加 user_id 字段
///
/// 给 10 张 KB 表添加 user_id（DEFAULT 1），保证现有 admin 数据自动归属 user_id=1。
/// 每张表均采用「检查列是否存在 → ALTER + CREATE INDEX」的幂等模式，避免重复执行报错。
async fn add_user_id_to_kb_tables(pool: &SqlitePool) -> Result<(), AppError> {
    // 辅助：给指定表添加 user_id 列（若不存在）+ 创建索引
    async fn add_user_id_to_table(
        pool: &SqlitePool,
        table: &str,
        index_name: &str,
        extra_index_sql: &[&str],
    ) -> Result<(), AppError> {
        let cols = sqlx::query(&format!("PRAGMA table_info({});", table))
            .fetch_all(pool)
            .await
            .map_err(AppError::Database)?;
        let names: Vec<String> = cols.iter().map(|r| r.get::<String, _>("name")).collect();
        if !names.contains(&"user_id".to_string()) {
            sqlx::query(&format!(
                "ALTER TABLE {} ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;",
                table
            ))
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
            sqlx::query(&format!(
                "CREATE INDEX IF NOT EXISTS {} ON {}(user_id);",
                index_name, table
            ))
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
            for sql in extra_index_sql {
                sqlx::query(sql).execute(pool).await.map_err(AppError::Database)?;
            }
        }
        Ok(())
    }

    add_user_id_to_table(
        pool, "kb_categories", "idx_kb_categories_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_kb_categories_user_library ON kb_categories(user_id, library);"],
    ).await?;
    add_user_id_to_table(
        pool, "kb_entries", "idx_kb_entries_user_id",
        &[
            "CREATE INDEX IF NOT EXISTS idx_kb_entries_user_cat ON kb_entries(user_id, category_id);",
            "CREATE INDEX IF NOT EXISTS idx_kb_entries_user_updated ON kb_entries(user_id, updated_at);",
            "CREATE INDEX IF NOT EXISTS idx_kb_entries_user_fav ON kb_entries(user_id, is_favorited);",
        ],
    ).await?;
    add_user_id_to_table(
        pool, "kb_tags", "idx_kb_tags_user_id", &[],
    ).await?;
    add_user_id_to_table(
        pool, "kb_entry_tags", "idx_kb_entry_tags_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_kb_entry_tags_user_entry ON kb_entry_tags(user_id, entry_id);"],
    ).await?;
    add_user_id_to_table(
        pool, "kb_recent_access", "idx_kb_recent_access_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_kb_recent_access_user_entry ON kb_recent_access(user_id, entry_id);"],
    ).await?;
    add_user_id_to_table(
        pool, "kb_tracked_paths", "idx_kb_tracked_paths_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_kb_tracked_paths_user_path ON kb_tracked_paths(user_id, path);"],
    ).await?;
    add_user_id_to_table(
        pool, "kb_references", "idx_kb_references_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_kb_references_user_source ON kb_references(user_id, source_entry_id);"],
    ).await?;
    add_user_id_to_table(
        pool, "kb_snapshots", "idx_kb_snapshots_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_kb_snapshots_user_entry ON kb_snapshots(user_id, entry_id);"],
    ).await?;
    add_user_id_to_table(
        pool, "kb_templates", "idx_kb_templates_user_id", &[],
    ).await?;
    add_user_id_to_table(
        pool, "kb_attachment_cache", "idx_kb_attachment_cache_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_kb_attachment_cache_user_entry ON kb_attachment_cache(user_id, entry_id);"],
    ).await?;

    Ok(())
}

/// v119: 多用户数据隔离批次 4 — 小欣（Xin）模块 12 张表添加 user_id 字段
///
/// 给以下表添加 user_id 列 + 索引：
/// - xin_config / xin_memories / xin_summaries / xin_moods / xin_reminders
/// - xin_habits / xin_conversations / xin_checkpoints / xin_compaction_records
/// - xin_compaction_config / xin_persona_memories / xin_persona_switch_log
///
/// DEFAULT 1 保证现有 admin 数据自动归属 user_id=1。
async fn add_user_id_to_xin_tables(pool: &SqlitePool) -> Result<(), AppError> {
    // 辅助函数：复用 kb 的模式（检查列是否存在 → ALTER + CREATE INDEX）
    async fn add_user_id_to_table(
        pool: &SqlitePool,
        table: &str,
        index_name: &str,
        extra_index_sql: &[&str],
    ) -> Result<(), AppError> {
        let cols = sqlx::query(&format!("PRAGMA table_info({});", table))
            .fetch_all(pool)
            .await
            .map_err(AppError::Database)?;
        let names: Vec<String> = cols.iter().map(|r| r.get::<String, _>("name")).collect();
        if !names.contains(&"user_id".to_string()) {
            sqlx::query(&format!(
                "ALTER TABLE {} ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;",
                table
            ))
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
            sqlx::query(&format!(
                "CREATE INDEX IF NOT EXISTS {} ON {}(user_id);",
                index_name, table
            ))
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
            for sql in extra_index_sql {
                sqlx::query(sql).execute(pool).await.map_err(AppError::Database)?;
            }
        }
        Ok(())
    }

    add_user_id_to_table(pool, "xin_config", "idx_xin_config_user_id", &[]).await?;
    add_user_id_to_table(
        pool, "xin_memories", "idx_xin_memories_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_memories_user_cat ON xin_memories(user_id, category);"],
    ).await?;
    add_user_id_to_table(
        pool, "xin_summaries", "idx_xin_summaries_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_summaries_user_conv ON xin_summaries(user_id, conversation_id);"],
    ).await?;
    add_user_id_to_table(
        pool, "xin_moods", "idx_xin_moods_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_moods_user_cat ON xin_moods(user_id, category);"],
    ).await?;
    add_user_id_to_table(
        pool, "xin_reminders", "idx_xin_reminders_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_reminders_user_active ON xin_reminders(user_id, is_active);"],
    ).await?;
    add_user_id_to_table(pool, "xin_habits", "idx_xin_habits_user_id", &[]).await?;
    add_user_id_to_table(
        pool, "xin_conversations", "idx_xin_conversations_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_conversations_user_updated ON xin_conversations(user_id, updated_at);"],
    ).await?;
    add_user_id_to_table(
        pool, "xin_checkpoints", "idx_xin_checkpoints_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_checkpoints_user_conv ON xin_checkpoints(user_id, conversation_id);"],
    ).await?;
    add_user_id_to_table(
        pool, "xin_compaction_records", "idx_xin_compaction_records_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_compaction_records_user_conv ON xin_compaction_records(user_id, conversation_id);"],
    ).await?;
    add_user_id_to_table(pool, "xin_compaction_config", "idx_xin_compaction_config_user_id", &[]).await?;
    add_user_id_to_table(
        pool, "xin_persona_memories", "idx_xin_persona_memories_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_persona_memories_user_persona ON xin_persona_memories(user_id, persona_id);"],
    ).await?;
    add_user_id_to_table(
        pool, "xin_persona_switch_log", "idx_xin_persona_switch_log_user_id",
        &["CREATE INDEX IF NOT EXISTS idx_xin_persona_switch_log_user_switched ON xin_persona_switch_log(user_id, switched_at DESC);"],
    ).await?;

    Ok(())
}

/// v121: 多用户数据隔离批次 6 — 剩余 14 张用户私有表添加 user_id 字段
///
/// 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
///
/// 11 张无 UNIQUE 约束的表：直接 ALTER ADD COLUMN + CREATE INDEX
///   ssh_profiles / prompt_templates / resumes / quotes / yuan_code_snippets /
///   yuan_code_workspaces / terminal_sessions / recycle_bin / game_worlds /
///   news_cache / mcp_servers
///
/// 3 张有 UNIQUE 约束的表：需重建表以改 UNIQUE 约束为 (user_id, 原列)
///   - news_sources: UNIQUE(url) → UNIQUE(user_id, url)
///   - custom_themes: UNIQUE(name) → UNIQUE(user_id, name)
///   - model_routing_rules: UNIQUE(task_type) → UNIQUE(user_id, task_type)
async fn add_user_id_to_remaining_tables(pool: &SqlitePool) -> Result<(), AppError> {
    use sqlx::Row;

    // 辅助函数：检查表是否已有 user_id 字段
    async fn has_user_id(pool: &SqlitePool, table: &str) -> Result<bool, AppError> {
        let cols = sqlx::query(&format!("PRAGMA table_info({});", table))
            .fetch_all(pool)
            .await
            .map_err(AppError::Database)?;
        let names: Vec<String> = cols.iter().map(|r| r.get::<String, _>("name")).collect();
        Ok(names.contains(&"user_id".to_string()))
    }

    // 辅助函数：对无 UNIQUE 约束的表执行 ALTER ADD COLUMN + CREATE INDEX
    async fn alter_add_user_id(
        pool: &SqlitePool,
        table: &str,
        index_name: &str,
    ) -> Result<(), AppError> {
        if !has_user_id(pool, table).await? {
            sqlx::query(&format!(
                "ALTER TABLE {} ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;",
                table
            ))
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
            sqlx::query(&format!(
                "CREATE INDEX IF NOT EXISTS {} ON {}(user_id);",
                index_name, table
            ))
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
            tracing::info!("📦 [批次6] {} 表已添加 user_id 字段", table);
        }
        Ok(())
    }

    // === 11 张无 UNIQUE 约束的表：直接 ALTER ===
    alter_add_user_id(pool, "ssh_profiles", "idx_ssh_profiles_user_id").await?;
    alter_add_user_id(pool, "prompt_templates", "idx_prompt_templates_user_id").await?;
    alter_add_user_id(pool, "resumes", "idx_resumes_user_id").await?;
    alter_add_user_id(pool, "quotes", "idx_quotes_user_id").await?;
    alter_add_user_id(pool, "yuan_code_snippets", "idx_yuan_code_snippets_user_id").await?;
    alter_add_user_id(pool, "yuan_code_workspaces", "idx_yuan_code_workspaces_user_id").await?;
    alter_add_user_id(pool, "terminal_sessions", "idx_terminal_sessions_user_id").await?;
    alter_add_user_id(pool, "recycle_bin", "idx_recycle_bin_user_id").await?;
    alter_add_user_id(pool, "game_worlds", "idx_game_worlds_user_id").await?;
    alter_add_user_id(pool, "news_cache", "idx_news_cache_user_id").await?;
    alter_add_user_id(pool, "mcp_servers", "idx_mcp_servers_user_id").await?;

    // === 3 张有 UNIQUE 约束的表：重建表 ===

    // --- news_sources: UNIQUE(url) → UNIQUE(user_id, url) ---
    if !has_user_id(pool, "news_sources").await? {
        sqlx::query(
            "CREATE TABLE news_sources_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL DEFAULT 1,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'security',
                feed_type TEXT NOT NULL DEFAULT 'rss',
                UNIQUE(user_id, url)
            );",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query(
            "INSERT INTO news_sources_new (user_id, name, url, category, feed_type)
             SELECT 1, name, url, category, feed_type FROM news_sources;",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query("DROP TABLE news_sources;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("ALTER TABLE news_sources_new RENAME TO news_sources;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_news_sources_user_id ON news_sources(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        tracing::info!("📰 [批次6] news_sources 表已重建（UNIQUE(url) → UNIQUE(user_id, url)）");
    }

    // --- custom_themes: UNIQUE(name) → UNIQUE(user_id, name) ---
    if !has_user_id(pool, "custom_themes").await? {
        sqlx::query(
            "CREATE TABLE custom_themes_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL DEFAULT 1,
                name TEXT NOT NULL,
                base_theme TEXT NOT NULL DEFAULT 'terminal',
                variables TEXT NOT NULL DEFAULT '{}',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(user_id, name)
            );",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query(
            "INSERT INTO custom_themes_new (user_id, name, base_theme, variables, created_at, updated_at)
             SELECT 1, name, base_theme, variables, created_at, updated_at FROM custom_themes;",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query("DROP TABLE custom_themes;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("ALTER TABLE custom_themes_new RENAME TO custom_themes;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_custom_themes_user_id ON custom_themes(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        tracing::info!("🎨 [批次6] custom_themes 表已重建（UNIQUE(name) → UNIQUE(user_id, name)）");
    }

    // --- model_routing_rules: UNIQUE(task_type) → UNIQUE(user_id, task_type) ---
    if !has_user_id(pool, "model_routing_rules").await? {
        sqlx::query(
            "CREATE TABLE model_routing_rules_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL DEFAULT 1,
                task_type TEXT NOT NULL,
                provider TEXT NOT NULL,
                model_name TEXT NOT NULL,
                temperature REAL,
                max_tokens INTEGER,
                is_enabled INTEGER NOT NULL DEFAULT 1,
                is_cloud_only INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 100,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(user_id, task_type)
            );",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query(
            "INSERT INTO model_routing_rules_new
                (user_id, task_type, provider, model_name, temperature, max_tokens, is_enabled, is_cloud_only, priority, created_at, updated_at)
             SELECT 1, task_type, provider, model_name, temperature, max_tokens, is_enabled, is_cloud_only, priority, created_at, updated_at
             FROM model_routing_rules;",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query("DROP TABLE model_routing_rules;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("ALTER TABLE model_routing_rules_new RENAME TO model_routing_rules;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_model_routing_rules_user_id ON model_routing_rules(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        tracing::info!("🛣️ [批次6] model_routing_rules 表已重建（UNIQUE(task_type) → UNIQUE(user_id, task_type)）");
    }

    Ok(())
}

/// v122: 多用户数据隔离补充修复 — terminal_history / terminal_tab_layout 添加 user_id 字段
///
/// 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
///
/// 这两张表在批次 6 中被遗漏，完全没有 user_id 隔离。
/// terminal_history 存储用户执行的命令和输出，属于高敏感数据（可能含密码/密钥）。
/// terminal_tab_layout 存储用户的终端标签布局。
///
/// 两表均无 UNIQUE 约束，直接 ALTER ADD COLUMN 即可。
/// DEFAULT 1 保证现有数据自动归属 user_id=1（admin）。
async fn add_user_id_to_terminal_history_tab_layout(pool: &SqlitePool) -> Result<(), AppError> {
    use sqlx::Row;

    // === terminal_history 表 ===
    let th_cols = sqlx::query("PRAGMA table_info(terminal_history);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let th_names: Vec<String> = th_cols
        .iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();
    if !th_names.contains(&"user_id".to_string()) {
        sqlx::query(
            "ALTER TABLE terminal_history ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_terminal_history_user_id ON terminal_history(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_terminal_history_user_created ON terminal_history(user_id, created_at);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        tracing::info!("🖥️ [补充修复] terminal_history 表已添加 user_id 字段");
    }

    // === terminal_tab_layout 表 ===
    let tl_cols = sqlx::query("PRAGMA table_info(terminal_tab_layout);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let tl_names: Vec<String> = tl_cols
        .iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();
    if !tl_names.contains(&"user_id".to_string()) {
        sqlx::query(
            "ALTER TABLE terminal_tab_layout ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_terminal_tab_layout_user_id ON terminal_tab_layout(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        tracing::info!("🖥️ [补充修复] terminal_tab_layout 表已添加 user_id 字段");
    }

    Ok(())
}

/// v120: 多用户数据隔离批次 5 — 日程类 2 张表（journals / timers）添加 user_id 字段
///
/// 规范: 安全审计报告/2026-07-25-代码安全审计报告.md 附录七
///
/// journals 表特殊处理：
///   原表有 UNIQUE(date) 约束，多用户场景下两个用户无法在同一天写日记。
///   SQLite ALTER TABLE 不支持修改 UNIQUE 约束，必须重建表。
///   重建后约束改为 UNIQUE(user_id, date)，支持多用户同一天各写各的。
///   现有数据 user_id=1，date 已唯一 → 重建后无冲突。
///
/// timers 表无 UNIQUE 约束，直接 ALTER ADD COLUMN 即可。
async fn add_user_id_to_journals_timers(pool: &SqlitePool) -> Result<(), AppError> {
    use sqlx::Row;

    // === journals 表：重建以改 UNIQUE(date) → UNIQUE(user_id, date) ===
    let journal_cols = sqlx::query("PRAGMA table_info(journals);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let journal_names: Vec<String> = journal_cols
        .iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();
    if !journal_names.contains(&"user_id".to_string()) {
        // 1. 创建新表（user_id + UNIQUE(user_id, date)）
        sqlx::query(
            "CREATE TABLE journals_new (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL DEFAULT 1,
                date TEXT NOT NULL,
                content TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                UNIQUE(user_id, date)
            );",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        // 2. 复制现有数据（user_id 强制为 1）
        sqlx::query(
            "INSERT INTO journals_new (user_id, date, content, created_at, updated_at)
             SELECT 1, date, content, created_at, updated_at FROM journals;",
        )
        .execute(pool)
        .await
        .map_err(AppError::Database)?;
        // 3. 删除旧表并重命名
        sqlx::query("DROP TABLE journals;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("ALTER TABLE journals_new RENAME TO journals;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        // 4. 创建索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_journals_user_id ON journals(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_journals_user_date ON journals(user_id, date);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        tracing::info!("📅 [批次5] journals 表已重建（UNIQUE(date) → UNIQUE(user_id, date)）");
    }

    // === timers 表：无 UNIQUE 约束，直接 ALTER ===
    let timer_cols = sqlx::query("PRAGMA table_info(timers);")
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;
    let timer_names: Vec<String> = timer_cols
        .iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();
    if !timer_names.contains(&"user_id".to_string()) {
        sqlx::query("ALTER TABLE timers ADD COLUMN user_id INTEGER NOT NULL DEFAULT 1;")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_timers_user_id ON timers(user_id);")
            .execute(pool)
            .await
            .map_err(AppError::Database)?;
        tracing::info!("⏱️ [批次5] timers 表已添加 user_id 字段");
    }

    Ok(())
}
