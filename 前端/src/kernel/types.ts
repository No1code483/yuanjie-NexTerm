/** 与 crates/kernel-api/src/lib.rs 严格对齐——任何一侧改字段，两侧同改
 *  契约来源：07_TypeScript代码契约.md §二（插件化重构阶段1 F1） */

export type PluginId = string;

export type PluginLevel = 'board' | 'feature' | 'custom';

export interface Manifest {
  id: PluginId;
  name: string;
  level: PluginLevel;
  parent?: PluginId; // L2/L3 必填
  slot?: string; // L2 必填
  version: string; // semver 三段
  kernelApi: string; // 如 '1'
  permissions: Permissions;
  slots: SlotSpec[]; // L1 声明的插槽
  i18nNamespace: string;
}

export interface Permissions {
  db: string[];
  events: { subscribe: string[]; publish: string[] };
  ipc: string[]; // 如 ['hm_todo_*']
  fs: string[];
  net: string[];
}

export interface SlotSpec {
  id: string;
  type: 'ui-route' | 'panel' | 'settings-card' | 'command-palette' | 'service';
  description: string;
  capacity: number;
  routePrefix?: string;
}

/** 三域事件信封（与 kernel-api Event 对齐） */
export interface KernelEvent {
  name: string; // 'home:todo.updated'
  origin: PluginId;
  domain: string;
  scope: 'domain' | 'live' | 'capability';
  payload: unknown; // 只放摘要
  at: number; // 毫秒时间戳
}

export type PluginState = 'registered' | 'installed' | 'enabled' | 'disabled' | 'error';
