// 模型重试/降级策略 — 对标 Codex-rs 的 retry 机制
// 提供智能重试、指数退避和提供商标级切换

use std::time::Duration;
use tokio::time::sleep;

/// 重试策略
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_retries: u32,
    /// 初始退避时间 (毫秒)
    pub initial_backoff_ms: u64,
    /// 最大退避时间 (毫秒)
    pub max_backoff_ms: u64,
    /// 退避乘数
    pub backoff_multiplier: f64,
    /// 可重试的错误类型
    pub retryable_errors: Vec<RetryableError>,
}

/// 可重试的错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum RetryableError {
    /// 速率限制 (429)
    RateLimit,
    /// 服务端错误 (5xx)
    ServerError,
    /// 网络错误/超时
    NetworkError,
    /// 模型过载
    ModelOverloaded,
    /// 上下文过长
    ContextLengthExceeded,
}

/// 降级策略
#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    /// 不降级，直接失败
    None,
    /// 切换到备用模型
    SwitchModel(String),
    /// 切换到备用提供商
    SwitchProvider(String),
    /// 缩减上下文后重试
    ReduceContext,
    /// 使用缓存响应
    UseCached,
}

/// 重试执行器
pub struct RetryExecutor {
    policy: RetryPolicy,
    fallback: FallbackStrategy,
    attempt: u32,
}

impl RetryExecutor {
    pub fn new(policy: RetryPolicy, fallback: FallbackStrategy) -> Self {
        Self {
            policy,
            fallback,
            attempt: 0,
        }
    }

    /// 计算当前退避时间
    fn backoff_duration(&self, attempt: u32) -> Duration {
        let ms = (self.policy.initial_backoff_ms as f64
            * self.policy.backoff_multiplier.powi(attempt as i32))
            as u64;
        let capped = ms.min(self.policy.max_backoff_ms);
        Duration::from_millis(capped)
    }

    /// 判断错误是否可重试
    pub fn is_retryable(error_message: &str) -> Option<RetryableError> {
        let msg = error_message.to_lowercase();

        if msg.contains("429") || msg.contains("rate limit") || msg.contains("too many requests") {
            Some(RetryableError::RateLimit)
        } else if msg.contains("500") || msg.contains("502") || msg.contains("503") || msg.contains("504") {
            Some(RetryableError::ServerError)
        } else if msg.contains("timeout") || msg.contains("connection") || msg.contains("network") {
            Some(RetryableError::NetworkError)
        } else if msg.contains("overloaded") || msg.contains("capacity") {
            Some(RetryableError::ModelOverloaded)
        } else if msg.contains("context length") || msg.contains("token limit") || msg.contains("too long") {
            Some(RetryableError::ContextLengthExceeded)
        } else {
            None
        }
    }

    /// 决定降级策略
    pub fn determine_fallback(&self, error: &RetryableError) -> FallbackStrategy {
        match error {
            RetryableError::RateLimit => {
                // 速率限制 → 先等待，再切换提供商
                if self.attempt < self.policy.max_retries / 2 {
                    FallbackStrategy::None
                } else {
                    self.fallback.clone()
                }
            }
            RetryableError::ServerError => {
                // 服务端错误 → 切换提供商
                self.fallback.clone()
            }
            RetryableError::NetworkError => {
                // 网络错误 → 重试
                FallbackStrategy::None
            }
            RetryableError::ModelOverloaded => {
                // 模型过载 → 切换模型
                FallbackStrategy::SwitchModel("gpt-4o-mini".into())
            }
            RetryableError::ContextLengthExceeded => {
                // 上下文过长 → 缩减上下文
                FallbackStrategy::ReduceContext
            }
        }
    }

    /// 执行带重试的异步操作
    pub async fn execute<F, Fut, T, E>(
        &mut self,
        mut operation: F,
    ) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        loop {
            self.attempt += 1;

            match operation().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    let error_str = e.to_string();

                    if let Some(_retryable) = Self::is_retryable(&error_str) {
                        if self.attempt <= self.policy.max_retries {
                            let backoff = self.backoff_duration(self.attempt - 1);
                            eprintln!(
                                "模型调用失败 (attempt {}/{}), {} 后重试: {}",
                                self.attempt,
                                self.policy.max_retries,
                                backoff.as_secs_f64(),
                                error_str,
                            );
                            sleep(backoff).await;
                            continue;
                        }
                    }

                    return Err(e);
                }
            }
        }
    }
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 30000,
            backoff_multiplier: 2.0,
            retryable_errors: vec![
                RetryableError::RateLimit,
                RetryableError::ServerError,
                RetryableError::NetworkError,
                RetryableError::ModelOverloaded,
                RetryableError::ContextLengthExceeded,
            ],
        }
    }
}

impl Default for FallbackStrategy {
    fn default() -> Self {
        FallbackStrategy::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_rate_limit() {
        assert_eq!(
            RetryExecutor::is_retryable("429 Too Many Requests"),
            Some(RetryableError::RateLimit)
        );
    }

    #[test]
    fn test_is_retryable_server_error() {
        assert_eq!(
            RetryExecutor::is_retryable("502 Bad Gateway"),
            Some(RetryableError::ServerError)
        );
    }

    #[test]
    fn test_is_retryable_context_length() {
        assert_eq!(
            RetryExecutor::is_retryable("context length exceeded token limit"),
            Some(RetryableError::ContextLengthExceeded)
        );
    }

    #[test]
    fn test_not_retryable() {
        assert_eq!(RetryExecutor::is_retryable("invalid API key"), None);
    }
}