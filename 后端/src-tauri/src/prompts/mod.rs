use std::collections::HashMap;
use std::sync::LazyLock;

use crate::error::app_error::AppError;
use crate::models::prompt::{
    AgentsMdFile, AssembleSystemPromptRequest, AssembleSystemPromptResult, HierarchicalInstructions,
    PromptPart, PromptTemplateMeta, PromptTemplateType, PromptVariable, RenderPromptRequest,
    RenderPromptResult,
};

/// 嵌入模板文件
macro_rules! embed_template {
    ($path:literal) => {
        include_str!(concat!("templates/", $path))
    };
}

/// 系统主提示词模板
const SYSTEM_TEMPLATE: &str = embed_template!("system.md");
/// 代码审查模板
const REVIEW_TEMPLATE: &str = embed_template!("review.md");
/// 上下文压缩模板
const COMPACT_TEMPLATE: &str = embed_template!("compact.md");
/// 计划制定模板
const PLAN_TEMPLATE: &str = embed_template!("plan.md");
/// 子 Agent 模板
const SUB_AGENT_TEMPLATE: &str = embed_template!("sub_agent.md");
/// 技能使用指南模板
const SKILLS_HOW_TO_TEMPLATE: &str = embed_template!("skills_how_to.md");

/// 模板元数据注册表
static TEMPLATE_METAS: LazyLock<HashMap<PromptTemplateType, PromptTemplateMeta>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();
        map.insert(
            PromptTemplateType::System,
            PromptTemplateMeta {
                name: "system".into(),
                template_type: PromptTemplateType::System,
                description: "系统主提示词，定义 Agent 行为准则和工具调用规范".into(),
                version: "1.0.0".into(),
                variables: vec![
                    PromptVariable {
                        name: "workspace_root".into(),
                        description: "工作区根目录".into(),
                        required: false,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "git_branch".into(),
                        description: "当前 Git 分支".into(),
                        required: false,
                        default_value: Some("main".into()),
                    },
                    PromptVariable {
                        name: "platform".into(),
                        description: "操作系统平台".into(),
                        required: false,
                        default_value: Some(std::env::consts::OS.into()),
                    },
                    PromptVariable {
                        name: "current_date".into(),
                        description: "当前日期".into(),
                        required: false,
                        default_value: None,
                    },
                ],
            },
        );
        map.insert(
            PromptTemplateType::Review,
            PromptTemplateMeta {
                name: "review".into(),
                template_type: PromptTemplateType::Review,
                description: "代码审查提示词，指导 Agent 进行代码审查".into(),
                version: "1.0.0".into(),
                variables: vec![PromptVariable {
                    name: "review_target".into(),
                    description: "审查目标描述（分支、commit 或未提交变更）".into(),
                    required: true,
                    default_value: None,
                }],
            },
        );
        map.insert(
            PromptTemplateType::Compact,
            PromptTemplateMeta {
                name: "compact".into(),
                template_type: PromptTemplateType::Compact,
                description: "上下文压缩提示词，指导 Agent 压缩对话历史".into(),
                version: "1.0.0".into(),
                variables: vec![
                    PromptVariable {
                        name: "conversation_text".into(),
                        description: "待压缩的对话文本".into(),
                        required: true,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "max_tokens".into(),
                        description: "压缩后最大 token 数".into(),
                        required: true,
                        default_value: Some("2048".into()),
                    },
                    PromptVariable {
                        name: "turn_count".into(),
                        description: "当前对话轮数".into(),
                        required: false,
                        default_value: None,
                    },
                ],
            },
        );
        map.insert(
            PromptTemplateType::Plan,
            PromptTemplateMeta {
                name: "plan".into(),
                template_type: PromptTemplateType::Plan,
                description: "任务规划提示词，指导 Agent 制定执行计划".into(),
                version: "1.0.0".into(),
                variables: vec![
                    PromptVariable {
                        name: "user_goal".into(),
                        description: "用户目标".into(),
                        required: true,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "workspace_root".into(),
                        description: "工作区根目录".into(),
                        required: false,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "project_type".into(),
                        description: "项目类型".into(),
                        required: false,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "existing_modules".into(),
                        description: "已有模块列表".into(),
                        required: false,
                        default_value: None,
                    },
                ],
            },
        );
        map.insert(
            PromptTemplateType::SubAgent,
            PromptTemplateMeta {
                name: "sub_agent".into(),
                template_type: PromptTemplateType::SubAgent,
                description: "子 Agent 提示词，定义子 Agent 的行为规则".into(),
                version: "1.0.0".into(),
                variables: vec![
                    PromptVariable {
                        name: "task_name".into(),
                        description: "子任务名称".into(),
                        required: true,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "task_description".into(),
                        description: "子任务描述".into(),
                        required: true,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "parent_agent_id".into(),
                        description: "父 Agent ID".into(),
                        required: true,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "child_context".into(),
                        description: "子 Agent 上下文".into(),
                        required: false,
                        default_value: Some("无额外上下文".into()),
                    },
                ],
            },
        );
        map.insert(
            PromptTemplateType::SkillsHowTo,
            PromptTemplateMeta {
                name: "skills_how_to".into(),
                template_type: PromptTemplateType::SkillsHowTo,
                description: "技能使用指南，注入到 system prompt 中指导 Agent 使用技能".into(),
                version: "1.0.0".into(),
                variables: vec![
                    PromptVariable {
                        name: "available_skills".into(),
                        description: "可用技能列表描述".into(),
                        required: true,
                        default_value: None,
                    },
                    PromptVariable {
                        name: "skills_budget_pct".into(),
                        description: "技能描述预算百分比".into(),
                        required: false,
                        default_value: Some("2".into()),
                    },
                ],
            },
        );
        map
    });

