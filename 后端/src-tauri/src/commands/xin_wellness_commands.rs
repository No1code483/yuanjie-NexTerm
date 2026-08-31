use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

use crate::db::connection::AppState;
use crate::models::api_response::ApiResponse;
use crate::models::xin::{
    ActivityDigest, ConversationBridge, ConversationSummary, HabitCheckin, MemoryConsolidation,
    MemoryLink, MoodHistory, PersonalityEvolution, PomodoroSession, Reminder, UserMemory,
};
use crate::services::xin_wellness_service::{
    ConversationBridgeBuilder, DigestGenerator, HabitTracker, MemoryConsolidator,
    MoodTracker, PersonalityEvolver, PomodoroManager, ReminderManager,
};

static MOOD_TRACKER: once_cell::sync::Lazy<Arc<Mutex<MoodTracker>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(MoodTracker::new())));

static REMINDER_MANAGER: once_cell::sync::Lazy<Arc<Mutex<ReminderManager>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(ReminderManager::new())));

static HABIT_TRACKER: once_cell::sync::Lazy<Arc<Mutex<HabitTracker>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HabitTracker::new())));

static POMODORO_MANAGER: once_cell::sync::Lazy<Arc<Mutex<PomodoroManager>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(PomodoroManager::new())));

/// 安全审计修复（发现 18-60，MEDIUM）：剩余 43 个无认证 command 文件
/// 按「相同模式补充修复」要求，所有命令入口强制调用 `require_auth(&state).await?`。

#[tauri::command]
pub async fn xin_v2_mood_history(
    state: State<'_, AppState>,
    trigger_text: Option<String>,
) -> Result<ApiResponse<MoodHistory>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let current_mood = state.xiaoxin_service.get_current_mood(user_id).await;

    if let Some(text) = trigger_text {
        let updated = state.xiaoxin_service.update_mood(user_id, &text).await;
        let mut tracker = MOOD_TRACKER.lock().await;
        tracker.record(updated, Some(text));
    } else {
        let mut tracker = MOOD_TRACKER.lock().await;
        tracker.record(current_mood, None);
    }

    let tracker = MOOD_TRACKER.lock().await;
    Ok(ApiResponse::success(tracker.analyze()))
}

#[tauri::command]
pub async fn xin_v2_consolidate_memories(
    state: State<'_, AppState>,
) -> Result<ApiResponse<MemoryConsolidation>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let memories: Vec<UserMemory> = state.xiaoxin_service.get_memories(user_id, None, Some(500)).await;
    Ok(ApiResponse::success(MemoryConsolidator::consolidate(&memories)))
}

#[tauri::command]
pub async fn xin_v2_memory_links(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<MemoryLink>>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
    let memories: Vec<UserMemory> = state.xiaoxin_service.get_memories(user_id, None, Some(200)).await;
    Ok(ApiResponse::success(MemoryConsolidator::generate_links(&memories)))
}

#[tauri::command]
pub async fn xin_v2_conversation_bridge(
    state: State<'_, AppState>,
    session_id: Option<String>,
) -> Result<ApiResponse<ConversationBridge>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
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
    crate::commands::common::require_auth(&state).await?;
    let mut mgr = REMINDER_MANAGER.lock().await;
    Ok(ApiResponse::success(mgr.add_reminder(title, description, trigger_at, cron_expr)))
}

#[tauri::command]
pub async fn xin_v2_list_reminders(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<Reminder>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mgr = REMINDER_MANAGER.lock().await;
    Ok(ApiResponse::success(mgr.list_active()))
}

#[tauri::command]
pub async fn xin_v2_dismiss_reminder(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut mgr = REMINDER_MANAGER.lock().await;
    Ok(ApiResponse::success(mgr.dismiss(&id)))
}

#[tauri::command]
pub async fn xin_v2_delete_reminder(
    state: State<'_, AppState>,
    id: String,
) -> Result<ApiResponse<bool>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut mgr = REMINDER_MANAGER.lock().await;
    Ok(ApiResponse::success(mgr.delete(&id)))
}

#[tauri::command]
pub async fn xin_v2_register_habit(
    state: State<'_, AppState>,
    name: String,
    category: String,
) -> Result<ApiResponse<HabitCheckin>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut tracker = HABIT_TRACKER.lock().await;
    Ok(ApiResponse::success(tracker.register(name, category)))
}

#[tauri::command]
pub async fn xin_v2_checkin_habit(
    state: State<'_, AppState>,
    habit_id: String,
) -> Result<ApiResponse<Option<HabitCheckin>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut tracker = HABIT_TRACKER.lock().await;
    Ok(ApiResponse::success(tracker.checkin(&habit_id)))
}

#[tauri::command]
pub async fn xin_v2_list_habits(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Vec<HabitCheckin>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let tracker = HABIT_TRACKER.lock().await;
    Ok(ApiResponse::success(tracker.list()))
}

#[tauri::command]
pub async fn xin_v2_pomodoro_start(
    state: State<'_, AppState>,
    task_name: String,
    duration_minutes: u32,
    break_minutes: u32,
    total_cycles: u32,
) -> Result<ApiResponse<PomodoroSession>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut mgr = POMODORO_MANAGER.lock().await;
    Ok(ApiResponse::success(mgr.start(task_name, duration_minutes, break_minutes, total_cycles)))
}

#[tauri::command]
pub async fn xin_v2_pomodoro_complete_cycle(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<PomodoroSession>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut mgr = POMODORO_MANAGER.lock().await;
    Ok(ApiResponse::success(mgr.complete_cycle()))
}

#[tauri::command]
pub async fn xin_v2_pomodoro_stop(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<PomodoroSession>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mut mgr = POMODORO_MANAGER.lock().await;
    Ok(ApiResponse::success(mgr.stop()))
}

#[tauri::command]
pub async fn xin_v2_pomodoro_status(
    state: State<'_, AppState>,
) -> Result<ApiResponse<Option<PomodoroSession>>, String> {
    crate::commands::common::require_auth(&state).await?;
    let mgr = POMODORO_MANAGER.lock().await;
    Ok(ApiResponse::success(mgr.active()))
}

#[tauri::command]
pub async fn xin_v2_activity_digest(
    state: State<'_, AppState>,
    period: Option<String>,
) -> Result<ApiResponse<ActivityDigest>, String> {
    let user_id = crate::commands::common::require_auth(&state).await?;
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
    let user_id = crate::commands::common::require_auth(&state).await?;
    let persona = state.xiaoxin_service.get_active_persona(user_id).await;
    let mood_tracker = MOOD_TRACKER.lock().await;
    let history = mood_tracker.analyze();
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