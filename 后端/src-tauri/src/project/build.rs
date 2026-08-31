//! 构建配置

use serde::{Deserialize, Serialize};

/// 构建配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// 构建命令
    pub build_command: String,
    /// 运行命令
    pub run_command: String,
    /// 测试命令
    pub test_command: String,
    /// 输出目录
    pub output_dir: Option<String>,
    /// 构建参数
    pub args: Vec<String>,
    /// 环境变量
    pub env: Vec<(String, String)>,
    /// 构建前任务
    pub pre_build_tasks: Vec<String>,
    /// 构建后任务
    pub post_build_tasks: Vec<String>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            build_command: String::new(),
            run_command: String::new(),
            test_command: String::new(),
            output_dir: None,
            args: Vec::new(),
            env: Vec::new(),
            pre_build_tasks: Vec::new(),
            post_build_tasks: Vec::new(),
        }
    }
}

impl BuildConfig {
    /// Rust 项目默认构建配置
    pub fn rust_default() -> Self {
        Self {
            build_command: "cargo build".into(),
            run_command: "cargo run".into(),
            test_command: "cargo test".into(),
            output_dir: Some("target/debug".into()),
            ..Default::default()
        }
    }

    /// Python 项目默认构建配置
    pub fn python_default() -> Self {
        Self {
            build_command: "python -m compileall .".into(),
            run_command: "python main.py".into(),
            test_command: "python -m pytest".into(),
            output_dir: Some("__pycache__".into()),
            ..Default::default()
        }
    }

    /// JavaScript/TypeScript 项目默认构建配置
    pub fn js_default() -> Self {
        Self {
            build_command: "npm run build".into(),
            run_command: "npm start".into(),
            test_command: "npm test".into(),
            output_dir: Some("dist".into()),
            ..Default::default()
        }
    }
}