/// 获取模板原始内容
fn get_template_raw(template_type: &PromptTemplateType) -> &'static str {
    match template_type {
        PromptTemplateType::System => SYSTEM_TEMPLATE,
        PromptTemplateType::Review => REVIEW_TEMPLATE,
        PromptTemplateType::Compact => COMPACT_TEMPLATE,
        PromptTemplateType::Plan => PLAN_TEMPLATE,
        PromptTemplateType::SubAgent => SUB_AGENT_TEMPLATE,
        PromptTemplateType::Realtime => SYSTEM_TEMPLATE, // 暂无独立模板，回退到 system
        PromptTemplateType::SkillsHowTo => SKILLS_HOW_TO_TEMPLATE,
    }
}

/// Prompt 模板管理器
pub struct PromptManager {
    /// 自定义模板覆盖
    custom_templates: HashMap<PromptTemplateType, String>,
    /// 模板变量默认值
    variable_defaults: HashMap<String, String>,
}

impl Default for PromptManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptManager {
    pub fn new() -> Self {
        let mut variable_defaults = HashMap::new();
        // 设置默认平台
        variable_defaults.insert("platform".into(), std::env::consts::OS.into());

        Self {
            custom_templates: HashMap::new(),
            variable_defaults,
        }
    }

    /// 获取所有模板元数据
    pub fn list_templates(&self) -> Vec<PromptTemplateMeta> {
        TEMPLATE_METAS.values().cloned().collect()
    }

    /// 获取指定模板的元数据
    pub fn get_template_meta(
        &self,
        template_type: &PromptTemplateType,
    ) -> Option<&PromptTemplateMeta> {
        TEMPLATE_METAS.get(template_type)
    }

    /// 设置自定义模板（覆盖内置模板）
    pub fn set_custom_template(&mut self, template_type: PromptTemplateType, content: String) {
        self.custom_templates.insert(template_type, content);
    }

    /// 移除自定义模板
    pub fn remove_custom_template(&mut self, template_type: &PromptTemplateType) {
        self.custom_templates.remove(template_type);
    }

    /// 设置变量默认值
    pub fn set_variable_default(&mut self, name: &str, value: &str) {
        self.variable_defaults
            .insert(name.to_string(), value.to_string());
    }

