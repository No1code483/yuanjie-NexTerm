/**
 * TitleBar Store（C3 / v1.52）
 *
 * 管理窗口控制按钮的扩展状态：
 * - 扩展按钮的显示/隐藏（置顶/开发者工具/截图/录屏）
 * - 按钮风格（system/custom/minimal）
 * - 按钮排序
 * - 设置面板开关
 * - 平台检测（macOS 适配红绿灯样式）
 * - 截图区域选择状态
 * - 录屏计时
 *
 * 关联文档：功能展望/01_接下来可开发_55/体验深化_82/03_窗口控制按钮_自定义方案_100.md
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
    console.warn(`[titleBarStore] 配置 "${key}" 同步入队失败:`, e);
  });
}

/** 窗口按钮类型 */
export type WindowButtonType =
  | 'minimize'
  | 'maximize'
  | 'close'
  | 'pin'        // 置顶
  | 'devtools'   // 开发者工具
  | 'screenshot' // 截图
  | 'record';    // 录屏

/** 标题栏风格 */
export type TitleBarStyle = 'system' | 'custom' | 'minimal';

/** 运行平台（用于 macOS 红绿灯按钮适配） */
export type Platform = 'windows' | 'macos' | 'linux' | 'browser';

/** 扩展按钮开关状态 */
interface ButtonVisibility {
  pin: boolean;
  devtools: boolean;
  screenshot: boolean;
  record: boolean;
}

/** 截图区域选择矩形（屏幕坐标） */
export interface ScreenshotRegion {
  x: number;
  y: number;
  width: number;
  height: number;
}

/** titleBar 持久化数据结构 */
interface PersistedTitleBarState {
  style: TitleBarStyle;
  buttonVisibility: ButtonVisibility;
  buttonOrder: WindowButtonType[];
}

const STORAGE_KEY = 'nexterm-titlebar-config';

/** 默认快捷键映射（C3.6） */
export const DEFAULT_SHORTCUTS: Record<string, string> = {
  maximize: 'F11',
  pin: 'Ctrl+Shift+T',
  screenshot: 'Ctrl+Shift+S',
  devtools: 'F12',
  record: 'Ctrl+Shift+R',
};

/** 检测当前运行平台（通过 navigator.userAgent） */
export function detectPlatform(): Platform {
  if (typeof navigator === 'undefined') return 'windows';
  const ua = navigator.userAgent || '';
  if (/Mac|iPhone|iPod|iPad/.test(ua)) return 'macos';
  if (/Win/.test(ua)) return 'windows';
  if (/Linux/.test(ua)) return 'linux';
  return 'browser';
}

/** 默认按钮顺序 */
const DEFAULT_BUTTON_ORDER: WindowButtonType[] = [
  'pin', 'devtools', 'screenshot', 'record',
  'minimize', 'maximize', 'close',
];

/** 默认按钮可见性 */
const DEFAULT_VISIBILITY: ButtonVisibility = {
  pin: true,
  devtools: true,
  screenshot: false,
  record: false,
};

/** 从 localStorage 读取持久化配置 */
function loadPersistedState(): Partial<PersistedTitleBarState> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as PersistedTitleBarState;
    return {
      style: parsed.style || 'system',
      buttonVisibility: { ...DEFAULT_VISIBILITY, ...(parsed.buttonVisibility || {}) },
      buttonOrder: parsed.buttonOrder?.length === 7 ? parsed.buttonOrder : DEFAULT_BUTTON_ORDER,
    };
  } catch {
    return {};
  }
}

/** 持久化配置到 localStorage */
function persistState(state: PersistedTitleBarState): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch { /* ignore */ }
}

export interface TitleBarState {
  /** 标题栏风格 */
  style: TitleBarStyle;
  /** 扩展按钮可见性 */
  buttonVisibility: ButtonVisibility;
  /** 按钮顺序 */
  buttonOrder: WindowButtonType[];
  /** 设置面板是否打开 */
  settingsOpen: boolean;
  /** 窗口是否置顶 */
  isPinned: boolean;
  /** 是否正在录屏 */
  isRecording: boolean;
  /** 当前运行平台（macOS 适配用） */
  platform: Platform;
  /** 截图区域选择模式是否激活 */
  isSelectingScreenshot: boolean;
  /** 最近一次截图保存路径（用于通知展示） */
  lastScreenshotPath: string | null;
  /** 录屏累计秒数（用于UI显示） */
  recordingElapsedSecs: number;

