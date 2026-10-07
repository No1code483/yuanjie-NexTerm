//! boards.profile 的 L2 Feature 插件集合（个人中心「一切皆插件」拆分）。
//! profile.account：账号（个人中心必备子插件，默认视图，纯前端无命令无表）。
//! profile.resume：简历（纯前端子插件，无命令无表）。
//! profile.quote：语录（纯前端子插件，无命令无表）。
//! profile.settings：设置（纯前端子插件，无命令无表）。

pub mod account;
pub mod quote;
pub mod resume;
pub mod settings;
