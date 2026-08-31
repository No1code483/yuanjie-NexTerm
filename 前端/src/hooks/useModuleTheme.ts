/**
 * useModuleTheme Hook（C1.4 / v1.51.6）
 *
 * 功能：
 *   - 监听当前路由变化
 *   - 根据当前模块名 + themeStore.moduleThemes 映射，在 <html> 上设置 data-module-theme 属性
 *   - 当模块主题功能关闭、或当前模块未配置主题时，移除 data-module-theme 属性
 *
 * 实现原理：
 *   - themes.css 中每个主题选择器为 :root:is([data-theme="xxx"], [data-module-theme="xxx"])
 *   - :root 前缀 + :is() 将特异性提升至 (0,2,0)，确保优先于 theme.css 中 :root 默认值 (0,1,0)
 *   - 当 <html> 同时拥有 data-theme="A" 和 data-module-theme="B" 时，
 *     两个主题块都匹配 :root（特异性相等 0,2,0），CSS 源码中靠后的规则获胜。
 *     因此模块主题块需定义在全局主题块之后才能生效——当前各主题按固定顺序排列，
 *     模块主题能否覆盖全局主题取决于两者在文件中的先后位置。
 *   - 实际效果：模块主题生效时，当前页面会用模块主题的色板渲染，
 *     而全局 data-theme 属性不变 —— 仍然由全局 store 控制）
 *
 * 用法：
 *   在 Layout 组件中调用一次即可全局生效：
 *   useModuleTheme();
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.4
 */

import { useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import { useThemeStore } from '../stores/themeStore';

/**
 * 可配置模块级主题的目标模块列表。
 * key   —— 用于 moduleThemes 映射的键（与路由第一段一致）
 * label —— UI 展示用国际化 key（i18n 命名空间：hooks.useModuleTheme.<key>）
 */
export const MODULE_THEME_TARGETS: ReadonlyArray<{ key: string; labelKey: string }> = [
  { key: 'home', labelKey: 'hooks.useModuleTheme.home' },
  { key: 'profile', labelKey: 'hooks.useModuleTheme.profile' },
  { key: 'ai', labelKey: 'hooks.useModuleTheme.ai' },
  { key: 'knowledge', labelKey: 'hooks.useModuleTheme.knowledge' },
  { key: 'terminal', labelKey: 'hooks.useModuleTheme.terminal' },
  { key: 'xin', labelKey: 'hooks.useModuleTheme.xin' },
  { key: 'game', labelKey: 'hooks.useModuleTheme.game' },
  { key: 'recycle', labelKey: 'hooks.useModuleTheme.recycle' },
  { key: 'search', labelKey: 'hooks.useModuleTheme.search' },
  { key: 'spyglass', labelKey: 'hooks.useModuleTheme.spyglass' },
];

/** 合法的模块名集合（用于校验） */
const VALID_MODULE_KEYS = new Set(MODULE_THEME_TARGETS.map(t => t.key));

/**
 * 从路径提取主模块名（取第一段）
 * 例：/terminal/yuancode → 'terminal'
 *     /ai/chat/123 → 'ai'
 *     / → null
 */
export function getModuleNameFromPath(pathname: string): string | null {
  const match = pathname.match(/^\/([^/]+)/);
  if (!match) return null;
  const seg = match[1];
  // 仅返回已知模块；未知路径（如根 / 或 404）返回 null，表示使用全局主题
  return VALID_MODULE_KEYS.has(seg) ? seg : null;
}

/**
 * useModuleTheme Hook
 *
 * 在 Layout 中调用一次，自动同步 DOM 上的 data-module-theme 属性。
 * 不返回值 —— 这是一个副作用 hook。
 */
export function useModuleTheme(): void {
  const location = useLocation();
  const moduleThemes = useThemeStore(s => s.moduleThemes);
  const moduleThemeEnabled = useThemeStore(s => s.moduleThemeEnabled);

  const moduleName = getModuleNameFromPath(location.pathname);

  useEffect(() => {
    const root = document.documentElement;

    // C1.6 / Phase 4：模块主题切换也走过渡动画（与全局主题切换一致）
    // 复用 themeStore 的 animationsEnabled / animationDuration 判断
    const state = useThemeStore.getState();
    const shouldTransition = state.animationsEnabled
      && !(typeof window !== 'undefined'
        && window.matchMedia
        && window.matchMedia('(prefers-reduced-motion: reduce)').matches);

    if (shouldTransition) {
      root.setAttribute('data-theme-transitioning', '');
    }

    // 应用模块主题
    if (!moduleThemeEnabled || !moduleName) {
      root.removeAttribute('data-module-theme');
    } else {
      const moduleTheme = moduleThemes?.[moduleName];
      if (moduleTheme) {
        root.setAttribute('data-module-theme', moduleTheme);
      } else {
        root.removeAttribute('data-module-theme');
      }
    }

    // 过渡完成后移除标记（仅在启用过渡时）
    if (shouldTransition) {
      const duration = state.animationDuration + 50;
      const timer = window.setTimeout(() => {
        root.removeAttribute('data-theme-transitioning');
      }, duration);
      return () => window.clearTimeout(timer);
    }
  }, [moduleThemeEnabled, moduleName, moduleThemes]);
}

export default useModuleTheme;
