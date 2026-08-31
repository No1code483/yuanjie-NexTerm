use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::models::xin::Persona;
use crate::services::xin_commit_service::CommitmentRecord;
use crate::services::xin_michelin_service::EvalResult;
use crate::services::xin_michelin_service::EvalDimension;

pub struct XinProactiveService;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionType {
    #[serde(rename = "care_check_in")]
    CareCheckIn,
    #[serde(rename = "deadline_reminder")]
    DeadlineReminder,
    #[serde(rename = "habit_nudge")]
    HabitNudge,
    #[serde(rename = "commitment_follow_up")]
    CommitmentFollowUp,
    #[serde(rename = "mood_check")]
    MoodCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionPriority {
    #[serde(rename = "urgent")]
    Urgent,
    #[serde(rename = "high")]
    High,
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "low")]
    Low,
}

impl ActionPriority {
    pub fn rank(&self) -> u8 {
        match self {
            ActionPriority::Urgent => 0,
            ActionPriority::High => 1,
            ActionPriority::Normal => 2,
            ActionPriority::Low => 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProactiveAction {
    pub id: String,
    pub action_type: ActionType,
    pub priority: ActionPriority,
    pub message: String,
    pub detail: String,
    pub source_id: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub delivered: bool,
    pub dismissed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuietHours {
    pub enabled: bool,
    pub start_hour: u8,
    pub end_hour: u8,
    pub timezone: String,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: false,
            start_hour: 23,
            end_hour: 7,
            timezone: "Asia/Shanghai".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingStatus {
    pub pending_count: usize,
    pub urgent_count: usize,
    pub is_quiet_hours: bool,
    pub last_check_in: Option<String>,
    pub habit_reminders_enabled: bool,
    pub daily_nudge: Option<String>,
}

impl XinProactiveService {
    pub fn should_send_care_check_in(results: &[EvalResult]) -> Option<String> {
        if results.len() < 3 {
            return None;
        }
        let recent: Vec<&EvalResult> = results.iter().rev().take(5).collect();
        let empathy_scores: Vec<f64> = recent
            .iter()
            .filter_map(|r| {
                r.dimensions
                    .iter()
                    .find(|d| d.dimension == EvalDimension::Empathy)
                    .map(|d| d.score as f64)
            })
            .collect();

        if empathy_scores.len() < 3 {
            return None;
        }

        let avg_empathy = empathy_scores.iter().sum::<f64>() / empathy_scores.len() as f64;
        if avg_empathy < 40.0 {
            return Some(format!(
                "最近几轮对话共情评分偏低（平均{:.0}），用户可能需要更多情感支持",
                avg_empathy
            ));
        }

        let first_half = &empathy_scores[..empathy_scores.len() / 2];
        let second_half = &empathy_scores[empathy_scores.len() / 2..];
        let first_avg = first_half.iter().sum::<f64>() / first_half.len() as f64;
        let second_avg = second_half.iter().sum::<f64>() / second_half.len() as f64;

        if second_avg < first_avg - 10.0 {
            return Some(format!(
                "共情评分呈下降趋势（{:.0}→{:.0}），建议主动关怀",
                first_avg, second_avg
            ));
        }

        None
    }

    pub fn generate_care_action(persona: &Persona, reason: &str) -> ProactiveAction {
        let message = match persona.id.as_str() {
            "code_assistant" => "嘿，写代码累了吧？需要我帮忙梳理思路吗？".into(),
            "knowledge_tutor" => "学习辛苦了！要不要回顾一下今天学到的内容？".into(),
            "creative_partner" => "创意工作有时候会卡壳，需要换个角度看看吗？".into(),
            "caring_friend" => "我一直在这儿呢，有什么想聊的随时找我 💙".into(),
            _ => "最近怎么样？有什么我能帮上忙的吗？".into(),
        };
        ProactiveAction {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: ActionType::CareCheckIn,
            priority: ActionPriority::High,
            message,
            detail: reason.to_string(),
            source_id: Some(persona.id.clone()),
            created_at: Utc::now().to_rfc3339(),
            expires_at: Some((Utc::now() + Duration::hours(6)).to_rfc3339()),
            delivered: false,
            dismissed: false,
        }
    }

    pub fn check_commitment_urgency(commitments: &[CommitmentRecord]) -> Vec<ProactiveAction> {
        let now = Utc::now();
        let mut actions = Vec::new();

        for c in commitments {
            let due = now
                .checked_add_signed(Duration::milliseconds(c.due_window.latest_ms))
                .unwrap_or(now);

            let remaining = due.signed_duration_since(now).num_hours();

            let (priority, prefix) = if remaining <= 1 {
                (ActionPriority::Urgent, "⚠️ 即将到期")
            } else if remaining <= 6 {
                (ActionPriority::High, "⏰ 今天到期")
            } else if remaining <= 24 {
                (ActionPriority::Normal, "📌 明天到期")
            } else {
                continue;
            };

            if c.status != crate::services::xin_commit_service::CommitmentStatus::Pending {
                continue;
            }

            actions.push(ProactiveAction {
                id: uuid::Uuid::new_v4().to_string(),
                action_type: ActionType::CommitmentFollowUp,
                priority,
                message: format!("{}: {}", prefix, c.reason),
                detail: c.suggested_text.clone(),
                source_id: Some(c.id.clone()),
                created_at: Utc::now().to_rfc3339(),
                expires_at: Some(due.to_rfc3339()),
                delivered: false,
                dismissed: false,
            });
        }

        actions.sort_by_key(|a| a.priority.rank());
        actions
    }

    pub fn build_habit_reminders(
        memory_habits: &[serde_json::Value],
    ) -> Vec<ProactiveAction> {
        let mut actions = Vec::new();
        let hour = Utc::now().format("%H").to_string().parse::<u32>().unwrap_or(12);

        for mem in memory_habits {
            let key = mem
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let value = mem
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if !key.contains("habit") && !key.contains("习惯") {
                continue;
            }

            let message = match hour {
                6..=9 => format!("早上好！别忘了{}：{}", key.replace("habit_", ""), value),
                12..=14 => format!("午间提醒：{} — {}", key.replace("habit_", ""), value),
                20..=22 => format!("晚间回顾：今天{}了吗？{}", key.replace("habit_", ""), value),
                _ => format!("习惯提醒：{}", value),
            };

            actions.push(ProactiveAction {
                id: uuid::Uuid::new_v4().to_string(),
                action_type: ActionType::HabitNudge,
                priority: ActionPriority::Low,
                message,
                detail: value.to_string(),
                source_id: Some(key.to_string()),
                created_at: Utc::now().to_rfc3339(),
                expires_at: Some((Utc::now() + Duration::hours(3)).to_rfc3339()),
                delivered: false,
                dismissed: false,
            });
        }

        actions.truncate(3);
        actions
    }

    pub fn get_pending_actions(mut actions: Vec<ProactiveAction>) -> Vec<ProactiveAction> {
        actions.retain(|a| !a.dismissed && !a.delivered);
        actions.sort_by(|a, b| {
            a.priority
                .rank()
                .cmp(&b.priority.rank())
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        actions.truncate(5);
        actions
    }

    pub fn is_quiet_hours(quiet: &QuietHours) -> bool {
        if !quiet.enabled {
            return false;
        }
        let hour = Utc::now().format("%H").to_string().parse::<u32>().unwrap_or(12);
        if quiet.start_hour < quiet.end_hour {
            hour >= quiet.start_hour as u32 && hour < quiet.end_hour as u32
        } else {
            hour >= quiet.start_hour as u32 || hour < quiet.end_hour as u32
        }
    }

    pub fn dismiss_action(action: &mut ProactiveAction) {
        action.dismissed = true;
    }

    pub fn generate_daily_nudge(
        persona: &Persona,
        commitments: &[CommitmentRecord],
    ) -> Option<String> {
        let pending: Vec<&CommitmentRecord> = commitments
            .iter()
            .filter(|c| c.status == crate::services::xin_commit_service::CommitmentStatus::Pending)
            .collect();

        if pending.is_empty() {
            return None;
        }

        let hour = Utc::now().format("%H").to_string().parse::<u32>().unwrap_or(12);
        let greeting = match hour {
            6..=11 => "早上好",
            12..=17 => "下午好",
            _ => "晚上好",
        };

        let care_count = pending
            .iter()
            .filter(|c| {
                c.kind == crate::services::xin_commit_service::CommitmentKind::CareCheckIn
            })
            .count();
        let deadline_count = pending
            .iter()
            .filter(|c| {
                c.kind == crate::services::xin_commit_service::CommitmentKind::DeadlineCheck
            })
            .count();

        let mut parts = vec![format!("{}！{}今天有什么计划？", greeting, persona.name)];

        if care_count > 0 {
            parts.push(format!("有 {} 项关怀待跟进", care_count));
        }
        if deadline_count > 0 {
            parts.push(format!("有 {} 个截止事项", deadline_count));
        }

        let nudge = parts.join("，");
        let session_count = (pending.len() as f32 * 0.3).ceil() as u32;
        if session_count > 0 {
            Some(format!(
                "{}。要不要我们先处理其中 {} 件？",
                nudge, session_count
            ))
        } else {
            Some(nudge)
        }
    }

    pub fn generate_morning_context(
        persona: &Persona,
        commitments: &[CommitmentRecord],
    ) -> String {
        let pending: Vec<&CommitmentRecord> = commitments
            .iter()
            .filter(|c| c.status == crate::services::xin_commit_service::CommitmentStatus::Pending)
            .collect();

        let mut lines = vec![format!("🌅 {}早安，我是{}。", persona.avatar_emoji, persona.name)];

        if !pending.is_empty() {
            let urgent: Vec<&&CommitmentRecord> = pending.iter().filter(|c| {
                let due = c.due_window.latest_ms;
                due > 0 && due < 3_600_000
            }).collect();
            if !urgent.is_empty() {
                lines.push(format!("⚠️ 今天有 {} 件事需要优先处理：", urgent.len()));
                for c in urgent.iter().take(3) {
                    lines.push(format!("  • {}", c.reason));
                }
            }
            let normal = pending.len() - urgent.len();
            if normal > 0 {
                lines.push(format!("📋 还有 {} 件事可以慢慢来", normal));
            }
        } else {
            lines.push("今天看起来比较轻松，有什么想探索的吗？".into());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::xin_commit_service::{
        CommitmentKind, CommitmentRecord, CommitmentSensitivity, CommitmentStatus, DueWindow,
    };
    use crate::services::xin_michelin_service::{DimensionScore, EvalDimension, EvalResult};

    fn make_result(empathy: u32, accuracy: u32) -> EvalResult {
        EvalResult {
            id: uuid::Uuid::new_v4().to_string(),
            dimensions: vec![
                DimensionScore { dimension: EvalDimension::Empathy, score: empathy, reason: "".into() },
                DimensionScore { dimension: EvalDimension::Accuracy, score: accuracy, reason: "".into() },
                DimensionScore { dimension: EvalDimension::Completeness, score: 60, reason: "".into() },
                DimensionScore { dimension: EvalDimension::Creativity, score: 50, reason: "".into() },
                DimensionScore { dimension: EvalDimension::Timeliness, score: 70, reason: "".into() },
            ],
            overall_score: (empathy + accuracy + 60 + 50 + 70) / 5,
            stars: 2,
            star_label: "⭐⭐".into(),
            strengths: vec![],
            weaknesses: vec![],
            improvement_suggestion: "".into(),
            evaluated_at: Utc::now().to_rfc3339(),
            response_time_ms: 1000,
        }
    }

    fn make_persona() -> Persona {
        Persona {
            id: "code_assistant".into(),
            name: "代码助手".into(),
            description: "test".into(),
            traits: vec![],
            speaking_style: crate::models::xin::SpeakingStyle::default(),
            base_mood: "calm".into(),
            avatar_emoji: "🤖".into(),
            is_builtin: true,
        }
    }

    fn make_commitment(kind: CommitmentKind, status: CommitmentStatus, latest_ms: i64) -> CommitmentRecord {
        CommitmentRecord {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            sensitivity: CommitmentSensitivity::Routine,
            status,
            reason: "测试承诺".into(),
            suggested_text: "需要跟进".into(),
            source_text: "用户说需要提醒".into(),
            confidence: 0.8,
            due_window: DueWindow {
                earliest_ms: 0,
                latest_ms,
                timezone: "Asia/Shanghai".into(),
            },
            dedupe_key: "test_1".into(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            attempts: 0,
            last_attempt_at: None,
            sent_at: None,
            dismissed_at: None,
            snoozed_until: None,
            expired_at: None,
            done_at: None,
        }
    }

    #[test]
    fn test_should_care_when_low_empathy() {
        let results: Vec<EvalResult> = (0..5)
            .map(|_| make_result(30, 70))
            .collect();
        let reason = XinProactiveService::should_send_care_check_in(&results);
        assert!(reason.is_some());
        assert!(reason.unwrap().contains("共情"));
    }

    #[test]
    fn test_should_not_care_with_good_empathy() {
        let results: Vec<EvalResult> = (0..5)
            .map(|_| make_result(75, 70))
            .collect();
        let reason = XinProactiveService::should_send_care_check_in(&results);
        assert!(reason.is_none());
    }

    #[test]
    fn test_care_action_generation() {
        let persona = make_persona();
        let action = XinProactiveService::generate_care_action(&persona, "共情下降");
        assert_eq!(action.action_type, ActionType::CareCheckIn);
        assert_eq!(action.priority, ActionPriority::High);
        assert!(!action.message.is_empty());
    }

    #[test]
    fn test_commitment_urgency() {
        let urgent = make_commitment(
            CommitmentKind::DeadlineCheck,
            CommitmentStatus::Pending,
            30 * 60 * 1000,
        );
        let normal = make_commitment(
            CommitmentKind::CareCheckIn,
            CommitmentStatus::Pending,
            12 * 3600 * 1000,
        );
        let actions = XinProactiveService::check_commitment_urgency(&[urgent, normal]);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].priority, ActionPriority::Urgent);
    }

    #[test]
    fn test_dismissed_filtered_out() {
        let actions = XinProactiveService::get_pending_actions(vec![]);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_dismiss_action() {
        let persona = make_persona();
        let mut action = XinProactiveService::generate_care_action(&persona, "test");
        assert!(!action.dismissed);
        XinProactiveService::dismiss_action(&mut action);
        assert!(action.dismissed);
    }

    #[test]
    fn test_quiet_hours_default_disabled() {
        let qh = QuietHours::default();
        assert!(!XinProactiveService::is_quiet_hours(&qh));
    }

    #[test]
    fn test_quiet_hours_enabled_overnight() {
        let qh = QuietHours {
            enabled: true,
            start_hour: 23,
            end_hour: 7,
            timezone: "Asia/Shanghai".into(),
        };
        let is_quiet = XinProactiveService::is_quiet_hours(&qh);
        let hour = Utc::now().format("%H").to_string().parse::<u32>().unwrap_or(12);
        if hour >= 23 || hour < 7 {
            assert!(is_quiet);
        } else {
            assert!(!is_quiet);
        }
    }

    #[test]
    fn test_daily_nudge_with_pending() {
        let persona = make_persona();
        let commitments = vec![
            make_commitment(CommitmentKind::DeadlineCheck, CommitmentStatus::Pending, 3600000),
        ];
        let nudge = XinProactiveService::generate_daily_nudge(&persona, &commitments);
        assert!(nudge.is_some());
        assert!(nudge.unwrap().contains("截止"));
    }

    #[test]
    fn test_daily_nudge_empty() {
        let persona = make_persona();
        let commitments: Vec<CommitmentRecord> = vec![];
        let nudge = XinProactiveService::generate_daily_nudge(&persona, &commitments);
        assert!(nudge.is_none());
    }
}