// xin.chat L2 前端半体（小欣「一切皆插件」拆分；manifest 与后端 manifest.rs 同源）。
// 必备子插件：对话主界面（人格选择 / 会话列表 / 消息收发 / TTS / 多模态图片 / 语音输入），
// 组件由 L1 Xin 壳经 props 组合渲染；贡献板块内菜单选项（target=自身 slot id，routePath='?tab=chat'）。
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [],
    navItems: [
      { target: 'xin.chat', labelKey: 'layout.k17', icon: '💬', order: 10, routePath: '?tab=chat' },
    ],
    slotComponents: [],
  },
});