    /// 渲染模板
    pub fn render(&self, request: RenderPromptRequest) -> Result<RenderPromptResult, AppError> {
        let meta = TEMPLATE_METAS
            .get(&request.template_type)
            .ok_or_else(|| AppError::NotFound)?;

        // 获取模板内容（优先自定义）
        let template_content = self
            .custom_templates
            .get(&request.template_type)
            .map(|s| s.as_str())
            .unwrap_or_else(|| get_template_raw(&request.template_type));

        // 验证必填变量
        for var in &meta.variables {
            if var.required
                && !request.variables.contains_key(&var.name)
                && !self.variable_defaults.contains_key(&var.name)
                && var.default_value.is_none()
            {
                return Err(AppError::Validation(format!(
                    "模板 '{}' 缺少必填变量: {}",
                    meta.name, var.name
                )));
            }
        }

        // 变量替换
        let mut content = template_content.to_string();
        for (key, value) in &request.variables {
            let placeholder = "{{".to_string() + key + "}}";
            content = content.replace(&placeholder, value);
        }

        // 填充默认值
        for var in &meta.variables {
            if !request.variables.contains_key(&var.name) {
                let placeholder = "{{".to_string() + &var.name + "}}";
                if let Some(default) = &var.default_value {
                    content = content.replace(&placeholder, default);
                } else if let Some(default) = self.variable_defaults.get(&var.name) {
                    content = content.replace(&placeholder, default);
                } else {
                    // 未填充的变量保留空
                    content = content.replace(&placeholder, "");
                }
            }
        }

        // 估算 token 数（简单按 4 字符 ≈ 1 token）
        let estimated_tokens = content.len() / 4;

        Ok(RenderPromptResult {
            content,
            estimated_tokens,
            template_name: meta.name.clone(),
            template_version: meta.version.clone(),
        })
    }

    /// 快速渲染（使用默认值）
    pub fn render_default(&self, template_type: PromptTemplateType) -> Result<String, AppError> {
        let result = self.render(RenderPromptRequest {
            template_type,
            variables: HashMap::new(),
        })?;
        Ok(result.content)
    }

    /// 计算字节数
    pub fn byte_count(&self, content: &str) -> usize {
        content.len()
    }

    /// 估算 token 数
    pub fn estimate_tokens(&self, content: &str) -> usize {
        // 简单估算：中文约 1.5 字符/token，英文约 4 字符/token
        let chinese_chars = content.chars().filter(|c| *c as u32 > 0x4e00).count();
        let other_chars = content.len() - chinese_chars;
        (chinese_chars as f64 / 1.5 + other_chars as f64 / 4.0).ceil() as usize
    }
}

/// AGENTS.md 发现与层级指令管理
pub struct AgentsMdManager {
    /// 项目根标记文件名
    project_root_markers: Vec<String>,
    /// 备用文件名
    fallback_filenames: Vec<String>,
    /// 最大字节预算
    project_doc_max_bytes: usize,
}

impl Default for AgentsMdManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentsMdManager {
    pub fn new() -> Self {
        Self {
            project_root_markers: vec![
                ".git".into(),
                "Cargo.toml".into(),
                "package.json".into(),
                "AGENTS.md".into(),
            ],
            fallback_filenames: vec![
                "AGENTS.md".into(),
                "AGENTS.MD".into(),
                "agents.md".into(),
                "CLAUDE.md".into(),
                "CLAUDE.MD".into(),
                "claude.md".into(),
            ],
            project_doc_max_bytes: 500_000, // 默认 500KB
        }
    }

    /// 设置项目根标记
    pub fn set_project_root_markers(&mut self, markers: Vec<String>) {
        self.project_root_markers = markers;
    }

    /// 设置备用文件名
    pub fn set_fallback_filenames(&mut self, filenames: Vec<String>) {
        self.fallback_filenames = filenames;
    }

    /// 设置最大字节预算
    pub fn set_max_bytes(&mut self, max_bytes: usize) {
        self.project_doc_max_bytes = max_bytes;
    }

    /// 获取当前最大字节预算
    pub fn max_bytes(&self) -> usize {
        self.project_doc_max_bytes
    }

    /// 发现项目根目录
    fn find_project_root(&self, start_dir: &std::path::Path) -> Option<std::path::PathBuf> {
        let mut current = start_dir.to_path_buf();

        loop {
            for marker in &self.project_root_markers {
                if current.join(marker).exists() {
                    return Some(current);
                }
            }
            if !current.pop() {
                break;
            }
        }
        None
    }

