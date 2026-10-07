// auth.login L2 前端半体（认证「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：承载登录视图（login 表单 + 2FA 校验），视图由 L1 LoginModal 壳经 props 组合渲染；
// 登录页在 Router 之外渲染、无全局侧边栏，故不贡献 navItems/routes。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [],
    slotComponents: [],
  },
});
