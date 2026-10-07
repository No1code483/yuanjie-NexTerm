//! boards.home 的 L2 Feature 插件集合。
//! 批次1b-1：home.todo / home.journal。
//! 批次1b-2a：home.timer。
//! 批次1b-2b-1：home.news。
//! 批C4：home.focus（专注，可选子插件，纯前端无命令）。

pub mod focus;
pub mod journal;
pub mod news;
pub mod timer;
pub mod todo;
