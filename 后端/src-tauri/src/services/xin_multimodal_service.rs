use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};

use crate::error::app_error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: String,
    pub mime_type: String,
    pub path: Option<String>,
    pub data_b64: Option<String>,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedAttachment {
    pub name: String,
    pub mime_type: String,
    pub text_content: Option<String>,
    pub image_b64: Option<String>,
    pub is_image: bool,
    pub is_text: bool,
    pub is_video: bool,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBlock {
    pub block_type: String,
    pub text: Option<String>,
    pub image_url: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultimodalMessage {
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputEnhancement {
    pub has_code_blocks: bool,
    pub code_languages: Vec<String>,
    pub has_tables: bool,
    pub has_lists: bool,
    pub content_length: usize,
    pub suggested_mode: String,
}

pub struct XinMultimodalService;

impl XinMultimodalService {
    pub fn parse_attachment(attachment: &Attachment) -> Result<ParsedAttachment, AppError> {
        // D3.5 视频附件识别：标记为视频，实际帧抽取由 XinVideoService 处理
        if attachment.mime_type.starts_with("video/") {
            let label = attachment
                .label
                .clone()
                .unwrap_or_else(|| attachment.name.clone());
            return Ok(ParsedAttachment {
                name: attachment.name.clone(),
                mime_type: attachment.mime_type.clone(),
                text_content: None,
                image_b64: None,
                is_image: false,
                is_text: false,
                is_video: true,
                label,
            });
        }

        let is_image = attachment.mime_type.starts_with("image/");

        if is_image {
            let b64 = if let Some(ref data) = attachment.data_b64 {
                data.clone()
            } else if let Some(ref path) = attachment.path {
                let bytes =
                    std::fs::read(path).map_err(|e| AppError::FileSystem(e))?;
                STANDARD.encode(&bytes)
            } else {
                return Err(AppError::Validation(
                    "图片附件缺少 data_b64 或 path".into(),
                ));
            };

            Self::validate_image_b64(&b64, &attachment.mime_type)?;

            let label = attachment
                .label
                .clone()
                .unwrap_or_else(|| attachment.name.clone());

            return Ok(ParsedAttachment {
                name: attachment.name.clone(),
                mime_type: attachment.mime_type.clone(),
                text_content: None,
                image_b64: Some(b64),
                is_image: true,
                is_text: false,
                is_video: false,
                label,
            });
        }

        let text = if let Some(ref data) = attachment.data_b64 {
            let bytes = STANDARD
                .decode(data)
                .map_err(|e| AppError::Validation(format!("Base64解码失败: {}", e)))?;
            String::from_utf8(bytes)
                .map_err(|e| AppError::Validation(format!("UTF-8解码失败: {}", e)))?
        } else if let Some(ref path) = attachment.path {
            std::fs::read_to_string(path).map_err(|e| AppError::FileSystem(e))?
        } else {
            return Err(AppError::Validation(
                "文本附件缺少 data_b64 或 path".into(),
            ));
        };

        let is_text_file = matches!(
            attachment.mime_type.as_str(),
            "text/plain"
                | "text/markdown"
                | "text/csv"
                | "application/json"
                | "text/html"
                | "text/xml"
                | "application/xml"
        );

        let label = attachment
            .label
            .clone()
            .unwrap_or_else(|| attachment.name.clone());

        Ok(ParsedAttachment {
            name: attachment.name.clone(),
            mime_type: attachment.mime_type.clone(),
            text_content: Some(text),
            image_b64: None,
            is_image: false,
            is_text: is_text_file,
            is_video: false,
            label,
        })
    }

    fn validate_image_b64(b64: &str, mime_type: &str) -> Result<(), AppError> {
        if b64.len() > 20 * 1024 * 1024 {
            return Err(AppError::Validation("图片Base64过大（>20MB）".into()));
        }

        let valid_prefixes = [
            ("image/png", "iVBOR"),
            ("image/jpeg", "/9j/"),
            ("image/gif", "R0lGOD"),
            ("image/webp", "UklGR"),
            ("image/bmp", "Qk"),
        ];

        let known = valid_prefixes.iter().any(|(t, _p)| *t == mime_type);
        if known {
            let expected_prefix = valid_prefixes
                .iter()
                .find(|(t, _)| *t == mime_type)
                .map(|(_, p)| *p);

            if let Some(prefix) = expected_prefix {
                if !b64.starts_with(prefix) {
                    return Err(AppError::Validation(format!(
                        "Base64数据不符合 {} 格式签名",
                        mime_type
                    )));
                }
            }
        }

        Ok(())
    }

    pub fn build_multimodal_messages(
        messages: &[crate::services::xin_context_service::ChatMessage],
        attachments_map: &std::collections::HashMap<usize, Vec<ParsedAttachment>>,
    ) -> Vec<serde_json::Value> {
        use crate::services::xin_context_service::ChatRole;

        messages
            .iter()
            .enumerate()
            .map(|(idx, msg)| {
                let role_str = match msg.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                };

                let attachments = attachments_map.get(&idx);

                if let Some(atts) = attachments {
                    if atts.iter().any(|a| a.is_image) {
                        Self::build_multimodal_content(msg, atts, role_str)
                    } else {
                        let text_parts: Vec<String> = atts
                            .iter()
                            .filter_map(|a| {
                                a.text_content.as_ref().map(|text| {
                                    format!(
                                        "[附件: {}]\n内容:\n{}\n[/附件]",
                                        a.label, text
                                    )
                                })
                            })
                            .collect();
                        let mut content = msg.content.clone();
                        if !text_parts.is_empty() {
                            content.push_str("\n\n");
                            content.push_str(&text_parts.join("\n\n"));
                        }
                        serde_json::json!({
                            "role": role_str,
                            "content": content,
                        })
                    }
                } else {
                    serde_json::json!({
                        "role": role_str,
                        "content": msg.content,
                    })
                }
            })
            .collect()
    }

    fn build_multimodal_content(
        msg: &crate::services::xin_context_service::ChatMessage,
        attachments: &[ParsedAttachment],
        role_str: &str,
    ) -> serde_json::Value {
        let mut content_parts: Vec<serde_json::Value> = Vec::new();

        if !msg.content.is_empty() {
            content_parts.push(serde_json::json!({
                "type": "text",
                "text": msg.content,
            }));
        }

        for att in attachments {
            if att.is_image {
                if let Some(ref b64) = att.image_b64 {
                    content_parts.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": {
                            "url": format!("data:{};base64,{}", att.mime_type, b64),
                            "detail": "auto",
                        },
                    }));
                }
            } else if let Some(ref text) = att.text_content {
                content_parts.push(serde_json::json!({
                    "type": "text",
                    "text": format!("[附件: {}]\n{}", att.label, text),
                }));
            }
        }

        serde_json::json!({
            "role": role_str,
            "content": content_parts,
        })
    }

    pub fn build_attachment_context(attachments: &[ParsedAttachment]) -> String {
        if attachments.is_empty() {
            return String::new();
        }

        let mut ctx = String::from("\n## 用户提供的附件内容\n\n");
        for att in attachments {
            ctx.push_str(&format!("### {}\n", att.label));
            if att.is_image {
                ctx.push_str("(图片附件，已通过多模态接口发送给模型)\n\n");
            } else if att.is_video {
                ctx.push_str("(视频附件，请通过 xin_video_analyze 抽取关键帧后再分析)\n\n");
            } else if let Some(ref text) = att.text_content {
                let truncated = if text.len() > 3000 {
                    format!("{}...(内容已截断)\n\n", &text[..3000])
                } else {
                    format!("{}\n\n", text)
                };
                ctx.push_str(&truncated);
            }
        }
        ctx
    }

    pub fn enhance_output(response: &str) -> OutputEnhancement {
        let code_languages = Self::detect_code_languages(response);
        let has_code_blocks = !code_languages.is_empty();
        let has_tables = response.contains('\n') && response.contains(" | ");
        let has_lists = response.contains("\n- ") || response.contains("\n* ") || response.contains("\n1. ");

        let suggested_mode = if has_code_blocks {
            "code_view".to_string()
        } else if response.len() > 2000 {
            "detailed_view".to_string()
        } else if response.len() < 200 {
            "compact_view".to_string()
        } else {
            "default".to_string()
        };

        OutputEnhancement {
            has_code_blocks,
            code_languages,
            has_tables,
            has_lists,
            content_length: response.len(),
            suggested_mode,
        }
    }

    pub fn detect_code_languages(text: &str) -> Vec<String> {
        let mut languages = Vec::new();
        let mut chars = text.char_indices();

        while let Some((i, c)) = chars.next() {
            if c == '`' {
                let rest: String = text[i..].chars().take(10).collect();
                if rest.starts_with("```") && rest.len() > 3 {
                    let lang_candidate: String = rest[3..]
                        .chars()
                        .take_while(|ch| *ch != '\n' && *ch != ' ')
                        .collect();
                    if !lang_candidate.is_empty()
                        && lang_candidate.len() < 20
                        && lang_candidate
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '#')
                    {
                        if !languages.contains(&lang_candidate) {
                            languages.push(lang_candidate);
                        }
                    }
                }
            }
        }

        languages
    }

    pub fn format_for_display(
        raw_response: &str,
        enhancement: &OutputEnhancement,
    ) -> serde_json::Value {
        serde_json::json!({
            "content": raw_response,
            "display_hints": {
                "has_code_blocks": enhancement.has_code_blocks,
                "code_languages": enhancement.code_languages,
                "has_tables": enhancement.has_tables,
                "has_lists": enhancement.has_lists,
                "content_length": enhancement.content_length,
                "suggested_mode": enhancement.suggested_mode,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_code_languages() {
        let text = "解释:\n```rust\nfn main() {}\n```\n也可以:\n```python\nprint('hi')\n```";
        let langs = XinMultimodalService::detect_code_languages(text);
        assert!(langs.contains(&"rust".to_string()));
        assert!(langs.contains(&"python".to_string()));
    }

    #[test]
    fn test_detect_no_code() {
        let text = "普通回复，没有代码块";
        let langs = XinMultimodalService::detect_code_languages(text);
        assert!(langs.is_empty());
    }

    #[test]
    fn test_enhance_output_code() {
        let response = "```javascript\nconsole.log('hi');\n```";
        let enhancement = XinMultimodalService::enhance_output(response);
        assert!(enhancement.has_code_blocks);
        assert_eq!(enhancement.suggested_mode, "code_view");
    }

    #[test]
    fn test_enhance_output_short() {
        let response = "简短回复";
        let enhancement = XinMultimodalService::enhance_output(response);
        assert!(!enhancement.has_code_blocks);
        assert_eq!(enhancement.suggested_mode, "compact_view");
    }

    #[test]
    fn test_enhance_output_table() {
        let response = "名称 | 值\n--- | ---\na | 1\nb | 2";
        let enhancement = XinMultimodalService::enhance_output(response);
        assert!(enhancement.has_tables);
    }

    #[test]
    fn test_enhance_output_list() {
        let response = "- 项目1\n- 项目2\n- 项目3";
        let enhancement = XinMultimodalService::enhance_output(response);
        assert!(enhancement.has_lists);
    }

    #[test]
    fn test_validate_image_b64_valid() {
        let result =
            XinMultimodalService::validate_image_b64("iVBORw0KGgoAAAA", "image/png");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_image_b64_invalid() {
        let result =
            XinMultimodalService::validate_image_b64("invalid_data", "image/png");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_attachment_text_b64() {
        let content = "Hello, World!";
        let b64 = STANDARD.encode(content.as_bytes());
        let att = Attachment {
            name: "test.txt".into(),
            mime_type: "text/plain".into(),
            path: None,
            data_b64: Some(b64),
            label: None,
        };
        let result = XinMultimodalService::parse_attachment(&att);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert!(parsed.is_text);
        assert_eq!(parsed.text_content.unwrap(), "Hello, World!");
    }

    #[test]
    fn test_build_attachment_context() {
        let attachments = vec![
            ParsedAttachment {
                name: "doc.txt".into(),
                mime_type: "text/plain".into(),
                text_content: Some("文档内容".into()),
                image_b64: None,
                is_image: false,
                is_text: true,
                is_video: false,
                label: "doc.txt".into(),
            },
            ParsedAttachment {
                name: "photo.png".into(),
                mime_type: "image/png".into(),
                text_content: None,
                image_b64: Some("iVBORw0KGgoAAAA".into()),
                is_image: true,
                is_text: false,
                is_video: false,
                label: "photo.png".into(),
            },
        ];
        let ctx = XinMultimodalService::build_attachment_context(&attachments);
        assert!(ctx.contains("doc.txt"));
        assert!(ctx.contains("photo.png"));
        assert!(ctx.contains("图片附件"));
    }

    #[test]
    fn test_format_for_display() {
        let response = "测试回复";
        let enhancement = XinMultimodalService::enhance_output(response);
        let display = XinMultimodalService::format_for_display(response, &enhancement);
        let obj = display.as_object().unwrap();
        assert_eq!(obj["content"].as_str().unwrap(), "测试回复");
        assert!(obj["display_hints"].is_object());
    }
}