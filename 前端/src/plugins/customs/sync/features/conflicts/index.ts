// sync.conflicts L2 前端半体（同步「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 冲突解决视图（冲突列表 / 本地·远端·合并；原 components/Sync/ConflictResolver.tsx 物理迁入），
// 由 L1 /sync 壳经 props 组合渲染；贡献板块内菜单选项（target=自身 slot id，routePath='?tab=conflicts'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'sync.conflicts', labelKey: 'components.Sync.k3', icon: '⚖️', order: 20, routePath: '?tab=conflicts' },
    ],
    slotComponents: [],
  },
});
