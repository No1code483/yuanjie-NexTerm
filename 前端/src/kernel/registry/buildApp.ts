import { invoke } from '@tauri-apps/api/core';
import type { RouteObject } from 'react-router-dom';
import type { PluginId } from '../types';
import { PluginRegistry } from './PluginRegistry';
import type { NavNode, RejectRecord } from './PluginRegistry';
import type { FrontendPlugin } from './definePlugin';

/** B7 落地决策 + B8 传输名纠偏（07 契约 §十三）：
 *  统一经内核 dispatcher；Tauri 2 插件命令传输名为 plugin:<插件名>|<命令名> */
const KERNEL_DISPATCH = 'plugin:kernel|kernel_dispatch';

/** 阶段1：已知插件入口表（Vite 静态可分析的动态 import → 分 chunk）。
 *  阶段2 引入安装器后再做按清单的真正动态加载。 */
const PLUGIN_ENTRIES: Partial<Record<PluginId, () => Promise<{ default: FrontendPlugin }>>> = {
  _hello: () => import('@/plugins/boards/_hello'),
  'boards.profile': () => import('@/plugins/boards/profile'),
  // 个人中心「一切皆插件」：4 个 L2 子插件（account 必备；resume/quote/settings 可选）
  'profile.account': () => import('@/plugins/boards/profile/features/account'),
  'profile.resume': () => import('@/plugins/boards/profile/features/resume'),
  'profile.quote': () => import('@/plugins/boards/profile/features/quote'),
  'profile.settings': () => import('@/plugins/boards/profile/features/settings'),
  'customs.auth': () => import('@/plugins/customs/auth'),
  // 认证「一切皆插件」：4 个纯前端视图 L2 子插件（login 必备；register/recovery/temp 可选）
  'auth.login': () => import('@/plugins/customs/auth/features/login'),
  'auth.register': () => import('@/plugins/customs/auth/features/register'),
  'auth.recovery': () => import('@/plugins/customs/auth/features/recovery'),
  'auth.temp': () => import('@/plugins/customs/auth/features/temp'),
  'boards.home': () => import('@/plugins/boards/home'),
  'home.todo': () => import('@/plugins/boards/home/features/todo'),
  'home.journal': () => import('@/plugins/boards/home/features/journal'),
  'home.timer': () => import('@/plugins/boards/home/features/timer'),
  'home.news': () => import('@/plugins/boards/home/features/news'),
  // 批C4：home.focus（专注，可选子插件，纯前端 FocusMode 组件）
  'home.focus': () => import('@/plugins/boards/home/features/focus'),
  'boards.knowledge': () => import('@/plugins/boards/knowledge'),
  // 批C3：知识库 4 个 L2 子插件（material/learning 必备；templates/graph 可选）
  'knowledge.material': () => import('@/plugins/boards/knowledge/features/material'),
  'knowledge.learning': () => import('@/plugins/boards/knowledge/features/learning'),
  'knowledge.templates': () => import('@/plugins/boards/knowledge/features/templates'),
  'knowledge.graph': () => import('@/plugins/boards/knowledge/features/graph'),
  // 本轮「一切皆插件」：知识库 8 个 L2 子插件（browse 必备；其余可选）
  'knowledge.browse': () => import('@/plugins/boards/knowledge/features/browse'),
  'knowledge.search': () => import('@/plugins/boards/knowledge/features/search'),
  'knowledge.tags': () => import('@/plugins/boards/knowledge/features/tags'),
  'knowledge.media': () => import('@/plugins/boards/knowledge/features/media'),
  'knowledge.editors': () => import('@/plugins/boards/knowledge/features/editors'),
  'knowledge.import': () => import('@/plugins/boards/knowledge/features/import'),
  'knowledge.history': () => import('@/plugins/boards/knowledge/features/history'),
  'knowledge.ai': () => import('@/plugins/boards/knowledge/features/ai'),
  'boards.ai': () => import('@/plugins/boards/ai'),
  'ai.models': () => import('@/plugins/boards/ai/features/models'),
  'ai.sessions': () => import('@/plugins/boards/ai/features/sessions'),
  'ai.agent': () => import('@/plugins/boards/ai/features/agent'),
  'ai.groupchat': () => import('@/plugins/boards/ai/features/groupchat'),
  // AI会话「一切皆插件」：4 个新增 L2 子插件（chat 必备；其余可选）
  'ai.chat': () => import('@/plugins/boards/ai/features/chat'),
  'ai.multimodel': () => import('@/plugins/boards/ai/features/multimodel'),
  'ai.orchestration': () => import('@/plugins/boards/ai/features/orchestration'),
  'ai.prompts': () => import('@/plugins/boards/ai/features/prompts'),
  // 阶段3 批次3a：boards.terminal（L1；terminal.linux L2 属 3b）
  'boards.terminal': () => import('@/plugins/boards/terminal'),
  // 终端本体「一切皆插件」：3 个 L2 子插件（console 必备；mux/tools 可选）
  'terminal.console': () => import('@/plugins/boards/terminal/features/console'),
  'terminal.mux': () => import('@/plugins/boards/terminal/features/mux'),
  'terminal.tools': () => import('@/plugins/boards/terminal/features/tools'),
  // 阶段3 批次3b：terminal.linux（L2，Linux 子系统/Docker 25 命令，前端零消费）
  'terminal.linux': () => import('@/plugins/boards/terminal/features/linux'),
  // 阶段3 批次3c：terminal.yuancode（L2，Yuan Code 编辑器核心 54 命令 + Yuan Code 路由）
  'terminal.yuancode': () => import('@/plugins/boards/terminal/features/yuancode'),
  // YuanCode「一切皆插件」：6 个 L3 子插件（editor 必备；其余可选）
  'terminal.yuancode.editor': () => import('@/plugins/boards/terminal/features/yuancode/features/editor'),
  'terminal.yuancode.agent': () => import('@/plugins/boards/terminal/features/yuancode/features/agent'),
  'terminal.yuancode.git': () => import('@/plugins/boards/terminal/features/yuancode/features/git'),
  'terminal.yuancode.skills': () => import('@/plugins/boards/terminal/features/yuancode/features/skills'),
  'terminal.yuancode.sandbox': () => import('@/plugins/boards/terminal/features/yuancode/features/sandbox'),
  'terminal.yuancode.settings': () => import('@/plugins/boards/terminal/features/yuancode/features/settings'),
  // 批C2：terminal.manual（L2，命令手册 5 路由，纯前端静态页面无 IPC）
  'terminal.manual': () => import('@/plugins/boards/terminal/features/manual'),
  // 阶段3 批次4a-1：boards.xin（L1 基础面 28 命令 + Xin 路由）+ L2 wellness / realtime
  'boards.xin': () => import('@/plugins/boards/xin'),
  'xin.wellness': () => import('@/plugins/boards/xin/features/wellness'),
  'xin.realtime': () => import('@/plugins/boards/xin/features/realtime'),
  // 阶段3 批次4a-2：xin.orchestration（L2，编排面 87 命令，短码 xo）
  'xin.orchestration': () => import('@/plugins/boards/xin/features/orchestration'),
  // 小欣「一切皆插件」：12 个面板 L2 子插件（chat 必备；其余可选）
  'xin.chat': () => import('@/plugins/boards/xin/features/chat'),
  'xin.memory': () => import('@/plugins/boards/xin/features/memory'),
  'xin.mood': () => import('@/plugins/boards/xin/features/mood'),
  'xin.briefing': () => import('@/plugins/boards/xin/features/briefing'),
  'xin.compaction': () => import('@/plugins/boards/xin/features/compaction'),
  'xin.dream': () => import('@/plugins/boards/xin/features/dream'),
  'xin.checkpoint': () => import('@/plugins/boards/xin/features/checkpoint'),
  'xin.search': () => import('@/plugins/boards/xin/features/search'),
  'xin.review': () => import('@/plugins/boards/xin/features/review'),
  'xin.skill': () => import('@/plugins/boards/xin/features/skill'),
  'xin.tool': () => import('@/plugins/boards/xin/features/tool'),
  'xin.evolution': () => import('@/plugins/boards/xin/features/evolution'),
  // 阶段3 批次4b：boards.game（L1，44 条 IPC，短码 gm）
  'boards.game': () => import('@/plugins/boards/game'),
  // 游戏「一切皆插件」：3 个 L2 子插件（preview/play3d 必备；mapping 可选）
  'game.preview': () => import('@/plugins/boards/game/features/preview'),
  'game.play3d': () => import('@/plugins/boards/game/features/play3d'),
  'game.mapping': () => import('@/plugins/boards/game/features/mapping'),
  // 阶段3 批次4c：customs.systemtools（L1，10 条 IPC，短码 st；extension/adapter 裁定零消费删除）
  'customs.systemtools': () => import('@/plugins/customs/systemtools'),
  // 批次6a-6c：剩余 4 个 customs L1 插件（短码 rc/se/sp/sy）
  'customs.recycle': () => import('@/plugins/customs/recycle'),
  // 回收站「一切皆插件」：2 个纯前端视图 L2 子插件（list 必备；actions 可选）
  'recycle.list': () => import('@/plugins/customs/recycle/features/list'),
  'recycle.actions': () => import('@/plugins/customs/recycle/features/actions'),
  'customs.search': () => import('@/plugins/customs/search'),
  // 搜索「一切皆插件」：3 个 L2 子插件（browser 必备；global/bookmarks 可选）
  'search.browser': () => import('@/plugins/customs/search/features/browser'),
  'search.global': () => import('@/plugins/customs/search/features/global'),
  'search.bookmarks': () => import('@/plugins/customs/search/features/bookmarks'),
  'customs.intelligence': () => import('@/plugins/customs/intelligence'),
  // 底层智能「一切皆插件」：5 个 L2 子插件（dashboard/settings 必备；其余可选）
  'intelligence.dashboard': () => import('@/plugins/customs/intelligence/features/dashboard'),
  'intelligence.suggestions': () => import('@/plugins/customs/intelligence/features/suggestions'),
  'intelligence.behavior': () => import('@/plugins/customs/intelligence/features/behavior'),
  'intelligence.activity': () => import('@/plugins/customs/intelligence/features/activity'),
  'intelligence.settings': () => import('@/plugins/customs/intelligence/features/settings'),
  'customs.sync': () => import('@/plugins/customs/sync'),
  // 同步「一切皆插件」：2 个纯前端视图 L2 子插件（devices 必备；conflicts 可选）
  'sync.devices': () => import('@/plugins/customs/sync/features/devices'),
  'sync.conflicts': () => import('@/plugins/customs/sync/features/conflicts'),
};

