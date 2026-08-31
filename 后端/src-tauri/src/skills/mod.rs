use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use crate::error::app_error::AppError;
use crate::models::skill::{
    AutoDiscoverRequest, DetectImplicitRequest, InvocationType, MatchType, SkillAutoDiscoveryResult,
    SkillInterface, SkillInvocation, SkillLoadError, SkillLoadOutcome, SkillMatchResult,
    SkillMetadata, SkillPolicy, SkillRecommendation, SkillRegistration, SkillRenderConfig,
    SkillScope, SkillScopeCount, SkillTriggerRequest,
};

pub mod budget;
pub mod builtin;
pub mod skill_creator;
pub mod telemetry;

use self::budget::SkillBudgetManager;
use self::telemetry::SkillTelemetry;

const SKILL_FILE: &str = "SKILL.md";
const SKILL_DIR: &str = "skills";

#[derive(Debug, Clone, Default)]
struct FrontmatterData {
    name: Option<String>,
    description: Option<String>,
    short_description: Option<String>,
    interface: Option<SkillInterface>,
    policy: Option<SkillPolicy>,
}

pub struct SkillsManager {
    loaded_skills: HashMap<String, SkillMetadata>,
    invocations: HashSet<String>,
    invocation_history: Vec<SkillInvocation>,
    scan_roots: Vec<PathBuf>,
    project_files_cache: Option<Vec<String>>,
    budget_manager: SkillBudgetManager,
    telemetry: SkillTelemetry,
}

impl SkillsManager {
    pub fn new() -> Self {
        Self {
            loaded_skills: HashMap::new(),
            invocations: HashSet::new(),
            invocation_history: Vec::new(),
            scan_roots: Vec::new(),
            project_files_cache: None,
            budget_manager: SkillBudgetManager::new(100_000),
            telemetry: SkillTelemetry::new(),
        }
    }

    pub fn add_scan_root(&mut self, path: &Path) {
        self.scan_roots.push(path.to_path_buf());
    }

    pub fn set_project_files(&mut self, files: Vec<String>) {
        self.project_files_cache = Some(files);
    }

    pub async fn load_skills(&mut self) -> SkillLoadOutcome {
        let mut skills = Vec::new();
        let mut errors = Vec::new();

        for root in &self.scan_roots {
            let skills_dir = root.join(SKILL_DIR);
            if !skills_dir.exists() {
                continue;
            }

            let scope = self.detect_scope(root);
            match self.scan_skills_directory(&skills_dir, scope) {
                Ok(mut found) => skills.append(&mut found),
                Err(errs) => errors.extend(errs),
            }
        }

        let system_skills = self.load_system_skills();
        for skill in system_skills {
            skills.push(skill);
        }

        let mut by_scope = SkillScopeCount {
            user: 0,
            project: 0,
            system: 0,
            plugin: 0,
        };
        for skill in &skills {
            match skill.scope {
                SkillScope::User => by_scope.user += 1,
                SkillScope::Project => by_scope.project += 1,
                SkillScope::System => by_scope.system += 1,
                SkillScope::Plugin => by_scope.plugin += 1,
            }
        }

        let total = skills.len();
        let enabled = skills.iter().filter(|s| s.enabled).count();

        // 预算管理：按优先级分配上下文预算
        let budget_report = self.budget_manager.allocate_budget(&mut skills);
        for warning in &budget_report.warnings {
            tracing::warn!("{}", warning);
        }

        for skill in &skills {
            self.telemetry.record_load(&skill.name, true);
            self.loaded_skills
                .insert(skill.name.clone(), skill.clone());
        }

        // 记录预算不足被跳过的技能
        // (budget_report 已记录被跳过的技能数)

        SkillLoadOutcome {
            skills,
            errors,
            total_count: total,
            enabled_count: enabled,
            by_scope,
        }
    }

    fn detect_scope(&self, root: &Path) -> SkillScope {
        let path_str = root.to_string_lossy();
        if path_str.contains(".codex") || path_str.contains(".nexterm") {
            SkillScope::System
        } else if path_str.contains(".config") || path_str.contains("AppData") {
            SkillScope::User
        } else {
            SkillScope::Project
        }
    }

