// xin.realtime L2 前端半体（阶段3 批次4a-1；manifest 与后端 manifest.rs 同源）。
// 仅贡献 xr 命名空间 IPC 客户端：实时语音 UI 消费点 useEffectSession hook
// （useRealtimeSession.ts 随批迁入本目录），不设插槽组件。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { XIN_REALTIME_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [],
    slotComponents: [],
    ipc: {
      namespace: 'xr',
      methods: XIN_REALTIME_IPC_METHODS,
    },
  },
});