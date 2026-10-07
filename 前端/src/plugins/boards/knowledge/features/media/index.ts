// knowledge.media L2 前端半体（本轮知识库「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载内联/全屏媒体预览（图片/PDF/CSV/代码高亮/音视频）（组件由 L1 Knowledge 壳经 props 组合渲染）；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=media'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'knowledge.media', labelKey: 'knowledge.features.media', order: 40, routePath: '?tab=media' },
    ],
    slotComponents: [],
  },
});
