import { createContext, useContext, type ReactNode } from 'react';
import type { PluginRegistry, NavNode } from './PluginRegistry';

/** RegistryProvider：向插件组件树提供唯一 PluginRegistry 实例
 *  （07 契约 §六 SlotRenderer 依赖，契约未给完整实现——按最小实现补齐） */
const RegistryContext = createContext<PluginRegistry | null>(null);

/** 导航树上下文：buildApp 产出的 navTree 经此注入 Layout（侧边栏注册表驱动）；
 *  与 registry 同源、同生命周期，避免 Layout 重复 buildNavTree。 */
const NavTreeContext = createContext<NavNode[]>([]);

export function RegistryProvider({
  registry,
  navTree = [],
  children,
}: {
  registry: PluginRegistry;
  navTree?: NavNode[];
  children: ReactNode;
}) {
  return (
    <RegistryContext.Provider value={registry}>
      <NavTreeContext.Provider value={navTree}>{children}</NavTreeContext.Provider>
    </RegistryContext.Provider>
  );
}

export function useRegistry(): PluginRegistry {
  const reg = useContext(RegistryContext);
  if (!reg) throw new Error('useRegistry 必须在 <RegistryProvider> 内使用');
  return reg;
}

/** 读取注册表导航树（主干 + 各级板块内菜单） */
export function useNavTree(): NavNode[] {
  return useContext(NavTreeContext);
}
