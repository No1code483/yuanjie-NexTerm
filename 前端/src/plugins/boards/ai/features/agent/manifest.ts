import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/ai/features/agent/manifest.rs 逐字段对齐（L2 Feature） */
export const manifest: Manifest = {
  id: 'ai.agent',
  name: '智能体集',
  level: 'feature',
  parent: 'boards.ai',
  slot: 'ai.agent',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // ai_agents 为既有旧表（短码 `ag` ≠ 旧名前缀 `ai_` → name_prefixed=0），
    // 归属登记见后端 migrations/0001_baseline.sql。
    db: ['ai_agents'],
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['ag_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'ai.agent',
};