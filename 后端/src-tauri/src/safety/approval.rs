//! 安全审批工作流引擎
//!
//! 当 AI 请求执行敏感操作时，触发审批流程：
//! - 自动审批: 低风险操作
//! - 用户确认: 中高风险操作
//! - 自动拒绝: 严重风险操作

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::models::safety::RiskLevel;

/// 审批请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// 请求 ID
    pub id: String,
    /// 请求类型
    pub request_type: ApprovalType,
    /// 操作描述
    pub description: String,
    /// 操作目标（文件路径、命令等）
    pub target: String,
    /// 风险评估
    pub risk_level: RiskLevel,
    /// 风险分数
    pub risk_score: f64,
    /// 请求来源 (Agent ID)
    pub source: String,
    /// 请求时间
    pub timestamp: i64,
    /// 附加上下文
    pub context: HashMap<String, String>,
    /// 超时时间 (ms)
    pub timeout_ms: u64,
}

/// 审批类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ApprovalType {
    /// 文件写入
    FileWrite,
    /// 文件删除
    FileDelete,
    /// 命令执行
    CommandExecute,
    /// 网络请求
    NetworkRequest,
    /// 系统配置修改
    SystemConfig,
    /// 环境变量修改
    EnvModify,
    /// 包安装
    PackageInstall,
    /// 密钥访问
    SecretAccess,
    /// 数据库操作
    DatabaseOperation,
    /// 自定义
    Custom(String),
}

impl ApprovalType {
    /// 默认审批策略
    pub fn default_policy(&self) -> ApprovalPolicy {
        match self {
            ApprovalType::FileWrite => ApprovalPolicy::AskUser,
            ApprovalType::FileDelete => ApprovalPolicy::AskUser,
            ApprovalType::CommandExecute => ApprovalPolicy::AskUser,
            ApprovalType::NetworkRequest => ApprovalPolicy::AutoApprove,
            ApprovalType::SystemConfig => ApprovalPolicy::AskUser,
            ApprovalType::EnvModify => ApprovalPolicy::AskUser,
            ApprovalType::PackageInstall => ApprovalPolicy::AskUser,
            ApprovalType::SecretAccess => ApprovalPolicy::AutoDeny,
            ApprovalType::DatabaseOperation => ApprovalPolicy::AskUser,
            ApprovalType::Custom(_) => ApprovalPolicy::AskUser,
        }
    }
}

/// 审批策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPolicy {
    /// 自动批准
    AutoApprove,
    /// 自动拒绝
    AutoDeny,
    /// 询问用户
    AskUser,
}

/// 审批结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalResult {
    /// 已批准
    Approved {
        approved_by: String,
        timestamp: i64,
        note: Option<String>,
    },
    /// 已拒绝
    Denied {
        denied_by: String,
        timestamp: i64,
        reason: String,
    },
    /// 已超时
    Timeout {
        timestamp: i64,
    },
    /// 已取消
    Cancelled {
        timestamp: i64,
        reason: String,
    },
}

/// 审批工作流引擎
#[derive(Debug)]
pub struct ApprovalEngine {
    /// 待审批请求
    pending: HashMap<String, ApprovalRequest>,
    /// 已处理请求
    history: Vec<ApprovalRecord>,
    /// 审批策略覆盖
    policy_overrides: HashMap<ApprovalType, ApprovalPolicy>,
    /// 自动批准的风险阈值
    auto_approve_threshold: f64,
    /// 自动拒绝的风险阈值
    auto_deny_threshold: f64,
    /// 审批超时时间 (ms)
    #[allow(dead_code)]
    default_timeout_ms: u64,
}

/// 审批记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRecord {
    pub request: ApprovalRequest,
    pub result: ApprovalResult,
}