    /// 从目录发现 AGENTS.md 文件
    fn find_agents_files_in_dir(
        &self,
        dir: &std::path::Path,
    ) -> Vec<std::path::PathBuf> {
        let mut results = Vec::new();
        for filename in &self.fallback_filenames {
            let path = dir.join(filename);
            if path.exists() && path.is_file() {
                results.push(path);
            }
        }
        results
    }

    /// 从项目根到 CWD 逐层收集 AGENTS.md
    pub fn collect_agents_files(
        &self,
        cwd: &std::path::Path,
        project_root: Option<&std::path::Path>,
    ) -> Result<Vec<AgentsMdFile>, AppError> {
        let project_root = match project_root {
            Some(p) => p.to_path_buf(),
            None => self
                .find_project_root(cwd)
                .unwrap_or_else(|| cwd.to_path_buf()),
        };

        let mut files = Vec::new();

        // 从项目根到 CWD 逐层收集
        if let Ok(cwd_canonical) = cwd.canonicalize() {
            let mut current = cwd_canonical.clone();
            loop {
                let depth = current
                    .strip_prefix(&project_root)
                    .map(|p| p.components().count())
                    .unwrap_or(0);

                for path in self.find_agents_files_in_dir(&current) {
                    // 避免重复（同一文件已从更深的目录收集）
                    if files.iter().any(|f: &AgentsMdFile| f.path == path.to_string_lossy()) {
                        continue;
                    }

                    let content = std::fs::read_to_string(&path).map_err(|e| {
                        AppError::FileSystem(std::io::Error::new(std::io::ErrorKind::NotFound, e))
                    })?;

                    files.push(AgentsMdFile {
                        path: path.to_string_lossy().to_string(),
                        content,
                        depth,
                        scope_root: current.to_string_lossy().to_string(),
                    });
                }

                // 到达项目根就停止
                if current == project_root {
                    break;
                }
                if !current.pop() {
                    break;
                }
            }

            // 反转：从根到深（优先级递增）
            files.reverse();
        }

        Ok(files)
    }

    /// 组装层级指令
    pub fn assemble_hierarchical_instructions(
        &self,
        agents_files: &[AgentsMdFile],
        user_instructions: Option<&str>,
        max_bytes: Option<usize>,
    ) -> HierarchicalInstructions {
        let max_bytes = max_bytes.unwrap_or(self.project_doc_max_bytes);
        let mut parts: Vec<String> = Vec::new();
        let mut sources: Vec<String> = Vec::new();
        let mut total_bytes = 0usize;
        let mut truncated = false;

        // 用户指令优先
        if let Some(instructions) = user_instructions {
            let text = format!("## 用户指令\n{}", instructions);
            let bytes = text.len();
            if total_bytes + bytes <= max_bytes {
                parts.push(text);
                sources.push("user_instructions".into());
                total_bytes += bytes;
            } else {
                truncated = true;
            }
        }

        // AGENTS.md 按深度从浅到深（优先级递增）
        // 深层文件覆盖浅层文件的指令
        for file in agents_files {
            let text = format!(
                "## AGENTS.md (scope: {})\n{}",
                file.scope_root, file.content
            );
            let bytes = text.len();
            if total_bytes + bytes <= max_bytes {
                parts.push(text);
                sources.push(file.path.clone());
                total_bytes += bytes;
            } else {
                truncated = true;
                break;
            }
        }

        let assembled = parts.join("\n\n---\n\n");

        HierarchicalInstructions {
            sources,
            assembled,
            total_bytes,
            truncated,
        }
    }

    /// 获取指令源列表
    pub fn instruction_sources(&self, agents_files: &[AgentsMdFile]) -> Vec<String> {
        agents_files.iter().map(|f| f.path.clone()).collect()
    }
}

/// 系统 Prompt 组装器
pub struct SystemPromptAssembler {
    prompt_manager: PromptManager,
    agents_manager: AgentsMdManager,
}

impl Default for SystemPromptAssembler {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemPromptAssembler {
    pub fn new() -> Self {
        Self {
            prompt_manager: PromptManager::new(),
            agents_manager: AgentsMdManager::new(),
        }
    }

