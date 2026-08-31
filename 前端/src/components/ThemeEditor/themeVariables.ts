/**
 * 主题变量元数据（C1.1 主题编辑器）
 *
 * 定义所有可编辑的 --theme-* CSS 变量，按分组组织：
 *   - 背景色 / 文字色 / 主色调 / 辅助色 / 状态色 / 边框 / 灰度 / 阴影 / ANSI / 几何
 *
 * 变量类型：
 *   - color: 颜色值（HEX / RGB / RGBA），由 ColorPicker 编辑
 *   - shadow: 阴影值（CSS box-shadow），由文本输入编辑
 *   - number: 数值（如圆角 px），由数字输入编辑
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.1
 */

export type VariableType = 'color' | 'shadow' | 'number';

export interface ThemeVariableMeta {
  /** CSS 变量名（含 --theme- 前缀） */
  key: string;
  /** 显示名称 i18n key 后缀（如 'bgPrimary' → t('...themeEditor.var.bgPrimary')） */
  labelKey: string;
  /** 变量类型 */
  type: VariableType;
  /** 默认值（terminal 主题的值，用于重置） */
  defaultValue: string;
  /** 数值类型的最小值/最大值/单位（仅 type='number' 时有效） */
  min?: number;
  max?: number;
  step?: number;
  unit?: string;
}

export interface ThemeVariableGroup {
  /** 分组 ID */
  id: string;
  /** 分组名称 i18n key 后缀 */
  labelKey: string;
  /** 分组内的变量 */
  variables: ThemeVariableMeta[];
}

/**
 * 所有可编辑的主题变量分组（共 38 个变量，8 个分组）
 *
 * 分组顺序决定 UI 展示顺序：
 *   1. 背景色（5）  2. 文字色（4）  3. 主色调（3）  4. 辅助色（2）
 *   5. 状态色（5）  6. 边框与灰度（6）  7. 阴影（4）  8. ANSI 颜色（16）
 *   9. 几何（1）
 */