impl ApprovalEngine {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
            history: Vec::new(),
            policy_overrides: HashMap::new(),
            auto_approve_threshold: 0.3,
            auto_deny_threshold: 0.7,
            default_timeout_ms: 60_000, // 60 seconds
        }
    }

    /// 设置审批策略覆盖
    pub fn set_policy(&mut self, approval_type: ApprovalType, policy: ApprovalPolicy) {
        self.policy_overrides.insert(approval_type, policy);
    }

    /// 提交审批请求
    pub fn submit(&mut self, request: ApprovalRequest) -> ApprovalResult {
        // 检查风险等级自动决策
        if request.risk_score <= self.auto_approve_threshold {
            let result = ApprovalResult::Approved {
                approved_by: "auto".into(),
                timestamp: now_ms(),
                note: Some("低风险自动批准".into()),
            };
            self.history.push(ApprovalRecord {
                request: request.clone(),
                result: result.clone(),
            });
            return result;
        }

        if request.risk_score >= self.auto_deny_threshold {
            let result = ApprovalResult::Denied {
                denied_by: "auto".into(),
                timestamp: now_ms(),
                reason: "高风险自动拒绝".into(),
            };
            self.history.push(ApprovalRecord {
                request: request.clone(),
                result: result.clone(),
            });
            return result;
        }

        // 检查策略覆盖
        let policy = self
            .policy_overrides
            .get(&request.request_type)
            .copied()
            .unwrap_or_else(|| request.request_type.default_policy());

        match policy {
            ApprovalPolicy::AutoApprove => {
                let result = ApprovalResult::Approved {
                    approved_by: "policy".into(),
                    timestamp: now_ms(),
                    note: Some("策略自动批准".into()),
                };
                self.history.push(ApprovalRecord {
                    request: request.clone(),
                    result: result.clone(),
                });
                result
            }
            ApprovalPolicy::AutoDeny => {
                let result = ApprovalResult::Denied {
                    denied_by: "policy".into(),
                    timestamp: now_ms(),
                    reason: "策略自动拒绝".into(),
                };
                self.history.push(ApprovalRecord {
                    request: request.clone(),
                    result: result.clone(),
                });
                result
            }
            ApprovalPolicy::AskUser => {
                // 加入待审批队列
                self.pending.insert(request.id.clone(), request);
                // 返回待处理状态（前端会显示审批弹窗）
                ApprovalResult::Approved {
                    approved_by: "pending".into(),
                    timestamp: now_ms(),
                    note: Some("等待用户审批".into()),
                }
            }
        }
    }

    /// 用户批准请求
    pub fn approve(&mut self, request_id: &str, user: &str) -> Option<ApprovalResult> {
        if let Some(request) = self.pending.remove(request_id) {
            let result = ApprovalResult::Approved {
                approved_by: user.to_string(),
                timestamp: now_ms(),
                note: None,
            };
            self.history.push(ApprovalRecord {
                request,
                result: result.clone(),
            });
            Some(result)
        } else {
            None
        }
    }

    /// 用户拒绝请求
    pub fn deny(&mut self, request_id: &str, user: &str, reason: String) -> Option<ApprovalResult> {
        if let Some(request) = self.pending.remove(request_id) {
            let result = ApprovalResult::Denied {
                denied_by: user.to_string(),
                timestamp: now_ms(),
                reason,
            };
            self.history.push(ApprovalRecord {
                request,
                result: result.clone(),
            });
            Some(result)
        } else {
            None
        }
    }

    /// 获取待审批请求列表
    pub fn pending_requests(&self) -> Vec<&ApprovalRequest> {
        self.pending.values().collect()
    }

    /// 获取审批历史
    pub fn history(&self) -> &[ApprovalRecord] {
        &self.history
    }

    /// 清理超时的待审批请求
    pub fn cleanup_timeouts(&mut self) -> Vec<String> {
        let now = now_ms();
        let mut timed_out = Vec::new();

        self.pending.retain(|id, req| {
            let elapsed = now - req.timestamp;
            if elapsed > req.timeout_ms as i64 {
                timed_out.push(id.clone());
                self.history.push(ApprovalRecord {
                    request: req.clone(),
                    result: ApprovalResult::Timeout {
                        timestamp: now,
                    },
                });
                false
            } else {
                true
            }
        });

        timed_out
    }

    /// 获取待审批数量
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// 获取历史记录数量
    pub fn history_count(&self) -> usize {
        self.history.len()
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}