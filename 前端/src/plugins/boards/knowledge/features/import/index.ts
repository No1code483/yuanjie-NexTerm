// knowledge.import L2 前端半体（本轮知识库「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 承载导入文件/文件夹 + 目录扫描 + 追踪路径（组件由 L1 Knowledge 壳经 props 组合渲染）；
// 贡献板块内菜单选项（target=自身 slot id，routePath='?tab=import'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'knowledge.import', labelKey: 'knowledge.features.import', order: 60, routePath: '?tab=import' },
    ],
    slotComponents: [],
  },
});
