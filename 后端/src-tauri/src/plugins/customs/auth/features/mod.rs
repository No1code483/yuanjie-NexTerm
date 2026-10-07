//! customs.auth 的 L2 Feature 插件集合。
//! 认证「一切皆插件」拆分：4 个纯前端视图子插件，无命令无表（login 必备）。

pub mod login;
pub mod recovery;
pub mod register;
pub mod temp;
