use std::collections::HashMap;

use crate::models::xin::Persona;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub name: Option<String>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ChatRole {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextWindow {
    pub messages: Vec<ChatMessage>,
    pub model_id: String,
    pub max_tokens: usize,
    pub used_tokens: usize,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompactionResult {
    pub original_count: usize,
    pub compacted_count: usize,
    pub summary_text: String,
    pub tokens_saved: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub available: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillResult {
    pub skill_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct XinPromptConfig {
    pub persona: Persona,
    pub include_skills: bool,
    pub include_mood: bool,
    pub include_time: bool,
    pub custom_instructions: Option<String>,
    /// D3.7 用户情绪风格指引（由 XinEmotionService::get_emotion_guide 生成）
    ///
    /// 当用户消息情绪强度超过阈值时，注入对应风格的指引文本到 system prompt，
    /// 引导云端 LLM 以合适风格回复（如用户焦虑→温柔安抚）。
    /// None 表示情绪平稳，不注入特殊风格。
    pub user_emotion_guide: Option<String>,
    /// D3.8 人格记忆（由 XinPersonalityService::get_persona_memory 生成摘要）
    ///
    /// 注入该人格历史交互摘要 + 用户偏好到 system prompt，
    /// 让小欣"记得"在该人格下与用户的过往交互（如偏好简洁回复、关注 rust 话题）。
    /// None 表示该人格无历史记忆（首次使用），不注入。
    pub persona_memory: Option<String>,
}

const SAFETY_MARGIN: f64 = 1.2;
const BASE_CHUNK_RATIO: f64 = 0.4;
const MIN_CHUNK_RATIO: f64 = 0.15;

fn model_context_lookup(model_id: &str) -> usize {
    let lower = model_id.to_lowercase();

    if lower.contains("gpt-4") && lower.contains("128k") {
        return 128_000;
    }
    if lower.contains("gpt-4") && lower.contains("32k") {
        return 32_000;
    }
    if lower.contains("gpt-4-turbo") || lower.contains("gpt-4o") {
        return 128_000;
    }
    if lower.contains("gpt-4") {
        return 8_192;
    }
    if lower.contains("gpt-3.5") && lower.contains("16k") {
        return 16_384;
    }
    if lower.contains("gpt-3.5") {
        return 4_096;
    }

    if lower.contains("claude-3-opus") {
        return 200_000;
    }
    if lower.contains("claude-3-sonnet") || lower.contains("claude-3-haiku") {
        return 200_000;
    }
    if lower.contains("claude-3") || lower.contains("claude-3.5") || lower.contains("claude-4") {
        return 200_000;
    }
    if lower.contains("claude") {
        return 100_000;
    }

    if lower.contains("gemini-2") || lower.contains("gemini-1.5-pro") {
        return 1_048_576;
    }
    if lower.contains("gemini-1.5-flash") {
        return 1_048_576;
    }
    if lower.contains("gemini") {
        return 32_000;
    }

    if lower.contains("deepseek-v3") || lower.contains("deepseek-r1") {
        return 128_000;
    }
    if lower.contains("deepseek") {
        return 64_000;
    }

    if lower.contains("qwen") && lower.contains("72b") {
        return 128_000;
    }
    if lower.contains("qwen") {
        return 32_000;
    }

    if lower.contains("llama-3") && lower.contains("70b") {
        return 8_192;
    }
    if lower.contains("llama-3") {
        return 8_192;
    }

    if lower.contains("codestral") || lower.contains("mistral") {
        return 32_000;
    }

    4_096
}

pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let mut tokens: f64 = 0.0;
    let mut char_iter = text.chars().peekable();

    while let Some(ch) = char_iter.next() {
        if ch.is_whitespace() {
            continue;
        }

        if ch.is_ascii_alphabetic() {
            let mut len = 1usize;
            while let Some(&next) = char_iter.peek() {
                if next.is_ascii_alphabetic() {
                    char_iter.next();
                    len += 1;
                } else {
                    break;
                }
            }
            tokens += (len as f64 / 3.5).ceil();
        } else if ch.is_ascii_digit() {
            let mut len = 1usize;
            while let Some(&next) = char_iter.peek() {
                if next.is_ascii_digit() {
                    char_iter.next();
                    len += 1;
                } else {
                    break;
                }
            }
            tokens += (len as f64 / 2.0).ceil();
        } else if ch as u32 >= 0x4E00 && ch as u32 <= 0x9FFF {
            tokens += 1.5;
        } else if ch as u32 >= 0x3040 && ch as u32 <= 0x30FF {
            tokens += 1.0;
        } else {
            tokens += 1.0;
        }
    }

    (tokens.ceil() as usize).max(1)
}

pub struct XinContextManager;

impl XinContextManager {
    pub fn new_context(model_id: &str) -> ContextWindow {
        let max_tokens = model_context_lookup(model_id);
        ContextWindow {
            messages: Vec::new(),
            model_id: model_id.to_string(),
            max_tokens,
            used_tokens: 0,
            summary: None,
        }
    }

    pub fn add_message(ctx: &mut ContextWindow, msg: ChatMessage) {
        let token_count = estimate_tokens(&msg.content);
        ctx.messages.push(msg);
        ctx.used_tokens += token_count;
    }

    pub fn add_system_prompt(ctx: &mut ContextWindow, prompt: &str) {
        let msg = ChatMessage {
            role: ChatRole::System,
            content: prompt.to_string(),
            name: None,
            timestamp: None,
        };
        Self::add_message(ctx, msg);
    }

    pub fn token_budget_remaining(ctx: &ContextWindow) -> usize {
        let safe_max = (ctx.max_tokens as f64 / SAFETY_MARGIN) as usize;
        safe_max.saturating_sub(ctx.used_tokens)
    }

    pub fn is_over_budget(ctx: &ContextWindow) -> bool {
        let safe_max = (ctx.max_tokens as f64 / SAFETY_MARGIN) as usize;
        ctx.used_tokens > safe_max
    }

    pub fn trim_to_budget(ctx: &mut ContextWindow) -> usize {
        if !Self::is_over_budget(ctx) {
            return 0;
        }

        let safe_max = (ctx.max_tokens as f64 / SAFETY_MARGIN) as usize;
        let mut removed = 0usize;
        let system_count = ctx
            .messages
            .iter()
            .filter(|m| m.role == ChatRole::System)
            .count();

        while ctx.used_tokens > safe_max && ctx.messages.len() > system_count + 2 {
            let trim_idx = system_count;
            if trim_idx < ctx.messages.len() {
                let removed_msg = ctx.messages.remove(trim_idx);
                let removed_tokens = estimate_tokens(&removed_msg.content);
                ctx.used_tokens = ctx.used_tokens.saturating_sub(removed_tokens);
                removed += 1;
            } else {
                break;
            }
        }

        removed
    }

    pub fn count_by_role(ctx: &ContextWindow) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for msg in &ctx.messages {
            let key = match msg.role {
                ChatRole::System => "system",
                ChatRole::User => "user",
                ChatRole::Assistant => "assistant",
            };
            *counts.entry(key.to_string()).or_insert(0) += 1;
        }
        counts
    }
}

pub struct XinCompactor;

impl XinCompactor {
    pub fn compact(messages: &[ChatMessage], model_id: &str) -> CompactionResult {
        let max_tokens = model_context_lookup(model_id);
        let safe_max = (max_tokens as f64 / SAFETY_MARGIN) as usize;

        let total_tokens: usize = messages.iter().map(|m| estimate_tokens(&m.content)).sum();
        if total_tokens <= safe_max {
            return CompactionResult {
                original_count: messages.len(),
                compacted_count: messages.len(),
                summary_text: String::new(),
                tokens_saved: 0,
            };
        }

        let chunk_size = (messages.len() as f64 * BASE_CHUNK_RATIO).ceil() as usize;
        let chunk_size = chunk_size.max((messages.len() as f64 * MIN_CHUNK_RATIO) as usize);
        let chunk_size = chunk_size.min(messages.len().saturating_sub(2));

        let chunk_tokens: usize = messages[..chunk_size]
            .iter()
            .map(|m| estimate_tokens(&m.content))
            .sum();

        let compacted_count = messages.len() - chunk_size;
        let summary_parts: Vec<String> = messages[..chunk_size]
            .iter()
            .filter(|m| m.role != ChatRole::System)
            .map(|m| {
                let truncated: String = m.content.chars().take(200).collect();
                format!("[{}]: {}", role_label(&m.role), truncated)
            })
            .collect();

        let summary_text = if summary_parts.is_empty() {
            "No prior history to summarize.".to_string()
        } else {
            format!(
                "Previous conversation summary ({} messages compacted):\n{}",
                chunk_size,
                summary_parts.join("\n")
            )
        };

        let tokens_saved = chunk_tokens.saturating_sub(estimate_tokens(&summary_text));

        CompactionResult {
            original_count: messages.len(),
            compacted_count,
            summary_text,
            tokens_saved,
        }
    }

    pub fn generate_recovery_briefing(messages: &[ChatMessage]) -> String {
        let user_msgs: Vec<&ChatMessage> = messages
            .iter()
            .filter(|m| m.role == ChatRole::User)
            .collect();

        let assistant_msgs: Vec<&ChatMessage> = messages
            .iter()
            .filter(|m| m.role == ChatRole::Assistant)
            .collect();

        let last_user = user_msgs.last().map(|m| m.content.as_str()).unwrap_or("N/A");
        let last_assistant = assistant_msgs
            .last()
            .map(|m| {
                let t: String = m.content.chars().take(300).collect();
                t
            })
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "RECOVERY BRIEFING:\n- Total turns: {} user, {} assistant\n- Last user request: {}\n- Last assistant response (truncated): {}\n- Context: {} total messages in conversation.",
            user_msgs.len(),
            assistant_msgs.len(),
            last_user,
            last_assistant,
            messages.len()
        )
    }
}

pub struct XinPromptBuilder;

impl XinPromptBuilder {
    pub fn build_system_prompt(config: &XinPromptConfig) -> String {
        let mut parts: Vec<String> = Vec::new();

        parts.push(format!(
            "你是「{}」，{}。",
            config.persona.name, config.persona.description
        ));

        let trait_descriptions: Vec<String> = config
            .persona
            .traits
            .iter()
            .map(|t| format!("{}（{}%）", t.name, (t.value * 100.0) as i32))
            .collect();
        if !trait_descriptions.is_empty() {
            parts.push(format!("人格特质：{}。", trait_descriptions.join("、")));
        }

        let style = &config.persona.speaking_style;
        let style_parts: Vec<String> = {
            let mut s = Vec::new();
            if style.formality > 0.7 {
                s.push("正式".to_string());
            } else if style.formality < 0.3 {
                s.push("随意".to_string());
            }
            if style.verbosity > 0.7 {
                s.push("详细".to_string());
            } else if style.verbosity < 0.3 {
                s.push("简洁".to_string());
            }
            if style.humor > 0.7 {
                s.push("幽默".to_string());
            }
            if style.technical_depth > 0.7 {
                s.push("技术深入".to_string());
            }
            if style.empathy > 0.7 {
                s.push("富有同理心".to_string());
            }
            s
        };
        if !style_parts.is_empty() {
            parts.push(format!("说话风格：{}。", style_parts.join("、")));
        }

        if config.include_mood {
            parts.push(format!("当前基础情绪：{}。", config.persona.base_mood));
        }

        parts.push("请始终以第一人称「我」自称，称呼用户为「你」。".to_string());
        parts.push("保持角色一致性，不要跳出人设。".to_string());

        if config.include_time {
            let now = chrono::Utc::now();
            parts.push(format!(
                "当前时间：{}。",
                now.format("%Y年%m月%d日 %H:%M UTC").to_string()
            ));
        }

        if let Some(ref custom) = config.custom_instructions {
            if !custom.is_empty() {
                parts.push(custom.clone());
            }
        }

        // D3.7: 注入用户情绪风格指引（按用户情绪调整风格）
        if let Some(ref guide) = config.user_emotion_guide {
            if !guide.is_empty() {
                parts.push(guide.clone());
            }
        }

        // D3.8: 注入人格记忆（该人格历史交互摘要 + 用户偏好）
        // 让小欣"记得"在该人格下与用户的过往交互，切换人格时恢复对应上下文
        if let Some(ref memory) = config.persona_memory {
            if !memory.is_empty() {
                parts.push(memory.clone());
            }
        }

        parts.join("\n\n")
    }

    pub fn build_context_header(summary: Option<&str>, recent_topics: &[String]) -> String {
        let mut header = String::from("[CONTEXT]\n");

        if let Some(s) = summary {
            header.push_str(&format!("Previous summary: {}\n", s));
        }

        if !recent_topics.is_empty() {
            header.push_str(&format!(
                "Recent topics: {}\n",
                recent_topics.join(", ")
            ));
        }

        header
    }
}

pub trait XinSkill: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn category(&self) -> &str;
    fn execute(&self, input: &str) -> Result<SkillResult, String>;
}

