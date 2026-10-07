import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/ai/features/models/manifest.rs 逐字段对齐（L2 Feature） */
export const manifest: Manifest = {
  id: 'ai.models',
  name: '模型管理',
  level: 'feature',
  parent: 'boards.ai',
  slot: 'ai.models',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // ai_models 为既有旧表（短码 `am` ≠ 旧名前缀 `ai_` → name_prefixed=0），
    // 归属登记见后端 migrations/0001_baseline.sql。
    db: ['ai_models'],
    // 本批不发布/订阅领域事件；`ai-model-health-changed` 为 Tauri emit，非内核总线事件。
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['am_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'ai.models',
};