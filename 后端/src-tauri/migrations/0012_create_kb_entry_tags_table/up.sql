-- Migration v12: create_kb_entry_tags_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/平台级增强/02_数据完整性_迁移与备份.md §2.1
-- hash 源: 函数名 "create_kb_entry_tags_table"（T2.6.1 兼容性设计）
-- 说明: 知识库条目-标签多对多关联表，复合主键 + 双外键级联

CREATE TABLE IF NOT EXISTS kb_entry_tags (
    entry_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (entry_id, tag_id),
    FOREIGN KEY (entry_id) REFERENCES kb_entries(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES kb_tags(id) ON DELETE CASCADE
);
