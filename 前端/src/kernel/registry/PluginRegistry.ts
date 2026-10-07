import { createElement } from 'react';
import { Navigate } from 'react-router-dom';
import type { RouteObject } from 'react-router-dom';
import type { PluginId } from '../types';
import type { FrontendPlugin, SlotComponentContribution } from './definePlugin';

export interface RejectRecord {
  pluginId: PluginId;
  reason: string; // 如 'slot home.widgets 容量已满(4/4)'
}

export interface NavNode {
  id: string;
  labelKey: string;
  icon?: string;
  order: number;
  routePath: string;
  children?: NavNode[];
}

interface AppliedPlugin {
  plugin: FrontendPlugin;
  /** L1 路由前缀（首个 route.path）；L2 拼接父前缀用，custom 为 '' */
  routePrefix: string;
}

interface MainNavItem extends NavNode {
  pluginId: PluginId;
}

/** 拼接父子路由路径（规范化多余斜杠）。
 *  相对串以 '?' 开头时为查询串直连（前缀 + 相对串），用于同页 tab 型 L2 子插件
 *  （如 knowledge.search 侧边栏选项 routePath='?tab=search' → '/knowledge?tab=search'）。 */
function joinPath(prefix: string, relative: string): string {
  if (!relative) return prefix;
  if (relative.startsWith('?')) return `${prefix.replace(/\/+$/, '')}${relative}`;
  return `${prefix.replace(/\/+$/, '')}/${relative.replace(/^\/+/, '')}`;
}

export class PluginRegistry {
  private applied: AppliedPlugin[] = [];

  /** 校验失败集合（显式失败不静默：parent 未启用/slot 不存在/超容） */
  readonly rejects: RejectRecord[] = [];

  /** 应用一个插件的贡献（内部执行分类路由安装校验） */
  apply(p: FrontendPlugin): void {
    const { manifest, contributions } = p;

    // L2 前置校验（07 §四修订：显式失败不静默）
    const parentId = manifest.parent ?? '';
    const parent =
      manifest.level === 'feature'
        ? this.applied.find((a) => a.plugin.manifest.id === parentId)
        : undefined;

    if (manifest.level === 'feature') {
      if (!parent) {
        this.rejects.push({
          pluginId: manifest.id,
          reason: `parent ${parentId || '(未声明)'} 未启用`,
        });
        return;
      }
      const slotSpec = parent.plugin.manifest.slots.find((s) => s.id === manifest.slot);
      if (!slotSpec) {
        this.rejects.push({
          pluginId: manifest.id,
          reason: `slot ${manifest.slot ?? '(未声明)'} 不存在于 parent ${parentId}`,
        });
        return;
      }
      const used = this.getSlotComponents(slotSpec.id).length;
      if (used + 1 > slotSpec.capacity) {
        this.rejects.push({
          pluginId: manifest.id,
          reason: `slot ${slotSpec.id} 容量已满(${used}/${slotSpec.capacity})`,
        });
        return;
      }
    }
    // board / custom：阶段1 无前置校验（custom 横切，无 slot 强约束）

    const routePrefix =
      manifest.level === 'board'
        ? (contributions.routes[0]?.path ?? '')
        : manifest.level === 'feature'
          ? (parent?.routePrefix ?? '')
          : '';
    this.applied.push({ plugin: p, routePrefix });
  }

  /** 汇总生成 react-router 数据路由（替代集中 router.tsx）
   *  默认路由规则（内核固定兜底，不依赖任何插件）：
   *  '/' 重定向 → order 最小的【启用中且挂 kernel:main】的板块路由；
   *  无任何启用板块 → 渲染内核自有空态页（绝不白屏） */
  buildRouter(): RouteObject[] {
    const routes: RouteObject[] = [];
    for (const { plugin, routePrefix } of this.applied) {
      for (const r of plugin.contributions.routes) {
        const path =
          plugin.manifest.level === 'feature' ? joinPath(routePrefix, r.path) : r.path;
        routes.push({ path, element: createElement(r.component) });
      }
    }

    const main = this.mainNavItems();
    routes.push({
      index: true,
      element: main.length > 0
        ? createElement(Navigate, { to: main[0].routePath, replace: true })
        : createElement('div', { 'data-kernel-empty': true }, '未启用任何板块'),
    });
    return routes;
  }

