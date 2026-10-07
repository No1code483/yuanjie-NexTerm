//! terminal.yuancode 的 L3 Feature 插件集合（YuanCode 三级嵌套子插件）。
//! 六个纯前端子插件（无命令无表），均挂载于 L2 terminal.yuancode 之下：
//! editor / agent / git / skills / sandbox / settings。

pub mod agent;
pub mod editor;
pub mod git;
pub mod sandbox;
pub mod settings;
pub mod skills;
