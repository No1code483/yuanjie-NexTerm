//! customs.search 的 L2 Feature 插件集合（纯前端子插件，无命令无表）。
//! search.browser：浏览器搜索（浏览器历史/书签搜索入口）。
//! search.global：全站搜索（跨模块统一搜索入口）。
//! search.bookmarks：收藏站点（收藏站点列表与检索）。

pub mod bookmarks;
pub mod browser;
pub mod global;
