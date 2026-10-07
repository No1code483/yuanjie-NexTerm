// knowledge.templates L2 前端半体（批C3；manifest 与后端 manifest.rs 同源）。
// 贡献 kt 命名空间 IPC（4 条模版方法，自 L1 kb 命名空间迁入）；模版选择器/管理器
// 弹层组件（TemplateModals）物理迁入本目录，由 L1 Knowledge 页以 props 驱动渲染。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';
import { KT_IPC_METHODS } from './ipc';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    // 板块内菜单选项（本轮注册表驱动侧边栏）：target=自身 slot id，routePath 为 tab 查询串
    navItems: [
      { target: 'knowledge.templates', labelKey: 'layout.k6', order: 90, routePath: '?tab=templates' },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'kt',
      methods: KT_IPC_METHODS,
    },
  },
});