export const THEME_VARIABLE_GROUPS: ThemeVariableGroup[] = [
  {
    id: 'background',
    labelKey: 'groupBackground',
    variables: [
      { key: '--theme-bg-primary', labelKey: 'bgPrimary', type: 'color', defaultValue: '#090300' },
      { key: '--theme-bg-secondary', labelKey: 'bgSecondary', type: 'color', defaultValue: 'rgba(10, 0, 20, 0.6)' },
      { key: '--theme-bg-tertiary', labelKey: 'bgTertiary', type: 'color', defaultValue: 'rgba(10, 0, 20, 0.4)' },
      { key: '--theme-bg-elevated', labelKey: 'bgElevated', type: 'color', defaultValue: '#141414' },
      { key: '--theme-bg-overlay', labelKey: 'bgOverlay', type: 'color', defaultValue: 'rgba(10, 0, 20, 0.85)' },
    ],
  },
  {
    id: 'text',
    labelKey: 'groupText',
    variables: [
      { key: '--theme-text-primary', labelKey: 'textPrimary', type: 'color', defaultValue: '#c8c8dd' },
      { key: '--theme-text-secondary', labelKey: 'textSecondary', type: 'color', defaultValue: '#8a8aaa' },
      { key: '--theme-text-muted', labelKey: 'textMuted', type: 'color', defaultValue: '#6a6a8a' },
      { key: '--theme-text-bright', labelKey: 'textBright', type: 'color', defaultValue: '#ffffff' },
    ],
  },
  {
    id: 'accent',
    labelKey: 'groupAccent',
    variables: [
      { key: '--theme-accent', labelKey: 'accent', type: 'color', defaultValue: '#00F0FF' },
      { key: '--theme-accent-hover', labelKey: 'accentHover', type: 'color', defaultValue: '#33F5FF' },
      { key: '--theme-accent-active', labelKey: 'accentActive', type: 'color', defaultValue: '#00C8D9' },
    ],
  },
  {
    id: 'secondary',
    labelKey: 'groupSecondary',
    variables: [
      { key: '--theme-secondary', labelKey: 'secondary', type: 'color', defaultValue: '#B026FF' },
      { key: '--theme-secondary-hover', labelKey: 'secondaryHover', type: 'color', defaultValue: '#C44AFF' },
    ],
  },
  {
    id: 'status',
    labelKey: 'groupStatus',
    variables: [
      { key: '--theme-warning', labelKey: 'warning', type: 'color', defaultValue: '#FFD700' },
      { key: '--theme-warning-hover', labelKey: 'warningHover', type: 'color', defaultValue: '#FFE033' },
      { key: '--theme-success', labelKey: 'success', type: 'color', defaultValue: '#00F0FF' },
      { key: '--theme-error', labelKey: 'error', type: 'color', defaultValue: '#FF006E' },
      { key: '--theme-error-hover', labelKey: 'errorHover', type: 'color', defaultValue: '#FF3385' },
      { key: '--theme-info', labelKey: 'info', type: 'color', defaultValue: '#00F0FF' },
    ],
  },
  {
    id: 'border',
    labelKey: 'groupBorder',
    variables: [
      { key: '--theme-border-color', labelKey: 'borderColor', type: 'color', defaultValue: 'rgba(0, 240, 255, 0.25)' },
      { key: '--theme-border-subtle', labelKey: 'borderSubtle', type: 'color', defaultValue: 'rgba(255, 255, 255, 0.06)' },
      { key: '--theme-gray-dark', labelKey: 'grayDark', type: 'color', defaultValue: '#5a5a5a' },
      { key: '--theme-gray-medium', labelKey: 'grayMedium', type: 'color', defaultValue: '#3a3a3a' },
      { key: '--theme-gray-light', labelKey: 'grayLight', type: 'color', defaultValue: '#1a1a1a' },
      { key: '--theme-gray-ultra-light', labelKey: 'grayUltraLight', type: 'color', defaultValue: '#2a2a2a' },
    ],
  },
  {
    id: 'shadow',
    labelKey: 'groupShadow',
    variables: [
      { key: '--theme-shadow-primary', labelKey: 'shadowPrimary', type: 'shadow', defaultValue: '0 0 10px rgba(0, 240, 255, 0.3)' },
      { key: '--theme-shadow-accent', labelKey: 'shadowAccent', type: 'shadow', defaultValue: '0 0 10px rgba(255, 0, 110, 0.3)' },
      { key: '--theme-shadow-secondary', labelKey: 'shadowSecondary', type: 'shadow', defaultValue: '0 0 10px rgba(176, 38, 255, 0.3)' },
      { key: '--theme-shadow-warning', labelKey: 'shadowWarning', type: 'shadow', defaultValue: '0 0 10px rgba(255, 215, 0, 0.3)' },
    ],
  },
  {
    id: 'ansi',
    labelKey: 'groupAnsi',
    variables: [
      { key: '--theme-ansi-black', labelKey: 'ansiBlack', type: 'color', defaultValue: '#090300' },
      { key: '--theme-ansi-red', labelKey: 'ansiRed', type: 'color', defaultValue: '#FF006E' },
      { key: '--theme-ansi-green', labelKey: 'ansiGreen', type: 'color', defaultValue: '#00F0FF' },
      { key: '--theme-ansi-yellow', labelKey: 'ansiYellow', type: 'color', defaultValue: '#FFD700' },
      { key: '--theme-ansi-blue', labelKey: 'ansiBlue', type: 'color', defaultValue: '#01a0e4' },
      { key: '--theme-ansi-magenta', labelKey: 'ansiMagenta', type: 'color', defaultValue: '#B026FF' },
      { key: '--theme-ansi-cyan', labelKey: 'ansiCyan', type: 'color', defaultValue: '#00F0FF' },
      { key: '--theme-ansi-white', labelKey: 'ansiWhite', type: 'color', defaultValue: '#a5a2a2' },
      { key: '--theme-ansi-bright-black', labelKey: 'ansiBrightBlack', type: 'color', defaultValue: '#5c5855' },
      { key: '--theme-ansi-bright-red', labelKey: 'ansiBrightRed', type: 'color', defaultValue: '#FF3385' },
      { key: '--theme-ansi-bright-green', labelKey: 'ansiBrightGreen', type: 'color', defaultValue: '#33F5FF' },
      { key: '--theme-ansi-bright-yellow', labelKey: 'ansiBrightYellow', type: 'color', defaultValue: '#FFE033' },
      { key: '--theme-ansi-bright-blue', labelKey: 'ansiBrightBlue', type: 'color', defaultValue: '#00b8ff' },
      { key: '--theme-ansi-bright-magenta', labelKey: 'ansiBrightMagenta', type: 'color', defaultValue: '#C44AFF' },
      { key: '--theme-ansi-bright-cyan', labelKey: 'ansiBrightCyan', type: 'color', defaultValue: '#33F5FF' },
      { key: '--theme-ansi-bright-white', labelKey: 'ansiBrightWhite', type: 'color', defaultValue: '#ffffff' },
    ],
  },
  {
    id: 'geometry',
    labelKey: 'groupGeometry',
    variables: [
      { key: '--theme-border-radius', labelKey: 'borderRadius', type: 'number', defaultValue: '0', min: 0, max: 20, step: 1, unit: 'px' },
    ],
  },
];

/** 扁平化的所有变量列表（便于查找） */
export const ALL_THEME_VARIABLES: ThemeVariableMeta[] = THEME_VARIABLE_GROUPS.flatMap(g => g.variables);

/** 根据 key 查找变量元数据 */
export function findVariableMeta(key: string): ThemeVariableMeta | undefined {
  return ALL_THEME_VARIABLES.find(v => v.key === key);
}

/** 获取某个变量在当前 DOM 上的计算值（读取 CSS 变量） */
export function getVariableValue(key: string): string {
  if (typeof document === 'undefined') return '';
  return getComputedStyle(document.documentElement).getPropertyValue(key).trim();
}
