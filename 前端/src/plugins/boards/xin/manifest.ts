import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/xin/manifest.rs 逐字段对齐（L1 Board，小欣板块基础面，批次4a-1） */
export const manifest: Manifest = {
  id: 'boards.xin',
  name: '小欣',
  level: 'board',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 6 张自有表（短码 `xn` 与 `xin_` 前缀不命中 → name_prefixed=0，裁定 2）；
    // 归属登记见后端 migrations/0001_baseline.sql。
    db: [
      'xin_config',
      'xin_memories',
      'xin_summaries',
      'xin_moods',
      'xin_persona_memories',
      'xin_persona_switch_log',
    ],
    // 本批不发布/订阅领域事件（`realtime_event` 为 Tauri emit，非内核总线事件）。
    events: { subscribe: [], publish: [] },
    // L1 承载 28 条 IPC（namespace `xn`）。
    ipc: ['xn_*'],
    fs: [],
    net: [],
  },
  // 3 个插槽承载 xin.wellness / xin.realtime / xin.orchestration 三个 L2 Feature 插件；
  // 「一切皆插件」再拆 12 个纯前端面板 L2（chat 必备，其余可选）。
  slots: [
    { id: 'xin.wellness', type: 'panel', description: '小欣健康助手', capacity: 1 },
    { id: 'xin.realtime', type: 'panel', description: '小欣实时语音对话', capacity: 1 },
    { id: 'xin.orchestration', type: 'panel', description: '小欣编排面', capacity: 1 },
    // 小欣「一切皆插件」：12 个面板 L2（纯前端，无命令无表）
    { id: 'xin.chat', type: 'panel', description: '对话', capacity: 1 },
    { id: 'xin.memory', type: 'panel', description: '记忆', capacity: 1 },
    { id: 'xin.mood', type: 'panel', description: '心情', capacity: 1 },
    { id: 'xin.briefing', type: 'panel', description: '简报', capacity: 1 },
    { id: 'xin.compaction', type: 'panel', description: '上下文压缩', capacity: 1 },
    { id: 'xin.dream', type: 'panel', description: '梦境', capacity: 1 },
    { id: 'xin.checkpoint', type: 'panel', description: '检查点', capacity: 1 },
    { id: 'xin.search', type: 'panel', description: '对话搜索', capacity: 1 },
    { id: 'xin.review', type: 'panel', description: '复盘', capacity: 1 },
    { id: 'xin.skill', type: 'panel', description: '技能', capacity: 1 },
    { id: 'xin.tool', type: 'panel', description: '工具', capacity: 1 },
    { id: 'xin.evolution', type: 'panel', description: '人格进化', capacity: 1 },
  ],
  i18nNamespace: 'xin',
};