  /** 设置标题栏风格 */
  setStyle: (style: TitleBarStyle) => void;
  /** 切换单个按钮的可见性 */
  toggleButtonVisibility: (button: keyof ButtonVisibility) => void;
  /** 设置按钮顺序 */
  setButtonOrder: (order: WindowButtonType[]) => void;
  /** 移动按钮顺序（向上或向下） */
  moveButton: (button: WindowButtonType, direction: 'up' | 'down') => void;
  /** 打开/关闭设置面板 */
  toggleSettings: () => void;
  /** 切换窗口置顶 */
  togglePin: () => void;
  /** 切换录屏状态 */
  toggleRecording: () => void;
  /** 设置平台（初始化时调用 detectPlatform 后写入） */
  setPlatform: (platform: Platform) => void;
  /** 进入/退出截图区域选择模式 */
  setSelectingScreenshot: (selecting: boolean) => void;
  /** 记录最近截图路径 */
  setLastScreenshotPath: (path: string | null) => void;
  /** 累加录屏计时（每秒调用一次） */
  tickRecording: () => void;
  /** 重置录屏计时 */
  resetRecordingTimer: () => void;
}

const persisted = loadPersistedState();

export const useTitleBarStore = create<TitleBarState>((set, get) => ({
  style: persisted.style || 'system',
  buttonVisibility: persisted.buttonVisibility || DEFAULT_VISIBILITY,
  buttonOrder: persisted.buttonOrder || DEFAULT_BUTTON_ORDER,
  settingsOpen: false,
  isPinned: false,
  isRecording: false,
  platform: detectPlatform(),
  isSelectingScreenshot: false,
  lastScreenshotPath: null,
  recordingElapsedSecs: 0,

  setStyle: (style) => {
    persistState({
      style,
      buttonVisibility: get().buttonVisibility,
      buttonOrder: get().buttonOrder,
    });
    set({ style });
    syncConfigChange('titleBarStyle', style);
  },

  toggleButtonVisibility: (button) => {
    const next = {
      ...get().buttonVisibility,
      [button]: !get().buttonVisibility[button],
    };
    persistState({
      style: get().style,
      buttonVisibility: next,
      buttonOrder: get().buttonOrder,
    });
    set({ buttonVisibility: next });
    syncConfigChange(`titleBarButton:${button}`, String(next[button]));
  },

  setButtonOrder: (order) => {
    persistState({
      style: get().style,
      buttonVisibility: get().buttonVisibility,
      buttonOrder: order,
    });
    set({ buttonOrder: order });
    syncConfigChange('titleBarButtonOrder', JSON.stringify(order));
  },

  moveButton: (button, direction) => {
    const order = [...get().buttonOrder];
    const idx = order.indexOf(button);
    if (idx === -1) return;
    const swapIdx = direction === 'up' ? idx - 1 : idx + 1;
    if (swapIdx < 0 || swapIdx >= order.length) return;
    [order[idx], order[swapIdx]] = [order[swapIdx], order[idx]];
    get().setButtonOrder(order);
  },

  toggleSettings: () => set((s) => ({ settingsOpen: !s.settingsOpen })),
  togglePin: () => set((s) => ({ isPinned: !s.isPinned })),
  toggleRecording: () => set((s) => ({ isRecording: !s.isRecording })),
  setPlatform: (platform) => set({ platform }),
  setSelectingScreenshot: (selecting) => set({ isSelectingScreenshot: selecting }),
  setLastScreenshotPath: (path) => set({ lastScreenshotPath: path }),
  tickRecording: () => set((s) => ({ recordingElapsedSecs: s.recordingElapsedSecs + 1 })),
  resetRecordingTimer: () => set({ recordingElapsedSecs: 0 }),
}));
