-- Migration v96: create_distill_dataset_table
-- 类型: 静态（pure CREATE TABLE IF NOT EXISTS）
-- 规范: 功能展望/v2_核心战略/01_恐龙双脑智能架构_实施路线图_补充.md §2.1 Phase 0
--       训练规划/03_02_Schema 设计
-- hash 源: 函数名 "create_distill_dataset_table"
-- 说明: 恐龙双脑 V5 学生模型蒸馏数据集存储表
--       Phase 0 任务 2（Schema 落地）—— 用于存放从用户活动数据
--       (activity_logs / chat_messages / kb_entries) 抽取并经过
--       教师模型（云端 API）改写得到的训练样本。
-- 字段：
--   id              - 主键自增
--   source_type     - 数据来源类型：activity / chat / kb / manual
--   source_id       - 源数据行 ID（便于追溯）
--   input_text      - 输入文本（用户请求 / 活动上下文）
--   teacher_output  - 教师模型输出（云端 API 生成）
--   metadata        - JSON 元数据（场景标签、置信度等）
--   quality_score   - 质量评分 0.0~1.0（人工/自动评估）
--   status          - 状态：pending / approved / rejected / used
--   created_at      - 创建时间（毫秒时间戳）
--   reviewed_at     - 审核时间（毫秒时间戳，可空）

CREATE TABLE IF NOT EXISTS distill_dataset (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_type TEXT NOT NULL DEFAULT 'activity',
    source_id TEXT,
    input_text TEXT NOT NULL,
    teacher_output TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',
    quality_score REAL NOT NULL DEFAULT 0.0,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at INTEGER NOT NULL,
    reviewed_at INTEGER
);

-- 按状态索引（用于按状态筛选样本）
CREATE INDEX IF NOT EXISTS idx_distill_dataset_status ON distill_dataset(status);
-- 按来源类型索引（用于按数据源统计）
CREATE INDEX IF NOT EXISTS idx_distill_dataset_source ON distill_dataset(source_type);
-- 按质量评分降序索引（用于筛选高质量样本）
CREATE INDEX IF NOT EXISTS idx_distill_dataset_quality ON distill_dataset(quality_score DESC);
