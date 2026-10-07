//! boards.terminal 的 L2 Feature 插件集合。
//! 阶段3 批次3b：terminal.linux（Linux 子系统 / Docker，25 命令，短码 lx）。
//! 阶段3 批次3c：terminal.yuancode（Yuan Code 编辑器核心，54 命令，短码 yc）。
//! 批C2：terminal.manual（命令手册 5 路由，纯前端无命令）。
//! 批C4：terminal.console（终端命令行）/ terminal.mux（标签页与分屏）/
//! terminal.tools（命令辅助工具），均为纯前端 L2 子插件（无命令）。

pub mod console;
pub mod linux;
pub mod manual;
pub mod mux;
pub mod tools;
pub mod yuancode;
