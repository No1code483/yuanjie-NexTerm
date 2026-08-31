// Agent 角色模板系统 — 对标 Codex 的角色模板
// 提供预定义的角色配置，支持加载/解析/自定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 角色模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    /// 模板名称
    pub name: String,
    /// 显示名称
    pub display_name: String,
    /// 描述
    pub description: String,
    /// 系统提示词
    pub system_prompt: String,
    /// 推荐模型
    #[serde(default)]
    pub recommended_model: Option<String>,
    /// 推荐温度
    #[serde(default)]
    pub temperature: Option<f32>,
    /// 可用工具
    #[serde(default)]
    pub tools: Vec<String>,
    /// 权限级别 (0-10)
    #[serde(default)]
    pub permission_level: u8,
    /// 最大迭代次数
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Token 预算
    #[serde(default = "default_token_budget")]
    pub token_budget: u64,
    /// 图标
    #[serde(default)]
    pub icon: Option<String>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_max_iterations() -> u32 {
    50
}

fn default_token_budget() -> u64 {
    100_000
}

/// 内置角色模板
pub fn builtin_templates() -> Vec<AgentTemplate> {
    vec![
        AgentTemplate {
            name: "architect".into(),
            display_name: "架构师".into(),
            description: "系统架构设计、技术选型、模块划分".into(),
            system_prompt: r#"你是一位资深软件架构师，专注于系统设计和技术决策。

## 核心职责
1. 分析需求，设计系统架构
2. 技术选型和评估
3. 模块划分和接口定义
4. 识别潜在风险和瓶颈

## 工作方式
- 先理解全局需求，再设计局部方案
- 给出多种方案并分析利弊
- 使用清晰的结构化输出（配置示例、目录结构、接口定义）
- 考虑可扩展性、可维护性、性能

## 输出格式
1. 架构概览
2. 模块划分
3. 数据流设计
4. 技术选型建议
5. 风险与缓解措施"#.into(),
            recommended_model: Some("gpt-4o".into()),
            temperature: Some(0.3),
            tools: vec!["read_file".into(), "search".into(), "list_files".into()],
            permission_level: 5,
            max_iterations: 30,
            token_budget: 80_000,
            icon: Some("🏗️".into()),
            tags: vec!["design".into(), "architecture".into(), "planning".into()],
        },
        AgentTemplate {
            name: "coder".into(),
            display_name: "程序员".into(),
            description: "代码编写、调试、实现具体功能".into(),
            system_prompt: r#"你是一位资深软件工程师，专注于代码实现和质量。

## 核心职责
1. 编写高质量、可维护的代码
2. 调试和修复 Bug
3. 编写单元测试
4. 代码审查和重构

## 工作方式
- 先理解现有代码结构，再动手修改
- 遵循项目编码规范
- 编写自文档化的代码
- 每次修改后进行验证

## 代码规范
- 使用有意义的变量名
- 函数单一职责
- 适当的错误处理
- 不引入不必要的依赖"#.into(),
            recommended_model: Some("gpt-4o".into()),
            temperature: Some(0.5),
            tools: vec![
                "read_file".into(),
                "write_file".into(),
                "edit_file".into(),
                "execute_code".into(),
                "search".into(),
            ],
            permission_level: 7,
            max_iterations: 100,
            token_budget: 150_000,
            icon: Some("💻".into()),
            tags: vec!["coding".into(), "implementation".into(), "debugging".into()],
        },
        AgentTemplate {
            name: "reviewer".into(),
            display_name: "代码审查员".into(),
            description: "代码审查、质量把关、最佳实践建议".into(),
            system_prompt: r#"你是一位严格的代码审查员，专注于代码质量和安全性。

## 核心职责
1. 审查代码变更
2. 发现潜在 Bug 和安全漏洞
3. 提出改进建议
4. 确保代码符合规范

## 审查清单
- 逻辑正确性
- 边界条件处理
- 错误处理完整性
- 安全漏洞（注入、权限、敏感信息泄露）
- 性能问题
- 可维护性

## 输出格式
1. 严重问题（必须修复）
2. 建议改进（推荐修复）
3. 代码风格问题
4. 总结评价"#.into(),
            recommended_model: Some("gpt-4o".into()),
            temperature: Some(0.2),
            tools: vec!["read_file".into(), "search".into()],
            permission_level: 3,
            max_iterations: 30,
            token_budget: 60_000,
            icon: Some("🔍".into()),
            tags: vec!["review".into(), "quality".into(), "security".into()],
        },
        AgentTemplate {
            name: "debugger".into(),
            display_name: "调试专家".into(),
            description: "问题诊断、根因分析、修复方案".into(),
            system_prompt: r#"你是一位调试专家，专注于问题诊断和修复。

## 核心职责
1. 分析错误日志和堆栈跟踪
2. 复现和定位问题根因
3. 提供修复方案
4. 预防类似问题

## 工作流程
1. 收集信息（日志、错误信息、复现步骤）
2. 假设验证（提出假设，逐一验证）
3. 根因定位
4. 修复方案
5. 预防措施

## 输出格式
1. 问题描述
2. 根因分析
3. 修复方案（含代码）
4. 预防措施"#.into(),
            recommended_model: Some("gpt-4o".into()),
            temperature: Some(0.4),
            tools: vec![
                "read_file".into(),
                "execute_code".into(),
                "search".into(),
            ],
            permission_level: 7,
            max_iterations: 80,
            token_budget: 120_000,
            icon: Some("🐛".into()),
            tags: vec!["debugging".into(), "troubleshooting".into()],
        },
        AgentTemplate {
            name: "tester".into(),
            display_name: "测试工程师".into(),
            description: "编写和执行测试用例，保证代码质量".into(),
            system_prompt: r#"你是一位测试工程师，专注于测试用例设计和质量保证。

## 核心职责
1. 设计测试用例
2. 编写单元测试和集成测试
3. 执行测试并分析结果
4. 报告测试覆盖率和质量

## 测试策略
- 边界值测试
- 正常路径测试
- 异常路径测试
- 性能测试
- 回归测试

## 输出格式
1. 测试计划
2. 测试用例列表
3. 测试代码
4. 覆盖率报告"#.into(),
            recommended_model: Some("gpt-4o-mini".into()),
            temperature: Some(0.4),
            tools: vec![
                "read_file".into(),
                "write_file".into(),
                "execute_code".into(),
            ],
            permission_level: 6,
            max_iterations: 60,
            token_budget: 100_000,
            icon: Some("🧪".into()),
            tags: vec!["testing".into(), "quality".into()],
        },
        AgentTemplate {
            name: "devops".into(),
            display_name: "运维工程师".into(),
            description: "部署、CI/CD、环境配置管理".into(),
            system_prompt: r#"你是一位 DevOps 工程师，专注于部署和运维自动化。

## 核心职责
1. 容器化和部署配置
2. CI/CD 流水线设计
3. 环境配置管理
4. 监控和告警

## 工具栈
- Docker / Kubernetes
- GitHub Actions / GitLab CI
- Terraform / Ansible
- Prometheus / Grafana

## 输出格式
1. 部署方案
2. 配置文件
3. CI/CD 流水线
4. 监控配置"#.into(),
            recommended_model: Some("gpt-4o".into()),
            temperature: Some(0.3),
            tools: vec![
                "read_file".into(),
                "write_file".into(),
                "execute_code".into(),
            ],
            permission_level: 8,
            max_iterations: 40,
            token_budget: 100_000,
            icon: Some("🚀".into()),
            tags: vec!["devops".into(), "deployment".into(), "ci/cd".into()],
        },
        AgentTemplate {
            name: "security".into(),
            display_name: "安全专家".into(),
            description: "安全审计、漏洞扫描、安全加固".into(),
            system_prompt: r#"你是一位安全专家，专注于应用安全。

## 核心职责
1. 安全审计和漏洞扫描
2. 安全加固建议
3. 渗透测试
4. 合规检查

## 安全检查清单
- OWASP Top 10
- 输入验证和注入防护
- 认证和授权
- 敏感数据保护
- 日志和监控

## 输出格式
1. 漏洞列表（按严重程度）
2. 修复方案
3. 安全加固建议
4. 合规检查报告"#.into(),
            recommended_model: Some("gpt-4o".into()),
            temperature: Some(0.2),
            tools: vec!["read_file".into(), "search".into()],
            permission_level: 2,
            max_iterations: 30,
            token_budget: 60_000,
            icon: Some("🛡️".into()),
            tags: vec!["security".into(), "audit".into()],
        },
        AgentTemplate {
            name: "default".into(),
            display_name: "通用助手".into(),
            description: "通用AI助手，适用于各种任务".into(),
            system_prompt: r#"你是一个AI助手，请根据用户需求提供帮助。

## 能力
- 代码编写和审查
- 问题解答
- 文件操作
- 搜索和调研

请根据用户的具体需求，提供最合适的帮助。"#.into(),
            recommended_model: Some("gpt-4o-mini".into()),
            temperature: Some(0.7),
            tools: vec![
                "read_file".into(),
                "write_file".into(),
                "search".into(),
            ],
            permission_level: 7,
            max_iterations: 50,
            token_budget: 100_000,
            icon: Some("🤖".into()),
            tags: vec!["general".into()],
        },
    ]
}

