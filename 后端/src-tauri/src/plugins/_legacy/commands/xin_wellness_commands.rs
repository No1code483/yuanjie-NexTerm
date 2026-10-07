/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。
///
/// **批次4a-1 越权收口（裁定 3）**：原 4 个 tracker 全局单例（`MOOD_TRACKER` 等）
/// 为多用户共享状态 → 改为按 `user_id` 分片（`MOOD_TRACKERS` 等 map），消除跨用户
/// 状态泄漏；分片仅进程内存态有效（不落库，与改造前一致）。收口后各命令从
/// `require_auth` 取会话主体并据此选择分片。
use std::collections::HashMap;
use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::xin::{
    ActivityDigest, ConversationBridge, ConversationSummary, HabitCheckin, MemoryConsolidation,
    MemoryLink, MoodHistory, PersonalityEvolution, PomodoroSession, Reminder, UserMemory,
};
use crate::plugins::_legacy::services::xin_wellness_service::{
    ConversationBridgeBuilder, DigestGenerator, HabitTracker, MemoryConsolidator,
    MoodTracker, PersonalityEvolver, PomodoroManager, ReminderManager,
};

// ============================================================================
// 裁定 3（批次4a-1）：tracker 按 user_id 分片（替代原全局单例）
// ============================================================================

/// 分片容器：`user_id` → 该主体的 tracker 实例。
type TrackerMap<T> = Mutex<HashMap<i64, Arc<Mutex<T>>>>;

static MOOD_TRACKERS: once_cell::sync::Lazy<TrackerMap<MoodTracker>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

static REMINDER_MANAGERS: once_cell::sync::Lazy<TrackerMap<ReminderManager>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

static HABIT_TRACKERS: once_cell::sync::Lazy<TrackerMap<HabitTracker>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

static POMODORO_MANAGERS: once_cell::sync::Lazy<TrackerMap<PomodoroManager>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

async fn mood_tracker_for(user_id: i64) -> Arc<Mutex<MoodTracker>> {
    let mut map = MOOD_TRACKERS.lock().await;
    map.entry(user_id)
        .or_insert_with(|| Arc::new(Mutex::new(MoodTracker::new())))
        .clone()
}

async fn reminder_manager_for(user_id: i64) -> Arc<Mutex<ReminderManager>> {
    let mut map = REMINDER_MANAGERS.lock().await;
    map.entry(user_id)
        .or_insert_with(|| Arc::new(Mutex::new(ReminderManager::new())))
        .clone()
}

async fn habit_tracker_for(user_id: i64) -> Arc<Mutex<HabitTracker>> {
    let mut map = HABIT_TRACKERS.lock().await;
    map.entry(user_id)
        .or_insert_with(|| Arc::new(Mutex::new(HabitTracker::new())))
        .clone()
}

async fn pomodoro_manager_for(user_id: i64) -> Arc<Mutex<PomodoroManager>> {
    let mut map = POMODORO_MANAGERS.lock().await;
    map.entry(user_id)
        .or_insert_with(|| Arc::new(Mutex::new(PomodoroManager::new())))
        .clone()
}

#[tauri::command]
pub async fn xin_v2_mood_history(
    state: State<'_, AppState>,
    trigger_text: Option<String>,
) -> Result<ApiResponse<MoodHistory>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let current_mood = state.xiaoxin_service.get_current_mood(user_id).await;

    if let Some(text) = trigger_text {
        let updated = state.xiaoxin_service.update_mood(user_id, &text).await;
        let tracker = mood_tracker_for(user_id).await;
        let mut guard = tracker.lock().await;
        guard.record(updated, Some(text));
    } else {
        let tracker = mood_tracker_for(user_id).await;
        let mut guard = tracker.lock().await;
        guard.record(current_mood, None);
    }

    let tracker = mood_tracker_for(user_id).await;
    let guard = tracker.lock().await;
    Ok(ApiResponse::success(guard.analyze()))
}

#[tauri::command]
pub async fn xin_v2_consolidate_memories(
    state: State<'_, AppState>,
) -> Result<ApiResponse<MemoryConsolidation>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let memories: Vec<UserMemory> = state.xiaoxin_service.get_memories(user_id, None, Some(500)).await;
    Ok(ApiResponse::success(MemoryConsolidator::consolidate(&memories)))
}

#[tauri::command]
pub async fn xin_v2_memory_links(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<MemoryLink>>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let memories: Vec<UserMemory> = state.xiaoxin_service.get_memories(user_id, None, Some(200)).await;
    Ok(ApiResponse::success(MemoryConsolidator::generate_links(&memories)))
}

#[tauri::command]
pub async fn xin_v2_conversation_bridge(
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<ApiResponse<ConversationBridge>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let summaries: Vec<ConversationSummary> = state.xiaoxin_service.get_conversation_summaries(user_id, Some(20)).await;
    Ok(ApiResponse::success(ConversationBridgeBuilder::build(session_id, &summaries)))
}

