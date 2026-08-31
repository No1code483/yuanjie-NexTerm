use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SkillScope {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "plugin")]
    Plugin,
}

impl SkillScope {
    pub fn as_str(&self) -> &str {
        match self {
            SkillScope::User => "user",
            SkillScope::Project => "project",
            SkillScope::System => "system",
            SkillScope::Plugin => "plugin",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "user" => SkillScope::User,
            "project" | "repo" => SkillScope::Project,
            "system" | "admin" => SkillScope::System,
            "plugin" => SkillScope::Plugin,
            _ => SkillScope::Project,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub short_description: Option<String>,
    pub interface: Option<SkillInterface>,
    pub dependencies: Option<SkillDependencies>,
    pub policy: Option<SkillPolicy>,
    pub path: String,
    pub scope: SkillScope,
    pub plugin_id: Option<String>,
    pub enabled: bool,
    pub file_size: u64,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInterface {
    pub display_name: Option<String>,
    pub short_description: Option<String>,
    pub icon_small: Option<String>,
    pub icon_large: Option<String>,
    pub brand_color: Option<String>,
    pub default_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependencies {
    pub tools: Vec<SkillToolDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToolDependency {
    pub tool_type: String,
    pub value: String,
    pub description: Option<String>,
    pub transport: Option<String>,
    pub command: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPolicy {
    pub allow_implicit_invocation: Option<bool>,
    pub auto_discover: Option<bool>,
    pub trigger_patterns: Option<Vec<String>>,
    pub file_patterns: Option<Vec<String>>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLoadOutcome {
    pub skills: Vec<SkillMetadata>,
    pub errors: Vec<SkillLoadError>,
    pub total_count: usize,
    pub enabled_count: usize,
    pub by_scope: SkillScopeCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLoadError {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillScopeCount {
    pub user: usize,
    pub project: usize,
    pub system: usize,
    pub plugin: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAutoDiscoveryResult {
    pub project_type: Option<String>,
    pub detected_frameworks: Vec<String>,
    pub recommended_skills: Vec<SkillRecommendation>,
    pub project_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRecommendation {
    pub skill_name: String,
    pub reason: String,
    pub confidence: f64,
    pub source_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInvocation {
    pub skill_name: String,
    pub scope: SkillScope,
    pub invocation_type: InvocationType,
    pub trigger: Option<String>,
    pub timestamp: i64,
    pub duration_ms: Option<u64>,
    pub match_type: MatchType,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InvocationType {
    #[serde(rename = "explicit")]
    Explicit,
    #[serde(rename = "implicit")]
    Implicit,
    #[serde(rename = "auto_discovered")]
    AutoDiscovered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRegistration {
    pub name: String,
    pub scope: SkillScope,
    pub path: String,
    pub skill_file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillTriggerRequest {
    pub skill_name: String,
    pub context: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectImplicitRequest {
    pub command: String,
    pub workdir: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDiscoverRequest {
    pub project_dir: String,
    pub force_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillRenderConfig {
    pub max_description_chars: usize,
    pub include_interface: bool,
    pub include_dependencies: bool,
    pub group_by_scope: bool,
}

impl Default for SkillRenderConfig {
    fn default() -> Self {
        Self {
            max_description_chars: 8000,
            include_interface: true,
            include_dependencies: false,
            group_by_scope: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMatchResult {
    pub skill: SkillMetadata,
    pub match_reason: String,
    pub confidence: f64,
    pub match_type: MatchType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchType {
    #[serde(rename = "exact_name")]
    ExactName,
    #[serde(rename = "pattern_match")]
    PatternMatch,
    #[serde(rename = "semantic")]
    Semantic,
    #[serde(rename = "file_detection")]
    FileDetection,
    #[serde(rename = "implicit")]
    Implicit,
    #[serde(rename = "none")]
    NoMatch,
}