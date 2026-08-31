use std::time::Duration;
use tokio::time::timeout;

use crate::error::app_error::AppError;
use super::client::McpClient;

/// MCP 服务器生命周期状态
#[derive(Debug, Clone, PartialEq)]
pub enum McpLifecycleState {
    /// 未连接
    Disconnected,
    /// 连接中
    Connecting,
    /// 已连接
    Connected,
    /// 健康检查中
    HealthChecking,
    /// 不健康（需要重连）
    Unhealthy,
    /// 重连中
    Reconnecting,
    /// 已关闭
    Shutdown,
}

/// 生命周期配置
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    /// 连接超时（毫秒）
    pub connect_timeout_ms: u64,
    /// 健康检查间隔（毫秒）
    pub health_check_interval_ms: u64,
    /// 健康检查超时（毫秒）
    pub health_check_timeout_ms: u64,
    /// 最大重连次数
    pub max_reconnect_attempts: u32,
    /// 重连间隔（毫秒）
    pub reconnect_interval_ms: u64,
    /// 优雅关闭超时（毫秒）
    pub graceful_shutdown_timeout_ms: u64,
    /// 是否启用健康检查
    pub health_check_enabled: bool,
    /// 是否启用自动重连
    pub auto_reconnect: bool,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            connect_timeout_ms: 30_000,
            health_check_interval_ms: 30_000,
            health_check_timeout_ms: 5_000,
            max_reconnect_attempts: 3,
            reconnect_interval_ms: 5_000,
            graceful_shutdown_timeout_ms: 10_000,
            health_check_enabled: true,
            auto_reconnect: true,
        }
    }
}

/// MCP 服务器生命周期管理器
pub struct McpLifecycleManager {
    config: LifecycleConfig,
    state: McpLifecycleState,
    reconnect_count: u32,
    last_health_check: Option<std::time::Instant>,
    consecutive_failures: u32,
}

impl McpLifecycleManager {
    pub fn new(config: Option<LifecycleConfig>) -> Self {
        Self {
            config: config.unwrap_or_default(),
            state: McpLifecycleState::Disconnected,
            reconnect_count: 0,
            last_health_check: None,
            consecutive_failures: 0,
        }
    }

    pub fn state(&self) -> &McpLifecycleState {
        &self.state
    }

    pub fn config(&self) -> &LifecycleConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: LifecycleConfig) {
        self.config = config;
    }

    /// 开始连接
    pub fn begin_connect(&mut self) {
        self.state = McpLifecycleState::Connecting;
        self.reconnect_count = 0;
        self.consecutive_failures = 0;
    }

    /// 连接成功
    pub fn on_connected(&mut self) {
        self.state = McpLifecycleState::Connected;
        self.last_health_check = Some(std::time::Instant::now());
    }

    /// 连接失败
    pub fn on_connect_failed(&mut self) -> bool {
        self.consecutive_failures += 1;

        if self.config.auto_reconnect
            && self.reconnect_count < self.config.max_reconnect_attempts
        {
            self.state = McpLifecycleState::Reconnecting;
            self.reconnect_count += 1;
            true
        } else {
            self.state = McpLifecycleState::Disconnected;
            false
        }
    }

    /// 执行健康检查
    pub async fn health_check(&mut self, client: &mut McpClient) -> Result<(), AppError> {
        if !self.config.health_check_enabled {
            return Ok(());
        }

        self.state = McpLifecycleState::HealthChecking;

        let result = timeout(
            Duration::from_millis(self.config.health_check_timeout_ms),
            client.ping(),
        )
        .await;

        match result {
            Ok(Ok(())) => {
                self.state = McpLifecycleState::Connected;
                self.last_health_check = Some(std::time::Instant::now());
                self.consecutive_failures = 0;
                Ok(())
            }
            _ => {
                self.state = McpLifecycleState::Unhealthy;
                self.consecutive_failures += 1;

                if self.config.auto_reconnect
                    && self.reconnect_count < self.config.max_reconnect_attempts
                {
                    self.state = McpLifecycleState::Reconnecting;
                    Ok(())
                } else {
                    Err(AppError::Internal(format!(
                        "MCP 服务器健康检查失败，连续失败 {} 次",
                        self.consecutive_failures
                    )))
                }
            }
        }
    }

    /// 需要健康检查？
    pub fn needs_health_check(&self) -> bool {
        if !self.config.health_check_enabled {
            return false;
        }

        if self.state != McpLifecycleState::Connected {
            return false;
        }

        match self.last_health_check {
            Some(last) => {
                last.elapsed().as_millis() as u64 >= self.config.health_check_interval_ms
            }
            None => true,
        }
    }

    /// 优雅关闭
    pub async fn graceful_shutdown(&mut self, client: &mut McpClient) {
        self.state = McpLifecycleState::Shutdown;

        let result = timeout(
            Duration::from_millis(self.config.graceful_shutdown_timeout_ms),
            client.shutdown(),
        )
        .await;

        if result.is_err() {
            // 超时，强制关闭
            client.force_kill().await;
        }

        self.state = McpLifecycleState::Disconnected;
    }

    /// 获取重连延迟
    pub fn reconnect_delay(&self) -> Duration {
        Duration::from_millis(self.config.reconnect_interval_ms)
    }

    /// 重置重连计数
    pub fn reset_reconnect(&mut self) {
        self.reconnect_count = 0;
        self.consecutive_failures = 0;
    }
}