#[tauri::command]
pub async fn xin_v2_add_reminder(
    state: State<'_, AppState>,
    title: String,
    description: String,
    trigger_at: Option<String>,
    cron_expr: Option<String>,
) -> Result<ApiResponse<Reminder>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let mgr = reminder_manager_for(user_id).await;
    let mut guard = mgr.lock().await;
    Ok(ApiResponse::success(guard.add_reminder(title, description, trigger_at, cron_expr)))
}

#[tauri::command]
pub async fn xin_v2_list_reminders(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<Reminder>>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let mgr = reminder_manager_for(user_id).await;
    let guard = mgr.lock().await;
    Ok(ApiResponse::success(guard.list_active()))
}

#[tauri::command]
pub async fn xin_v2_dismiss_reminder(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let mgr = reminder_manager_for(user_id).await;
    let mut guard = mgr.lock().await;
    Ok(ApiResponse::success(guard.dismiss(&id)))
}

#[tauri::command]
pub async fn xin_v2_delete_reminder(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiResponse<bool>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let mgr = reminder_manager_for(user_id).await;
    let mut guard = mgr.lock().await;
    Ok(ApiResponse::success(guard.delete(&id)))
}

#[tauri::command]
pub async fn xin_v2_register_habit(
    state: State<'_, AppState>,
    name: String,
    category: String,
) -> Result<ApiResponse<HabitCheckin>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let tracker = habit_tracker_for(user_id).await;
    let mut guard = tracker.lock().await;
    Ok(ApiResponse::success(guard.register(name, category)))
}

#[tauri::command]
pub async fn xin_v2_checkin_habit(
    state: State<'_, AppState>,
    habit_id: String,
) -> Result<ApiResponse<Option<HabitCheckin>>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let tracker = habit_tracker_for(user_id).await;
    let mut guard = tracker.lock().await;
    Ok(ApiResponse::success(guard.checkin(&habit_id)))
}

#[tauri::command]
pub async fn xin_v2_list_habits(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<HabitCheckin>>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let tracker = habit_tracker_for(user_id).await;
    let guard = tracker.lock().await;
    Ok(ApiResponse::success(guard.list()))
}

#[tauri::command]
pub async fn xin_v2_pomodoro_start(
    state: State<'_, AppState>,
    task_name: String,
    duration_minutes: u32,
    break_minutes: u32,
    total_cycles: u32,
) -> Result<ApiResponse<PomodoroSession>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let mgr = pomodoro_manager_for(user_id).await;
    let mut guard = mgr.lock().await;
    Ok(ApiResponse::success(guard.start(task_name, duration_minutes, break_minutes, total_cycles)))
}

#[tauri::command]
pub async fn xin_v2_pomodoro_complete_cycle(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<PomodoroSession>>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let mgr = pomodoro_manager_for(user_id).await;
    let mut guard = mgr.lock().await;
    Ok(ApiResponse::success(guard.complete_cycle()))
}

#[tauri::command]
pub async fn xin_v2_pomodoro_stop(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<PomodoroSession>>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let mgr = pomodoro_manager_for(user_id).await;
    let mut guard = mgr.lock().await;
    Ok(ApiResponse::success(guard.stop()))
}

#[tauri::command]
pub async fn xin_v2_pomodoro_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<PomodoroSession>>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let mgr = pomodoro_manager_for(user_id).await;
    let guard = mgr.lock().await;
    Ok(ApiResponse::success(guard.active()))
}

#[tauri::command]
pub async fn xin_v2_activity_digest(
    state: State<'_, AppState>,
    period: Option<String>,
) -> Result<ApiResponse<ActivityDigest>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let period = period.unwrap_or_else(|| "week".into());
    let memories: Vec<UserMemory> = state.xiaoxin_service.get_memories(user_id, None, Some(500)).await;
    let summaries: Vec<ConversationSummary> = state.xiaoxin_service.get_conversation_summaries(user_id, Some(50)).await;
    let interactions = memories.len() + summaries.len() * 5;
    Ok(ApiResponse::success(DigestGenerator::generate(&period, &memories, &summaries, interactions)))
}

#[tauri::command]
pub async fn xin_v2_personality_evolution(
    state: State<'_, AppState>,
) -> Result<ApiResponse<PersonalityEvolution>, String> {
    let user_id = crate::plugins::_legacy::commands::common::require_auth(&state).await?;
    let persona = state.xiaoxin_service.get_active_persona(user_id).await;
    let tracker = mood_tracker_for(user_id).await;
    let history = {
        let guard = tracker.lock().await;
        guard.analyze()
    };
    let memories: Vec<UserMemory> = state.xiaoxin_service.get_memories(user_id, None, Some(500)).await;

    let mut topic_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for m in &memories {
        for word in m.value.split(|c: char| !c.is_alphanumeric()) {
            if word.len() >= 2 {
                topic_set.insert(word.to_string());
            }
        }
    }
    let topics: Vec<String> = topic_set.into_iter().take(10).collect();

    Ok(ApiResponse::success(PersonalityEvolver::evolve(
        &persona,
        &history,
        memories.len(),
        &topics,
    )))
}