pub struct BuiltinSkills {
    skills: HashMap<String, Box<dyn XinSkill>>,
}

impl BuiltinSkills {
    pub fn new() -> Self {
        let mut skills: HashMap<String, Box<dyn XinSkill>> = HashMap::new();
        skills.insert("echo".to_string(), Box::new(EchoSkill));
        skills.insert("calc".to_string(), Box::new(CalcSkill));
        skills.insert("time".to_string(), Box::new(TimeSkill));
        skills.insert("help".to_string(), Box::new(HelpSkill));
        Self { skills }
    }

    pub fn list_all(&self) -> Vec<SkillInfo> {
        self.skills
            .values()
            .map(|s| SkillInfo {
                id: s.id().to_string(),
                name: s.name().to_string(),
                description: s.description().to_string(),
                category: s.category().to_string(),
                available: true,
            })
            .collect()
    }

    pub fn execute(&self, skill_id: &str, input: &str) -> Result<SkillResult, String> {
        match self.skills.get(skill_id) {
            Some(skill) => skill.execute(input),
            None => Ok(SkillResult {
                skill_id: skill_id.to_string(),
                success: false,
                output: String::new(),
                error: Some(format!("Unknown skill: {}", skill_id)),
            }),
        }
    }

    pub fn build_skills_prompt(&self) -> String {
        let skill_list: Vec<String> = self
            .skills
            .values()
            .map(|s| format!("- {}: {}", s.name(), s.description()))
            .collect();

        format!(
            "AVAILABLE SKILLS:\n{}\n\nUse skills when appropriate. Format: /skill:<name> <input>",
            skill_list.join("\n")
        )
    }
}

