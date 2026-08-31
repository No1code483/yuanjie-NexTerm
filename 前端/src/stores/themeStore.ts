import { t } from "i18next";
/**
 * 主题状态管理（v1.51.4 体验深化 Phase 1）
 *
 * 架构说明：
 *   - 主题切换通过修改 <html data-theme="xxx"> 属性实现
 *   - CSS 变量由 themes.css 中的 [data-theme="xxx"] 选择器定义
 *   - localStorage key: 'nexterm-theme'（与 index.html FOUC 防闪烁脚本一致）
 *
 * 功能：
 *   - 10 套预设主题切换
 *   - 系统主题跟随（prefers-color-scheme）
 *   - 自定义变量覆盖（Phase 2 扩展）
 *   - 主题导出/导入（Phase 3 扩展）
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_详细设计.md §2.1.2
 */

import { create } from 'zustand';
import { sync } from '@/lib/ipc';

/**
 * A5 Phase 3 Task 5：将配置变更记录到 sync_queue（fire-and-forget）
 *
 * - 非阻塞：不等待后端返回，不影响本地立即生效
 * - 非抛错：后端不可用或调用失败仅告警，不破坏现有逻辑
 * - 仅在 Tauri 环境下调用（非 Tauri 环境静默跳过）
 */
function syncConfigChange(key: string, value: string): void {
  if (typeof window === 'undefined' || !window.__TAURI__) return;
  void sync.recordConfigChange(key, value).catch((e) => {
    console.warn(`[themeStore] 配置 "${key}" 同步入队失败:`, e);
  });
}

// ===== 类型定义 =====

export type ThemeName = 'terminal' | 'matrix' | 'dracula' | 'monokai' | 'one-dark' | 'nord' | 'solarized-dark' | 'solarized-light' | 'github-light' | 'github-dark' | 'high-contrast' | 'system';
export interface ThemeMeta {
  name: ThemeName;
  label: string;
  description: string;
  isLight: boolean;
  /** 预览色块（用于主题切换器 UI） */
  preview: {
    bg: string;
    text: string;
    accent: string;
    secondary: string;
  };
}
export interface ThemeState {
  /** 当前主题名称 */
  themeName: ThemeName;
  /** 是否跟随系统主题 */
  followSystem: boolean;
  /** 自定义变量覆盖（Phase 2，当前未使用） */
  customOverrides: Record<string, string> | null;
  /** C1.2：当前字体家族（CSS font-family 值） */
  fontFamily: string;
  /** C1.2：当前字号（px） */
  fontSize: number;
  /** C1.3：系统切换到 dark 时使用的主题（followSystem=true 时生效） */
  systemThemeDark: string;
  /** C1.3：系统切换到 light 时使用的主题（followSystem=true 时生效） */
  systemThemeLight: string;
  /** C1.6：是否启用动画 */
  animationsEnabled: boolean;
  /** C1.6：动画时长（ms），可选 100/200/400 */
  animationDuration: number;
  /** C1.4：模块级主题映射（moduleName → themeName） */
  moduleThemes: Record<string, string>;
  /** C1.4：是否启用模块级主题 */
  moduleThemeEnabled: boolean;

  // Actions
  /** 切换主题 */
  setTheme: (name: ThemeName) => void;
  /** 切换系统主题跟随 */
  toggleFollowSystem: () => void;
  /** 设置自定义变量覆盖（Phase 2） */
  setCustomVariable: (key: string, value: string) => void;
  /** 清除自定义变量覆盖 */
  resetCustomVariables: () => void;
  /** C1.2：设置字体家族 */
  setFontFamily: (family: string) => void;
  /** C1.2：设置字号（12-24px） */
  setFontSize: (size: number) => void;
  /** C1.3：设置系统 dark 主题 */
  setSystemThemeDark: (themeName: string) => void;
  /** C1.3：设置系统 light 主题 */
  setSystemThemeLight: (themeName: string) => void;
  /** C1.6：设置动画开关 */
  setAnimationsEnabled: (enabled: boolean) => void;
  /** C1.6：设置动画时长（100/200/400） */
  setAnimationDuration: (duration: number) => void;
  /** C1.4：设置某模块的主题（moduleName 如 'terminal'/'knowledge'/'chat'） */
  setModuleTheme: (module: string, themeName: string) => void;
  /** C1.4：清除某模块的主题映射 */
  clearModuleTheme: (module: string) => void;
  /** C1.4：启用/禁用模块级主题 */
  setModuleThemeEnabled: (enabled: boolean) => void;
  /** 导出当前主题配置为 JSON */
  exportTheme: () => string;
  /** 导入主题配置 JSON */
  importTheme: (json: string) => {
    success: boolean;
    error?: string;
  };
}

