// auth.recovery L2 前端半体（认证「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载找回密码视图（校验恢复短语 + 重置密码），视图由 L1 LoginModal 壳经 props 组合渲染；
// 停用时登录页隐藏「忘记密码」入口。不贡献 navItems/routes。
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
