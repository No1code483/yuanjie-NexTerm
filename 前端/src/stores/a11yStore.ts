/**
 * A11y Store（C4 §2.7 字号调整 + 减弱动画偏好）
 *
 * 管理用户级无障碍偏好：
 *   - 字号档位（small / default / large / xlarge）
 *   - 强制减弱动画（覆盖系统 prefers-reduced-motion）
 *
 * 状态变化会同步到 <html> 的 data-font-size / data-reduced-motion 属性，
 * 由 CSS 通过属性选择器响应（见 styles/global.css）。
 *
 * 设计依据：功能展望/体验深化/04_无障碍_A11y合规.md §2.5 / §2.7
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
    console.warn(`[a11yStore] 配置 "${key}" 同步入队失败:`, e);
  });
}

export type FontSizeLevel = 'small' | 'default' | 'large' | 'xlarge';

/** 字号 → 像素映射（root font-size） */
export const FONT_SIZE_PX: Record<FontSizeLevel, number> = {
  small: 12,
  default: 14,
  large: 18,
  xlarge: 24,
};

interface PersistedA11yState {
  fontSize: FontSizeLevel;
  reducedMotionOverride: 'auto' | 'on' | 'off';
}

const STORAGE_KEY = 'nexterm-a11y-prefs';

function loadPersisted(): Partial<PersistedA11yState> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    return JSON.parse(raw) as PersistedA11yState;
  } catch {
    return {};
  }
}

function persist(state: PersistedA11yState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch { /* ignore */ }
}

/** 计算最终是否减弱动画（合并用户覆盖与系统偏好） */
function resolveReducedMotion(override: 'auto' | 'on' | 'off'): boolean {
  if (override === 'on') return true;
  if (override === 'off') return false;
  if (typeof window === 'undefined' || !window.matchMedia) return false;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export interface A11yState {
  fontSize: FontSizeLevel;
  reducedMotionOverride: 'auto' | 'on' | 'off';
  /** 当前生效的减弱动画状态（只读派生值） */
  reducedMotion: boolean;

  setFontSize: (level: FontSizeLevel) => void;
  setReducedMotionOverride: (override: 'auto' | 'on' | 'off') => void;
}

const persisted = loadPersisted();

/** 把字号同步到 <html data-font-size="..." style="font-size: Npx"> */
function applyFontSize(level: FontSizeLevel): void {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.setAttribute('data-font-size', level);
  root.style.fontSize = `${FONT_SIZE_PX[level]}px`;
}

/** 把减弱动画状态同步到 <html data-reduced-motion="true|false"> */
function applyReducedMotion(reduced: boolean): void {
  if (typeof document === 'undefined') return;
  document.documentElement.setAttribute('data-reduced-motion', String(reduced));
}

export const useA11yStore = create<A11yState>((set, get) => {
  const fontSize: FontSizeLevel = persisted.fontSize || 'default';
  const reducedMotionOverride = persisted.reducedMotionOverride || 'auto';
  const reducedMotion = resolveReducedMotion(reducedMotionOverride);

  // 初始化时同步到 DOM
  if (typeof document !== 'undefined') {
    applyFontSize(fontSize);
    applyReducedMotion(reducedMotion);

    // 监听系统偏好变化（仅在 override=auto 时生效）
    if (window.matchMedia) {
      const mql = window.matchMedia('(prefers-reduced-motion: reduce)');
      mql.addEventListener('change', () => {
        if (get().reducedMotionOverride === 'auto') {
          const next = resolveReducedMotion('auto');
          applyReducedMotion(next);
          set({ reducedMotion: next });
        }
      });
    }
  }

  return {
    fontSize,
    reducedMotionOverride,
    reducedMotion,

    setFontSize: (level) => {
      persist({ fontSize: level, reducedMotionOverride: get().reducedMotionOverride });
      applyFontSize(level);
      set({ fontSize: level });
      syncConfigChange('a11yFontSize', level);
    },

    setReducedMotionOverride: (override) => {
      persist({ fontSize: get().fontSize, reducedMotionOverride: override });
      const next = resolveReducedMotion(override);
      applyReducedMotion(next);
      set({ reducedMotionOverride: override, reducedMotion: next });
      syncConfigChange('a11yReducedMotion', override);
    },
  };
});
