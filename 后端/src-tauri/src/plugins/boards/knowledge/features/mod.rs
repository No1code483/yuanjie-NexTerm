//! boards.knowledge 的 L2 Feature 插件集合（批C3 知识库子插件拆分，手稿 20260926）。
//! knowledge.material / knowledge.learning【必备】：资料库 / 学习库视图启停登记（无命令无表）。
//! knowledge.templates：插入模版（kb_templates 表归属迁出 L1 + 4 条模版命令，短码 kt）。
//! knowledge.graph：关系图谱（纯前端 GraphView，无命令无表）。
//! knowledge.browse / search / tags / media / editors / import / history / ai：
//! 纯前端子插件（无命令无表），条目浏览 / 搜索 / 标签 / 媒体查看器 / 编辑器 /
//! 导入 / 快照与反链 / AI 辅助。

pub mod graph;
pub mod learning;
pub mod material;
pub mod templates;
// 批C3：8 个纯前端子插件（无命令无表）
pub mod ai;
pub mod browse;
pub mod editors;
pub mod history;
pub mod import;
pub mod media;
pub mod search;
pub mod tags;
