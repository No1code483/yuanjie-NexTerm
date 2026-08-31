# NexTerm・元界 构建说明（BUILD.md）

> 适用于开源版 v1.0.0.0。以下命令均在 **Windows** 环境验证。

## 环境准备

| 依赖 | 版本要求 | 说明 |
|------|---------|------|
| Node.js | ≥ 20 | 前端构建 |
| Rust | stable（1.8+） | 后端编译，需包含 `cargo` |
| Tauri 系统依赖 | Windows 10/11 | WebView2 随系统自带；桌面构建需 VC++ Build Tools |

## 安装依赖

```bash
# 前端
cd 前端
npm install

# 后端（Tauri CLI）
cd ../后端
npm install
```

## 开发模式运行

```bash
cd 后端
npm run tauri dev
```

首次运行会同时编译 Rust 后端（数分钟）。应用启动后：

- 首次启动创建数据库（`%APPDATA%/com.nexterm.app/nexterm.db`）
- 默认无登录用户，可注册本地账号

## 生产构建

```bash
cd 后端
npm run tauri build
```

产物输出到 `后端/src-tauri/target/release/bundle/`（NSIS 安装包 / 免安装 exe）。

## 各模块自检

```bash
# 前端类型检查
cd 前端 && npx tsc --noEmit

# 后端集成测试（跳过 lib 单元测试 target 以规避 Windows 工具链限制）
cd 后端/src-tauri && cargo test --test '*'
```

## 常见问题

### 数据库迁移

数据库迁移由后端启动时自动执行（`db/migrations.rs`），版本 001-123；升级应用时旧库自动迁移，无需手动操作。

### 无 WebView2

Windows 10/11 通常已内置 WebView2。若缺失，从 Microsoft 官方安装 WebView2 Runtime。

### AI 能力未生效

各 AI 模块默认关闭或需要配置 API Key / 本地模型（如 Ollama），见应用内「设置 → AI 模型管理」。

> 开源版不捆绑任何第三方 API 密钥，首次使用请自行配置。