    pub fn prompt_manager(&self) -> &PromptManager {
        &self.prompt_manager
    }

    pub fn prompt_manager_mut(&mut self) -> &mut PromptManager {
        &mut self.prompt_manager
    }

    pub fn agents_manager(&self) -> &AgentsMdManager {
        &self.agents_manager
    }

    pub fn agents_manager_mut(&mut self) -> &mut AgentsMdManager {
        &mut self.agents_manager
    }

    /// 组装完整的系统 Prompt
    pub fn assemble(
        &self,
        request: AssembleSystemPromptRequest,
    ) -> Result<AssembleSystemPromptResult, AppError> {
        let mut parts: Vec<PromptPart> = Vec::new();
        let mut total_bytes = 0usize;
        let max_bytes = request.max_bytes.unwrap_or(1_000_000); // 默认 1MB

        // 1. 渲染系统主提示词
        let mut system_vars = HashMap::new();
        if let Some(ref root) = request.project_root {
            system_vars.insert("workspace_root".into(), root.clone());
        }
        if let Some(ref cwd) = request.cwd {
            if !system_vars.contains_key("workspace_root") {
                system_vars.insert("workspace_root".into(), cwd.clone());
            }
        }
        system_vars.insert(
            "current_date".into(),
            chrono::Local::now().format("%Y-%m-%d").to_string(),
        );

        let system_result = self.prompt_manager.render(RenderPromptRequest {
            template_type: PromptTemplateType::System,
            variables: system_vars,
        })?;

        let system_bytes = system_result.content.len();
        let system_content = if total_bytes + system_bytes <= max_bytes {
            system_result.content.clone()
        } else {
            // 截断
            let available = max_bytes.saturating_sub(total_bytes);
            system_result.content[..available.min(system_result.content.len())].to_string()
        };
        total_bytes += system_content.len();

        parts.push(PromptPart {
            name: "system_base".into(),
            source: "system.md".into(),
            content: system_content,
            bytes: system_bytes,
        });

        // 2. 注入技能使用指南
        if let Some(ref skill_names) = request.skill_names {
            if !skill_names.is_empty() {
                let skills_list = skill_names
                    .iter()
                    .map(|s| format!("- **{}**: 可用技能", s))
                    .collect::<Vec<_>>()
                    .join("\n");

                let mut skills_vars = HashMap::new();
                skills_vars.insert("available_skills".into(), skills_list);
                skills_vars.insert("skills_budget_pct".into(), "2".into());

                if let Ok(skills_result) = self.prompt_manager.render(RenderPromptRequest {
                    template_type: PromptTemplateType::SkillsHowTo,
                    variables: skills_vars,
                }) {
                    let bytes = skills_result.content.len();
                    if total_bytes + bytes <= max_bytes {
                        parts.push(PromptPart {
                            name: "skills_how_to".into(),
                            source: "skills_how_to.md".into(),
                            content: skills_result.content,
                            bytes,
                        });
                        total_bytes += bytes;
                    }
                }
            }
        }

        // 3. 收集并注入 AGENTS.md 层级指令
        let cwd_path = request
            .cwd
            .as_ref()
            .map(|s| std::path::Path::new(s))
            .unwrap_or_else(|| std::path::Path::new("."));
        let project_root = request.project_root.as_ref().map(|s| std::path::Path::new(s));

        if let Ok(agents_files) = self
            .agents_manager
            .collect_agents_files(cwd_path, project_root)
        {
            if !agents_files.is_empty() {
                let remaining = max_bytes.saturating_sub(total_bytes);
                let instructions = self.agents_manager.assemble_hierarchical_instructions(
                    &agents_files,
                    request.user_instructions.as_deref(),
                    Some(remaining),
                );

                if !instructions.assembled.is_empty() {
                    let bytes = instructions.assembled.len();
                    parts.push(PromptPart {
                        name: "agents_instructions".into(),
                        source: instructions
                            .sources
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "AGENTS.md".into()),
                        content: instructions.assembled,
                        bytes,
                    });
                    total_bytes += bytes;
                }
            }
        }