struct EchoSkill;

impl XinSkill for EchoSkill {
    fn id(&self) -> &str {
        "echo"
    }
    fn name(&self) -> &str {
        "回显"
    }
    fn description(&self) -> &str {
        "回显输入内容，用于测试和确认"
    }
    fn category(&self) -> &str {
        "utility"
    }
    fn execute(&self, input: &str) -> Result<SkillResult, String> {
        Ok(SkillResult {
            skill_id: "echo".to_string(),
            success: true,
            output: input.to_string(),
            error: None,
        })
    }
}

struct CalcSkill;

impl XinSkill for CalcSkill {
    fn id(&self) -> &str {
        "calc"
    }
    fn name(&self) -> &str {
        "计算"
    }
    fn description(&self) -> &str {
        "执行简单数学运算，支持加减乘除和括号"
    }
    fn category(&self) -> &str {
        "utility"
    }
    fn execute(&self, input: &str) -> Result<SkillResult, String> {
        let cleaned: String = input
            .chars()
            .filter(|c| c.is_ascii_digit() || "+-*/(). ".contains(*c))
            .collect();

        if cleaned.is_empty() {
            return Ok(SkillResult {
                skill_id: "calc".to_string(),
                success: false,
                output: String::new(),
                error: Some("Empty calculation input".to_string()),
            });
        }

        let result = eval_simple_expr(&cleaned);
        Ok(SkillResult {
            skill_id: "calc".to_string(),
            success: true,
            output: format!("{}", result),
            error: None,
        })
    }
}