// ===== 预设主题元数据 =====

export const PRESET_THEMES: ThemeMeta[] = [{
  name: 'terminal',
  label: 'Terminal',
  description: t("stores.themeStore.k1"),
  isLight: false,
  preview: {
    bg: '#090300',
    text: '#c8c8dd',
    accent: '#00F0FF',
    secondary: '#FF006E'
  }
}, {
  name: 'matrix',
  label: 'Matrix',
  description: t("stores.themeStore.k2"),
  isLight: false,
  preview: {
    bg: '#000000',
    text: '#00FF00',
    accent: '#00FF00',
    secondary: '#FF0000'
  }
}, {
  name: 'dracula',
  label: 'Dracula',
  description: t("stores.themeStore.k3"),
  isLight: false,
  preview: {
    bg: '#282a36',
    text: '#f8f8f2',
    accent: '#bd93f9',
    secondary: '#ff79c6'
  }
}, {
  name: 'monokai',
  label: 'Monokai',
  description: t("stores.themeStore.k4"),
  isLight: false,
  preview: {
    bg: '#272822',
    text: '#f8f8f2',
    accent: '#a6e22e',
    secondary: '#fd971f'
  }
}, {
  name: 'one-dark',
  label: 'One Dark',
  description: t("stores.themeStore.k5"),
  isLight: false,
  preview: {
    bg: '#282c34',
    text: '#abb2bf',
    accent: '#61afef',
    secondary: '#c678dd'
  }
}, {
  name: 'nord',
  label: 'Nord',
  description: t("stores.themeStore.k6"),
  isLight: false,
  preview: {
    bg: '#2e3440',
    text: '#d8dee9',
    accent: '#88c0d0',
    secondary: '#b48ead'
  }
}, {
  name: 'solarized-dark',
  label: 'Solarized Dark',
  description: t("stores.themeStore.k7"),
  isLight: false,
  preview: {
    bg: '#002b36',
    text: '#93a1a1',
    accent: '#268bd2',
    secondary: '#6c71c4'
  }
}, {
  name: 'solarized-light',
  label: 'Solarized Light',
  description: t("stores.themeStore.k8"),
  isLight: true,
  preview: {
    bg: '#fdf6e3',
    text: '#657b83',
    accent: '#268bd2',
    secondary: '#6c71c4'
  }
}, {
  name: 'github-light',
  label: 'GitHub Light',
  description: t("stores.themeStore.k9"),
  isLight: true,
  preview: {
    bg: '#ffffff',
    text: '#24292f',
    accent: '#0969da',
    secondary: '#8250df'
  }
}, {
  name: 'github-dark',
  label: 'GitHub Dark',
  description: t("stores.themeStore.k10"),
  isLight: false,
  preview: {
    bg: '#0d1117',
    text: '#c9d1d9',
    accent: '#58a6ff',
    secondary: '#bc8cff'
  }
}, {
  name: 'high-contrast',
  label: 'High Contrast',
  description: t("stores.themeStore.k11"),
  isLight: false,
  preview: {
    bg: '#000000',
    text: '#ffffff',
    accent: '#ffff00',
    secondary: '#ff0000'
  }
}];

// ===== 工具函数 =====

