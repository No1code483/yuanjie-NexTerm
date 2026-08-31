use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::app_error::AppError;
use crate::models::skill::{
    AutoDiscoverRequest, DetectImplicitRequest, SkillLoadOutcome, SkillMatchResult,
    SkillMetadata, SkillRecommendation, SkillRegistration, SkillRenderConfig, SkillScope,
    SkillTriggerRequest,
};
use crate::services::skill_market_service::{self, SkillMarketEntry};
use crate::skills::SkillsManager;

pub struct SkillService {
    manager: Arc<RwLock<SkillsManager>>,
}

impl SkillService {
    pub fn new() -> Self {
        Self {
            manager: Arc::new(RwLock::new(SkillsManager::new())),
        }
    }

    pub async fn add_scan_root(&self, path: &str) {
        let mut mgr = self.manager.write().await;
        mgr.add_scan_root(&PathBuf::from(path));
    }

    pub async fn set_project_files(&self, files: Vec<String>) {
        let mut mgr = self.manager.write().await;
        mgr.set_project_files(files);
    }

    pub async fn load_skills(&self) -> SkillLoadOutcome {
        let mut mgr = self.manager.write().await;
        mgr.load_skills().await
    }

    pub async fn find_skill(&self, name: &str) -> Option<SkillMetadata> {
        let mgr = self.manager.read().await;
        mgr.find_skill(name).cloned()
    }

    pub async fn list_skills(&self, scope: Option<SkillScope>) -> Vec<SkillMetadata> {
        let mgr = self.manager.read().await;
        mgr.list_skills(scope).into_iter().cloned().collect()
    }

    pub async fn detect_implicit(&self, req: DetectImplicitRequest) -> Vec<SkillMetadata> {
        let mgr = self.manager.read().await;
        mgr.detect_implicit_invocation(&req)
    }

    pub async fn register_skill(&self, registration: SkillRegistration) -> Result<(), AppError> {
        let mut mgr = self.manager.write().await;
        mgr.register_skill(registration)
    }

    pub async fn unregister_skill(&self, name: &str) -> Result<(), AppError> {
        let mut mgr = self.manager.write().await;
        mgr.unregister_skill(name)
    }

    pub async fn trigger_skill(&self, req: SkillTriggerRequest) -> Option<SkillMetadata> {
        let mut mgr = self.manager.write().await;
        mgr.record_invocation(&req);
        let name = req.skill_name.clone();
        mgr.find_skill(&name).cloned()
    }

    pub async fn auto_discover(&self, req: AutoDiscoverRequest) -> SkillRecommendation {
        let mgr = self.manager.read().await;
        let result = mgr.auto_discover(&req);
        let recommendation = SkillRecommendation {
            skill_name: format!("发现{}个框架，推荐{}个技能", result.detected_frameworks.len(), result.recommended_skills.len()),
            reason: result.recommended_skills.iter().map(|r| r.reason.clone()).collect::<Vec<_>>().join("; "),
            confidence: result.recommended_skills.iter().map(|r| r.confidence).fold(0.0, f64::max),
            source_file: result.project_type.unwrap_or_default(),
        };
        recommendation
    }

    pub async fn match_skills(&self, query: &str, limit: usize) -> Vec<SkillMatchResult> {
        let mgr = self.manager.read().await;
        mgr.match_skills(query, limit)
    }

    pub async fn render_context(&self, config: &SkillRenderConfig) -> String {
        let mgr = self.manager.read().await;
        mgr.render_skills_for_context(config)
    }

    pub async fn skill_count(&self) -> usize {
        let mgr = self.manager.read().await;
        mgr.skill_count()
    }

    pub async fn enabled_count(&self) -> usize {
        let mgr = self.manager.read().await;
        mgr.enabled_count()
    }

    pub async fn invocation_count(&self) -> usize {
        let mgr = self.manager.read().await;
        mgr.invocation_count()
    }

    // ===== D1.8 Skill 市场 =====

    pub async fn list_market(&self) -> Vec<SkillMarketEntry> {
        let mgr = self.manager.read().await;
        let names: Vec<String> = mgr.list_skills(None).iter().map(|s| s.name.clone()).collect();
        skill_market_service::list_market(&names)
    }

    pub async fn search_market(&self, query: &str) -> Vec<SkillMarketEntry> {
        let mgr = self.manager.read().await;
        let names: Vec<String> = mgr.list_skills(None).iter().map(|s| s.name.clone()).collect();
        skill_market_service::search_market(query, &names)
    }

    pub async fn get_market_entry(&self, name: &str) -> Option<SkillMarketEntry> {
        let mgr = self.manager.read().await;
        let names: Vec<String> = mgr.list_skills(None).iter().map(|s| s.name.clone()).collect();
        skill_market_service::get_entry(name, &names)
    }

    pub async fn install_market_skill(&self, name: &str) -> Result<SkillMetadata, AppError> {
        let entry = skill_market_service::get_entry(name, &[])
            .ok_or(AppError::NotFound)?;
        let metadata = skill_market_service::to_metadata(&entry);
        self.manager.write().await.register_market_skill(metadata.clone())?;
        tracing::info!("🛒 [D1.8] 市场技能已安装: {}", name);
        Ok(metadata)
    }

    pub async fn uninstall_market_skill(&self, name: &str) -> Result<(), AppError> {
        self.manager.write().await.unregister_skill(name)
    }
}