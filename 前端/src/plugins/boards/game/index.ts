// boards.game L1 前端半体（阶段3 批次4b；manifest 与后端 manifest.rs 同源）。
// 贡献 3 条游戏路由（/game、/game/play、/game/play/mapping，各携带自身 routePath
// 供 AuthGuard 解析权限资源，均映射 game）+ 1 个主干 navItem + gm 命名空间 IPC 客户端。
// 游戏「一切皆插件」（2026-10-05）：3 个 L2（game.preview / game.play3d / game.mapping）
// 页面物理迁入各自 features/ 目录，L1 仅以 lazy + 相对路径装载；跨 L2 共享模块置于 shared/。
import { createElement, lazy, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { withAuthGuard } from '@/routes/AuthGuard';
import { ROUTES } from '@/routes/routes';
import { GAME_IPC_METHODS, game } from './ipc';
import { manifest } from './manifest';

export { game };

const GamePreview = lazy(() => import('./features/preview/GamePreview'));
const Game3D = lazy(() => import('./features/play3d/Game3D'));
const KnowledgeMapping = lazy(() => import('./features/mapping/KnowledgeMapping'));

/** withAuthGuard 的 routePath 决定权限资源，与静态注册期保持同一 routePath */
const GuardedGamePreview = lazy(async () => ({
  default: withAuthGuard(GamePreview, ROUTES.GAME),
}));
const GuardedGame3D = lazy(async () => ({
  default: withAuthGuard(Game3D, ROUTES.GAME_PLAY),
}));

/** game.mapping 启停门控：停用时 /game/play/mapping 回退到 /game/play
 *  （null=加载中放行，口径同 Home.tsx 的 focusEnabled）。 */
function MappingRoute() {
  const navigate = useNavigate();
  const [enabled, setEnabled] = useState<boolean | null>(null);
  useEffect(() => {
    invoke<Array<{ id: string }>>('plugin:kernel|kernel_dispatch', {
      cmd: 'kernel:plugin:get_enabled',
      args: {},
    }).then((list) => setEnabled(list.some((p) => p.id === 'game.mapping')))
      .catch(() => setEnabled(true));
  }, []);
  useEffect(() => {
    if (enabled === false) navigate(ROUTES.GAME_PLAY, { replace: true });
  }, [enabled, navigate]);
  if (enabled === false) return null;
  return createElement(KnowledgeMapping);
}

const GuardedKnowledgeMapping = lazy(async () => ({
  default: withAuthGuard(MappingRoute, ROUTES.GAME_PLAY_MAPPING),
}));

export default definePlugin({
  manifest,
  contributions: {
    routes: [
      { path: ROUTES.GAME, component: GuardedGamePreview },
      { path: ROUTES.GAME_PLAY, component: GuardedGame3D },
      { path: ROUTES.GAME_PLAY_MAPPING, component: GuardedKnowledgeMapping },
    ],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: 'components.AddExtensionModal.k11',
        order: 60, // 主干第六位（约定稀疏值 10/20/.../70；新板块默认 ≥90）
        routePath: ROUTES.GAME,
      },
    ],
    slotComponents: [],
    ipc: {
      namespace: 'gm',
      methods: GAME_IPC_METHODS,
    },
  },
});
