use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::skill::{SkillInvocation, InvocationType, MatchType};

/// 技能遥测追踪器
/// 对标 Codex 的 skill_invocation 事件追踪
pub struct SkillTelemetry {
    events: Mutex<Vec<TelemetryEvent>>,
    enabled: bool,
    max_events: usize,
}

#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    pub event_type: TelemetryEventType,
    pub skill_name: String,
    pub timestamp: u64,
    pub duration_ms: Option<u64>,
    pub success: bool,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TelemetryEventType {
    /// 技能被加载
    SkillLoaded,
    /// 技能被显式调用
    SkillInvoked,
    /// 技能被隐式触发
    SkillImplicitlyTriggered,
    /// 技能被自动发现
    SkillAutoDiscovered,
    /// 技能执行完成
    SkillCompleted,
    /// 技能执行失败
    SkillFailed,
    /// 技能被跳过（预算不足）
    SkillSkipped,
    /// 技能描述被截断
    SkillTruncated,
}

impl SkillTelemetry {
    pub fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            enabled: true,
            max_events: 10_000,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn with_max_events(mut self, max: usize) -> Self {
        self.max_events = max;
        self
    }

    /// 记录技能调用事件
    pub fn record_invocation(&self, invocation: &SkillInvocation) {
        if !self.enabled {
            return;
        }

        let event_type = match invocation.invocation_type {
            InvocationType::Explicit => TelemetryEventType::SkillInvoked,
            InvocationType::Implicit => TelemetryEventType::SkillImplicitlyTriggered,
            InvocationType::AutoDiscovered => TelemetryEventType::SkillAutoDiscovered,
        };

        self.push(TelemetryEvent {
            event_type,
            skill_name: invocation.skill_name.clone(),
            timestamp: now_ms(),
            duration_ms: invocation.duration_ms,
            success: invocation.match_type != MatchType::NoMatch,
            metadata: invocation.metadata.clone(),
        });
    }

    /// 记录技能加载
    pub fn record_load(&self, skill_name: &str, success: bool) {
        if !self.enabled {
            return;
        }
        self.push(TelemetryEvent {
            event_type: TelemetryEventType::SkillLoaded,
            skill_name: skill_name.to_string(),
            timestamp: now_ms(),
            duration_ms: None,
            success,
            metadata: None,
        });
    }

    /// 记录技能完成
    pub fn record_completion(&self, skill_name: &str, duration_ms: u64, success: bool) {
        if !self.enabled {
            return;
        }
        self.push(TelemetryEvent {
            event_type: if success {
                TelemetryEventType::SkillCompleted
            } else {
                TelemetryEventType::SkillFailed
            },
            skill_name: skill_name.to_string(),
            timestamp: now_ms(),
            duration_ms: Some(duration_ms),
            success,
            metadata: None,
        });
    }

    /// 记录技能被跳过
    pub fn record_skipped(&self, skill_name: &str) {
        if !self.enabled {
            return;
        }
        self.push(TelemetryEvent {
            event_type: TelemetryEventType::SkillSkipped,
            skill_name: skill_name.to_string(),
            timestamp: now_ms(),
            duration_ms: None,
            success: false,
            metadata: None,
        });
    }

    /// 获取所有事件
    pub fn events(&self) -> Vec<TelemetryEvent> {
        self.events.lock().unwrap().clone()
    }

    /// 获取指定技能的事件
    pub fn events_for_skill(&self, skill_name: &str) -> Vec<TelemetryEvent> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|e| e.skill_name == skill_name)
            .cloned()
            .collect()
    }

    /// 获取调用统计
    pub fn invocation_stats(&self) -> SkillInvocationStats {
        let mut stats = SkillInvocationStats::default();
        let events = self.events.lock().unwrap();

        for event in events.iter() {
            match event.event_type {
                TelemetryEventType::SkillInvoked => stats.explicit_invocations += 1,
                TelemetryEventType::SkillImplicitlyTriggered => stats.implicit_invocations += 1,
                TelemetryEventType::SkillAutoDiscovered => stats.auto_discoveries += 1,
                TelemetryEventType::SkillCompleted => stats.completions += 1,
                TelemetryEventType::SkillFailed => stats.failures += 1,
                TelemetryEventType::SkillLoaded => stats.loads += 1,
                TelemetryEventType::SkillSkipped => stats.skipped += 1,
                TelemetryEventType::SkillTruncated => stats.truncated += 1,
            }
        }

        stats
    }

    /// 清空事件
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }

    fn push(&self, event: TelemetryEvent) {
        let mut events = self.events.lock().unwrap();
        if events.len() >= self.max_events {
            events.remove(0);
        }
        events.push(event);
    }
}

/// 技能调用统计
#[derive(Debug, Clone, Default)]
pub struct SkillInvocationStats {
    pub explicit_invocations: usize,
    pub implicit_invocations: usize,
    pub auto_discoveries: usize,
    pub completions: usize,
    pub failures: usize,
    pub loads: usize,
    pub skipped: usize,
    pub truncated: usize,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_load() {
        let mut telemetry = SkillTelemetry::new();
        telemetry.record_load("test-skill", true);
        telemetry.record_load("test-skill", false);

        let skill_events = telemetry.events_for_skill("test-skill");
        assert_eq!(skill_events.len(), 2);
    }

    #[test]
    fn test_invocation_stats() {
        let mut telemetry = SkillTelemetry::new();
        telemetry.record_load("s1", true);
        telemetry.record_completion("s1", 100, true);
        telemetry.record_completion("s2", 50, false);

        let stats = telemetry.invocation_stats();
        assert_eq!(stats.loads, 1);
        assert_eq!(stats.completions, 1);
        assert_eq!(stats.failures, 1);
    }

    #[test]
    fn test_max_events() {
        let mut telemetry = SkillTelemetry::new().with_max_events(3);
        for i in 0..5 {
            telemetry.record_load(&format!("skill-{}", i), true);
        }
        assert_eq!(telemetry.events().len(), 3);
        // 应该保留最新的 3 个
        assert_eq!(telemetry.events()[0].skill_name, "skill-2");
    }
}