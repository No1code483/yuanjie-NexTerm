import { t } from "i18next";
/**
 * 路由配置常量
 * 定义所有路由路径和对应的页面组件
 */

import type { ResourceName } from '@/types';
export const ROUTES = {
  // 主路由
  ROOT: '/',
  HOME: '/home',
  PROFILE: '/profile',
  AI: '/ai',
  KNOWLEDGE: '/knowledge',
  TERMINAL: '/terminal',
  XIN: '/xin',
  GAME: '/game',
  GAME_PLAY: '/game/play',
  GAME_PLAY_MAPPING: '/game/play/mapping',
  RECYCLE: '/recycle',
  SEARCH: '/search',
  SPYGLASS: '/spyglass',
  // 首页子路由
  HOME_NEWS: '/home/news',
  HOME_TODO: '/home/todo',
  HOME_LOG: '/home/log',
  HOME_TIMER: '/home/timer',
  HOME_TIMER_SHORT: '/home/timer/short',
  HOME_TIMER_LONG: '/home/timer/long',
  // 个人中心子路由
  PROFILE_ACCOUNT: '/profile/account',
  PROFILE_RESUME: '/profile/resume',
  PROFILE_SETTING: '/profile/setting',
  PROFILE_LOGOUT: '/profile/logout',
  PROFILE_QUOTE: '/profile/quote',
  // AI会话子路由
  AI_MODEL: '/ai/model',
  AI_CHAT: '/ai/chat/:id',
  AI_GROUP: '/ai/group/:id',
  // 终端子路由
  TERMINAL_TERMINAL: '/terminal/terminal',
  TERMINAL_YUANCODE: '/terminal/yuancode',
  TERMINAL_LINUX: '/terminal/linux',
  TERMINAL_MANUAL: '/terminal/manual',
  TERMINAL_MANUAL_TERMINAL: '/terminal/manual/terminal',
  TERMINAL_MANUAL_YUANCODE: '/terminal/manual/yuancode',
  TERMINAL_MANUAL_LINUX: '/terminal/manual/linux',
  TERMINAL_MANUAL_SHORTCUTS: '/terminal/manual/shortcuts'
} as const;

// 只为主路由定义元数据，子路由使用父路由的元数据
export type MainRoutePath = typeof ROUTES.ROOT | typeof ROUTES.HOME | typeof ROUTES.PROFILE | typeof ROUTES.AI | typeof ROUTES.KNOWLEDGE | typeof ROUTES.TERMINAL | typeof ROUTES.XIN | typeof ROUTES.GAME | typeof ROUTES.RECYCLE | typeof ROUTES.SEARCH | typeof ROUTES.SPYGLASS;
export type RoutePath = typeof ROUTES[keyof typeof ROUTES];

/**
 * 路由元数据配置
 */
export interface RouteMeta {
  title: string;
  description: string;
  requiresAuth: boolean;
  allowedForTemp: boolean;
  icon?: string;
}
export const ROUTE_META: Record<MainRoutePath, RouteMeta> = {
  [ROUTES.ROOT]: {
    title: t("components.intelligence.DashboardPanel.k106"),
    description: t("routes.routes.k1"),
    requiresAuth: true,
    allowedForTemp: true
  },
  [ROUTES.HOME]: {
    title: t("components.intelligence.DashboardPanel.k106"),
    description: t("routes.routes.k1"),
    requiresAuth: true,
    allowedForTemp: true
  },
  [ROUTES.PROFILE]: {
    title: t("components.intelligence.DashboardPanel.k105"),
    description: t("routes.routes.k2"),
    requiresAuth: true,
    allowedForTemp: true // 临时账号可以访问个人中心，但内部功能受限
  },
  [ROUTES.AI]: {
    title: t("components.PermissionRestricted.k2"),
    description: t("routes.routes.k3"),
    requiresAuth: true,
    allowedForTemp: true // 临时账号可以访问AI会话，但部分功能只读
  },
  [ROUTES.KNOWLEDGE]: {
    title: t("components.intelligence.ActivityPanel.k1"),
    description: t("routes.routes.k4"),
    requiresAuth: true,
    allowedForTemp: false // 临时账号无法访问知识库
  },
  [ROUTES.TERMINAL]: {
    title: t("components.intelligence.ActivityPanel.k8"),
    description: t("routes.routes.k5"),
    requiresAuth: true,
    allowedForTemp: false // 临时账号无法访问终端
  },
  [ROUTES.XIN]: {
    title: t("components.FloatingXin.k26"),
    description: t("routes.routes.k6"),
    requiresAuth: true,
    allowedForTemp: true,
    icon: 'xin'
  },
  [ROUTES.GAME]: {
    title: t("components.AddExtensionModal.k11"),
    description: t("routes.routes.k7"),
    requiresAuth: true,
    allowedForTemp: true,
    icon: 'game'
  },
  [ROUTES.RECYCLE]: {
    title: t("components.PermissionRestricted.k4"),
    description: t("routes.routes.k8"),
    requiresAuth: true,
    allowedForTemp: false
  },
  [ROUTES.SEARCH]: {
    title: t("common.search"),
    description: t("routes.routes.k9"),
    requiresAuth: true,
    allowedForTemp: true
  },
  [ROUTES.SPYGLASS]: {
    title: t("components.intelligence.DashboardPanel.k103"),
    description: t("routes.routes.k10"),
    requiresAuth: true,
    allowedForTemp: false
  }
};