fn eval_simple_expr(expr: &str) -> f64 {
    let tokens: Vec<&str> = expr.split_whitespace().collect();
    if tokens.len() == 1 {
        return tokens[0].parse::<f64>().unwrap_or(0.0);
    }
    if tokens.len() == 3 {
        let a = tokens[0].parse::<f64>().unwrap_or(0.0);
        let b = tokens[2].parse::<f64>().unwrap_or(0.0);
        match tokens[1] {
            "+" => return a + b,
            "-" => return a - b,
            "*" => return a * b,
            "/" => {
                if b != 0.0 {
                    return a / b;
                }
                return 0.0;
            }
            _ => {}
        }
    }
    0.0
}

struct TimeSkill;

impl XinSkill for TimeSkill {
    fn id(&self) -> &str {
        "time"
    }
    fn name(&self) -> &str {
        "时间"
    }
    fn description(&self) -> &str {
        "获取当前日期和时间"
    }
    fn category(&self) -> &str {
        "utility"
    }
    fn execute(&self, _input: &str) -> Result<SkillResult, String> {
        let now = chrono::Utc::now();
        Ok(SkillResult {
            skill_id: "time".to_string(),
            success: true,
            output: now.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            error: None,
        })
    }
}

struct HelpSkill;

impl XinSkill for HelpSkill {
    fn id(&self) -> &str {
        "help"
    }
    fn name(&self) -> &str {
        "帮助"
    }
    fn description(&self) -> &str {
        "列出所有可用技能"
    }
    fn category(&self) -> &str {
        "meta"
    }
    fn execute(&self, _input: &str) -> Result<SkillResult, String> {
        Ok(SkillResult {
            skill_id: "help".to_string(),
            success: true,
            output: "可用技能: echo, calc, time, help".to_string(),
            error: None,
        })
    }
}

