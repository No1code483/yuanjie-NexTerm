import { create } from 'zustand';
import type { PluginId, PluginLevel } from '@/kernel/types';

/** 与后端 tauri_glue.rs PluginInfo 对齐（camelCase 序列化） */
export interface PluginInfo {
  id: PluginId;
  name: string;
  level: PluginLevel;
  version: string;
  parent?: PluginId;
  slot?: string;
  /** "registered"/"enabled"/"disabled"/"error"/...（PluginState::as_str） */
  state: string;
  /** 必备插件（后端 REQUIRED_PLUGINS 名单，不可停用；手稿 20260926） */
  required: boolean;
}

/** 板块插件展示序（对齐顶部导航；手稿 20260926：八大板块）——未列出的板块按 id 稳定追加在后 */
export const BOARD_ORDER: PluginId[] = [
  'boards.home',      // 首页（必备根板块）
  'boards.ai',        // AI会话
  'boards.knowledge', // 知识库
  'boards.terminal',  // 终端
  'boards.xin',       // 小欣
  'boards.game',      // 游戏
  'customs.search',   // 搜索
  '_hello',           // Hello
];

/** 插件管理为首页必备子插件（内核级页面，非注册表插件；手稿 20260926：仅本页展示，不可停用/卸载） */
export const PLUGIN_MANAGER_ENTRY: PluginInfo = {
  id: 'kernel:plugin-manager',
  name: '插件管理',
  level: 'feature',
  version: '—',
  parent: 'boards.home',
  slot: 'home.pluginmanager',
  state: 'enabled',
  required: true,
};

/** 板块插件判定（level=board 且无父级） */
export function isBoardPlugin(p: PluginInfo): boolean {
  return p.level === 'board' && !p.parent;
}

/** 子插件排序：必备优先，其余按 id 稳定排序 */
function compareChildren(a: PluginInfo, b: PluginInfo): number {
  return Number(b.required) - Number(a.required) || a.id.localeCompare(b.id);
}

/** 某插件的一级子插件（含停用；首页另追加内核级「插件管理」条目，由调用方决定） */
export function childrenOf(plugins: PluginInfo[], parentId: PluginId): PluginInfo[] {
  return plugins.filter((p) => p.parent === parentId).sort(compareChildren);
}

/** 沿父链回溯到顶层项（选中子插件时侧边栏保持其根项高亮） */
export function rootOf(plugins: PluginInfo[], id: PluginId): PluginId {
  const byId = new Map(plugins.map((p) => [p.id, p]));
  let cur = byId.get(id);
  const guard = new Set<PluginId>();
  while (cur?.parent && !guard.has(cur.id)) {
    guard.add(cur.id);
    const next = byId.get(cur.parent);
    if (!next) break;
    cur = next;
  }
  return cur?.id ?? id;
}

/** 祖先链（顶层 → 当前，用于 n 级层级钻取的面包屑；不含当前自身） */
export function ancestorsOf(plugins: PluginInfo[], id: PluginId): PluginInfo[] {
  const byId = new Map(plugins.map((p) => [p.id, p]));
  const chain: PluginInfo[] = [];
  let cur = byId.get(id);
  const guard = new Set<PluginId>();
  while (cur?.parent && !guard.has(cur.id)) {
    guard.add(cur.id);
    const parent = byId.get(cur.parent);
    if (!parent) break;
    chain.unshift(parent);
    cur = parent;
  }
  return chain;
}

/** 状态徽标（契约 07 §九：✅🔶⬜❌；linear status-badge 结构） */
export const STATE_LABEL: Record<string, string> = {
  enabled: '✅ 已启用',
  disabled: '🔶 已停用',
  registered: '⬜ 未激活',
  installed: '⬜ 已安装',
  error: '❌ 错误',
};

export function stateLabel(state: string): string {
  return STATE_LABEL[state] ?? `⬜ ${state}`;
}

export function stateClass(state: string, base: string, styles: Record<string, string>): string {
  switch (state) {
    case 'enabled': return `${base} ${styles.stateEnabled}`;
    case 'disabled': return `${base} ${styles.stateDisabled}`;
    case 'error': return `${base} ${styles.stateError}`;
    default: return `${base} ${styles.stateRegistered}`;
  }
}

/** 侧边栏圆点（语义色，与状态徽标同源） */
export function dotClass(state: string): string {
  switch (state) {
    case 'enabled': return 'dotEnabled';
    case 'disabled': return 'dotDisabled';
    case 'error': return 'dotError';
    default: return 'dotRegistered';
  }
}

interface PluginManagerState {
  /** 全部已注册插件（含停用；数据单一来源 = PluginManagerPage 的 kernel:plugin:list） */
  plugins: PluginInfo[] | null;
  /** 当前选中的插件 id（板块/扩展/钻取中的子插件） */
  selectedId: PluginId;
  setPlugins: (plugins: PluginInfo[]) => void;
  setSelectedId: (id: PluginId) => void;
}

/** 插件管理共享状态：管理页（数据源）与全局侧边栏导航（PluginManagerNav）共用。
 *  数据只在管理页拉取（kernel:plugin:list + state-changed 自刷新），导航组件只读。 */
export const usePluginManagerStore = create<PluginManagerState>()((set) => ({
  plugins: null,
  selectedId: 'boards.home',
  setPlugins: (plugins) => set({ plugins }),
  setSelectedId: (selectedId) => set({ selectedId }),
}));