    fn scan_skills_directory(
        &self,
        skills_dir: &Path,
        scope: SkillScope,
    ) -> Result<Vec<SkillMetadata>, Vec<SkillLoadError>> {
        let mut skills = Vec::new();
        let mut errors = Vec::new();

        let entries = match fs::read_dir(skills_dir) {
            Ok(e) => e,
            Err(e) => {
                errors.push(SkillLoadError {
                    path: skills_dir.to_string_lossy().to_string(),
                    message: format!("无法读取目录: {}", e),
                });
                return Err(errors);
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let skill_file = path.join(SKILL_FILE);
                if skill_file.exists() {
                    match self.parse_skill_file(&skill_file, &path, scope.clone()) {
                        Ok(skill) => skills.push(skill),
                        Err(e) => errors.push(e),
                    }
                } else {
                    match self.scan_skills_directory(&path, scope.clone()) {
                        Ok(mut nested) => skills.append(&mut nested),
                        Err(nested_errors) => errors.extend(nested_errors),
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(skills)
        } else {
            Err(errors)
        }
    }

    fn parse_skill_file(
        &self,
        skill_file: &Path,
        skill_dir: &Path,
        scope: SkillScope,
    ) -> Result<SkillMetadata, SkillLoadError> {
        let content = fs::read_to_string(skill_file).map_err(|e| SkillLoadError {
            path: skill_file.to_string_lossy().to_string(),
            message: format!("无法读取SKILL.md: {}", e),
        })?;

        let metadata = fs::metadata(skill_file).ok();
        let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        let frontmatter = self.parse_frontmatter(&content);

        let created_at = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string());

        let name = frontmatter
            .name
            .or_else(|| {
                skill_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
            })
            .unwrap_or_else(|| "unknown_skill".to_string());

        let description = frontmatter
            .description
            .unwrap_or_else(|| "No description provided".to_string());

        Ok(SkillMetadata {
            name,
            description,
            short_description: frontmatter.short_description,
            interface: frontmatter.interface,
            dependencies: None,
            policy: frontmatter.policy,
            path: skill_file.to_string_lossy().to_string(),
            scope,
            plugin_id: None,
            enabled: true,
            file_size,
            created_at,
        })
    }

    fn parse_frontmatter(&self, content: &str) -> FrontmatterData {
        let mut data = FrontmatterData::default();

        let lines: Vec<&str> = content.lines().collect();
        if lines.len() < 3 {
            return data;
        }

        if lines[0].trim() != "---" {
            let first_line = lines[0].trim_start_matches('#').trim();
            data.name = Some(first_line.to_string());
            if lines.len() > 1 && !lines[1].is_empty() {
                data.description = Some(lines[1].trim().to_string());
            }
            return data;
        }

        let mut in_frontmatter = false;
        let mut frontmatter_lines = Vec::new();

        for line in &lines {
            let trimmed = line.trim();
            if trimmed == "---" {
                if !in_frontmatter {
                    in_frontmatter = true;
                    continue;
                } else {
                    break;
                }
            }
            if in_frontmatter {
                frontmatter_lines.push(trimmed);
            }
        }

        for line in frontmatter_lines {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim().trim_matches('"').trim_matches('\'');

                match key.as_str() {
                    "name" => data.name = Some(value.to_string()),
                    "description" => data.description = Some(value.to_string()),
                    "short-description" | "short_description" => {
                        data.short_description = Some(value.to_string())
                    }
                    "allow_implicit_invocation" => {
                        let policy = data.policy.get_or_insert(SkillPolicy {
                            allow_implicit_invocation: None,
                            auto_discover: None,
                            trigger_patterns: None,
                            file_patterns: None,
                            priority: None,
                        });
                        policy.allow_implicit_invocation =
                            Some(value.parse().unwrap_or(true));
                    }
                    "trigger" => {
                        let policy = data.policy.get_or_insert(SkillPolicy {
                            allow_implicit_invocation: None,
                            auto_discover: None,
                            trigger_patterns: None,
                            file_patterns: None,
                            priority: None,
                        });
                        policy
                            .trigger_patterns
                            .get_or_insert_with(Vec::new)
                            .push(value.to_string());
                    }
                    _ => {}
                }
            }
        }

        data
    }

    fn load_system_skills(&self) -> Vec<SkillMetadata> {
        vec![
            SkillMetadata {
                name: "code-review".to_string(),
                description: "审查代码变更，检查潜在问题、安全漏洞和最佳实践违规".to_string(),
                short_description: Some("代码审查".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["review".to_string(), "审查".to_string(), "PR".to_string()]),
                    file_patterns: Some(vec!["*.rs".to_string(), "*.ts".to_string(), "*.py".to_string()]),
                    priority: Some(10),
                }),
                path: "[system]/code-review".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "test-writer".to_string(),
                description: "为代码自动生成单元测试和集成测试，覆盖边界情况和错误路径".to_string(),
                short_description: Some("测试生成".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["test".to_string(), "测试".to_string(), "coverage".to_string()]),
                    file_patterns: Some(vec!["*.rs".to_string(), "*.ts".to_string(), "*.py".to_string(), "*.go".to_string()]),
                    priority: Some(9),
                }),
                path: "[system]/test-writer".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "refactor".to_string(),
                description: "重构现有代码，提取方法、内联变量、优化结构和可读性".to_string(),
                short_description: Some("代码重构".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["refactor".to_string(), "重构".to_string(), "clean".to_string()]),
                    file_patterns: None,
                    priority: Some(8),
                }),
                path: "[system]/refactor".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "debug".to_string(),
                description: "分析调试问题和错误日志，定位根因并提供修复建议".to_string(),
                short_description: Some("调试分析".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["debug".to_string(), "fix".to_string(), "调试".to_string(), "error".to_string()]),
                    file_patterns: None,
                    priority: Some(10),
                }),
                path: "[system]/debug".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "doc-writer".to_string(),
                description: "为代码和API编写文档、注释和README文件".to_string(),
                short_description: Some("文档编写".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["doc".to_string(), "文档".to_string(), "README".to_string()]),
                    file_patterns: None,
                    priority: Some(5),
                }),
                path: "[system]/doc-writer".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "security-audit".to_string(),
                description: "扫描代码中的安全漏洞，包括注入攻击、敏感数据泄露和不安全配置".to_string(),
                short_description: Some("安全审计".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["security".to_string(), "安全".to_string(), "audit".to_string(), "vulnerability".to_string()]),
                    file_patterns: None,
                    priority: Some(9),
                }),
                path: "[system]/security-audit".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "dependency-check".to_string(),
                description: "检查项目依赖项，识别过时版本、已知漏洞和许可证问题".to_string(),
                short_description: Some("依赖检查".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["dependency".to_string(), "依赖".to_string(), "update".to_string(), "upgrade".to_string()]),
                    file_patterns: Some(vec!["Cargo.toml".to_string(), "package.json".to_string(), "requirements.txt".to_string()]),
                    priority: Some(6),
                }),
                path: "[system]/dependency-check".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "git-helper".to_string(),
                description: "辅助Git操作，生成提交信息、解决合并冲突和管理分支".to_string(),
                short_description: Some("Git助手".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["git".to_string(), "commit".to_string(), "提交".to_string(), "merge".to_string(), "branch".to_string()]),
                    file_patterns: None,
                    priority: Some(7),
                }),
                path: "[system]/git-helper".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "skill-creator".to_string(),
                description: "元技能：交互式创建 SKILL.md 文件。支持验证名称、生成模板、写入技能目录。当用户说「创建技能」或「添加skill」时触发。".to_string(),
                short_description: Some("技能创建器".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["skill".to_string(), "create".to_string(), "技能".to_string(), "创建".to_string(), "add skill".to_string()]),
                    file_patterns: Some(vec!["SKILL.md".to_string()]),
                    priority: Some(10),
                }),
                path: "[system]/skill-creator".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
            SkillMetadata {
                name: "plan".to_string(),
                description: "任务规划与步骤追踪。创建和管理多步骤执行计划，追踪进度状态。当用户说「制定计划」或「拆分任务」时触发。".to_string(),
                short_description: Some("任务规划".to_string()),
                interface: None,
                dependencies: None,
                policy: Some(SkillPolicy {
                    allow_implicit_invocation: Some(true),
                    auto_discover: Some(true),
                    trigger_patterns: Some(vec!["plan".to_string(), "计划".to_string(), "任务".to_string(), "拆分".to_string(), "规划".to_string()]),
                    file_patterns: None,
                    priority: Some(9),
                }),
                path: "[system]/plan".to_string(),
                scope: SkillScope::System,
                plugin_id: None,
                enabled: true,
                file_size: 0,
                created_at: None,
            },
        ]
    }

    pub fn find_skill(&self, name: &str) -> Option<&SkillMetadata> {
        self.loaded_skills.get(name)
    }

    pub fn list_skills(&self, scope: Option<SkillScope>) -> Vec<&SkillMetadata> {
        let mut skills: Vec<&SkillMetadata> = self.loaded_skills.values().collect();
        if let Some(s) = scope {
            skills.retain(|sk| sk.scope == s);
        }
        skills.sort_by_key(|s| s.policy.as_ref().and_then(|p| p.priority).unwrap_or(0));
        skills.reverse();
        skills
    }

    /// 获取遥测统计数据
    pub fn invocation_stats(&self) -> telemetry::SkillInvocationStats {
        self.telemetry.invocation_stats()
    }

    /// 获取遥测事件列表
    pub fn telemetry_events(&self) -> Vec<telemetry::TelemetryEvent> {
        self.telemetry.events()
    }

    /// 记录技能完成事件
    pub fn record_completion(&self, skill_name: &str, duration_ms: u64, success: bool) {
        self.telemetry.record_completion(skill_name, duration_ms, success);
    }

    pub fn detect_implicit_invocation(
        &self,
        req: &DetectImplicitRequest,
    ) -> Vec<SkillMetadata> {
        let lower_cmd = req.command.to_lowercase();
        let mut matches = Vec::new();

        for skill in self.loaded_skills.values() {
            if !skill.enabled {
                continue;
            }

            let allow_implicit = skill
                .policy
                .as_ref()
                .and_then(|p| p.allow_implicit_invocation)
                .unwrap_or(true);
            if !allow_implicit {
                continue;
            }

            let name_lower = skill.name.to_lowercase();
            let desc_lower = if let Some(ref sd) = skill.short_description {
                sd.to_lowercase()
            } else {
                skill.description.to_lowercase()
            };

            if lower_cmd.contains(&name_lower) || lower_cmd.contains(&desc_lower) {
                matches.push(skill.clone());
                continue;
            }

            if let Some(ref policy) = skill.policy {
                if let Some(ref patterns) = policy.trigger_patterns {
                    for pattern in patterns.iter() {
                        let p_lower: String = pattern.to_lowercase();
                        if lower_cmd.contains(&p_lower) || name_lower.contains(&p_lower) {
                            matches.push(skill.clone());
                            break;
                        }
                    }
                }
            }
        }

        // 遥测：记录隐式触发
        for m in &matches {
            let invocation = SkillInvocation {
                skill_name: m.name.clone(),
                scope: m.scope.clone(),
                invocation_type: InvocationType::Implicit,
                trigger: Some(req.command.clone()),
                timestamp: 0,
                duration_ms: None,
                match_type: MatchType::Implicit,
                metadata: None,
            };
            self.telemetry.record_invocation(&invocation);
        }

        matches
    }

    pub fn register_skill(&mut self, registration: SkillRegistration) -> Result<(), AppError> {
        if self.loaded_skills.contains_key(&registration.name) {
            return Err(AppError::Conflict(format!(
                "技能 '{}' 已注册",
                registration.name
            )));
        }

        let skill = SkillMetadata {
            name: registration.name.clone(),
            description: String::new(),
            short_description: None,
            interface: None,
            dependencies: None,
            policy: None,
            path: registration.skill_file_path,
            scope: registration.scope,
            plugin_id: None,
            enabled: true,
            file_size: 0,
            created_at: None,
        };

        self.loaded_skills
            .insert(registration.name, skill);
        Ok(())
    }

    /// D1.8 安装市场技能（带完整元数据，含 policy + trigger_patterns）
    pub fn register_market_skill(&mut self, metadata: SkillMetadata) -> Result<(), AppError> {
        if self.loaded_skills.contains_key(&metadata.name) {
            return Err(AppError::Conflict(format!(
                "技能 '{}' 已安装",
                metadata.name
            )));
        }
        self.loaded_skills.insert(metadata.name.clone(), metadata);
        Ok(())
    }

    pub fn unregister_skill(&mut self, name: &str) -> Result<(), AppError> {
        self.loaded_skills
            .remove(name)
            .ok_or_else(|| AppError::NotFound)?;
        Ok(())
    }

    pub fn record_invocation(&mut self, req: &SkillTriggerRequest) -> SkillInvocation {
        let scope = self
            .loaded_skills
            .get(&req.skill_name)
            .map(|s| s.scope.clone())
            .unwrap_or(SkillScope::Project);

        let invocation = SkillInvocation {
            skill_name: req.skill_name.clone(),
            scope,
            invocation_type: InvocationType::Explicit,
            trigger: req.command.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            duration_ms: None,
            match_type: MatchType::ExactName,
            metadata: None,
        };

        self.invocation_history.push(invocation.clone());
        self.invocations
            .insert(req.skill_name.clone());

        invocation
    }

    pub fn auto_discover(&self, req: &AutoDiscoverRequest) -> SkillAutoDiscoveryResult {
        let project_dir = Path::new(&req.project_dir);
        let mut detected_files = Vec::new();
        let mut frameworks = Vec::new();
        let mut recommendations = Vec::new();

        let indicators: Vec<(&str, &str, &str)> = vec![
            ("package.json", "Node.js", "npm/yarn项目"),
            ("Cargo.toml", "Rust", "Rust/Cargo项目"),
            ("requirements.txt", "Python", "Python项目"),
            ("go.mod", "Go", "Go项目"),
            ("CMakeLists.txt", "C/C++", "CMake项目"),
            ("pom.xml", "Java", "Maven项目"),
            ("build.gradle", "Java/Kotlin", "Gradle项目"),
            ("Dockerfile", "Docker", "容器化项目"),
            (".github/workflows", "CI/CD", "GitHub Actions项目"),
            (".gitlab-ci.yml", "CI/CD", "GitLab CI项目"),
        ];

        for (file, framework, _) in &indicators {
            let path = project_dir.join(file);
            if path.exists() {
                detected_files.push(file.to_string());
                if !frameworks.contains(&framework.to_string()) {
                    frameworks.push(framework.to_string());
                }
            }
        }

        if detected_files.contains(&"Cargo.toml".to_string()) {
            recommendations.push(SkillRecommendation {
                skill_name: "code-review".to_string(),
                reason: "Rust项目建议启用代码审查技能".to_string(),
                confidence: 0.9,
                source_file: "Cargo.toml".to_string(),
            });
            recommendations.push(SkillRecommendation {
                skill_name: "test-writer".to_string(),
                reason: "Rust项目建议启用测试生成技能".to_string(),
                confidence: 0.85,
                source_file: "Cargo.toml".to_string(),
            });
            recommendations.push(SkillRecommendation {
                skill_name: "dependency-check".to_string(),
                reason: "检测到Cargo.toml，建议开启依赖检查".to_string(),
                confidence: 0.8,
                source_file: "Cargo.toml".to_string(),
            });
        }

        if detected_files.contains(&"package.json".to_string()) {
            recommendations.push(SkillRecommendation {
                skill_name: "code-review".to_string(),
                reason: "Node.js项目建议启用代码审查技能".to_string(),
                confidence: 0.9,
                source_file: "package.json".to_string(),
            });
            recommendations.push(SkillRecommendation {
                skill_name: "dependency-check".to_string(),
                reason: "检测到package.json，建议开启依赖检查".to_string(),
                confidence: 0.85,
                source_file: "package.json".to_string(),
            });
        }

        if detected_files.contains(&"Dockerfile".to_string()) {
            recommendations.push(SkillRecommendation {
                skill_name: "security-audit".to_string(),
                reason: "容器化项目建议启用安全审计".to_string(),
                confidence: 0.75,
                source_file: "Dockerfile".to_string(),
            });
        }

        if frameworks.contains(&"CI/CD".to_string()) {
            recommendations.push(SkillRecommendation {
                skill_name: "git-helper".to_string(),
                reason: "CI/CD项目建议启用Git辅助技能".to_string(),
                confidence: 0.7,
                source_file: ".github/workflows".to_string(),
            });
        }

        let project_type = if frameworks.is_empty() {
            None
        } else {
            Some(frameworks.join(" + "))
        };

        SkillAutoDiscoveryResult {
            project_type,
            detected_frameworks: frameworks,
            recommended_skills: recommendations,
            project_files: detected_files,
        }
    }

    pub fn match_skills(&self, query: &str, limit: usize) -> Vec<SkillMatchResult> {
        let lower_q = query.to_lowercase();
        let mut results = Vec::new();

        for skill in self.loaded_skills.values() {
            if !skill.enabled {
                continue;
            }

            let name_lower = skill.name.to_lowercase();

            if name_lower == lower_q {
                results.push(SkillMatchResult {
                    skill: skill.clone(),
                    match_reason: format!("精确匹配技能名称: {}", skill.name),
                    confidence: 1.0,
                    match_type: MatchType::ExactName,
                });
                continue;
            }

            if name_lower.contains(&lower_q) || lower_q.contains(&name_lower) {
                results.push(SkillMatchResult {
                    skill: skill.clone(),
                    match_reason: format!("名称部分匹配: {}", skill.name),
                    confidence: 0.7,
                    match_type: MatchType::PatternMatch,
                });
                continue;
            }

            let desc_lower = skill.description.to_lowercase();
            if desc_lower.contains(&lower_q) {
                results.push(SkillMatchResult {
                    skill: skill.clone(),
                    match_reason: format!("描述匹配: {}", skill.name),
                    confidence: 0.5,
                    match_type: MatchType::Semantic,
                });
            }
        }

        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    pub fn render_skills_for_context(
        &self,
        config: &SkillRenderConfig,
    ) -> String {
        let mut lines = Vec::new();
        lines.push("## Skills".to_string());

        let enabled_skills: Vec<&SkillMetadata> = self
            .loaded_skills
            .values()
            .filter(|s| s.enabled)
            .map(|s| s)
            .collect();

        if enabled_skills.is_empty() {
            lines.push("(当前无可用技能)".to_string());
            return lines.join("\n");
        }

        if config.group_by_scope {
            let scopes = vec![
                (SkillScope::System, "### 系统技能"),
                (SkillScope::User, "### 用户技能"),
                (SkillScope::Project, "### 项目技能"),
                (SkillScope::Plugin, "### 插件技能"),
            ];

            for (scope, header) in &scopes {
                let scope_skills: Vec<_> = enabled_skills
                    .iter()
                    .filter(|s| s.scope == *scope)
                    .copied()
                    .collect();
                if scope_skills.is_empty() {
                    continue;
                }
                lines.push(header.to_string());
                for skill in scope_skills {
                    lines.push(self.render_single_skill(skill, config));
                }
            }
        } else {
            lines.push("### Available skills".to_string());
            for skill in &enabled_skills {
                lines.push(self.render_single_skill(skill, config));
            }
        }

        lines.push("### How to use skills".to_string());
        lines.push("- 使用 `$SkillName` 或直接提及技能名称来激活".to_string());
        lines.push("- 多个技能可以同时激活".to_string());
        lines.push("- 技能不跨轮次继承，每轮需重新激活".to_string());

        lines.join("\n")
    }

    fn render_single_skill(&self, skill: &SkillMetadata, config: &SkillRenderConfig) -> String {
        let mut desc = if let Some(ref sd) = skill.short_description {
            sd.clone()
        } else {
            skill.description.clone()
        };

        if desc.chars().count() > config.max_description_chars {
            desc = desc
                .chars()
                .take(config.max_description_chars)
                .collect::<String>()
                + "...";
        }

        let mut line = format!("- **{}**: {}", skill.name, desc);

        if config.include_interface {
            if let Some(ref iface) = skill.interface {
                if let Some(ref prompt) = iface.default_prompt {
                    line.push_str(&format!("\n  默认提示: {}", prompt));
                }
            }
        }

        if config.include_dependencies {
            if let Some(ref deps) = skill.dependencies {
                if !deps.tools.is_empty() {
                    let tool_names: Vec<&str> =
                        deps.tools.iter().map(|t| t.value.as_str()).collect();
                    line.push_str(&format!("\n  依赖工具: {}", tool_names.join(", ")));
                }
            }
        }

        line
    }

    pub fn skill_count(&self) -> usize {
        self.loaded_skills.len()
    }

    pub fn enabled_count(&self) -> usize {
        self.loaded_skills.values().filter(|s| s.enabled).count()
    }

    pub fn invocation_count(&self) -> usize {
        self.invocation_history.len()
    }
}