const STORAGE_KEY = 'nexterm-theme';
const STORAGE_KEY_FOLLOW = 'nexterm-theme-follow';
/** C1.1：自定义变量覆盖持久化 key */
const STORAGE_KEY_CUSTOM = 'nexterm-theme-custom';
/** C1.2：字体家族持久化 key */
const STORAGE_KEY_FONT_FAMILY = 'nexterm-font-family';
/** C1.2：字号持久化 key */
const STORAGE_KEY_FONT_SIZE = 'nexterm-font-size';
/** C1.6：动画开关持久化 key */
const STORAGE_KEY_ANIMATIONS_ENABLED = 'nexterm-animations-enabled';
/** C1.6：动画时长持久化 key */
const STORAGE_KEY_ANIMATION_DURATION = 'nexterm-animation-duration';
/** C1.4：模块级主题映射持久化 key */
const STORAGE_KEY_MODULE_THEMES = 'nexterm-module-themes';
/** C1.4：模块级主题开关持久化 key */
const STORAGE_KEY_MODULE_THEME_ENABLED = 'nexterm-module-theme-enabled';

/** 从 localStorage 读取初始主题 */
function getInitialTheme(): ThemeName {
  try {
    const saved = localStorage.getItem(STORAGE_KEY) as ThemeName | null;
    if (saved && PRESET_THEMES.some(t => t.name === saved)) {
      return saved;
    }
    // 检查是否跟随系统
    const followSystem = localStorage.getItem(STORAGE_KEY_FOLLOW) === 'true';
    if (followSystem) return 'system';
  } catch {/* localStorage 不可用 */}
  return 'terminal';
}

/** 从 localStorage 读取初始 followSystem */
function getInitialFollowSystem(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY_FOLLOW) === 'true';
  } catch {
    return false;
  }
}

/** C1.1：从 localStorage 读取自定义变量覆盖 */
function getInitialCustomOverrides(): Record<string, string> | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_CUSTOM);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    return (parsed && typeof parsed === 'object') ? parsed : null;
  } catch {
    return null;
  }
}

/** C1.1：持久化自定义变量覆盖到 localStorage */
function persistCustomOverrides(overrides: Record<string, string> | null): void {
  try {
    if (overrides && Object.keys(overrides).length > 0) {
      localStorage.setItem(STORAGE_KEY_CUSTOM, JSON.stringify(overrides));
    } else {
      localStorage.removeItem(STORAGE_KEY_CUSTOM);
    }
  } catch { /* localStorage 不可用 */ }
}

/** C1.2：从 localStorage 读取初始字体家族 */
function getInitialFontFamily(): string {
  try {
    return localStorage.getItem(STORAGE_KEY_FONT_FAMILY) || "'JetBrains Mono', 'Consolas', monospace";
  } catch {
    return "'JetBrains Mono', 'Consolas', monospace";
  }
}

/** C1.2：从 localStorage 读取初始字号 */
function getInitialFontSize(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_FONT_SIZE);
    if (raw) {
      const size = parseInt(raw, 10);
      if (!isNaN(size) && size >= 12 && size <= 24) return size;
    }
  } catch { /* ignore */ }
  return 14;
}

/** C1.2：应用字体到 DOM */
function applyFontToDOM(family: string, size: number): void {
  if (typeof document === 'undefined') return;
  document.documentElement.style.setProperty('--nt-font-mono', family);
  document.documentElement.style.setProperty('--nt-font-size', `${size}px`);
  document.documentElement.style.fontSize = `${size}px`;
}

/** C1.6：从 localStorage 读取初始动画开关（默认开启） */
function getInitialAnimationsEnabled(): boolean {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_ANIMATIONS_ENABLED);
    if (raw === 'false') return false;
    if (raw === 'true') return true;
    // 默认尊重系统 prefers-reduced-motion
    if (typeof window !== 'undefined' && window.matchMedia) {
      return !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    }
  } catch { /* ignore */ }
  return true;
}

/** C1.6：从 localStorage 读取初始动画时长（默认 200ms） */
function getInitialAnimationDuration(): number {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_ANIMATION_DURATION);
    if (raw) {
      const dur = parseInt(raw, 10);
      if ([100, 200, 400].includes(dur)) return dur;
    }
  } catch { /* ignore */ }
  return 200;
}

