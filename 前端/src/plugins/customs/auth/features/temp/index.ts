// auth.temp L2 前端半体（认证「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载临时账号视图（生成/登录临时账号），视图由 L1 LoginModal 壳经 props 组合渲染；
// 停用时登录页隐藏「临时账号」入口。不贡献 navItems/routes。
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
