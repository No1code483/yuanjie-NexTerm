import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/ai/manifest.rs 逐字段对齐（L1 Board，AI 会话板块） */
export const manifest: Manifest = {
  id: 'boards.ai',
  name: 'AI 会话',
  level: 'board',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // BUG-033 修订（2026-09-27）：4 张会话表归属已转 ai.sessions（其后端
    // migrations/0002_transfer_ownership.sql），L1 不再声明 db（避免二次冷激活
    // 重复登记冲突 → Error）。L1 仍保留会话面 18 条 IPC（namespace `ai`）。
    db: [],
    // 本批不发布/订阅领域事件；3 条前端事件通道（ai-stream / ai-orchestrator /
    // ai-model-health-changed）为 Tauri emit，不走内核事件总线（S5 零改动）。
    events: { subscribe: [], publish: [] },
    // 2b-2：L1 承载会话面 18 条 IPC（namespace `ai`）。
    ipc: ['ai_*'],
    fs: [],
    net: [],
  },
  // 4 个插槽承载 ai.models / ai.sessions / ai.agent / ai.groupchat 四个 L2 Feature 插件。
  // AI会话「一切皆插件」：再增 4 个 L2 插槽（chat 必备；multimodel/orchestration/prompts 可选）
  slots: [
    { id: 'ai.models', type: 'panel', description: '模型管理', capacity: 1 },
    { id: 'ai.sessions', type: 'panel', description: '会话列表', capacity: 1 },
    { id: 'ai.agent', type: 'panel', description: '智能体集', capacity: 1 },
    { id: 'ai.groupchat', type: 'panel', description: '辩论群聊', capacity: 1 },
    { id: 'ai.chat', type: 'panel', description: '对话', capacity: 1 },
    { id: 'ai.multimodel', type: 'panel', description: '多模型对比', capacity: 1 },
    { id: 'ai.orchestration', type: 'panel', description: '群聊编排', capacity: 1 },
    { id: 'ai.prompts', type: 'panel', description: '提示词模板', capacity: 1 },
  ],
  i18nNamespace: 'ai',
};