/** C1.4：从 localStorage 读取模块级主题映射（默认空对象） */
function getInitialModuleThemes(): Record<string, string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY_MODULE_THEMES);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    // 仅保留合法键值对（moduleName → themeName），且 themeName 必须是已知的预设主题
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      const validNames = new Set<string>(PRESET_THEMES.map(t => t.name));
      const result: Record<string, string> = {};
      Object.entries(parsed).forEach(([key, value]) => {
        if (typeof key === 'string' && typeof value === 'string' && validNames.has(value)) {
          result[key] = value;
        }
      });
      return result;
    }
  } catch { /* ignore */ }
  return {};
}

/** C1.4：从 localStorage 读取模块级主题开关（默认关闭） */
function getInitialModuleThemeEnabled(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY_MODULE_THEME_ENABLED) === 'true';
  } catch {
    return false;
  }
}

/** C1.4：持久化模块级主题映射到 localStorage */
function persistModuleThemes(map: Record<string, string>): void {
  try {
    if (Object.keys(map).length > 0) {
      localStorage.setItem(STORAGE_KEY_MODULE_THEMES, JSON.stringify(map));
    } else {
      localStorage.removeItem(STORAGE_KEY_MODULE_THEMES);
    }
  } catch { /* localStorage 不可用 */ }
}

/** C1.4：持久化模块级主题开关到 localStorage */
function persistModuleThemeEnabled(enabled: boolean): void {
  try {
    localStorage.setItem(STORAGE_KEY_MODULE_THEME_ENABLED, String(enabled));
  } catch { /* ignore */ }
}

/** C1.6：应用动画配置到 DOM
 * - enabled=false 时在 <html> 上设置 data-animations="off"
 * - 设置全局 --animation-duration 变量
 * - 系统启用 prefers-reduced-motion 时强制设为 0ms
 */
function applyAnimationsToDOM(enabled: boolean, duration: number): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  if (enabled) {
    root.removeAttribute('data-animations');
  } else {
    root.setAttribute('data-animations', 'off');
  }
  // 系统级 reduced-motion 优先：覆盖为 0ms
  let effectiveDuration = duration;
  if (typeof window !== 'undefined' && window.matchMedia) {
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
      effectiveDuration = 0;
    }
  }
  root.style.setProperty('--animation-duration', `${effectiveDuration}ms`);
}

/** C1.3：localStorage keys */
const STORAGE_KEY_SYSTEM_DARK = 'nexterm-system-theme-dark';
const STORAGE_KEY_SYSTEM_LIGHT = 'nexterm-system-theme-light';

/** C1.3：从 localStorage 读取系统 dark 主题 */
function getInitialSystemThemeDark(): string {
  try {
    return localStorage.getItem(STORAGE_KEY_SYSTEM_DARK) || 'terminal';
  } catch {
    return 'terminal';
  }
}

/** C1.3：从 localStorage 读取系统 light 主题 */
function getInitialSystemThemeLight(): string {
  try {
    return localStorage.getItem(STORAGE_KEY_SYSTEM_LIGHT) || 'solarized-light';
  } catch {
    return 'solarized-light';
  }
}

/** C1.3：根据系统当前 prefers-color-scheme 选择对应主题 */
function pickThemeBySystemScheme(darkTheme: string, lightTheme: string): string {
  if (typeof window === 'undefined' || !window.matchMedia) return darkTheme;
  return window.matchMedia('(prefers-color-scheme: dark)').matches ? darkTheme : lightTheme;
}

/** 应用主题到 DOM
 *
 * C1.6 / Phase 4：主题切换过渡动画
 * - 若 animationsEnabled=true 且系统未启用 prefers-reduced-motion，
 *   在切换 data-theme 前先添加 data-theme-transitioning 属性，
 *   让所有元素的颜色相关属性启用 transition；
 *   animationDuration 毫秒后移除该属性。
 * - 否则直接切换，无过渡。
 */
