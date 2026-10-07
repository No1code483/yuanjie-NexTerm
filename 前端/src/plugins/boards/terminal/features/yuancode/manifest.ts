import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/terminal/features/yuancode/manifest.rs 逐字段对齐
 *  （L2 Feature，terminal.yuancode，批次3c） */
export const manifest: Manifest = {
  id: 'terminal.yuancode',
  name: 'Yuan Code 编辑器核心',
  level: 'feature',
  parent: 'boards.terminal',
  slot: 'terminal.yuancode',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 14 张表（3c 2 张 + 5a 4 张 + 5e 8 张）归属登记于本插件（主应用迁移创建，本插件只登记归属）。
    db: [
      'yuan_code_snippets',
      'yuan_code_workspaces',
      'editor_documents',
      'editor_versions',
      'editor_sessions',
      'distill_dataset',
      'api_keys',
      'model_routing_rules',
      'project_index_projects',
      'project_index_files',
      'project_index_symbols',
      'project_index_imports',
      'project_index_dependencies',
      'project_index_changes',
    ],
    // 不发布/订阅内核领域事件（emit 均为 Tauri 事件 ai-stream / yuan-code-*-done）。
    events: { subscribe: [], publish: [] },
    // 54 条 IPC（编辑器核心/AI 补全/Git/LSP/内嵌浏览器）。
    ipc: ['yc_*'],
    fs: [],
    net: [],
  },
  // YuanCode「一切皆插件」：6 个 L3 子插件插槽（editor 必备；其余可选）
  slots: [
    { id: 'terminal.yuancode.editor', type: 'panel', description: '编辑器核心', capacity: 1 },
    { id: 'terminal.yuancode.agent', type: 'panel', description: 'Agent 与目标', capacity: 1 },
    { id: 'terminal.yuancode.git', type: 'panel', description: '版本控制', capacity: 1 },
    { id: 'terminal.yuancode.skills', type: 'panel', description: '技能与片段', capacity: 1 },
    { id: 'terminal.yuancode.sandbox', type: 'panel', description: '沙箱运行', capacity: 1 },
    { id: 'terminal.yuancode.settings', type: 'panel', description: '编辑器设置', capacity: 1 },
  ],
  i18nNamespace: 'terminal.yuancode',
};
