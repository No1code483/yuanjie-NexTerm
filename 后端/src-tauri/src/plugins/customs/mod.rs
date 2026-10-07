//! 定制级插件（L3）集合。
//! 阶段3 批次1a-1：customs.auth 完成 registry/manifest/migration 适配。
//! 阶段3 批次4c：customs.systemtools 完成 registry/manifest/migration 适配。
//! 阶段4 批次6a：customs.recycle 完成 registry/manifest/migration 适配。
//! 阶段4 批次6b：customs.sync + customs.search 完成 registry/manifest/migration 适配。
//! 阶段4 批次6c：customs.intelligence 完成 registry/manifest/migration 适配。

pub mod auth;
pub mod intelligence;
pub mod recycle;
pub mod search;
pub mod sync;
pub mod systemtools;
