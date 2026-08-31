use async_trait::async_trait;
use std::fs;
use std::path::Path;

use crate::models::tool::{
    PatchChange, PatchChangeType, PatchRequest, PatchResult, ToolCallResult,
    ToolCategory, ToolDefinition, ToolError, ToolPermission,
};
use super::RegisteredTool;

/// Apply Patch 工具 - 对标 Codex 的 apply_patch
pub struct ApplyPatchTool;

impl ApplyPatchTool {
    pub fn new() -> Self {
        Self
    }

    pub fn apply_patch(&self, request: PatchRequest) -> Result<PatchResult, ToolError> {
        let path = Path::new(&request.file_path);
        let create_if_not_exists = request.create_if_not_exists.unwrap_or(false);

        let original_content = if path.exists() {
            Some(fs::read_to_string(path).map_err(|e| ToolError {
                code: "FILE_READ_ERROR".into(),
                message: format!("无法读取文件: {}", e),
                details: None,
            })?)
        } else if create_if_not_exists {
            None
        } else {
            return Err(ToolError {
                code: "FILE_NOT_FOUND".into(),
                message: format!("文件不存在: {}", request.file_path),
                details: None,
            });
        };

        let original = original_content.clone().unwrap_or_default();
        let new_content = self.parse_and_apply_patch(&original, &request.patch_content)?;
        let changes = self.compute_changes(&original, &new_content);

        fs::write(path, &new_content).map_err(|e| ToolError {
            code: "FILE_WRITE_ERROR".into(),
            message: format!("无法写入文件: {}", e),
            details: None,
        })?;

        Ok(PatchResult {
            file_path: request.file_path.clone(),
            applied: true,
            changes,
            original_content: original_content,
            new_content: Some(new_content),
        })
    }

    fn parse_and_apply_patch(&self, original: &str, patch: &str) -> Result<String, ToolError> {
        let original_lines: Vec<&str> = original.lines().collect();
        let mut result = String::new();
        let mut current_line = 0usize;

        let patch_lines: Vec<&str> = patch.lines().collect();
        let mut i = 0;

        while i < patch_lines.len() {
            let line = patch_lines[i];

            if line.starts_with("@@") {
                // 解析 hunk header: @@ -L,C +L,C @@
                if let Some((minus, _plus)) = parse_hunk_header(line) {
                    let target = minus.0.saturating_sub(1); // 0-based
                    while current_line < target && current_line < original_lines.len() {
                        result.push_str(original_lines[current_line]);
                        result.push('\n');
                        current_line += 1;
                    }
                }
                i += 1;
            } else if line.starts_with('+') {
                // 添加行
                result.push_str(&line[1..]);
                result.push('\n');
                i += 1;
            } else if line.starts_with('-') {
                // 删除行（跳过）
                current_line += 1;
                i += 1;
            } else if line.starts_with(' ') {
                // 上下文行
                if current_line < original_lines.len() {
                    result.push_str(original_lines[current_line]);
                    result.push('\n');
                }
                current_line += 1;
                i += 1;
            } else {
                i += 1;
            }
        }

        // 追加剩余行
        while current_line < original_lines.len() {
            result.push_str(original_lines[current_line]);
            result.push('\n');
            current_line += 1;
        }

        Ok(result)
    }

    fn compute_changes(&self, original: &str, new: &str) -> Vec<PatchChange> {
        let orig_lines: Vec<&str> = original.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();
        let mut changes = Vec::new();

        let max_len = orig_lines.len().max(new_lines.len());
        let mut i = 0;

        while i < max_len {
            let orig = orig_lines.get(i);
            let new = new_lines.get(i);

            match (orig, new) {
                (Some(o), Some(n)) if o != n => {
                    changes.push(PatchChange {
                        line_start: i + 1,
                        line_end: i + 1,
                        change_type: PatchChangeType::Modify,
                        content: n.to_string(),
                    });
                }
                (None, Some(n)) => {
                    changes.push(PatchChange {
                        line_start: i + 1,
                        line_end: i + 1,
                        change_type: PatchChangeType::Add,
                        content: n.to_string(),
                    });
                }
                (Some(_), None) => {
                    changes.push(PatchChange {
                        line_start: i + 1,
                        line_end: i + 1,
                        change_type: PatchChangeType::Remove,
                        content: String::new(),
                    });
                }
                _ => {}
            }
            i += 1;
        }

        changes
    }
}

