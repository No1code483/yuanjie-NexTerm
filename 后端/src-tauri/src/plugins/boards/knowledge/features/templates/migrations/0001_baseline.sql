-- knowledge.templates 基线迁移（批C3 知识库子插件拆分）。
-- kb_templates 表归属迁移：boards.knowledge → knowledge.templates（深度物理拆分）。
-- ⚠️ 必须同时改写 name_prefixed 1→0：该标志由内核按「表名是否以本插件短码开头」
--    推导（registry.rs::register_manifest_tables），本插件短码为 kt，kb_templates
--    不以 kt_ 开头 → 推导 0；若保留 L1 登记的 1，register_table 判「已登记为 1，
--    不能改写为 0」导致插件激活失败。
-- ⚠️ 执行时序：本迁移在 register_manifest_tables 之前应用（activate_all / set_enabled
--    同序），保证归属改写先于新归属幂等登记。
UPDATE kernel_table_ownership
SET plugin_id = 'knowledge.templates', name_prefixed = 0
WHERE table_name = 'kb_templates' AND plugin_id = 'boards.knowledge';
