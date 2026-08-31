use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::html::highlighted_html_for_string;

use crate::error::app_error::AppError;
use crate::models::editor::{HighlightResult, SyntaxToken};

pub fn highlight_content(content: &str, language: &str) -> Result<HighlightResult, AppError> {
    let ss = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();

    let syntax = ss
        .find_syntax_by_token(language)
        .or_else(|| ss.find_syntax_by_extension(language))
        .ok_or_else(|| AppError::Validation(format!("不支持的语言: {}", language)))?;

    let line_count = content.lines().count();

    let html = highlighted_html_for_string(content, &ss, syntax, &ts.themes["base16-ocean.dark"])
        .map_err(|e| AppError::Crypto(format!("语法高亮失败: {}", e)))?;

    let tokens = parse_html_tokens(&html);

    Ok(HighlightResult {
        tokens,
        line_count,
    })
}

fn parse_html_tokens(html: &str) -> Vec<SyntaxToken> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let bytes = html.as_bytes();

    while cursor < bytes.len() {
        if bytes[cursor] == b'<' {
            let tag_end = match bytes[cursor..].iter().position(|&b| b == b'>') {
                Some(pos) => cursor + pos,
                None => break,
            };

            if bytes[cursor + 1] == b'/' {
                cursor = tag_end + 1;
                continue;
            }

            let content_start = tag_end + 1;
            let closing_tag = if bytes[cursor + 1] == b's' {
                "</span>"
            } else {
                "</pre>"
            };
            let closing_end = match html[content_start..].find(closing_tag) {
                Some(pos) => content_start + pos,
                None => break,
            };

            let inner_text = &html[content_start..closing_end];
            let mut scope = String::new();

            if bytes[cursor + 1] == b's' {
                let tag_str = &html[cursor..=tag_end];
                if let Some(class_start) = tag_str.find("class=\"") {
                    let class_val = &tag_str[class_start + 7..];
                    if let Some(class_end) = class_val.find('"') {
                        scope = class_val[..class_end].to_string();
                    }
                }
            }

            let text = html_entity_decode(inner_text);
            let start = cursor;
            let end = closing_end + closing_tag.len();

            if !text.is_empty() {
                tokens.push(SyntaxToken {
                    text,
                    scope,
                    start,
                    end,
                });
            }

            cursor = end;
        } else {
            cursor += 1;
        }
    }

    tokens
}

fn html_entity_decode(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}