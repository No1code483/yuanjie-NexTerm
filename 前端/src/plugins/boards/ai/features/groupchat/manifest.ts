import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/ai/features/groupchat/manifest.rs 逐字段对齐（L2 Feature） */
export const manifest: Manifest = {
  id: 'ai.groupchat',
  name: '辩论群聊',
  level: 'feature',
  parent: 'boards.ai',
  slot: 'ai.groupchat',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 本 L2 不持有业务表：编排状态经 `chat_service::get_messages` 读取
    // （messages 表归属属批次 2b-2），强制结束经 `chat_service::stop_generation`。
    db: [],
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['gc_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'ai.groupchat',
};