/**
 * 路由到资源的映射表
 * 
 * v2.0 新增：统一权限系统的核心映射
 * 将路由路径映射到 RBAC 权限系统中的 ResourceName
 * 
 * 使用场景：
 * - AuthGuard 路由守卫：根据路由路径获取对应资源，调用 usePermissions().can() 检查权限
 * - 组件权限检查：页面组件可根据当前路由确定资源，进行细粒度权限控制
 * 
 * 映射规则：
 * 1. 主路由 → 对应主模块资源（如 '/home' → 'home'）
 * 2. 子路由 → 对应子功能资源（如 '/home/news' → 'home_news'）
 * 3. 特殊白名单路由 → 独立资源（如 '/terminal/manual' → 'terminal_manual'）
 */
export const ROUTE_TO_RESOURCE: Record<string, ResourceName> = {
  // ===== 主路由 (11个) =====
  [ROUTES.ROOT]: 'home',
  // 根路径 → 首页
  [ROUTES.HOME]: 'home',
  // 首页
  [ROUTES.PROFILE]: 'profile_settings',
  // 个人中心
  [ROUTES.AI]: 'ai_chat',
  // AI会话
  [ROUTES.KNOWLEDGE]: 'knowledge',
  // 知识库
  [ROUTES.TERMINAL]: 'terminal',
  // 终端
  [ROUTES.XIN]: 'xin',
  // 小欣
  [ROUTES.GAME]: 'game',
  // 游戏
  [ROUTES.GAME_PLAY]: 'game',
  // 游戏-3D游戏页（复用 game 资源权限）
  [ROUTES.GAME_PLAY_MAPPING]: 'game',
  // 游戏-知识领域映射设置页（复用 game 资源权限）
  [ROUTES.RECYCLE]: 'recycle_bin',
  // 回收站
  [ROUTES.SEARCH]: 'search',
  // 搜索
  [ROUTES.SPYGLASS]: 'spyglass',
  // 底层智能

  // ===== 首页子路由 (6个) =====
  [ROUTES.HOME_NEWS]: 'home_news',
  // 首页-新闻
  [ROUTES.HOME_TODO]: 'home_todo',
  // 首页-待办
  [ROUTES.HOME_LOG]: 'home_log',
  // 首页-日志
  [ROUTES.HOME_TIMER]: 'home_timer',
  // 首页-计时器
  [ROUTES.HOME_TIMER_SHORT]: 'home_timer',
  // 首页-短计时器（复用 home_timer）
  [ROUTES.HOME_TIMER_LONG]: 'home_timer',
  // 首页-长计时器（复用 home_timer）

  // ===== 个人中心子路由 (4个) → 统一使用 profile_settings =====
  [ROUTES.PROFILE_ACCOUNT]: 'profile_settings',
  // 个人中心-账号
  [ROUTES.PROFILE_RESUME]: 'profile_settings',
  // 个人中心-简历
  [ROUTES.PROFILE_SETTING]: 'profile_settings',
  // 个人中心-设置
  [ROUTES.PROFILE_LOGOUT]: 'profile_settings',
  // 个人中心-登出
  [ROUTES.PROFILE_QUOTE]: 'profile_settings',
  // 个人中心-语录

  // ===== AI会话子路由 (3个) → 统一使用 ai_chat =====
  [ROUTES.AI_MODEL]: 'ai_chat',
  // AI-模型管理
  [ROUTES.AI_CHAT]: 'ai_chat',
  // AI-聊天
  [ROUTES.AI_GROUP]: 'ai_chat',
  // AI-群聊

  // ===== 终端子路由 (8个) =====
  [ROUTES.TERMINAL_TERMINAL]: 'terminal',
  // 终端-终端（主终端功能）
  [ROUTES.TERMINAL_YUANCODE]: 'terminal_yuancode',
  // 终端-YuanCode（独立资源，支持白名单）
  [ROUTES.TERMINAL_LINUX]: 'terminal_linux',
  // 终端-Linux（独立资源，支持白名单）
  [ROUTES.TERMINAL_MANUAL]: 'terminal_manual',
  // 终端-命令手册（独立资源，支持白名单）
  [ROUTES.TERMINAL_MANUAL_TERMINAL]: 'terminal_manual',
  // 命令手册-终端命令
  [ROUTES.TERMINAL_MANUAL_YUANCODE]: 'terminal_manual',
  // 命令手册-YuanCode命令
  [ROUTES.TERMINAL_MANUAL_LINUX]: 'terminal_manual',
  // 命令手册-Linux命令
  [ROUTES.TERMINAL_MANUAL_SHORTCUTS]: 'terminal_manual' // 命令手册-快捷键
};

