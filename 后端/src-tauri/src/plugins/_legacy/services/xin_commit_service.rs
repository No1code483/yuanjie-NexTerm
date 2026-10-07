use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommitmentKind {
    #[serde(rename = "event_check_in")]
    EventCheckIn,
    #[serde(rename = "deadline_check")]
    DeadlineCheck,
    #[serde(rename = "care_check_in")]
    CareCheckIn,
    #[serde(rename = "open_loop")]
    OpenLoop,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommitmentSensitivity {
    #[serde(rename = "routine")]
    Routine,
    #[serde(rename = "personal")]
    Personal,
    #[serde(rename = "care")]
    Care,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommitmentStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "sent")]
    Sent,
    #[serde(rename = "dismissed")]
    Dismissed,
    #[serde(rename = "snoozed")]
    Snoozed,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "done")]
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DueWindow {
    pub earliest_ms: i64,
    pub latest_ms: i64,
    pub timezone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentRecord {
    pub id: String,
    pub kind: CommitmentKind,
    pub sensitivity: CommitmentSensitivity,
    pub status: CommitmentStatus,
    pub reason: String,
    pub suggested_text: String,
    pub source_text: String,
    pub confidence: f64,
    pub due_window: DueWindow,
    pub dedupe_key: String,
    pub created_at: String,
    pub updated_at: String,
    pub attempts: u32,
    pub last_attempt_at: Option<String>,
    pub sent_at: Option<String>,
    pub dismissed_at: Option<String>,
    pub snoozed_until: Option<String>,
    pub expired_at: Option<String>,
    pub done_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitmentCandidate {
    pub kind: CommitmentKind,
    pub sensitivity: CommitmentSensitivity,
    pub reason: String,
    pub suggested_text: String,
    pub source_text: String,
    pub confidence: f64,
    pub dedupe_key: String,
    pub earliest_hours: u32,
    pub latest_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitExtractResult {
    pub candidates: Vec<CommitmentCandidate>,
    pub source_analysis: CommitSourceAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSourceAnalysis {
    pub has_deadline_mention: bool,
    pub has_event_mention: bool,
    pub has_care_mention: bool,
    pub has_open_question: bool,
    pub time_expressions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitStats {
    pub total: usize,
    pub pending: usize,
    pub due_now: usize,
    pub sent: usize,
    pub done: usize,
    pub expired: usize,
}

pub struct XinCommitService;

impl XinCommitService {
    pub fn extract_from_dialogue(
        user_msg: &str,
        assistant_msg: &str,
        conversation_id: &str,
    ) -> CommitExtractResult {
        let combined = format!("{} {}", user_msg, assistant_msg);
        let analysis = Self::analyze_source(&combined);

        let mut candidates = Vec::new();

        if analysis.has_deadline_mention {
            for time_expr in &analysis.time_expressions {
                let (earliest_hours, latest_hours) = Self::parse_time_expression(time_expr);
                let dedupe_key = format!("deadline:{}:{}", conversation_id, Self::hash_str(time_expr));
                candidates.push(CommitmentCandidate {
                    kind: CommitmentKind::DeadlineCheck,
                    sensitivity: CommitmentSensitivity::Routine,
                    reason: format!("对话中提到截止时间: {}", time_expr),
                    suggested_text: format!("提醒：之前提到的「{}」快到截止时间了，需要确认进展吗？", Self::extract_topic(&combined)),
                    source_text: combined.clone(),
                    confidence: 0.75,
                    dedupe_key,
                    earliest_hours,
                    latest_hours,
                });
            }
        }

        if analysis.has_event_mention {
            for time_expr in &analysis.time_expressions {
                let (earliest_hours, latest_hours) = Self::parse_time_expression(time_expr);
                let dedupe_key = format!("event:{}:{}", conversation_id, Self::hash_str(time_expr));
                candidates.push(CommitmentCandidate {
                    kind: CommitmentKind::EventCheckIn,
                    sensitivity: CommitmentSensitivity::Routine,
                    reason: format!("对话中提及事件时间: {}", time_expr),
                    suggested_text: format!(
                        "你之前提到的「{}」，时间差不多了，需要我帮你准备什么吗？",
                        Self::extract_topic(&combined)
                    ),
                    source_text: combined.clone(),
                    confidence: 0.70,
                    dedupe_key,
                    earliest_hours,
                    latest_hours: latest_hours.max(earliest_hours + 4),
                });
            }
        }

        if analysis.has_care_mention {
            let dedupe_key = format!("care:{}:{}", conversation_id, Self::hash_str(user_msg));
            candidates.push(CommitmentCandidate {
                kind: CommitmentKind::CareCheckIn,
                sensitivity: CommitmentSensitivity::Care,
                reason: "对话表现出需要关怀的信号".to_string(),
                suggested_text: "之前聊到的内容，一切还好吗？有需要我帮忙的地方随时说。".to_string(),
                source_text: user_msg.to_string(),
                confidence: 0.68,
                dedupe_key,
                earliest_hours: 6,
                latest_hours: 24,
            });
        }

        if analysis.has_open_question {
            let dedupe_key = format!("open:{}", conversation_id);
            candidates.push(CommitmentCandidate {
                kind: CommitmentKind::OpenLoop,
                sensitivity: CommitmentSensitivity::Routine,
                reason: "对话中有未解决的问题".to_string(),
                suggested_text: format!("之前聊到「{}」，需要我继续帮你跟进吗？", Self::extract_topic(&combined)),
                source_text: combined.clone(),
                confidence: 0.65,
                dedupe_key,
                earliest_hours: 2,
                latest_hours: 12,
            });
        }

        CommitExtractResult { candidates, source_analysis: analysis }
    }

    pub fn create_commitment(candidate: &CommitmentCandidate, now: DateTime<Utc>) -> CommitmentRecord {
        let now_str = now.to_rfc3339();
        let earliest = now + Duration::hours(candidate.earliest_hours as i64);
        let latest = now + Duration::hours(candidate.latest_hours as i64);

        CommitmentRecord {
            id: uuid::Uuid::new_v4().to_string(),
            kind: candidate.kind.clone(),
            sensitivity: candidate.sensitivity.clone(),
            status: CommitmentStatus::Pending,
            reason: candidate.reason.clone(),
            suggested_text: candidate.suggested_text.clone(),
            source_text: candidate.source_text.clone(),
            confidence: candidate.confidence,
            due_window: DueWindow {
                earliest_ms: earliest.timestamp_millis(),
                latest_ms: latest.timestamp_millis(),
                timezone: "Asia/Shanghai".to_string(),
            },
            dedupe_key: candidate.dedupe_key.clone(),
            created_at: now_str.clone(),
            updated_at: now_str,
            attempts: 0,
            last_attempt_at: None,
            sent_at: None,
            dismissed_at: None,
            snoozed_until: None,
            expired_at: None,
            done_at: None,
        }
    }

    pub fn check_now_due(commitments: &[CommitmentRecord]) -> Vec<&CommitmentRecord> {
        let now_ms = Utc::now().timestamp_millis();
        commitments
            .iter()
            .filter(|c| {
                c.status == CommitmentStatus::Pending
                    && now_ms >= c.due_window.earliest_ms
                    && now_ms <= c.due_window.latest_ms
            })
            .collect()
    }

    pub fn check_expired(commitments: &[CommitmentRecord]) -> Vec<&CommitmentRecord> {
        let now_ms = Utc::now().timestamp_millis();
        commitments
            .iter()
            .filter(|c| {
                c.status == CommitmentStatus::Pending && now_ms > c.due_window.latest_ms
            })
            .collect()
    }

    pub fn update_status(
        record: &mut CommitmentRecord,
        status: CommitmentStatus,
    ) {
        let now = Utc::now().to_rfc3339();
        record.status = status;
        record.updated_at = now.clone();
        match record.status {
            CommitmentStatus::Sent => {
                record.sent_at = Some(now);
                record.attempts += 1;
            }
            CommitmentStatus::Dismissed => record.dismissed_at = Some(now),
            CommitmentStatus::Done => record.done_at = Some(now),
            CommitmentStatus::Expired => record.expired_at = Some(now),
            _ => {}
        }
    }

    pub fn snooze(record: &mut CommitmentRecord, hours: i64) {
        let now = Utc::now();
        let snooze_until = now + Duration::hours(hours);
        record.status = CommitmentStatus::Snoozed;
        record.snoozed_until = Some(snooze_until.to_rfc3339());
        record.due_window.earliest_ms = snooze_until.timestamp_millis();
        record.due_window.latest_ms = (snooze_until + Duration::hours(12)).timestamp_millis();
        record.updated_at = now.to_rfc3339();
    }

    pub fn mark_done(record: &mut CommitmentRecord) {
        Self::update_status(record, CommitmentStatus::Done);
    }

    pub fn dismiss(record: &mut CommitmentRecord) {
        Self::update_status(record, CommitmentStatus::Dismissed);
    }

    pub fn generate_reminder_text(commitment: &CommitmentRecord) -> String {
        commitment.suggested_text.clone()
    }

    pub fn get_stats(commitments: &[CommitmentRecord]) -> CommitStats {
        let total = commitments.len();
        let pending = commitments.iter().filter(|c| c.status == CommitmentStatus::Pending).count();
        let due_now = Self::check_now_due(commitments).len();
        let sent = commitments.iter().filter(|c| c.status == CommitmentStatus::Sent).count();
        let done = commitments.iter().filter(|c| c.status == CommitmentStatus::Done).count();
        let expired = commitments.iter().filter(|c| c.status == CommitmentStatus::Expired).count();
        CommitStats { total, pending, due_now, sent, done, expired }
    }

    pub fn deduplicate_candidates(
        candidates: Vec<CommitmentCandidate>,
        existing: &[CommitmentRecord],
    ) -> Vec<CommitmentCandidate> {
        let existing_keys: std::collections::HashSet<&str> =
            existing.iter().map(|r| r.dedupe_key.as_str()).collect();
        candidates
            .into_iter()
            .filter(|c| !existing_keys.contains(c.dedupe_key.as_str()))
            .collect()
    }

    fn analyze_source(text: &str) -> CommitSourceAnalysis {
        let lower = text.to_lowercase();

        let deadline_keywords = [
            "截止", "deadline", "到期", "ddl", "之前", "before", "due",
            "期限", "马上", "赶紧", "尽快", "asap", "urgent",
        ];
        let event_keywords = [
            "开会", "会议", "面试", "meeting", "interview", "明天", "下周",
            "后天", "今晚", "晚上", "早上", "下午", "中午", "周末", "周",
            "约了", "安排了", "scheduled",
        ];
        let care_keywords = [
            "难过", "不开心", "压力", "累", "烦", "焦虑", "失眠", "困",
            "不舒服", "生病", "担心", "害怕", "孤独", "想哭", "sad",
            "tired", "stressed", "anxious", "worried",
        ];
        let open_loop_keywords = [
            "怎么办", "怎么弄", "帮我", "不懂", "不理解", "不太明白",
            "接下来", "然后呢", "下一步", "how to", "what next",
            "不知道", "不确定",
        ];

        let has_deadline = deadline_keywords.iter().any(|kw| lower.contains(kw));
        let has_event = event_keywords.iter().any(|kw| lower.contains(kw));
        let has_care = care_keywords.iter().any(|kw| lower.contains(kw));
        let has_open = open_loop_keywords.iter().any(|kw| lower.contains(kw));

        let time_expressions = Self::extract_time_expressions(text);

        CommitSourceAnalysis {
            has_deadline_mention: has_deadline,
            has_event_mention: has_event,
            has_care_mention: has_care,
            has_open_question: has_open,
            time_expressions,
        }
    }

    fn extract_time_expressions(text: &str) -> Vec<String> {
        let mut results = Vec::new();
        let patterns = [
            r"\d{1,2}点", r"\d{1,2}:\d{2}", r"\d{1,2}小时", r"\d{1,2}天",
            r"明天", r"后天", r"下周", r"下周[一二三四五六日]", r"周[一二三四五六日]",
            r"\d{1,2}月\d{1,2}日", r"\d{1,2}号",  r"今晚", r"明早",
            r"早上", r"中午", r"下午", r"晚上", r"周末",
            r"\d{1,2}分钟后", r"\d{1,2}小时后", r"\d{1,2}天后",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for m in re.find_iter(text) {
                    results.push(m.as_str().to_string());
                }
            }
        }
        results
    }

    fn parse_time_expression(expr: &str) -> (u32, u32) {
        if expr.contains("分钟") {
            if let Some(n) = Self::extract_number(expr) {
                let hours = (n as f64 / 60.0).ceil() as u32;
                return (hours.max(1), hours.max(1) + 1);
            }
        }
        if expr.contains("小时") {
            if let Some(n) = Self::extract_number(expr) {
                return (n as u32, n as u32 + 2);
            }
        }
        if expr.contains("天") || expr.contains("日") || expr.contains("号") {
            if let Some(n) = Self::extract_number(expr) {
                return (n as u32 * 24 - 4, n as u32 * 24 + 4);
            }
        }
        if expr == "明天" || expr == "明早" {
            return (20, 28);
        }
        if expr == "后天" {
            return (44, 52);
        }
        if expr == "今晚" {
            return (4, 10);
        }
        if expr == "早上" {
            return (8, 14);
        }
        if expr == "中午" {
            return (4, 10);
        }
        if expr == "下午" {
            return (4, 10);
        }
        if expr == "晚上" {
            return (4, 12);
        }
        if expr == "周末" || expr.contains("下周") {
            return (24, 72);
        }
        (4, 12)
    }

    fn extract_number(s: &str) -> Option<i64> {
        s.chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<i64>()
            .ok()
    }

    fn extract_topic(text: &str) -> String {
        let indicators = ["关于", "讨论", "了", "过", "到"];
        for indicator in &indicators {
            if let Some(pos) = text.find(indicator) {
                let start = pos + indicator.len();
                let end = (start + 30).min(text.len());
                let slice = &text[start..end];
                let trimmed = slice
                    .trim_start_matches(|c: char| !c.is_alphanumeric())
                    .trim();
                let topic: String = trimmed
                    .chars()
                    .take(20)
                    .collect();
                if !topic.is_empty() {
                    return topic;
                }
            }
        }
        text.chars().take(15).collect()
    }

    fn hash_str(s: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_deadline() {
        let result = XinCommitService::extract_from_dialogue(
            "这个项目的截止时间是明天下午5点",
            "好的，我会在截止前完成",
            "conv-1",
        );
        assert!(result.source_analysis.has_deadline_mention);
        assert!(!result.candidates.is_empty());
        assert_eq!(result.candidates[0].kind, CommitmentKind::DeadlineCheck);
    }

    #[test]
    fn test_extract_event() {
        let result = XinCommitService::extract_from_dialogue(
            "下周一有个面试，有点紧张",
            "别担心，好好准备就行",
            "conv-2",
        );
        assert!(result.source_analysis.has_event_mention);
        assert!(result.candidates.iter().any(|c| c.kind == CommitmentKind::EventCheckIn));
    }

    #[test]
    fn test_extract_care() {
        let result = XinCommitService::extract_from_dialogue(
            "最近压力好大，晚上总是失眠",
            "理解你的感受，适当放松一下",
            "conv-3",
        );
        assert!(result.source_analysis.has_care_mention);
        assert!(result.candidates.iter().any(|c| c.kind == CommitmentKind::CareCheckIn));
    }

    #[test]
    fn test_extract_open_loop() {
        let result = XinCommitService::extract_from_dialogue(
            "这个bug怎么修啊，不太明白",
            "让我看看...",
            "conv-4",
        );
        assert!(result.source_analysis.has_open_question);
        assert!(result.candidates.iter().any(|c| c.kind == CommitmentKind::OpenLoop));
    }

    #[test]
    fn test_create_commitment() {
        let candidate = CommitmentCandidate {
            kind: CommitmentKind::DeadlineCheck,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "测试".to_string(),
            suggested_text: "提醒".to_string(),
            source_text: "原文".to_string(),
            confidence: 0.8,
            dedupe_key: "test-key".to_string(),
            earliest_hours: 4,
            latest_hours: 12,
        };
        let record = XinCommitService::create_commitment(&candidate, Utc::now());
        assert_eq!(record.status, CommitmentStatus::Pending);
        assert_eq!(record.kind, CommitmentKind::DeadlineCheck);
        assert_eq!(record.confidence, 0.8);
        assert!(record.due_window.earliest_ms > Utc::now().timestamp_millis());
    }

    #[test]
    fn test_check_now_due() {
        let now = Utc::now();
        let candidate = CommitmentCandidate {
            kind: CommitmentKind::EventCheckIn,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "t".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.7,
            dedupe_key: "k".to_string(),
            earliest_hours: 0,
            latest_hours: 24,
        };
        let record = XinCommitService::create_commitment(&candidate, now - Duration::hours(1));
        assert_eq!(XinCommitService::check_now_due(&[record]).len(), 1);
    }

    #[test]
    fn test_check_not_due_yet() {
        let candidate = CommitmentCandidate {
            kind: CommitmentKind::DeadlineCheck,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "t".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.7,
            dedupe_key: "k2".to_string(),
            earliest_hours: 24,
            latest_hours: 48,
        };
        let record = XinCommitService::create_commitment(&candidate, Utc::now());
        assert_eq!(XinCommitService::check_now_due(&[record]).len(), 0);
    }

    #[test]
    fn test_update_status() {
        let candidate = CommitmentCandidate {
            kind: CommitmentKind::OpenLoop,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "t".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.7,
            dedupe_key: "k3".to_string(),
            earliest_hours: 1,
            latest_hours: 4,
        };
        let mut record = XinCommitService::create_commitment(&candidate, Utc::now());
        XinCommitService::mark_done(&mut record);
        assert_eq!(record.status, CommitmentStatus::Done);
        assert!(record.done_at.is_some());
    }

    #[test]
    fn test_snooze() {
        let candidate = CommitmentCandidate {
            kind: CommitmentKind::EventCheckIn,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "t".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.7,
            dedupe_key: "k4".to_string(),
            earliest_hours: 1,
            latest_hours: 4,
        };
        let mut record = XinCommitService::create_commitment(&candidate, Utc::now());
        XinCommitService::snooze(&mut record, 6);
        assert_eq!(record.status, CommitmentStatus::Snoozed);
        assert!(record.snoozed_until.is_some());
    }

    #[test]
    fn test_dismiss() {
        let candidate = CommitmentCandidate {
            kind: CommitmentKind::OpenLoop,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "t".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.7,
            dedupe_key: "k5".to_string(),
            earliest_hours: 1,
            latest_hours: 4,
        };
        let mut record = XinCommitService::create_commitment(&candidate, Utc::now());
        XinCommitService::dismiss(&mut record);
        assert_eq!(record.status, CommitmentStatus::Dismissed);
    }

    #[test]
    fn test_deduplicate_candidates() {
        let existing = vec![CommitmentRecord {
            id: "1".to_string(),
            kind: CommitmentKind::DeadlineCheck,
            sensitivity: CommitmentSensitivity::Routine,
            status: CommitmentStatus::Pending,
            reason: "r".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.8,
            due_window: DueWindow { earliest_ms: 0, latest_ms: 0, timezone: "UTC".to_string() },
            dedupe_key: "deadline:conv-x:abc".to_string(),
            created_at: "".to_string(),
            updated_at: "".to_string(),
            attempts: 0,
            last_attempt_at: None,
            sent_at: None,
            dismissed_at: None,
            snoozed_until: None,
            expired_at: None,
            done_at: None,
        }];

        let candidates = vec![
            CommitmentCandidate {
                kind: CommitmentKind::DeadlineCheck,
                sensitivity: CommitmentSensitivity::Routine,
                reason: "r1".to_string(),
                suggested_text: "s1".to_string(),
                source_text: "s1".to_string(),
                confidence: 0.8,
                dedupe_key: "deadline:conv-x:abc".to_string(),
                earliest_hours: 4,
                latest_hours: 12,
            },
            CommitmentCandidate {
                kind: CommitmentKind::EventCheckIn,
                sensitivity: CommitmentSensitivity::Routine,
                reason: "r2".to_string(),
                suggested_text: "s2".to_string(),
                source_text: "s2".to_string(),
                confidence: 0.7,
                dedupe_key: "event:conv-x:def".to_string(),
                earliest_hours: 6,
                latest_hours: 24,
            },
        ];

        let result = XinCommitService::deduplicate_candidates(candidates, &existing);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].dedupe_key, "event:conv-x:def");
    }

    #[test]
    fn test_get_stats() {
        let now = Utc::now();
        let candidate1 = CommitmentCandidate {
            kind: CommitmentKind::DeadlineCheck,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "r".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.8,
            dedupe_key: "k10".to_string(),
            earliest_hours: 0,
            latest_hours: 24,
        };
        let record1 = XinCommitService::create_commitment(&candidate1, now - Duration::hours(1));
        let candidate2 = CommitmentCandidate {
            kind: CommitmentKind::EventCheckIn,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "r".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.7,
            dedupe_key: "k11".to_string(),
            earliest_hours: 24,
            latest_hours: 48,
        };
        let mut record2 = XinCommitService::create_commitment(&candidate2, now);
        XinCommitService::mark_done(&mut record2);

        let stats = XinCommitService::get_stats(&[record1, record2]);
        assert_eq!(stats.total, 2);
        assert_eq!(stats.pending, 1);
        assert_eq!(stats.due_now, 1);
        assert_eq!(stats.done, 1);
    }

    #[test]
    fn test_check_expired() {
        let now = Utc::now();
        let candidate = CommitmentCandidate {
            kind: CommitmentKind::DeadlineCheck,
            sensitivity: CommitmentSensitivity::Routine,
            reason: "r".to_string(),
            suggested_text: "s".to_string(),
            source_text: "s".to_string(),
            confidence: 0.8,
            dedupe_key: "k12".to_string(),
            earliest_hours: 1,
            latest_hours: 2,
        };
        let record = XinCommitService::create_commitment(&candidate, now - Duration::hours(5));
        let records = [record];
        let expired = XinCommitService::check_expired(&records);
        assert_eq!(expired.len(), 1);
    }

    #[test]
    fn test_no_candidates_plain_text() {
        let result = XinCommitService::extract_from_dialogue(
            "今天天气真好",
            "是啊，阳光明媚的",
            "conv-5",
        );
        assert!(result.candidates.is_empty());
    }
}