function applyThemeToDOM(name: ThemeName): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;

  // C1.6 / Phase 4：判断是否启用过渡动画
  const state = useThemeStore.getState();
  const shouldTransition = state.animationsEnabled
    && !(typeof window !== 'undefined'
      && window.matchMedia
      && window.matchMedia('(prefers-reduced-motion: reduce)').matches);

  if (shouldTransition) {
    // 添加过渡标记（CSS 在 [data-theme-transitioning] 期间启用颜色 transition）
    root.setAttribute('data-theme-transitioning', '');
    // 切换主题（CSS 变量变化会触发 transition）
    root.setAttribute('data-theme', name);
    // 过渡完成后移除标记（多留 50ms 余量，避免 transition 还没结束就被移除）
    const duration = state.animationDuration + 50;
    window.setTimeout(() => {
      root.removeAttribute('data-theme-transitioning');
    }, duration);
  } else {
    // 不启用过渡：直接切换
    root.setAttribute('data-theme', name);
  }
}

/** 系统主题变化监听器（模块级单例） */
let systemThemeUnlisten: (() => void) | null = null;

/** C1.3：系统主题变化时的回调（由 startSystemThemeListener 注册） */
let systemThemeChangeHandler: ((isDark: boolean) => void) | null = null;

/** 注册系统主题变化监听 */
function startSystemThemeListener(): void {
  if (systemThemeUnlisten || typeof window === 'undefined') return;
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  const handler = (e: MediaQueryListEvent) => {
    console.debug('[themeStore] 系统主题变化:', e.matches ? 'dark' : 'light');
    // C1.3：如果当前是 system 跟随模式，切换到用户指定的 dark/light 主题
    if (systemThemeChangeHandler) {
      systemThemeChangeHandler(e.matches);
    }
  };
  mq.addEventListener('change', handler);
  systemThemeUnlisten = () => mq.removeEventListener('change', handler);
}

/** 停止系统主题变化监听 */
function stopSystemThemeListener(): void {
  if (systemThemeUnlisten) {
    systemThemeUnlisten();
    systemThemeUnlisten = null;
  }
}

// ===== Store 创建 =====