/** 拓扑序：L1 先于 L2/L3 apply */
const LEVEL_ORDER: Record<string, number> = { board: 0, feature: 1, custom: 2 };

/**
 * 启动序列（与后端 02 篇 §八对应）：
 * 1. invoke('plugin:kernel|kernel_dispatch', { cmd: 'kernel:plugin:get_enabled' }) 取启用清单
 * 2. 父链过滤：父插件未启用 → 子插件随父停用（手稿 20260926）
 * 3. 按清单动态 import 各插件入口（import(...) 分 chunk）
 * 4. registry.apply 逐插件（拓扑序：L1 先于 L2/L3）
 * 5. rejects 非空 → console.warn 逐条
 * 6. 返回 { router, navTree } 供 App 渲染
 */
export async function buildApp(): Promise<{
  router: RouteObject[];
  navTree: NavNode[];
  rejects: RejectRecord[];
  /** registry 实例（BUG-035：App 层经 RegistryProvider 注入，SlotRenderer 的
   *  useRegistry 依赖此上下文——缺失时首页四子插件 modal 渲染即抛
   *  'useRegistry 必须在 <RegistryProvider> 内使用' 白屏） */
  registry: PluginRegistry;
}> {
  const registry = new PluginRegistry();

  // 1. 启用清单（kernel:plugin:get_enabled，owner=kernel 可信直查）
  type EnabledItem = { id: PluginId; parent?: PluginId };
  let enabled: EnabledItem[] = [];
  try {
    enabled = await invoke<EnabledItem[]>(KERNEL_DISPATCH, {
      cmd: 'kernel:plugin:get_enabled',
      args: {},
    });
  } catch (e) {
    console.warn('[plugin] 获取启用清单失败，按无插件模式启动:', e);
  }

  // 2. 父链过滤（手稿 20260926：子插件随父插件同步停用）——
  // 父插件不在启用清单时，子插件不装配，避免 slot 缺失的孤儿路由/悬空子插件
  const enabledIds = new Set(enabled.map((e) => e.id));
  const effective = enabled.filter((e) => !e.parent || enabledIds.has(e.parent));
  if (effective.length !== enabled.length) {
    const dropped = enabled
      .filter((e) => e.parent && !enabledIds.has(e.parent))
      .map((e) => e.id);
    console.warn(`[plugin] 父插件未启用，子插件随父停用: ${dropped.join(', ')}`);
  }

  // 3. 动态 import 已知插件入口
  const loaders = effective
    .map((e) => e.id)
    .filter((id) => PLUGIN_ENTRIES[id] != null)
    .map((id) => PLUGIN_ENTRIES[id]!());
  const modules = await Promise.all(loaders);

  // 4. 拓扑序 apply（按父链深度升序：父插件先于子插件，覆盖 L1→L2→L3 及嵌套 feature）
  const plugins = modules.map((m) => m.default);
  const byId = new Map(plugins.map((p) => [p.manifest.id, p]));
  const depthOf = (p: FrontendPlugin): number => {
    let depth = 0;
    let cur = p.manifest.parent;
    const seen = new Set<string>();
    while (cur && !seen.has(cur)) {
      seen.add(cur);
      depth += 1;
      cur = byId.get(cur)?.manifest.parent;
    }
    return depth;
  };
  const depths = new Map(plugins.map((p) => [p.manifest.id, depthOf(p)]));
  plugins.sort(
    (a, b) =>
      (depths.get(a.manifest.id)! - depths.get(b.manifest.id)!) ||
      LEVEL_ORDER[a.manifest.level] - LEVEL_ORDER[b.manifest.level]
  );
  for (const p of plugins) registry.apply(p);

  // 4. 显式失败不静默
  for (const r of registry.rejects) {
    console.warn(`[plugin] 注册被拒 ${r.pluginId}: ${r.reason}`);
  }

  // 5. 数据路由 + 导航树
  return {
    router: registry.buildRouter(),
    navTree: registry.buildNavTree(),
    rejects: registry.rejects,
    registry,
  };
}
