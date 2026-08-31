//! 项目模板

use serde::{Deserialize, Serialize};

/// 项目模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTemplate {
    /// 模板 ID
    pub id: String,
    /// 模板名称
    pub name: String,
    /// 模板描述
    pub description: String,
    /// 项目类型
    pub project_type: String,
    /// 模板文件 (文件名 -> 内容)
    pub files: Vec<TemplateFile>,
    /// 模板标签
    pub tags: Vec<String>,
}

/// 模板文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFile {
    /// 文件路径
    pub path: String,
    /// 文件内容
    pub content: String,
    /// 是否为可执行文件
    pub executable: bool,
}

/// 内置模板
pub fn builtin_templates() -> Vec<ProjectTemplate> {
    vec![
        ProjectTemplate {
            id: "rust-lib".into(),
            name: "Rust Library".into(),
            description: "Rust 库项目模板".into(),
            project_type: "Rust".into(),
            tags: vec!["rust".into(), "library".into()],
            files: vec![
                TemplateFile {
                    path: "Cargo.toml".into(),
                    content: r#"[package]
name = "my-lib"
version = "0.1.0"
edition = "2021"

[dependencies]
"#
                    .into(),
                    executable: false,
                },
                TemplateFile {
                    path: "src/lib.rs".into(),
                    content: r#"pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
"#
                    .into(),
                    executable: false,
                },
            ],
        },
        ProjectTemplate {
            id: "python-cli".into(),
            name: "Python CLI".into(),
            description: "Python 命令行工具模板".into(),
            project_type: "Python".into(),
            tags: vec!["python".into(), "cli".into()],
            files: vec![
                TemplateFile {
                    path: "main.py".into(),
                    content: r#"#!/usr/bin/env python3
"""CLI tool description."""

import argparse
import sys


def main() -> int:
    parser = argparse.ArgumentParser(description="CLI tool")
    parser.add_argument("--name", default="World", help="Name to greet")
    args = parser.parse_args()

    print(f"Hello, {args.name}!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
"#
                    .into(),
                    executable: true,
                },
                TemplateFile {
                    path: "requirements.txt".into(),
                    content: "# Add your dependencies here\n".into(),
                    executable: false,
                },
            ],
        },
        ProjectTemplate {
            id: "web-html".into(),
            name: "Web Page".into(),
            description: "简单 HTML/CSS/JS 网页模板".into(),
            project_type: "Web".into(),
            tags: vec!["web".into(), "html".into(), "css".into()],
            files: vec![
                TemplateFile {
                    path: "index.html".into(),
                    content: r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>My Page</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <h1>Hello, World!</h1>
    <script src="main.js"></script>
</body>
</html>
"#
                    .into(),
                    executable: false,
                },
                TemplateFile {
                    path: "style.css".into(),
                    content: r#"* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: system-ui, sans-serif;
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: 100vh;
    background: #1a1a2e;
    color: #eee;
}
"#
                    .into(),
                    executable: false,
                },
                TemplateFile {
                    path: "main.js".into(),
                    content: r#"document.addEventListener('DOMContentLoaded', () => {
    console.log('Page loaded!');
});
"#
                    .into(),
                    executable: false,
                },
            ],
        },
    ]
}