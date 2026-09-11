# NexTerm・元界

<p align="center">
  <strong>终端风格的一体化桌面应用</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/React-18-blue" alt="React" />
  <img src="https://img.shields.io/badge/TypeScript-5.3-blue" alt="TypeScript" />
  <img src="https://img.shields.io/badge/Vite-7.3-purple" alt="Vite" />
  <img src="https://img.shields.io/badge/Tauri-2.x-orange" alt="Tauri" />
  <img src="https://img.shields.io/badge/Rust-1.8+-brown" alt="Rust" />
  <img src="https://img.shields.io/badge/xterm.js-6.0-green" alt="xterm" />
  <img src="https://img.shields.io/badge/版本-v1.0.4-blueviolet" alt="Version" />
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License" />
</p>

---

## 📖 项目简介

**NexTerm・元界** 是一款采用**终端黑客美学（Terminal Hacker Aesthetic）**设计的桌面应用，面向开发者和技术爱好者。项目融合 AI 对话、知识管理、终端模拟、在线笔记、游戏化学习等多种能力，提供独特的命令行风格交互体验。

> 本仓库为**开源发布版**，仅包含应用本体源代码与构建说明。开发过程中的规划文档、测试材料、设计素材等不在此公开。
>
> **v1.0.4** 修复打包生产环境资源加载失败：路由懒加载改为字面量动态导入（修复登录后 Home 页白屏）；Monaco Editor 与 PDF.js worker 从 CDN 改为本地依赖加载；移除 Google favicon 外链与不存在的 /vite.svg 引用；生产构建关闭 sourcemap。

## ✨ 核心功能

- **终端模拟**：基于 xterm.js 的完整终端能力（本地 Shell / PowerShell / WSL / SSH 会话）
- **AI 对话**：多模型管理、Agent、群聊编排、提示词模板
- **知识库**：分类/标签/全文搜索/双链图谱/文档快照
- **Yuan Code**：代码编辑器、AI 补全、Skill、MCP 生态、Agent 自主执行
- **小欣 AI 助手**：记忆、人格、情感、语音、日程
- **游戏化学习**：3D 世界、闯关突破、知识联动
- **数据安全**：本地加密存储（argon2id + AES-GCM + MEK 密钥轮换）、认证、2FA
- **离线同步**：多设备同步队列 + ECDH 端到端加密
- **回收站 / 全局搜索 / 底层智能建议** 等横切能力

## 🧱 技术栈

| 层 | 技术 |
|----|------|
| 前端 | React 18 + TypeScript + Vite + Zustand + xterm.js |
| 后端 | Rust + Tauri 2.x + SQLite (sqlx) |
| 加密 | argon2id / AES-256-GCM / MEK 轮换 / ECDH (x25519) |
| 构建 | Vite + Cargo |

## 🚀 快速开始

### 环境要求

- Node.js ≥ 20
- Rust stable（1.8+）
- Tauri 2 系统依赖（Windows 需 WebView2，随系统自带）

### 本地开发

```bash
# 1. 安装前端依赖（依赖树含 legacy peer 组合，需加 --legacy-peer-deps）
cd 前端
npm install --legacy-peer-deps

# 2. 安装后端 Tauri CLI
cd ../后端
npm install

# 3. 启动桌面应用（编译 + 运行）
cd 后端
npm run tauri dev
```

### 构建安装包

```bash
cd 后端
npm run tauri build
```

> 详细构建说明见 [BUILD.md](BUILD.md)。

## 📁 目录结构

```
├── 前端/                 # React/Vite 前端源码
│   ├── src/              # 组件、页面、状态、IPC 封装
│   └── package.json
└── 后端/
    ├── package.json      # Tauri CLI 入口
    └── src-tauri/        # Rust 后端 + Tauri 配置
        ├── src/          # commands / services / db / crypto / engine 等
        ├── migrations/   # SQLite 迁移（001-123）
        └── tauri.conf.json
```

## 🏛️ 架构概览

- **IPC**：前端通过 `lib/ipc/` 统一封装调用 Rust commands（`generate_handler` 静态注册，864+ 命令）
- **数据层**：SQLite + sqlx，迁移编号管理（`db/migrations.rs`），加密字段走 `crypto/`
- **多用户**：认证体系（注册/登录/2FA/会话恢复），数据按 user_id 隔离
- **AI 边界**：应用内各 AI 模块支持接入云端 API 或本地模型（如 Ollama），可配置可关闭

## 📄 License

本项目基于 [MIT License](LICENSE) 开源。

## ⚠️ 声明

- 本仓库不含 **API Key / 数据库文件 / 用户数据**
- 隐私与安全相关：所有密钥仅本地保存，传输走 E2E 加密
- 如需二次开发，请遵守 MIT 条款并保留版权声明