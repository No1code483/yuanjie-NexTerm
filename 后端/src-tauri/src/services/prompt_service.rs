use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::prompt::{
    AgentsMdFile, AssembleSystemPromptRequest, AssembleSystemPromptResult,
    HierarchicalInstructions, PromptTemplateMeta, PromptTemplateType, RenderPromptRequest,
    RenderPromptResult,
};
use crate::prompts::SystemPromptAssembler;

/// 提示词服务
pub struct PromptService {
    assembler: Arc<RwLock<SystemPromptAssembler>>,
}

impl PromptService {
    pub fn new() -> Self {
        Self {
            assembler: Arc::new(RwLock::new(SystemPromptAssembler::new())),
        }
    }

    // ===== 模板引擎 (2.1) =====

    /// 列出所有模板元数据
    pub async fn list_templates(&self) -> Vec<PromptTemplateMeta> {
        let a = self.assembler.read().await;
        a.prompt_manager().list_templates()
    }

    /// 获取指定模板元数据
    pub async fn get_template(&self, template_type: &PromptTemplateType) -> Option<PromptTemplateMeta> {
        let a = self.assembler.read().await;
        a.prompt_manager().get_template_meta(template_type).cloned()
    }

    /// 渲染模板
    pub async fn render_template(
        &self,
        request: RenderPromptRequest,
    ) -> Result<RenderPromptResult, AppError> {
        let a = self.assembler.read().await;
        a.prompt_manager().render(request)
    }

    /// 设置自定义模板
    pub async fn set_custom_template(
        &self,
        template_type: PromptTemplateType,
        content: String,
    ) {
        let mut a = self.assembler.write().await;
        a.prompt_manager_mut().set_custom_template(template_type, content);
    }

    /// 移除自定义模板
    pub async fn remove_custom_template(&self, template_type: &PromptTemplateType) {
        let mut a = self.assembler.write().await;
        a.prompt_manager_mut().remove_custom_template(template_type);
    }

    /// 设置变量默认值
    pub async fn set_variable_default(&self, name: &str, value: &str) {
        let mut a = self.assembler.write().await;
        a.prompt_manager_mut().set_variable_default(name, value);
    }

    // ===== AGENTS.md 发现 (2.2) =====

    /// 发现 AGENTS.md 文件
    pub async fn discover_agents_md(
        &self,
        cwd: Option<String>,
        project_root: Option<String>,
    ) -> Result<Vec<AgentsMdFile>, AppError> {
        let a = self.assembler.read().await;
        let cwd_path = cwd
            .map(|p| std::path::PathBuf::from(p))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
        let project_root_path = project_root.map(|p| std::path::PathBuf::from(p));
        a.agents_manager().collect_agents_files(
            &cwd_path,
            project_root_path.as_deref(),
        )
    }

    /// 获取指令源列表
    pub async fn get_instruction_sources(
        &self,
        cwd: Option<String>,
        project_root: Option<String>,
    ) -> Result<Vec<String>, AppError> {
        let a = self.assembler.read().await;
        let cwd_path = cwd
            .map(|p| std::path::PathBuf::from(p))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
        let project_root_path = project_root.map(|p| std::path::PathBuf::from(p));
        let agents_files = a.agents_manager().collect_agents_files(
            &cwd_path,
            project_root_path.as_deref(),
        )?;
        Ok(a.agents_manager().instruction_sources(&agents_files))
    }

    // ===== 层级指令组装 (2.3) =====

    /// 组装层级指令
    pub async fn assemble_instructions(
        &self,
        agents_files: &[AgentsMdFile],
        user_instructions: Option<&str>,
        max_bytes: Option<usize>,
    ) -> HierarchicalInstructions {
        let a = self.assembler.read().await;
        a.agents_manager().assemble_hierarchical_instructions(
            agents_files,
            user_instructions,
            max_bytes,
        )
    }

    // ===== 字节预算 (2.4) =====

    /// 设置最大字节预算
    pub async fn set_project_doc_max_bytes(&self, max_bytes: usize) {
        let mut a = self.assembler.write().await;
        a.agents_manager_mut().set_max_bytes(max_bytes);
    }

    /// 获取当前最大字节预算
    pub async fn get_project_doc_max_bytes(&self) -> usize {
        let a = self.assembler.read().await;
        a.agents_manager().max_bytes()
    }

    // ===== 系统 Prompt 组装 (包含 2.5 子 Agent 指令) =====

    /// 组装完整系统 Prompt
    pub async fn assemble_system_prompt(
        &self,
        request: AssembleSystemPromptRequest,
    ) -> Result<AssembleSystemPromptResult, AppError> {
        let a = self.assembler.read().await;
        a.assemble(request)
    }
}