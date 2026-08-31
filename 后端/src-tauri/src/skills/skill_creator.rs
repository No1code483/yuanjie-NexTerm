use std::fs;
use std::path::PathBuf;

use crate::models::skill::{SkillMetadata, SkillPolicy, SkillScope, SkillInterface};

/// Skill 创建器 - 元技能：交互式创建 SKILL.md
pub struct SkillCreator {
    output_dir: PathBuf,
}

impl SkillCreator {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    /// 创建 SKILL.md 文件
    pub fn create_skill(
        &self,
        name: &str,
        description: &str,
        short_description: Option<&str>,
        trigger_patterns: Option<Vec<String>>,
        file_patterns: Option<Vec<String>>,
        interface: Option<SkillInterface>,
        content: &str,
    ) -> Result<SkillMetadata, String> {
        let skill_dir = self.output_dir.join(name);
        fs::create_dir_all(&skill_dir).map_err(|e| {
            format!("无法创建技能目录: {}", e)
        })?;

        let skill_file = skill_dir.join("SKILL.md");
        let mut file_content = String::new();

        // YAML frontmatter
        file_content.push_str("---\n");
        file_content.push_str(&format!("name: \"{}\"\n", name));
        file_content.push_str(&format!("description: \"{}\"\n", description));
        if let Some(sd) = short_description {
            file_content.push_str(&format!("short_description: \"{}\"\n", sd));
        }
        if let Some(patterns) = &trigger_patterns {
            for p in patterns {
                file_content.push_str(&format!("trigger: \"{}\"\n", p));
            }
        }
        if let Some(patterns) = &file_patterns {
            file_content.push_str("file_patterns:\n");
            for p in patterns {
                file_content.push_str(&format!("  - \"{}\"\n", p));
            }
        }
        file_content.push_str("allow_implicit_invocation: true\n");
        file_content.push_str("auto_discover: true\n");
        file_content.push_str("---\n\n");

        // 正文内容
        file_content.push_str(content);
        file_content.push('\n');

        fs::write(&skill_file, &file_content).map_err(|e| {
            format!("无法写入 SKILL.md: {}", e)
        })?;

        let metadata = fs::metadata(&skill_file).ok();
        let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
        let created_at = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs().to_string());

        Ok(SkillMetadata {
            name: name.to_string(),
            description: description.to_string(),
            short_description: short_description.map(|s| s.to_string()),
            interface,
            dependencies: None,
            policy: Some(SkillPolicy {
                allow_implicit_invocation: Some(true),
                auto_discover: Some(true),
                trigger_patterns,
                file_patterns,
                priority: Some(5),
            }),
            path: skill_file.to_string_lossy().to_string(),
            scope: SkillScope::Project,
            plugin_id: None,
            enabled: true,
            file_size,
            created_at,
        })
    }

    /// 验证技能名称是否合法
    pub fn validate_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("技能名称不能为空".into());
        }
        if name.len() > 64 {
            return Err("技能名称不能超过64个字符".into());
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err("技能名称只能包含字母、数字、连字符和下划线".into());
        }
        Ok(())
    }

    /// 生成技能模板
    pub fn generate_template(name: &str, description: &str) -> String {
        format!(
            r#"# {} 技能

## 描述
{}

## 用途
在此描述此技能的具体用途和使用场景。

## 触发条件
- 当用户请求...
- 当检测到...
- 当文件匹配...

## 工作流程
1. 接收用户请求
2. 分析上下文
3. 执行操作
4. 返回结果

## 示例
### 示例 1
用户输入: "..."
技能输出: "..."

## 注意事项
- 注意事项1
- 注意事项2
"#,
            name, description
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn test_dir() -> PathBuf {
        env::temp_dir().join("nexterm_skill_test")
    }

    #[test]
    fn test_validate_name() {
        assert!(SkillCreator::validate_name("my-skill").is_ok());
        assert!(SkillCreator::validate_name("my_skill_123").is_ok());
        assert!(SkillCreator::validate_name("").is_err());
        assert!(SkillCreator::validate_name("bad name!").is_err());
    }

    #[test]
    fn test_generate_template() {
        let template = SkillCreator::generate_template("test-skill", "A test skill");
        assert!(template.contains("# test-skill 技能"));
        assert!(template.contains("A test skill"));
        assert!(template.contains("## 示例"));
    }

    #[test]
    fn test_create_skill() {
        let dir = test_dir();
        let _ = fs::remove_dir_all(&dir);
        let creator = SkillCreator::new(dir.clone());

        let result = creator.create_skill(
            "test-skill",
            "A test skill",
            Some("Test"),
            Some(vec!["test".into()]),
            Some(vec!["*.rs".into()]),
            None,
            "# Test Skill\n\nThis is a test.",
        );

        assert!(result.is_ok());
        let metadata = result.unwrap();
        assert_eq!(metadata.name, "test-skill");
        assert!(metadata.path.contains("test-skill/SKILL.md"));

        let _ = fs::remove_dir_all(&dir);
    }
}