export const useThemeStore = create<ThemeState>((set, get) => ({
  themeName: getInitialTheme(),
  followSystem: getInitialFollowSystem(),
  customOverrides: getInitialCustomOverrides(),
  fontFamily: getInitialFontFamily(),
  fontSize: getInitialFontSize(),
  systemThemeDark: getInitialSystemThemeDark(),
  systemThemeLight: getInitialSystemThemeLight(),
  animationsEnabled: getInitialAnimationsEnabled(),
  animationDuration: getInitialAnimationDuration(),
  moduleThemes: getInitialModuleThemes(),
  moduleThemeEnabled: getInitialModuleThemeEnabled(),
  setTheme: name => {
    applyThemeToDOM(name);
    try {
      localStorage.setItem(STORAGE_KEY, name);
    } catch {/* localStorage 不可用 */}

    // 如果手动切换到具体主题，关闭系统跟随
    if (name !== 'system' && get().followSystem) {
      try {
        localStorage.setItem(STORAGE_KEY_FOLLOW, 'false');
      } catch {/* ignore */}
      stopSystemThemeListener();
      set({
        themeName: name,
        followSystem: false
      });
    } else {
      set({
        themeName: name
      });
    }
    syncConfigChange('theme', name);
  },
  toggleFollowSystem: () => {
    const next = !get().followSystem;
    try {
      localStorage.setItem(STORAGE_KEY_FOLLOW, String(next));
    } catch {/* ignore */}
    if (next) {
      // 开启系统跟随：根据当前系统 prefers-color-scheme 选择用户预设的 dark/light 主题
      const { systemThemeDark, systemThemeLight } = get();
      const picked = pickThemeBySystemScheme(systemThemeDark, systemThemeLight) as ThemeName;
      // 注册系统主题变化回调：变化时自动切换到对应主题
      systemThemeChangeHandler = (isDark: boolean) => {
        const { systemThemeDark: darkT, systemThemeLight: lightT, followSystem: fs } = get();
        if (!fs) return;
        const target = (isDark ? darkT : lightT) as ThemeName;
        applyThemeToDOM(target);
        try {
          localStorage.setItem(STORAGE_KEY, target);
        } catch {/* ignore */}
        set({ themeName: target });
      };
      startSystemThemeListener();
      applyThemeToDOM(picked);
      try {
        localStorage.setItem(STORAGE_KEY, picked);
      } catch {/* ignore */}
      set({
        followSystem: true,
        themeName: picked
      });
    } else {
      // 关闭系统跟随，恢复到具体主题（默认 terminal）
      stopSystemThemeListener();
      systemThemeChangeHandler = null;
      const fallback: ThemeName = 'terminal';
      applyThemeToDOM(fallback);
      try {
        localStorage.setItem(STORAGE_KEY, fallback);
      } catch {/* ignore */}
      set({
        followSystem: false,
        themeName: fallback
      });
    }
    syncConfigChange('followSystem', String(next));
  },
  setCustomVariable: (key, value) => {
    if (typeof document === 'undefined') return;
    document.documentElement.style.setProperty(key, value);
    const nextOverrides = {
      ...(get().customOverrides || {}),
      [key]: value
    };
    persistCustomOverrides(nextOverrides);
    set({
      customOverrides: nextOverrides
    });
    syncConfigChange(`customVariable:${key}`, value);
  },
  resetCustomVariables: () => {
    if (typeof document === 'undefined') return;
    const {
      customOverrides
    } = get();
    if (customOverrides) {
      Object.keys(customOverrides).forEach(key => {
        document.documentElement.style.removeProperty(key);
      });
    }
    persistCustomOverrides(null);
    set({
      customOverrides: null
    });
    syncConfigChange('customVariables', '');
  },
  setFontFamily: family => {
    applyFontToDOM(family, get().fontSize);
    try {
      localStorage.setItem(STORAGE_KEY_FONT_FAMILY, family);
    } catch { /* ignore */ }
    set({ fontFamily: family });
    syncConfigChange('fontFamily', family);
  },
  setFontSize: size => {
    // 限制字号范围 12-24
    const clamped = Math.max(12, Math.min(24, Math.round(size)));
    applyFontToDOM(get().fontFamily, clamped);
    try {
      localStorage.setItem(STORAGE_KEY_FONT_SIZE, String(clamped));
    } catch { /* ignore */ }
    set({ fontSize: clamped });
    syncConfigChange('fontSize', String(clamped));
  },
  setSystemThemeDark: themeName => {
    try {
      localStorage.setItem(STORAGE_KEY_SYSTEM_DARK, themeName);
    } catch { /* ignore */ }
    set({ systemThemeDark: themeName });
    // 如果当前是 system 跟随且系统处于 dark，立即应用
    const { followSystem } = get();
    if (followSystem && typeof window !== 'undefined' && window.matchMedia) {
      const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      if (isDark) {
        const target = themeName as ThemeName;
        applyThemeToDOM(target);
        try { localStorage.setItem(STORAGE_KEY, target); } catch {/* ignore */}
        set({ themeName: target });
      }
    }
    syncConfigChange('systemThemeDark', themeName);
  },
  setSystemThemeLight: themeName => {
    try {
      localStorage.setItem(STORAGE_KEY_SYSTEM_LIGHT, themeName);
    } catch { /* ignore */ }
    set({ systemThemeLight: themeName });
    // 如果当前是 system 跟随且系统处于 light，立即应用
    const { followSystem } = get();
    if (followSystem && typeof window !== 'undefined' && window.matchMedia) {
      const isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      if (!isDark) {
        const target = themeName as ThemeName;
        applyThemeToDOM(target);
        try { localStorage.setItem(STORAGE_KEY, target); } catch {/* ignore */}
        set({ themeName: target });
      }
    }
    syncConfigChange('systemThemeLight', themeName);
  },
  setAnimationsEnabled: enabled => {
    const { animationDuration } = get();
    applyAnimationsToDOM(enabled, animationDuration);
    try {
      localStorage.setItem(STORAGE_KEY_ANIMATIONS_ENABLED, String(enabled));
    } catch { /* ignore */ }
    set({ animationsEnabled: enabled });
    syncConfigChange('animationsEnabled', String(enabled));
  },
  setAnimationDuration: duration => {
    // 仅允许 100/200/400
    const allowed = [100, 200, 400];
    const clamped = allowed.includes(duration) ? duration : 200;
    const { animationsEnabled } = get();
    applyAnimationsToDOM(animationsEnabled, clamped);
    try {
      localStorage.setItem(STORAGE_KEY_ANIMATION_DURATION, String(clamped));
    } catch { /* ignore */ }
    set({ animationDuration: clamped });
    syncConfigChange('animationDuration', String(clamped));
  },
  setModuleTheme: (module, themeName) => {
    // 仅允许合法的预设主题名
    const validNames = new Set<string>(PRESET_THEMES.map(t => t.name));
    if (!validNames.has(themeName)) return;
    const next = {
      ...(get().moduleThemes || {}),
      [module]: themeName
    };
    persistModuleThemes(next);
    set({ moduleThemes: next });
    // 注意：DOM 上的 data-module-theme 属性由 useModuleTheme hook 根据
    // 当前路由对应的 module 名 + moduleThemeEnabled 状态来设置，store 不直接操作 DOM。
    syncConfigChange(`moduleTheme:${module}`, themeName);
  },
  clearModuleTheme: module => {
    const { moduleThemes: prev } = get();
    if (!prev || !(module in prev)) return;
    const next = { ...prev };
    delete next[module];
    persistModuleThemes(next);
    set({ moduleThemes: next });
    syncConfigChange(`moduleTheme:${module}`, '');
  },
  setModuleThemeEnabled: enabled => {
    persistModuleThemeEnabled(enabled);
    set({ moduleThemeEnabled: enabled });
    // 同样：DOM 上的 data-module-theme 属性由 useModuleTheme hook 处理。
    syncConfigChange('moduleThemeEnabled', String(enabled));
  },
  exportTheme: () => {
    const {
      themeName,
      customOverrides,
      followSystem,
      fontFamily,
      fontSize,
      systemThemeDark,
      systemThemeLight,
      animationsEnabled,
      animationDuration,
      moduleThemes,
      moduleThemeEnabled
    } = get();
    return JSON.stringify({
      name: themeName,
      followSystem,
      custom: customOverrides,
      fontFamily,
      fontSize,
      systemThemeDark,
      systemThemeLight,
      animationsEnabled,
      animationDuration,
      moduleThemes,
      moduleThemeEnabled,
      exportedAt: new Date().toISOString(),
      version: '1.0'
    }, null, 2);
  },
  importTheme: json => {
    try {
      const data = JSON.parse(json);
      if (!data.name || !PRESET_THEMES.some(t => t.name === data.name)) {
        return {
          success: false,
          error: t("stores.themeStore.k11")
        };
      }
      get().setTheme(data.name as ThemeName);
      if (data.custom && typeof data.custom === 'object') {
        Object.entries(data.custom).forEach(([key, value]) => {
          if (typeof value === 'string') {
            get().setCustomVariable(key, value);
          }
        });
      }
      // C1.2：导入字体设置
      if (typeof data.fontFamily === 'string') {
        get().setFontFamily(data.fontFamily);
      }
      if (typeof data.fontSize === 'number') {
        get().setFontSize(data.fontSize);
      }
      // C1.3：导入系统主题映射
      if (typeof data.systemThemeDark === 'string') {
        get().setSystemThemeDark(data.systemThemeDark);
      }
      if (typeof data.systemThemeLight === 'string') {
        get().setSystemThemeLight(data.systemThemeLight);
      }
      // C1.6：导入动画设置
      if (typeof data.animationsEnabled === 'boolean') {
        get().setAnimationsEnabled(data.animationsEnabled);
      }
      if (typeof data.animationDuration === 'number') {
        get().setAnimationDuration(data.animationDuration);
      }
      // C1.4：导入模块级主题设置
      if (typeof data.moduleThemeEnabled === 'boolean') {
        get().setModuleThemeEnabled(data.moduleThemeEnabled);
      }
      if (data.moduleThemes && typeof data.moduleThemes === 'object' && !Array.isArray(data.moduleThemes)) {
        // 先清空旧的，再逐个设置（复用 setModuleTheme 的合法性校验）
        const validNames = new Set<string>(PRESET_THEMES.map(t => t.name));
        const validMap: Record<string, string> = {};
        Object.entries(data.moduleThemes).forEach(([key, value]) => {
          if (typeof key === 'string' && typeof value === 'string' && validNames.has(value)) {
            validMap[key] = value;
          }
        });
        persistModuleThemes(validMap);
        set({ moduleThemes: validMap });
      }
      return {
        success: true
      };
    } catch (e) {
      return {
        success: false,
        error: t("stores.themeStore.k12", {
          message: (e as Error).message
        })
      };
    }
  }
}));

