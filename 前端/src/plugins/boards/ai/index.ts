// boards.ai L1 前端半体（阶段3 批次2b-1；manifest 与后端 manifest.rs 同源）。
// 贡献 4 条 AI 会话路由（/ai、/ai/model、/ai/chat/:id、/ai/group/:id，各携带自身
// routePath 供 AuthGuard 解析权限资源，均映射 ai_chat）+ 1 个主干 navItem；
// ai.models / ai.agent / ai.groupchat 三个插槽由 L2 Feature 插件贡献。
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { manifest } from './manifest';
import { AI_BOARD_IPC_METHODS } from './ipc';

const AiPage = lazy(() => import('./AI'));

/** 每条路由需独立的守卫包装：withAuthGuard 的 routePath 决定权限资源，不可共用 */
const guardAi = (routePath: string) =>
  lazy(async () => ({ default: withAuthGuard(AiPage, routePath) }));

export default definePlugin({
  manifest,
  contributions: {
    routes: [
      { path: ROUTES.AI, component: guardAi(ROUTES.AI) },
      { path: ROUTES.AI_MODEL, component: guardAi(ROUTES.AI_MODEL) },
      { path: ROUTES.AI_CHAT, component: guardAi(ROUTES.AI_CHAT) },
      { path: ROUTES.AI_GROUP, component: guardAi(ROUTES.AI_GROUP) },
    ],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'components.PermissionRestricted.k2',
        order: 20, // 主干第二位（约定稀疏值 10/20/.../70）
        routePath: ROUTES.AI,
      },
    ],
    slotComponents: [],
    // batchC1 收编后 L1 剩余 5 条 IPC：sendMessage 1 + Prompt 模板 4（namespace `ai`）。
    // 会话面 13 条命令归 L2 ai.sessions（ss）承载。
    ipc: {
      namespace: 'ai',
      methods: AI_BOARD_IPC_METHODS,
    },
  },
});