/**
 * 获取路由对应的资源名称
 * 
 * @param routePath 路由路径（如 '/home', '/terminal/manual'）
 * @returns 对应的 ResourceName，如果未找到则返回 'home' 作为默认值
 * 
 * 匹配策略：
 * 1. 精确匹配：直接查找 ROUTE_TO_RESOURCE 表
 * 2. 前缀匹配：对于动态路由（如 '/ai/chat/:id'），尝试前缀匹配
 * 3. 默认值：如果都匹配不到，返回 'home'
 * 
 * 使用示例：
 * ```typescript
 * const resource = getRouteResource('/terminal/manual')
 * // 返回: 'terminal_manual'
 * 
 * const resource = getRouteResource('/ai/chat/123')
 * // 返回: 'ai_chat' (通过前缀匹配)
 * ```
 */
export function getRouteResource(routePath: string): ResourceName {
  // 1. 精确匹配：直接查找映射表
  if (ROUTE_TO_RESOURCE[routePath]) {
    return ROUTE_TO_RESOURCE[routePath];
  }

  // 2. 前缀匹配：处理动态路由参数（如 /ai/chat/:id）
  // 按路径长度降序排序，优先匹配更长的路径
  const sortedRoutes = Object.keys(ROUTE_TO_RESOURCE).sort((a, b) => b.length - a.length);
  for (const route of sortedRoutes) {
    // 处理动态路由参数（:id, :userId 等）
    const routePattern = route.replace(/\/:[^/]+/g, '/[^/]+');
    const regex = new RegExp(`^${routePattern}$`);
    if (regex.test(routePath)) {
      return ROUTE_TO_RESOURCE[route];
    }

    // 简单前缀匹配（用于子页面）
    if (routePath.startsWith(route + '/') || routePath === route) {
      return ROUTE_TO_RESOURCE[route];
    }
  }

  // 3. 默认值：返回首页资源
  console.warn(`[Route] ⚠️ 未找到路由 '${routePath}' 的资源映射，使用默认值 'home'`);
  return 'home';
}

/**
 * 获取路由对应的主路由路径
 */
function getMainRoutePath(routePath: RoutePath): MainRoutePath {
  // 如果是主路由，直接返回
  if (Object.values(ROUTES).includes(routePath as MainRoutePath)) {
    return routePath as MainRoutePath;
  }

  // 如果是子路由，找到对应的主路由
  for (const mainPath of Object.values(ROUTES).slice(0, 11)) {
    // 前11个是主路由
    if (routePath.startsWith(mainPath)) {
      return mainPath as MainRoutePath;
    }
  }

  // 默认返回首页
  return ROUTES.HOME;
}

/**
 * ⚠️ @deprecated 使用 `usePermissions()` hook 替代
 * 
 * 检查用户是否有权限访问指定路由 (v1.0 旧版本)
 * 
 * 废弃时间：2026-05-26
 * 替代方案：
 * ```typescript
 * import { usePermissions } from '@/hooks/usePermissions'
 * import { getRouteResource } from './routes'
 * 
 * const { can } = usePermissions()
 * const resource = getRouteResource(routePath)
 * const hasAccess = can(resource, 'read')
 * ```
 * 
 * 当前行为（向后兼容）：
 * - 此函数仍然可以正常工作
 * - 内部已重构为使用 ROUTE_TO_RESOURCE 映射
 * - 基于默认权限矩阵进行判断（不依赖后端实时数据）
 * - 建议新代码直接使用 usePermissions hook
 * 
 * @param routePath 路由路径
 * @param isTempAccount 是否为临时账号（已废弃参数，保留兼容性）
 * @returns 是否有权限访问
 */
export function canAccessRoute(routePath: RoutePath, isTempAccount: boolean): boolean {
  console.warn('[Routes] ⚠️ canAccessRoute() 已废弃，请使用 usePermissions().can(getRouteResource(route), \'read\')');

  // 获取路由对应的资源名称
  const routeStr = typeof routePath === 'string' ? routePath : '';
  const resource = getRouteResource(routeStr);

  // 导入默认权限配置（避免循环依赖，此处内联简化逻辑）
  // 注意：此函数无法使用 hook，因此只能基于静态配置判断
  // 对于需要实时权限的场景，请使用 usePermissions hook

  // 临时账号的默认权限规则（从 DEFAULT_PERMISSIONS.guest 提取的简化版）
  if (isTempAccount) {
    // 白名单资源（临时账号可读）
    const guestReadonlyResources: ResourceName[] = ['home', 'home_news', 'home_todo', 'home_log', 'home_timer', 'ai_chat', 'knowledge', 'game', 'profile_settings', 'search', 'xin', 'terminal_manual', 'terminal_yuancode', 'terminal_linux' // 终端白名单
    ];
    return guestReadonlyResources.includes(resource);
  }

  // 永久账号 / 管理员默认拥有所有资源的 read 权限
  return true;
}

/**
 * 获取路由标题
 */
export function getRouteTitle(routePath: RoutePath): string {
  const mainPath = getMainRoutePath(routePath);
  return ROUTE_META[mainPath]?.title || t("xin.mockData.k1");
}