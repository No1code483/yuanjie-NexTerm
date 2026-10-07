import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/knowledge/manifest.rs 逐字段对齐（L1 Board，知识库板块） */
export const manifest: Manifest = {
  id: 'boards.knowledge',
  name: '知识库',
  level: 'board',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 9 张 kb_* 表归属登记见后端 migrations/0001_baseline.sql；
    // kb_templates 已随批C3 迁出至 knowledge.templates。
    // ⚠️ kb_categories.id 被 game_kb_category_mapping 逻辑引用（无物理 FK），不得改值。
    db: [
      'kb_attachment_cache',
      'kb_categories',
      'kb_entries',
      'kb_entry_tags',
      'kb_recent_access',
      'kb_references',
      'kb_snapshots',
      'kb_tags',
      'kb_tracked_paths',
    ],
    // 2a-1 不发布/订阅领域事件；`knowledge:item.*` 契约按批次 1b-2c 先例另批接线。
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['kb_*'],
    fs: [],
    net: [],
  },
  // 批C3：4 个 L2 子插件插槽（material/learning 必备；templates/graph 可选）
  // 本轮「一切皆插件」拆分：再增 8 个 L2 插槽（browse 必备；search/tags/media/editors/import/history/ai 可选）
  slots: [
    { id: 'knowledge.material', type: 'panel', description: '资料库', capacity: 1 },
    { id: 'knowledge.learning', type: 'panel', description: '学习库', capacity: 1 },
    { id: 'knowledge.templates', type: 'panel', description: '插入模版', capacity: 1 },
    { id: 'knowledge.graph', type: 'panel', description: '关系图谱', capacity: 1 },
    { id: 'knowledge.browse', type: 'panel', description: '条目浏览', capacity: 1 },
    { id: 'knowledge.search', type: 'panel', description: '搜索', capacity: 1 },
    { id: 'knowledge.tags', type: 'panel', description: '标签', capacity: 1 },
    { id: 'knowledge.media', type: 'panel', description: '媒体查看器', capacity: 1 },
    { id: 'knowledge.editors', type: 'panel', description: '编辑器', capacity: 1 },
    { id: 'knowledge.import', type: 'panel', description: '导入', capacity: 1 },
    { id: 'knowledge.history', type: 'panel', description: '快照与反链', capacity: 1 },
    { id: 'knowledge.ai', type: 'panel', description: 'AI 辅助', capacity: 1 },
  ],
  i18nNamespace: 'knowledge',
};
