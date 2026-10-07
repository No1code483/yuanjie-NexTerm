// terminal.yuancode L2 前端半体（阶段3 批次3c；manifest 与后端 manifest.rs 同源）。
// 贡献 yc 命名空间 IPC（54 条方法 spec）+ Yuan Code 路由（单页多 Tab 形态，
// pages/yuan-code/ 41 文件 + YuanCode.tsx 随批 git mv 迁入插件目录，裁定 C1）。
// 无 navItem 贡献（Yuan Code 入口由终端侧边栏/路由直达承载）。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { manifest } from './manifest';
import { YC_IPC_METHODS } from './ipc';

const YuanCodePage = lazy(() => import('../../YuanCode'));

/** 每条路由需独立的守卫包装：withAuthGuard 的 routePath 决定权限资源，不可共用 */
const guardYuanCode = (routePath: string) =>
  lazy(async () => ({ default: withAuthGuard(YuanCodePage, routePath) }));

export default definePlugin({
  manifest,
  contributions: {
    routes: [
      // L2 路径必须相对（PluginRegistry.buildRouter 会拼父前缀 /terminal）；
      // 绝对路径 '/terminal/yuancode' 会注册成 '/terminal/terminal/yuancode' → 404
      { path: 'yuancode', component: guardYuanCode(ROUTES.TERMINAL_YUANCODE) },
    ],
    navItems: [],
    slotComponents: [],
    ipc: {
      namespace: 'yc',
      methods: YC_IPC_METHODS,
    },
  },
});