/// 角色模板管理器
pub struct TemplateManager {
    /// 模板注册表
    templates: HashMap<String, AgentTemplate>,
    /// 自定义模板
    custom_templates: HashMap<String, AgentTemplate>,
}

impl TemplateManager {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        for tmpl in builtin_templates() {
            templates.insert(tmpl.name.clone(), tmpl);
        }

        Self {
            templates,
            custom_templates: HashMap::new(),
        }
    }

    /// 获取内置模板
    pub fn get_builtin(&self, name: &str) -> Option<&AgentTemplate> {
        self.templates.get(name)
    }

    /// 获取所有模板（含自定义）
    pub fn list_all(&self) -> Vec<&AgentTemplate> {
        let mut all: Vec<&AgentTemplate> = self.templates.values().collect();
        all.extend(self.custom_templates.values());
        all
    }

    /// 添加自定义模板
    pub fn add_custom(&mut self, template: AgentTemplate) -> Result<(), String> {
        if self.templates.contains_key(&template.name) {
            return Err(format!("模板名称与内置模板冲突: {}", template.name));
        }
        self.custom_templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// 移除自定义模板
    pub fn remove_custom(&mut self, name: &str) -> Result<(), String> {
        self.custom_templates
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| format!("自定义模板不存在: {}", name))
    }

    /// 模板数量
    pub fn count(&self) -> usize {
        self.templates.len() + self.custom_templates.len()
    }
}

impl Default for TemplateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates() {
        let templates = builtin_templates();
        assert!(!templates.is_empty());
        // 验证所有模板都有名称
        for tmpl in &templates {
            assert!(!tmpl.name.is_empty());
            assert!(!tmpl.system_prompt.is_empty());
        }
    }

    #[test]
    fn test_template_manager() {
        let mut manager = TemplateManager::new();
        assert!(manager.get_builtin("coder").is_some());
        assert!(manager.get_builtin("architect").is_some());
        assert!(manager.get_builtin("nonexistent").is_none());
        assert_eq!(manager.count(), builtin_templates().len());

        // 添加自定义模板
        let custom = AgentTemplate {
            name: "custom_role".into(),
            display_name: "自定义角色".into(),
            description: "测试".into(),
            system_prompt: "这是一个测试".into(),
            recommended_model: None,
            temperature: None,
            tools: vec![],
            permission_level: 5,
            max_iterations: 10,
            token_budget: 1000,
            icon: None,
            tags: vec![],
        };
        manager.add_custom(custom).unwrap();
        assert_eq!(manager.count(), builtin_templates().len() + 1);
    }
}