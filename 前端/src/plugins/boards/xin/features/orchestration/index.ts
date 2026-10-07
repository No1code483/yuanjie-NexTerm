// xin.orchestration L2 前端半体（阶段3 批次4a-2；manifest 与后端 manifest.rs 同源）。
// 贡献 xo 命名空间 IPC 客户端（87 条 alias）。编排面 UI 未单独接线（前端消费为 Xin.tsx / FloatingXin.tsx 直接调用 xo 客户端），不设插槽路由。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { XIN_ORCHESTRATION_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export { xinOrchestration } from './ipc';
export type { XinAttachment, XinParsedAttachment, XinOutputEnhancement, XinDialogueSendOptions } from '../../ipc';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [],
    slotComponents: [],
    ipc: {
      namespace: 'xo',
      methods: XIN_ORCHESTRATION_IPC_METHODS,
    },
  },
});
