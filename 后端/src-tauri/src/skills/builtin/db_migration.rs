//! db_migration Skill — 数据库迁移（生成数据库迁移脚本）
//!
//! 强制规则：通过 CloudApiRouter 走云端 API，禁止本地底层智能模型。

use async_trait::async_trait;

use crate::error::app_error::AppError;
use crate::skills::builtin::{BuiltinSkill, SkillExecutionContext, SkillInput, SkillOutput};

pub struct DbMigrationSkill;

#[async_trait]
impl BuiltinSkill for DbMigrationSkill {
    fn name(&self) -> &str {
        "db_migration"
    }

    fn description(&self) -> &str {
        "生成数据库迁移脚本：表结构变更、索引调整、数据回填，支持 up/down 双向迁移"
    }

    fn short_description(&self) -> &str {
        "数据库迁移"
    }

    fn category(&self) -> &str {
        "架构"
    }

    fn trigger_patterns(&self) -> Vec<String> {
        vec![
            "migration".into(),
            "迁移".into(),
            "schema".into(),
            "ddl".into(),
        ]
    }

    fn priority(&self) -> i32 {
        7
    }

    fn file_patterns(&self) -> Vec<String> {
        vec!["*.sql".into(), "migrations/*".into()]
    }

    fn system_prompt(&self) -> String {
        "你是一名数据库工程师，擅长编写可回滚的迁移脚本。\
         请根据用户的变更需求生成数据库迁移脚本。\
         要求：\
         1. 输出 up.sql（正向迁移）与 down.sql（回滚迁移）两个代码块；\
         2. 使用幂等语句（IF NOT EXISTS / IF EXISTS）；\
         3. 大表变更需给出分批迁移策略；\
         4. 标注潜在的锁表风险与预估耗时。"
            .to_string()
    }

    fn build_prompt(&self, input: &SkillInput) -> String {
        format!(
            "# 数据库迁移任务\n\n\
             ## 数据库类型\n{}\n\n\
             ## 变更需求\n{}\n\n\
             ## 现有表结构/上下文\n{}",
            if input.language.is_empty() {
                "SQLite".to_string()
            } else {
                input.language.clone()
            },
            if input.instruction.is_empty() {
                if input.code.is_empty() {
                    "请描述需要迁移的表结构变更".to_string()
                } else {
                    input.code.clone()
                }
            } else {
                input.instruction.clone()
            },
            input.context,
        )
    }

    async fn execute(
        &self,
        input: SkillInput,
        ctx: &SkillExecutionContext,
    ) -> Result<SkillOutput, AppError> {
        let prompt = self.build_prompt(&input);
        ctx.call_cloud(self.system_prompt(), prompt, self.name()).await
    }
}