// ===== 初始化：应用主题到 DOM（如果 index.html FOUC 脚本未执行） =====

if (typeof document !== 'undefined') {
  const {
    themeName,
    followSystem,
    customOverrides,
    fontFamily,
    fontSize,
    systemThemeDark,
    systemThemeLight,
    animationsEnabled,
    animationDuration
  } = useThemeStore.getState();
  // 首次加载：直接设置 data-theme，不走过渡动画（避免首屏闪烁）
  document.documentElement.setAttribute('data-theme', themeName);
  if (followSystem) {
    // C1.3：注册系统主题变化回调，并立即根据当前系统 scheme 应用对应主题
    systemThemeChangeHandler = (isDark: boolean) => {
      const { followSystem: fs } = useThemeStore.getState();
      if (!fs) return;
      const target = (isDark ? systemThemeDark : systemThemeLight) as ThemeName;
      applyThemeToDOM(target);
      try { localStorage.setItem(STORAGE_KEY, target); } catch {/* ignore */}
      useThemeStore.setState({ themeName: target });
    };
    startSystemThemeListener();
    // 启动时立即根据系统 scheme 应用对应主题
    const picked = pickThemeBySystemScheme(systemThemeDark, systemThemeLight) as ThemeName;
    applyThemeToDOM(picked);
    try { localStorage.setItem(STORAGE_KEY, picked); } catch {/* ignore */}
    useThemeStore.setState({ themeName: picked });
  }
  // C1.1：应用持久化的自定义变量覆盖到 DOM
  if (customOverrides) {
    Object.entries(customOverrides).forEach(([key, value]) => {
      document.documentElement.style.setProperty(key, value);
    });
  }
  // C1.2：应用持久化的字体设置到 DOM
  applyFontToDOM(fontFamily, fontSize);
  // C1.6：应用持久化的动画设置到 DOM
  applyAnimationsToDOM(animationsEnabled, animationDuration);
  // C1.6：监听系统 prefers-reduced-motion 变化，自动同步 --animation-duration
  // 注意：仅同步 DOM 表现，不修改用户的开关偏好（用户偏好优先于系统瞬时变化）
  if (typeof window !== 'undefined' && window.matchMedia) {
    const reducedMq = window.matchMedia('(prefers-reduced-motion: reduce)');
    const reducedHandler = () => {
      const { animationsEnabled: en, animationDuration: dur } = useThemeStore.getState();
      applyAnimationsToDOM(en, dur);
    };
    reducedMq.addEventListener('change', reducedHandler);
  }
}

// ===== 便捷导出 =====

/** 获取当前主题元数据 */
export function getCurrentThemeMeta(): ThemeMeta | undefined {
  return PRESET_THEMES.find(t => t.name === useThemeStore.getState().themeName);
}

/** 判断当前主题是否为亮色 */
export function isCurrentThemeLight(): boolean {
  const meta = getCurrentThemeMeta();
  return meta?.isLight ?? false;
}