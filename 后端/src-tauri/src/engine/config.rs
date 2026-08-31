// 会话配置 — 对标 Codex-rs 的 SessionConfiguration

/// 会话配置
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// 模型名称
    pub model: String,
    /// 系统提示
    pub system_prompt: String,
    /// Token 预算
    pub token_budget: u64,
    /// 最大回合数
    pub max_turns: usize,
    /// 温度
    pub temperature: f32,
    /// 最大输出 Token
    pub max_output_tokens: u64,
    /// Top P
    pub top_p: f32,
    /// 活跃工具列表
    pub active_tools: Vec<String>,
    /// 是否启用沙箱
    pub sandbox_enabled: bool,
    /// 是否启用自动压缩
    pub auto_compact: bool,
    /// 压缩触发阈值 (Token数)
    pub compact_threshold: u64,
    /// 是否启用 Skill
    pub skills_enabled: bool,
    /// 超时时间 (秒)
    pub timeout_secs: u64,
    /// 工作区路径
    pub workspace_path: Option<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o".into(),
            system_prompt: String::new(),
            token_budget: 200_000,
            max_turns: 100,
            temperature: 0.7,
            max_output_tokens: 16_384,
            top_p: 1.0,
            active_tools: vec![
                "read_file".into(),
                "write_file".into(),
                "execute_code".into(),
                "search".into(),
            ],
            sandbox_enabled: true,
            auto_compact: true,
            compact_threshold: 150_000,
            skills_enabled: true,
            timeout_secs: 300,
            workspace_path: None,
        }
    }
}

impl SessionConfig {
    /// 使用指定模型创建配置
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    /// 设置系统提示
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// 设置 Token 预算
    pub fn with_token_budget(mut self, budget: u64) -> Self {
        self.token_budget = budget;
        self
    }

    /// 设置工作区
    pub fn with_workspace(mut self, path: impl Into<String>) -> Self {
        self.workspace_path = Some(path.into());
        self
    }

    /// 启用/禁用沙箱
    pub fn with_sandbox(mut self, enabled: bool) -> Self {
        self.sandbox_enabled = enabled;
        self
    }

    /// 设置活跃工具
    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.active_tools = tools;
        self
    }
}