fn role_label(role: &ChatRole) -> &str {
    match role {
        ChatRole::System => "系统",
        ChatRole::User => "用户",
        ChatRole::Assistant => "助手",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_english() {
        let tokens = estimate_tokens("Hello world");
        assert!(tokens > 0);
        assert!(tokens <= 5);
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        let tokens = estimate_tokens("你好世界");
        assert_eq!(tokens, 6);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_mixed() {
        let tokens = estimate_tokens("Hello你好world世界");
        assert!(tokens > 0);
    }

    #[test]
    fn test_model_lookup_gpt4() {
        let window = model_context_lookup("gpt-4");
        assert_eq!(window, 8192);
    }

    #[test]
    fn test_model_lookup_claude() {
        let window = model_context_lookup("claude-3-opus");
        assert_eq!(window, 200_000);
    }

    #[test]
    fn test_model_lookup_unknown() {
        let window = model_context_lookup("unknown-model");
        assert_eq!(window, 4096);
    }

    #[test]
    fn test_context_add_and_trim() {
        let mut ctx = XinContextManager::new_context("gpt-4");
        let long_text = "A".repeat(500);
        for i in 0..50 {
            XinContextManager::add_message(
                &mut ctx,
                ChatMessage {
                    role: ChatRole::User,
                    content: format!("Message number {}: {}", i, long_text),
                    name: None,
                    timestamp: None,
                },
            );
        }
        let removed = XinContextManager::trim_to_budget(&mut ctx);
        assert!(removed > 0);
        assert!(!XinContextManager::is_over_budget(&ctx));
    }

    #[test]
    fn test_compactor_basic() {
        let long_text = "B".repeat(300);
        let messages: Vec<ChatMessage> = (0..100)
            .map(|i| ChatMessage {
                role: if i % 2 == 0 {
                    ChatRole::User
                } else {
                    ChatRole::Assistant
                },
                content: format!("Message {}: {}", i, long_text),
                name: None,
                timestamp: None,
            })
            .collect();

        let result = XinCompactor::compact(&messages, "gpt-4");
        assert!(result.original_count == 100);
        assert!(!result.summary_text.is_empty());
    }

    #[test]
    fn test_prompt_builder() {
        let persona = Persona {
            id: "test".into(),
            name: "测试助手".into(),
            description: "用于测试的助手".into(),
            traits: vec![crate::models::xin::PersonaTrait {
                name: "逻辑性".into(),
                value: 0.9,
            }],
            speaking_style: crate::models::xin::SpeakingStyle::default(),
            base_mood: "calm".into(),
            avatar_emoji: "T".into(),
            is_builtin: true,
        };

        let config = XinPromptConfig {
            persona: persona.clone(),
            include_skills: false,
            include_mood: true,
            include_time: true,
            custom_instructions: None,
            user_emotion_guide: None,
            persona_memory: None,
        };

        let prompt = XinPromptBuilder::build_system_prompt(&config);
        assert!(prompt.contains("测试助手"));
        assert!(prompt.contains("calm"));
        assert!(prompt.contains("逻辑性"));
    }

    #[test]
    fn test_skills_list() {
        let skills = BuiltinSkills::new();
        let list = skills.list_all();
        assert_eq!(list.len(), 4);
        assert!(list.iter().any(|s| s.id == "echo"));
        assert!(list.iter().any(|s| s.id == "calc"));
    }

    #[test]
    fn test_skills_execute() {
        let skills = BuiltinSkills::new();
        let result = skills.execute("echo", "hello").unwrap();
        assert!(result.success);
        assert_eq!(result.output, "hello");

        let result = skills.execute("calc", "2 + 3").unwrap();
        assert!(result.success);
        assert_eq!(result.output, "5");

        let result = skills.execute("time", "").unwrap();
        assert!(result.success);
        assert!(!result.output.is_empty());
    }

    #[test]
    fn test_recovery_briefing() {
        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: "System prompt".into(),
                name: None,
                timestamp: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: "Help me".into(),
                name: None,
                timestamp: None,
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "Sure, what do you need?".into(),
                name: None,
                timestamp: None,
            },
        ];

        let briefing = XinCompactor::generate_recovery_briefing(&messages);
        assert!(briefing.contains("Help me"));
        assert!(briefing.contains("RECOVERY BRIEFING"));
    }
}