# NexTerm・元界

<p align="center">
  <strong>终端黑客美学的 AI 元界工作站 · 第二代（v2 最小内核 + 三级插件）</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/React-19-blue" alt="React" />
  <img src="https://img.shields.io/badge/TypeScript-5.3-blue" alt="TypeScript" />
  <img src="https://img.shields.io/badge/Vite-7.3-purple" alt="Vite" />
  <img src="https://img.shields.io/badge/Tauri-2.x-orange" alt="Tauri" />
  <img src="https://img.shields.io/badge/Rust-stable-brown" alt="Rust" />
  <img src="https://img.shields.io/badge/xterm.js-6.0-green" alt="xterm" />
  <img src="https://img.shields.io/badge/主题-11_套预设-blue" alt="Themes" />
  <img src="https://img.shields.io/badge/版本-v2.0.0-blueviolet" alt="Version" />
  <img src="https://img.shields.io/badge/License-MIT-green" alt="License" />
</p>

---

## 📖 项目简介

**NexTerm・元界** 是一款**终端黑客美学的 AI 元界工作站**，融合传统终端的专注感与现代 AI 工具的活力感，面向开发者和技术爱好者。项目以**最小内核 + 三级插件架构**（第二代 v2）组织 19 个功能模块：终端模拟、AI 对话、知识管理、代码编辑、游戏化学习、离线同步等，同时提供 11 套预设主题的完整设计系统。

> 本仓库为**开源发布版**，仅包含应用本体源代码与构建说明。开发过程中的规划文档、测试材料、设计素材等不在此公开。
>
> **v2.0.0**（2026-10-07）为**插件化重构完成后的首个开源主版本**：应用架构升级为「最小内核 + 三级插件体系」（后端 `crates/kernel` 统一 dispatcher + 插件 `IPC_ALIASES` 别名分发，859 条前端可达命令）；前端全板块细化为可独立启停的 L2/L3 子插件；19 个功能模块全部可用（完成度 93.7%–100%）。

## ✨ 核心功能

- **终端模拟**：基于 xterm.js 的完整终端能力（本地 Shell / PowerShell / WSL / SSH 会话）
- **AI 对话**：多模型管理、Agent、群聊编排、云端 API、模型路由、协作会话
- **知识库**：分类/标签/全文搜索/双链图谱/文档快照/模板
- **Yuan Code**：代码编辑器、AI 补全、Skill、MCP 生态、Agent 自主执行、沙箱
- **小欣 AI 助手**：记忆、人格、情感、语音、多模态、健康管理
- **游戏化学习**：3D 世界、闯关突破、知识联动、NPC 智能
- **数据安全**：本地加密存储（argon2id + AES-GCM + MEK 密钥轮换）、认证、2FA、多用户隔离
- **离线同步**：多设备同步队列 + ECDH 端到端加密 + 冲突解决
- **多主题设计系统**：11 套预设主题（terminal 默认 / matrix / dracula / monokai / one-dark / nord / solarized / github / high-contrast）+ 自定义主题编辑器
- **回收站 / 全局搜索 / 底层智能建议 / 插件管理** 等横切能力

## 🧱 技术栈

| 层 | 技术 |
|----|------|
| 前端 | React 19 + TypeScript + Vite + Zustand + xterm.js + CSS Modules（双变量主题系统） |
| 后端 | Rust + Tauri 2.x + SQLite (sqlx) + 最小内核（kernel / kernel-api）+ 三级插件 |
| 加密 | argon2id / AES-256-GCM / MEK 轮换 / ECDH (x25519) |
| 构建 | Vite + Cargo（Tauri CLI） |

## 🚀 快速开始

### 环境要求

- Node.js ≥ 20
- Rust stable（较新稳定版，Tauri 2 系列对 MSRV 有要求）
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
npm run tauri build -- --bundles nsis
```

> 详细构建说明见 [BUILD.md](BUILD.md)。

## 📁 目录结构

```
├── 前端/                 # React/Vite 前端源码
│   ├── src/
│   │   ├── kernel/       # 最小内核（插件注册表 / 插槽渲染 / 事件总线 / 横切状态）
│   │   ├── plugins/      # 三级插件（boards 板块 + 内嵌 features 子插件 / customs 定制）
│   │   └── ...           # 组件、IPC 封装、i18n、主题样式
│   └── package.json
└── 后端/
    ├── package.json      # Tauri CLI 入口
    └── src-tauri/        # Rust 后端 + Tauri 配置
        ├── crates/       # kernel（最小内核）+ kernel-api（契约）
        ├── src/plugins/  # 三级插件（boards / customs / _legacy 收编区）
        ├── migrations/   # SQLite 迁移（109 个迁移单元，编号至 0123）
        └── tauri.conf.json
```

## 🏛️ 架构概览

- **IPC**：唯一入口 `kernel_dispatch`（`generate_handler!` 仅注册此命令）；业务命令以 `<短码/插件ID>:plugin:<命令名>` 别名经插件 `IPC_ALIASES` 分发（前端可达 859 条，2026-10-05 实测）
- **插件体系**：L1 板块（home / ai / knowledge / terminal / xin / game…）/ L2 功能（parent + slot 挂载，含嵌套插槽）/ L3 定制；插件支持独立启停（可逆注册），插件管理页按层级钻取
- **数据层**：SQLite + sqlx，迁移编号管理（`db/migrations.rs` 注册表），加密字段走 `crypto/`
- **多用户**：认证体系（注册 / 登录 / 2FA / 会话恢复），43 张用户私有表按 user_id 隔离
- **AI 边界**：应用内各 AI 模块支持接入云端 API 或本地模型（如 Ollama），可配置可关闭

## 📄 License

本项目基于 [MIT License](LICENSE) 开源。

## ⚠️ 声明

- 本仓库不含 **API Key / 数据库文件 / 用户数据**
- 隐私与安全相关：所有密钥仅本地保存，传输走 E2E 加密
- 如需二次开发，请遵守 MIT 条款并保留版权声明