/// 连接配置（带超时）
pub async fn connect_with_timeout(
    client: &mut McpClient,
    command: &str,
    args: &[String],
    env: Option<&std::collections::HashMap<String, String>>,
    timeout_ms: u64,
) -> Result<(), AppError> {
    timeout(
        Duration::from_millis(timeout_ms),
        client.connect(command, args, env),
    )
    .await
    .map_err(|_| AppError::Internal(format!(
        "MCP 服务器连接超时 ({}ms)",
        timeout_ms
    )))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_state_transitions() {
        let mut mgr = McpLifecycleManager::new(None);
        assert_eq!(*mgr.state(), McpLifecycleState::Disconnected);

        mgr.begin_connect();
        assert_eq!(*mgr.state(), McpLifecycleState::Connecting);

        mgr.on_connected();
        assert_eq!(*mgr.state(), McpLifecycleState::Connected);

        let should_retry = mgr.on_connect_failed();
        assert!(should_retry);
        assert_eq!(*mgr.state(), McpLifecycleState::Reconnecting);
    }

    #[test]
    fn test_max_reconnect_attempts() {
        let config = LifecycleConfig {
            max_reconnect_attempts: 2,
            ..Default::default()
        };
        let mut mgr = McpLifecycleManager::new(Some(config));

        mgr.begin_connect();
        mgr.on_connected();

        // 第一次失败：应该重试 (count=0 < 2)
        assert!(mgr.on_connect_failed());
        assert_eq!(*mgr.state(), McpLifecycleState::Reconnecting);

        // 模拟重连成功
        mgr.on_connected();

        // 第二次失败：应该重试 (count=1 < 2)
        assert!(mgr.on_connect_failed());

        // 模拟重连成功
        mgr.on_connected();

        // 第三次失败：不应再重试 (count=2, 2 < 2 = false)
        assert!(!mgr.on_connect_failed());
        assert_eq!(*mgr.state(), McpLifecycleState::Disconnected);
    }

    #[test]
    fn test_needs_health_check() {
        let config = LifecycleConfig {
            health_check_interval_ms: 0, // 立即需要
            ..Default::default()
        };
        let mut mgr = McpLifecycleManager::new(Some(config));

        mgr.begin_connect();
        mgr.on_connected();

        assert!(mgr.needs_health_check());
    }
}