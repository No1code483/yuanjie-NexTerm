import type { ComponentType, LazyExoticComponent } from 'react';
import type { Manifest } from '../types';

/** 插件可贡献的七类注册物（路由/导航/组件/命令/服务/监听/数据模式）
 *  前端表达为六种（服务在后端） */
export interface PluginContributions {
  /** 挂载路由。path 相对：L1 为绝对前缀如 '/home'；L2 为相对父插槽如 'todo' */
  routes: RouteContribution[];
  /** 导航项：L1 挂主干；L2 挂板块内菜单 */
  navItems: NavItemContribution[];
  /** 插槽组件（嵌插槽核心） */
  slotComponents: SlotComponentContribution[];
  /** IPC 命名空间客户端 */
  ipc?: { namespace: string; methods: Record<string, IpcMethodSpec> };
  /** 插件私有 store 工厂（Zustand） */
  stores?: Record<string, () => unknown>;
  /** 设置中心卡片 */
  settingsCards?: LazyExoticComponent<ComponentType>[];
}

export interface RouteContribution {
  path: string;
  component: LazyExoticComponent<ComponentType>;
}

export interface NavItemContribution {
  /** 挂载目标三类：
   *  'kernel:main'   —— 顶部导航主干 tab 行（7 个 L1 板块默认挂载处）
   *  'kernel:corner' —— 角区（头像/账号区外壳等，profile 类板块默认挂载处）
   *  '<slotId>'      —— 板块内菜单（L2 挂 L1 声明的插槽） */
  target: string;
  labelKey: string; // i18n key
  icon?: string;
  /** 导航排序值。约定：L1 主干采用稀疏值 10/20/.../70，新板块默认 ≥90（尾部后接），
   *  想插入中间取相邻两值之间的值（如 15）；同 order 冲突时按插件 id 字典序稳定排序 */
  order: number;
  routePath: string;
}

export interface SlotComponentContribution {
  slot: string; // 如 'home.widgets'
  component: LazyExoticComponent<ComponentType>;
}

export interface IpcMethodSpec {
  /** 后端命令名（不含插件前缀），如 'list_todos' */
  cmd: string;
  /** mock 供 Vitest/MSW 使用（可选） */
  mock?: (args: unknown) => unknown;
}

export interface FrontendPlugin {
  manifest: Manifest;
  contributions: PluginContributions;
}

/** 插件入口唯一导出形式（与后端 mod.rs 的 init 对应） */
export function definePlugin(p: FrontendPlugin): FrontendPlugin {
  return p;
}