        // 4. 子 Agent 指令
        if request.include_child_agent_instructions.unwrap_or(false) {
            let child_instructions = "\n\n## 子 Agent 指令\n当使用子 Agent 处理任务时，请遵循子 Agent 通信协议。";
            let bytes = child_instructions.len();
            if total_bytes + bytes <= max_bytes {
                parts.push(PromptPart {
                    name: "child_agent_instructions".into(),
                    source: "system".into(),
                    content: child_instructions.into(),
                    bytes,
                });
                total_bytes += bytes;
            }
        }

        // 组装最终 prompt
        let system_prompt = parts
            .iter()
            .map(|p| p.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

        let estimated_tokens = self.prompt_manager.estimate_tokens(&system_prompt);

        Ok(AssembleSystemPromptResult {
            system_prompt,
            parts,
            estimated_tokens,
            total_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_system_template() {
        let manager = PromptManager::new();
        let mut vars = HashMap::new();
        vars.insert("workspace_root".into(), "/test/workspace".into());
        vars.insert("git_branch".into(), "main".into());
        vars.insert("current_date".into(), "2026-06-06".into());

        let result = manager
            .render(RenderPromptRequest {
                template_type: PromptTemplateType::System,
                variables: vars,
            })
            .unwrap();

        assert!(result.content.contains("Yuan Code"));
        assert!(result.content.contains("/test/workspace"));
        assert!(result.content.contains("main"));
        assert!(result.estimated_tokens > 0);
    }

    #[test]
    fn test_render_review_template() {
        let manager = PromptManager::new();
        let mut vars = HashMap::new();
        vars.insert("review_target".into(), "uncommitted changes".into());

        let result = manager
            .render(RenderPromptRequest {
                template_type: PromptTemplateType::Review,
                variables: vars,
            })
            .unwrap();

        assert!(result.content.contains("代码审查"));
        assert!(result.content.contains("uncommitted changes"));
    }

    #[test]
    fn test_missing_required_variable() {
        let manager = PromptManager::new();
        let result = manager.render(RenderPromptRequest {
            template_type: PromptTemplateType::Review,
            variables: HashMap::new(),
        });

        assert!(result.is_err());
        if let Err(AppError::Validation(msg)) = result {
            assert!(msg.contains("review_target"));
        } else {
            panic!("expected Validation error");
        }
    }

    #[test]
    fn test_estimate_tokens() {
        let manager = PromptManager::new();
        let english = "hello world this is a test";
        let chinese = "你好世界这是一个测试";

        let en_tokens = manager.estimate_tokens(english);
        let cn_tokens = manager.estimate_tokens(chinese);

        assert!(en_tokens > 0);
        assert!(cn_tokens > 0);
        // 中文 token 数应该相对更多（因为中文字符更密集）
        assert!(cn_tokens >= en_tokens / 2);
    }

    #[test]
    fn test_list_templates() {
        let manager = PromptManager::new();
        let templates = manager.list_templates();
        assert!(!templates.is_empty());
        assert!(templates.iter().any(|t| t.name == "system"));
        assert!(templates.iter().any(|t| t.name == "review"));
    }

    #[test]
    fn test_agents_md_collection() {
        let agents = AgentsMdManager::new();
        let cwd = std::path::Path::new(".");

        // 在当前项目中应该有 AGENTS.md 或类似文件
        let result = agents.collect_agents_files(cwd, None);
        // 可能找到也可能找不到，取决于当前目录
        assert!(result.is_ok());
    }

    #[test]
    fn test_system_prompt_assembly() {
        let assembler = SystemPromptAssembler::new();
        let result = assembler.assemble(AssembleSystemPromptRequest {
            project_root: Some("/test/project".into()),
            cwd: Some("/test/project".into()),
            user_instructions: Some("请使用中文回复".into()),
            skill_names: Some(vec!["code-review".into(), "test-runner".into()]),
            max_bytes: Some(100_000),
            include_child_agent_instructions: Some(true),
        });

        assert!(result.is_ok());
        let assembled = result.unwrap();
        assert!(assembled.system_prompt.contains("Yuan Code"));
        assert!(assembled.estimated_tokens > 0);
        assert!(!assembled.parts.is_empty());
    }
}