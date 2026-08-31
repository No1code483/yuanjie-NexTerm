use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("认证错误: {0}")]
    Auth(String),

    #[error("加密错误: {0}")]
    Crypto(String),

    #[error("MEK 解密失败: {0}")]
    MekDecryption(String),

    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    #[error("资源未找到")]
    NotFound,

    #[error("版本冲突: {0}")]
    Conflict(String),

    #[error("文件系统错误: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("AI 调用错误: {0}")]
    AiApi(String),

    #[error("群聊轮次已达上限")]
    RoundLimitReached,

    #[error("Token 预算已耗尽")]
    TokenBudgetExhausted,

    #[error("模型响应超时")]
    ModelTimeout,

    #[error("权限不足: {resource} 需要 {action} 权限")]
    Permission { resource: String, action: String },

    #[error("参数错误: {0}")]
    Validation(String),

    #[error("终端错误: {0}")]
    TerminalError(String),

    #[error("恢复短语验证失败")]
    RecoveryPhraseInvalid,

    #[error("临时账号已过期")]
    TempAccountExpired,

    #[error("备份错误: {0}")]
    Backup(String),

    #[error("内部错误: {0}")]
    Internal(String),

    /// A5 Phase 3 Task 2: 云端 AI 服务离线降级错误
    ///
    /// 触发场景：小欣 / Yuan Code / 游戏 等云端 AI 调用入口在
    /// `AiModelService::check_online_before_call` 检测到 `is_online == false` 时返回。
    ///
    /// 设计依据：.trae/rules/项目核心设计意图.md §八 8.2（可关闭性 / 非侵入式）+
    /// §二（底层智能是优化层，云端 AI 不能在离线时静默切换到本地 ollama）。
    ///
    /// 与底层智能的边界：底层智能（D2）本身使用本地 ollama 模型，离线时继续工作，
    /// **不**通过此错误返回；此错误仅用于云端 AI 服务的友好降级提示。
    #[error("AI 服务离线：{0}")]
    Offline(String),
}

impl AppError {
    pub fn error_code(&self) -> i32 {
        match self {
            AppError::Auth(_) => 1005,
            AppError::Crypto(_) => 6001,
            AppError::MekDecryption(_) => 6002,
            AppError::Database(_) => 2001,
            AppError::NotFound => 2002,
            AppError::Conflict(_) => 2001,
            AppError::FileSystem(_) => 3001,
            AppError::AiApi(_) => 5001,
            AppError::RoundLimitReached => 5003,
            AppError::TokenBudgetExhausted => 5004,
            AppError::ModelTimeout => 5005,
            AppError::Permission { .. } => 1002,
            AppError::Validation(_) => 1003,
            AppError::TerminalError(_) => 4001,
            AppError::RecoveryPhraseInvalid => 1006,
            AppError::TempAccountExpired => 1007,
            AppError::Backup(_) => 7001,
            AppError::Internal(_) => 9001,
            // A5 Phase 3 Task 2: 云端 AI 离线错误码 5006（紧邻 ModelTimeout 5005，归 ai 类）
            AppError::Offline(_) => 5006,
        }
    }

    pub fn category(&self) -> &str {
        match self {
            AppError::Auth(_) | AppError::Permission { .. } | AppError::RecoveryPhraseInvalid | AppError::TempAccountExpired => "auth",
            AppError::Crypto(_) | AppError::MekDecryption(_) => "crypto",
            AppError::Database(_) | AppError::NotFound | AppError::Conflict(_) => "database",
            AppError::FileSystem(_) => "filesystem",
            AppError::AiApi(_) | AppError::RoundLimitReached | AppError::TokenBudgetExhausted | AppError::ModelTimeout | AppError::Offline(_) => "ai",
            AppError::Validation(_) => "validation",
            AppError::TerminalError(_) => "terminal",
            AppError::Backup(_) => "backup",
            AppError::Internal(_) => "internal",
        }
    }
}

impl From<AppError> for String {
    fn from(e: AppError) -> String {
        tracing::error!(error = %e, category = e.category(), code = e.error_code());
        serde_json::to_string(&crate::models::api_response::ApiResponse::<()>::error(e.error_code(), &e.to_string()))
            .unwrap_or_else(|_| format!("{{\"code\":9001,\"message\":\"内部错误\"}}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_code() {
        let err = AppError::Auth("invalid credentials".into());
        assert_eq!(err.error_code(), 1005);
        assert_eq!(err.category(), "auth");
    }

    #[test]
    fn test_crypto_error_code() {
        let err = AppError::Crypto("encryption failed".into());
        assert_eq!(err.error_code(), 6001);
        assert_eq!(err.category(), "crypto");
    }

    #[test]
    fn test_not_found_error() {
        let err = AppError::NotFound;
        assert_eq!(err.error_code(), 2002);
        assert_eq!(err.category(), "database");
    }

    #[test]
    fn test_ai_api_error() {
        let err = AppError::AiApi("model not found".into());
        assert_eq!(err.error_code(), 5001);
        assert_eq!(err.category(), "ai");
    }

    #[test]
    fn test_permission_error() {
        let err = AppError::Permission { resource: "users".into(), action: "write".into() };
        assert_eq!(err.error_code(), 1002);
        assert_eq!(err.category(), "auth");
        assert!(err.to_string().contains("users"));
        assert!(err.to_string().contains("write"));
    }

    #[test]
    fn test_round_limit_error() {
        let err = AppError::RoundLimitReached;
        assert_eq!(err.error_code(), 5003);
        assert_eq!(err.category(), "ai");
    }

    #[test]
    fn test_token_budget_error() {
        let err = AppError::TokenBudgetExhausted;
        assert_eq!(err.error_code(), 5004);
        assert_eq!(err.category(), "ai");
    }

    #[test]
    fn test_terminal_error() {
        let err = AppError::TerminalError("PTY creation failed".into());
        assert_eq!(err.error_code(), 4001);
        assert_eq!(err.category(), "terminal");
    }

    #[test]
    fn test_validation_error() {
        let err = AppError::Validation("email is invalid".into());
        assert_eq!(err.error_code(), 1003);
        assert_eq!(err.category(), "validation");
    }

    #[test]
    fn test_error_to_string_conversion() {
        let err = AppError::Auth("test error".into());
        let str_repr: String = err.into();
        assert!(str_repr.contains("\"code\":1005"));
        assert!(str_repr.contains("test error"));
    }

    #[test]
    fn test_offline_error_code_and_category() {
        // A5 Phase 3 Task 2: Offline 错误码 5006，归 ai 类
        let err = AppError::Offline("AI 服务不可用".into());
        assert_eq!(err.error_code(), 5006);
        assert_eq!(err.category(), "ai");
        assert!(err.to_string().contains("AI 服务离线"));
        assert!(err.to_string().contains("AI 服务不可用"));
    }
}