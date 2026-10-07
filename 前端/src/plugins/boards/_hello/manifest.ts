import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/_hello/manifest.rs 同数据（一致性由 F8 校验脚本保证） */
export const manifest: Manifest = {
  id: '_hello',
  name: 'Hello 插件化',
  level: 'board',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: ['hw_notes'],
    events: { subscribe: ['_hello:*'], publish: ['_hello:*'] },
    ipc: ['_hello_*'],
    fs: [],
    net: [],
  },
  slots: [{ id: 'hello.panel', type: 'panel', description: '演示插槽', capacity: 2 }],
  i18nNamespace: '_hello',
};