fn parse_hunk_header(line: &str) -> Option<((usize, usize), (usize, usize))> {
    let content = line.strip_prefix("@@")?.strip_suffix("@@")?.trim();
    let parts: Vec<&str> = content.split_whitespace().collect();
    if parts.len() >= 2 {
        let minus = parse_line_range(parts[0].strip_prefix('-')?)?;
        let plus = parse_line_range(parts[1].strip_prefix('+')?)?;
        Some((minus, plus))
    } else {
        None
    }
}

fn parse_line_range(s: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = s.split(',').collect();
    let start: usize = parts[0].parse().ok()?;
    let count: usize = if parts.len() > 1 {
        parts[1].parse().unwrap_or(1)
    } else {
        1
    };
    Some((start, count))
}

#[async_trait]
impl RegisteredTool for ApplyPatchTool {
    fn definition(&self) -> ToolDefinition {
        use crate::models::tool::ToolExample;
        ToolDefinition {
            name: "apply_patch".into(),
            description: "应用自由格式 unified diff 补丁到文件。支持标准 diff 格式（@@ 标记行范围，+ 添加，- 删除），用于修改文件内容。".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "要修改的文件路径"
                    },
                    "patch_content": {
                        "type": "string",
                        "description": "unified diff 格式的补丁内容"
                    },
                    "description": {
                        "type": "string",
                        "description": "变更描述"
                    },
                    "create_if_not_exists": {
                        "type": "boolean",
                        "description": "文件不存在时是否创建"
                    }
                },
                "required": ["file_path", "patch_content"]
            }),
            examples: vec![
                ToolExample {
                    description: "修改函数体".into(),
                    arguments: serde_json::json!({
                        "file_path": "src/main.rs",
                        "patch_content": "@@ -10,3 +10,3 @@\n fn main() {\n-    println!(\"old\");\n+    println!(\"new\");\n }",
                        "description": "更新 greet 函数"
                    }),
                },
            ],
            category: ToolCategory::FileEdit,
            tags: vec!["edit".into(), "file".into(), "patch".into()],
        }
    }

    fn permission(&self) -> ToolPermission {
        ToolPermission {
            requires_approval: true,
            sandbox_required: true,
            max_retries: 2,
            timeout_ms: 30_000,
            ..Default::default()
        }
    }

    fn clone_box(&self) -> Box<dyn RegisteredTool> {
        Box::new(Self)
    }

    async fn call(&self, arguments: serde_json::Value) -> Result<ToolCallResult, ToolError> {
        let request: PatchRequest = serde_json::from_value(arguments).map_err(|e| {
            ToolError {
                code: "INVALID_ARGUMENTS".into(),
                message: format!("参数解析失败: {}", e),
                details: None,
            }
        })?;

        match self.apply_patch(request) {
            Ok(result) => Ok(ToolCallResult {
                success: true,
                content: format!(
                    "补丁已应用: {} ({} 处更改)",
                    result.file_path,
                    result.changes.len()
                ),
                metadata: Some(crate::models::tool::ToolCallMetadata {
                    duration_ms: 0,
                    tokens_used: None,
                    files_modified: vec![result.file_path],
                    sandbox_used: false,
                }),
                error: None,
            }),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_patch_simple() {
        let tool = ApplyPatchTool::new();
        let original = "line1\nline2\nline3\n";
        let patch = "@@ -2 +2 @@\n-line2\n+modified_line2\n";
        let result = tool.parse_and_apply_patch(original, patch).unwrap();
        assert_eq!(result, "line1\nmodified_line2\nline3\n");
    }

    #[test]
    fn test_apply_patch_add_line() {
        let tool = ApplyPatchTool::new();
        let original = "line1\nline2\n";
        let patch = "@@ -1,2 +1,3 @@\n line1\n line2\n+line3\n";
        let result = tool.parse_and_apply_patch(original, patch).unwrap();
        assert_eq!(result, "line1\nline2\nline3\n");
    }
}