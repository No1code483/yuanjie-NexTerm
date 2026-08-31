/**
 * CSS 变量类型定义
 * 提供类型安全的 CSS 变量访问
 */

export type CssColorVariable = 
  | '--nt-black'
  | '--nt-green'
  | '--nt-green-hover'
  | '--nt-green-active'
  | '--nt-cyan'
  | '--nt-cyan-hover'
  | '--nt-purple'
  | '--nt-purple-hover'
  | '--nt-red'
  | '--nt-red-hover'
  | '--nt-light-red'
  | '--nt-gray-dark'
  | '--nt-gray-medium'
  | '--nt-gray-light'

export type CssSizeVariable =
  | '--nt-border-width'
  | '--nt-button-padding-small'
  | '--nt-button-padding-medium'
  | '--nt-button-padding-large'
  | '--nt-button-font-size-small'
  | '--nt-button-font-size-medium'
  | '--nt-button-font-size-large'
  | '--nt-button-min-width-small'
  | '--nt-button-min-width-medium'
  | '--nt-button-min-width-large'
  | '--nt-input-padding-small'
  | '--nt-input-padding-medium'
  | '--nt-input-padding-large'
  | '--nt-input-font-size-small'
  | '--nt-input-font-size-medium'
  | '--nt-input-font-size-large'

export type CssFontVariable =
  | '--nt-font-mono'
  | '--nt-font-chinese'

export type CssShadowVariable =
  | '--nt-shadow-green'
  | '--nt-shadow-red'
  | '--nt-shadow-cyan'
  | '--nt-shadow-purple'

export type CssTransitionVariable =
  | '--nt-transition-fast'
  | '--nt-transition-normal'
  | '--nt-transition-slow'

export type CssSpacingVariable =
  | '--nt-spacing-xs'
  | '--nt-spacing-sm'
  | '--nt-spacing-md'
  | '--nt-spacing-lg'
  | '--nt-spacing-xl'

export type CssRadiusVariable =
  | '--nt-radius-none'
  | '--nt-radius-sm'
  | '--nt-radius-md'
  | '--nt-radius-lg'

export type CssVariable = 
  | CssColorVariable 
  | CssSizeVariable 
  | CssFontVariable 
  | CssShadowVariable
  | CssTransitionVariable
  | CssSpacingVariable
  | CssRadiusVariable

/**
 * 获取 CSS 变量值的工具函数
 * 提供类型安全的 CSS 变量访问
 */
export function getCssVariable(variable: CssVariable): string {
  return `var(${variable})`
}

/**
 * 获取颜色变量的快捷方法
 */
export function getColor(variable: CssColorVariable): string {
  return `var(${variable})`
}

/**
 * 获取尺寸变量的快捷方法
 */
export function getSize(variable: CssSizeVariable): string {
  return `var(${variable})`
}

/**
 * 获取字体变量的快捷方法
 */
export function getFont(variable: CssFontVariable): string {
  return `var(${variable})`
}

/**
 * 获取阴影变量的快捷方法
 */
export function getShadow(variable: CssShadowVariable): string {
  return `var(${variable})`
}

/**
 * 获取过渡变量的快捷方法
 */
export function getTransition(variable: CssTransitionVariable): string {
  return `var(${variable})`
}

/**
 * 获取间距变量的快捷方法
 */
export function getSpacing(variable: CssSpacingVariable): string {
  return `var(${variable})`
}

/**
 * 获取圆角变量的快捷方法
 */
export function getRadius(variable: CssRadiusVariable): string {
  return `var(${variable})`
}

/**
 * CSS 变量工具类型
 * 用于在 TypeScript 中定义样式对象时提供类型安全
 */
export interface CssVariableStyles {
  color?: CssColorVariable
  backgroundColor?: CssColorVariable
  borderColor?: CssColorVariable
  borderWidth?: CssSizeVariable
  fontSize?: CssSizeVariable
  fontFamily?: CssFontVariable
  boxShadow?: CssShadowVariable
  transition?: CssTransitionVariable
  padding?: CssSpacingVariable
  margin?: CssSpacingVariable
  borderRadius?: CssRadiusVariable
}

/**
 * 将 CSS 变量样式对象转换为实际的 CSS 样式对象
 */
export function createCssStyles(styles: CssVariableStyles): Record<string, string> {
  const result: Record<string, string> = {}
  
  if (styles.color) result.color = getColor(styles.color)
  if (styles.backgroundColor) result.backgroundColor = getColor(styles.backgroundColor)
  if (styles.borderColor) result.borderColor = getColor(styles.borderColor)
  if (styles.borderWidth) result.borderWidth = getSize(styles.borderWidth)
  if (styles.fontSize) result.fontSize = getSize(styles.fontSize)
  if (styles.fontFamily) result.fontFamily = getFont(styles.fontFamily)
  if (styles.boxShadow) result.boxShadow = getShadow(styles.boxShadow)
  if (styles.transition) result.transition = getTransition(styles.transition)
  if (styles.padding) result.padding = getSpacing(styles.padding)
  if (styles.margin) result.margin = getSpacing(styles.margin)
  if (styles.borderRadius) result.borderRadius = getRadius(styles.borderRadius)
  
  return result
}

/**
 * 预定义的样式组合
 * 提供常用的样式组合，方便复用
 */
export const CssPresets = {
  button: {
    primary: {
      color: '--nt-black' as CssColorVariable,
      backgroundColor: '--nt-green' as CssColorVariable,
      borderColor: '--nt-green' as CssColorVariable,
      borderWidth: '--nt-border-width' as CssSizeVariable,
      fontFamily: '--nt-font-mono' as CssFontVariable,
      transition: '--nt-transition-normal' as CssTransitionVariable
    },
    danger: {
      color: '--nt-black' as CssColorVariable,
      backgroundColor: '--nt-red' as CssColorVariable,
      borderColor: '--nt-red' as CssColorVariable,
      borderWidth: '--nt-border-width' as CssSizeVariable,
      fontFamily: '--nt-font-mono' as CssFontVariable,
      transition: '--nt-transition-normal' as CssTransitionVariable
    }
  },
  input: {
    default: {
      color: '--nt-green' as CssColorVariable,
      backgroundColor: '--nt-black' as CssColorVariable,
      borderColor: '--nt-green' as CssColorVariable,
      borderWidth: '--nt-border-width' as CssSizeVariable,
      fontFamily: '--nt-font-mono' as CssFontVariable,
      transition: '--nt-transition-normal' as CssTransitionVariable
    },
    error: {
      color: '--nt-red' as CssColorVariable,
      backgroundColor: '--nt-black' as CssColorVariable,
      borderColor: '--nt-red' as CssColorVariable,
      borderWidth: '--nt-border-width' as CssSizeVariable,
      fontFamily: '--nt-font-mono' as CssFontVariable,
      transition: '--nt-transition-normal' as CssTransitionVariable
    }
  }
}