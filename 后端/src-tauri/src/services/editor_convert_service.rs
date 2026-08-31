use pulldown_cmark::{html, Options, Parser};

use crate::crypto::aes_gcm;
use crate::error::app_error::AppError;
use crate::models::editor::ConvertResult;
use crate::services::editor_storage_service::EditorStorageService;

pub fn convert_document(
    storage: &EditorStorageService,
    mek: &[u8; 32],
    doc_uuid: &str,
    source_format: &str,
    target_format: &str,
) -> Result<ConvertResult, AppError> {
    let content_path = storage.content_path(doc_uuid);
    let raw = std::fs::read(&content_path).map_err(AppError::FileSystem)?;

    let (nonce_bytes, ciphertext) = raw.split_at(12);
    let nonce: [u8; 12] = nonce_bytes
        .try_into()
        .map_err(|_| AppError::Crypto("nonce 错误".into()))?;

    let plaintext = aes_gcm::decrypt_bytes(ciphertext, mek, &nonce)?;
    let source_text = String::from_utf8(plaintext)
        .map_err(|e| AppError::Crypto(format!("UTF-8 解码失败: {}", e)))?;

    let converted = match (source_format, target_format) {
        ("markdown", "html") | ("md", "html") => markdown_to_html(&source_text),
        ("html", "markdown") | ("html", "md") => html_to_markdown(&source_text),
        ("plain", "html") | ("text", "html") => plain_to_html(&source_text),
        ("plain", "markdown") | ("text", "md") => Ok(source_text.clone()),
        (src, tgt) if src == tgt => Ok(source_text.clone()),
        _ => Err(AppError::Validation(format!(
            "不支持的格式转换: {} -> {}",
            source_format, target_format
        ))),
    }?;

    Ok(ConvertResult {
        doc_uuid: doc_uuid.to_string(),
        source_format: source_format.to_string(),
        target_format: target_format.to_string(),
        content: converted,
    })
}

pub fn convert_content(
    content: &str,
    source_format: &str,
    target_format: &str,
) -> Result<String, AppError> {
    match (source_format, target_format) {
        ("markdown", "html") | ("md", "html") => markdown_to_html(content),
        ("html", "markdown") | ("html", "md") => html_to_markdown(content),
        ("plain", "html") | ("text", "html") => plain_to_html(content),
        ("plain", "markdown") | ("text", "md") => Ok(content.to_string()),
        (src, tgt) if src == tgt => Ok(content.to_string()),
        _ => Err(AppError::Validation(format!(
            "不支持的格式转换: {} -> {}",
            source_format, target_format
        ))),
    }
}

fn markdown_to_html(md: &str) -> Result<String, AppError> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES;

    let parser = Parser::new_ext(md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    Ok(html_output)
}

fn html_to_markdown(html: &str) -> Result<String, AppError> {
    let mut md = html
        .replace("<p>", "\n\n")
        .replace("</p>", "")
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<strong>", "**")
        .replace("</strong>", "**")
        .replace("<b>", "**")
        .replace("</b>", "**")
        .replace("<em>", "*")
        .replace("</em>", "*")
        .replace("<i>", "*")
        .replace("</i>", "*")
        .replace("<code>", "`")
        .replace("</code>", "`")
        .replace("<pre>", "\n```\n")
        .replace("</pre>", "\n```\n")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    md = md.trim().to_string();
    Ok(md)
}

fn plain_to_html(text: &str) -> Result<String, AppError> {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");

    let html = escaped
        .split("\n\n")
        .map(|para| {
            let trimmed = para.trim();
            if trimmed.is_empty() {
                String::new()
            } else {
                format!("<p>{}</p>", trimmed.replace('\n', "<br>"))
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(html)
}