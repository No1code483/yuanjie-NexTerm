// xin.wellness L2 前端半体（阶段3 批次4a-1；manifest 与后端 manifest.rs 同源）。
// 贡献 xw 命名空间 IPC 客户端（17 条 alias）+ 健康/效率面板（productivity tab：提醒/习惯/番茄钟），
// 组件由 L1 Xin 壳经 props 组合渲染；贡献板块内菜单选项（target=自身 slot id，routePath='?tab=productivity'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { XIN_WELLNESS_IPC_METHODS } from './ipc';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'xin.wellness', labelKey: 'layout.k20', icon: '⏱️', order: 40, routePath: '?tab=productivity' },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'xw',
      methods: XIN_WELLNESS_IPC_METHODS,
    },
  },
});