  /** 汇总生成导航树（主干 + 各板块内菜单，支持多级嵌套）
   *  主干 = target 'kernel:main'，按 order 升序、同值按插件 id 字典序；
   *  L2 板块内菜单 = target 为其 parent 声明的 slot id → 挂到父节点 children
   *  （父可为 L1 或已挂载的 L2，`nodeByPlugin` 索引支持任意层级嵌套）。 */
  buildNavTree(): NavNode[] {
    const main = this.mainNavItems();

    // 节点索引：插件 id → 已建节点（主干 + 各级子节点），支持父为 feature 的递归挂载
    const nodeByPlugin = new Map<PluginId, NavNode>();
    for (const m of main) nodeByPlugin.set(m.pluginId, m);

    // applied 已按父链拓扑序（buildApp 保证父先于子），故单遍即可
    for (const { plugin, routePrefix } of this.applied) {
      if (plugin.manifest.level !== 'feature') continue;
      const slotId = plugin.manifest.slot;
      if (slotId == null) continue;

      for (const nav of plugin.contributions.navItems) {
        if (nav.target !== slotId) continue;
        const parentNode = plugin.manifest.parent
          ? nodeByPlugin.get(plugin.manifest.parent)
          : undefined;
        if (!parentNode) continue; // 父未挂载（apply 阶段已 reject），防御性跳过
        const node: NavNode = {
          id: `${plugin.manifest.id}:${nav.routePath}`,
          labelKey: nav.labelKey,
          icon: nav.icon,
          order: nav.order,
          routePath: joinPath(routePrefix, nav.routePath),
        };
        (parentNode.children ??= []).push(node);
        nodeByPlugin.set(plugin.manifest.id, node);
      }
    }
    sortNavChildren(main);

    return main.map((m) => ({
      id: m.id,
      labelKey: m.labelKey,
      icon: m.icon,
      order: m.order,
      routePath: m.routePath,
      children: m.children,
    }));
  }

  /** 查询插槽挂载组件（SlotRenderer 调用） */
  getSlotComponents(slotId: string): SlotComponentContribution[] {
    const out: SlotComponentContribution[] = [];
    for (const { plugin } of this.applied) {
      for (const sc of plugin.contributions.slotComponents) {
        if (sc.slot === slotId) out.push(sc);
      }
    }
    return out;
  }

  /** 撤销插件全部注册（禁用回滚） */
  revoke(pluginId: PluginId): void {
    this.applied = this.applied.filter((a) => a.plugin.manifest.id !== pluginId);
  }

  /** 主干导航项（kernel:main），order 升序、同值按插件 id 字典序 */
  private mainNavItems(): MainNavItem[] {
    const items: MainNavItem[] = [];
    for (const { plugin } of this.applied) {
      for (const nav of plugin.contributions.navItems) {
        if (nav.target !== 'kernel:main') continue;
        items.push({
          id: `${plugin.manifest.id}:${nav.routePath}`,
          labelKey: nav.labelKey,
          icon: nav.icon,
          order: nav.order,
          routePath: nav.routePath,
          pluginId: plugin.manifest.id,
        });
      }
    }
    items.sort((a, b) => a.order - b.order || a.pluginId.localeCompare(b.pluginId));
    return items;
  }
}

/** 递归按 order 排序各层子节点（多级嵌套导航树） */
function sortNavChildren(nodes: NavNode[]): void {
  for (const n of nodes) {
    if (!n.children) continue;
    n.children.sort((a, b) => a.order - b.order);
    sortNavChildren(n.children);
  }
}
