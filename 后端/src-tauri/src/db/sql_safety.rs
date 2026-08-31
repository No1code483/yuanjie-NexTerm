//! T2.11 SQL 注入防护 - 表名/列名白名单校验工具
//!
//! 设计依据：功能展望/平台级增强/05_安全加固_A4.md §2.1.2
//!
//! 用途：
//!   对于少数必须使用 `format!` 拼接表名/列名的场景（如动态表名查询），
//!   提供白名单校验，确保只允许字母、数字、下划线，防止注入。
//!
//! 适用场景：
//!   - `health_check_service::check_table_counts` — `format!("SELECT COUNT(*) FROM {}", table_name)`
//!   - `health_check_service::repair_orphaned_records` — `format!("CREATE TABLE {}_orphaned ...", table)`
//!   - 其他动态 SQL 场景
//!
//! 不适用场景：
//!   - 用户输入的值参数（应使用 `?` 占位符 + `.bind()` 参数化）
//!   - 静态 SQL 字符串（无动态拼接）

use crate::error::app_error::AppError;

/// 校验 SQL 标识符（表名/列名/索引名）是否安全
///
/// 规则：仅允许字母、数字、下划线，且必须以字母或下划线开头
///
/// 返回：
/// - `Ok(())` — 标识符安全
/// - `Err(AppError::Database)` — 标识符包含非法字符
///
/// # 示例
///
/// ```ignore
/// use crate::db::sql_safety::validate_identifier;
///
/// validate_identifier("users")?;           // OK
/// validate_identifier("kb_entries")?;      // OK
/// validate_identifier("users; DROP TABLE users;")?;  // Err
/// validate_identifier("users' OR '1'='1")?;          // Err
/// ```
pub fn validate_identifier(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::Validation(
            "SQL 标识符不能为空".to_string(),
        ));
    }

    // 必须以字母或下划线开头
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(AppError::Validation(format!(
            "SQL 标识符 '{}' 以非法字符开头（必须为字母或下划线）",
            name
        )));
    }

    // 所有字符必须是字母、数字、下划线
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::Validation(format!(
            "SQL 标识符 '{}' 包含非法字符（仅允许字母、数字、下划线）",
            name
        )));
    }

    Ok(())
}

/// 校验并引用 SQL 标识符（用双引号包裹，防 SQL 保留字冲突）
///
/// 与 `validate_identifier` 配合使用：
/// 1. 校验标识符仅包含安全字符
/// 2. 用双引号包裹，避免与 SQL 保留字冲突（如 "order"、"group"）
///
/// # 示例
///
/// ```ignore
/// let safe = quote_identifier("users")?;       // 返回 r#""users""#
/// let safe = quote_identifier("order")?;       // 返回 r#""order""#
/// ```
pub fn quote_identifier(name: &str) -> Result<String, AppError> {
    validate_identifier(name)?;
    // 双引号内转义双引号为两个双引号
    let escaped = name.replace('"', "\"\"");
    Ok(format!("\"{}\"", escaped))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_identifier_accepts_simple_names() {
        assert!(validate_identifier("users").is_ok());
        assert!(validate_identifier("kb_entries").is_ok());
        assert!(validate_identifier("_internal").is_ok());
        assert!(validate_identifier("table_123").is_ok());
    }

    #[test]
    fn test_validate_identifier_rejects_empty() {
        assert!(validate_identifier("").is_err());
    }

    #[test]
    fn test_validate_identifier_rejects_special_chars() {
        assert!(validate_identifier("users; DROP TABLE users;").is_err());
        assert!(validate_identifier("users' OR '1'='1").is_err());
        assert!(validate_identifier("users--").is_err());
        assert!(validate_identifier("users/*comment*/").is_err());
        assert!(validate_identifier("users UNION SELECT * FROM passwords").is_err());
        assert!(validate_identifier("users)").is_err());
        assert!(validate_identifier("(SELECT 1)").is_err());
    }

    #[test]
    fn test_validate_identifier_rejects_leading_digit() {
        assert!(validate_identifier("1users").is_err());
        assert!(validate_identifier("123").is_err());
    }

    #[test]
    fn test_validate_identifier_accepts_underscore_prefix() {
        assert!(validate_identifier("_private").is_ok());
        assert!(validate_identifier("__internal").is_ok());
    }

    #[test]
    fn test_quote_identifier_simple() {
        assert_eq!(quote_identifier("users").unwrap(), "\"users\"");
        assert_eq!(quote_identifier("kb_entries").unwrap(), "\"kb_entries\"");
    }

    #[test]
    fn test_quote_identifier_sql_reserved_word() {
        // SQL 保留字通过双引号引用后可安全使用
        assert_eq!(quote_identifier("order").unwrap(), "\"order\"");
        assert_eq!(quote_identifier("group").unwrap(), "\"group\"");
        assert_eq!(quote_identifier("select").unwrap(), "\"select\"");
    }

    #[test]
    fn test_quote_identifier_rejects_injection() {
        assert!(quote_identifier("users; DROP TABLE users;").is_err());
        assert!(quote_identifier("users' OR '1'='1").is_err());
    }
}
