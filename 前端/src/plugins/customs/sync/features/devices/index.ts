// sync.devices L2 前端半体（同步「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：设备管理视图（设备列表 / 撤销 / ECDH 注册；原 components/Sync/DeviceManager.tsx 物理迁入），
// 由 L1 /sync 壳经 props 组合渲染；贡献板块内菜单选项（target=自身 slot id，routePath='?tab=devices'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'sync.devices', labelKey: 'components.Sync.k2', icon: '💻', order: 10, routePath: '?tab=devices' },
    ],
    slotComponents: [],
  },
});
