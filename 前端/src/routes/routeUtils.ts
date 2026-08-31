/**
 * 路由工具函数
 * 提供路由相关的工具方法
 */

import { ROUTES, RoutePath, ROUTE_META, RouteMeta } from './routes'

/**
 * 生成带参数的路径
 */
export function generatePath(path: RoutePath, params: Record<string, string>): string {
  let result = path as string
  
  Object.keys(params).forEach(key => {
    const paramKey = `:${key}`
    if (result.includes(paramKey)) {
      result = result.replace(paramKey, params[key])
    }
  })
  
  return result
}

/**
 * 检查路径是否匹配模式
 */
export function matchPath(pattern: RoutePath, pathname: string): boolean {
  const patternParts = pattern.split('/')
  const pathnameParts = pathname.split('/')
  
  if (patternParts.length !== pathnameParts.length) {
    return false
  }
  
  for (let i = 0; i < patternParts.length; i++) {
    const patternPart = patternParts[i]
    const pathnamePart = pathnameParts[i]
    
    // 如果是参数部分（以:开头），跳过检查
    if (patternPart.startsWith(':')) {
      continue
    }
    
    // 如果是普通部分，必须完全匹配
    if (patternPart !== pathnamePart) {
      return false
    }
  }
  
  return true
}

/**
 * 从路径中提取参数
 */
export function extractParams(pattern: RoutePath, pathname: string): Record<string, string> {
  const params: Record<string, string> = {}
  const patternParts = pattern.split('/')
  const pathnameParts = pathname.split('/')
  
  if (patternParts.length !== pathnameParts.length) {
    return params
  }
  
  for (let i = 0; i < patternParts.length; i++) {
    const patternPart = patternParts[i]
    const pathnamePart = pathnameParts[i]
    
    if (patternPart.startsWith(':')) {
      const paramName = patternPart.slice(1)
      params[paramName] = pathnamePart
    }
  }
  
  return params
}

/**
 * 获取父路径
 */
export function getParentPath(path: RoutePath): RoutePath | null {
  const parts = path.split('/').filter(part => part !== '')
  
  if (parts.length <= 1) {
    return null
  }
  
  parts.pop()
  return `/${parts.join('/')}` as RoutePath
}

/**
 * 检查是否是子路由
 */
export function isSubRoute(parent: RoutePath, child: RoutePath): boolean {
  return child.startsWith(parent + '/')
}

/**
 * 获取路由层级
 */
export function getRouteLevel(path: RoutePath): number {
  return path.split('/').filter(part => part !== '').length
}

/**
 * 面包屑导航数据
 */
export interface BreadcrumbItem {
  path: RoutePath
  title: string
}

/**
 * 生成面包屑导航数据
 */
export function generateBreadcrumbs(currentPath: RoutePath): BreadcrumbItem[] {
  const breadcrumbs: BreadcrumbItem[] = []
  const parts = currentPath.split('/').filter(part => part !== '')
  
  let accumulatedPath = ''
  
  for (const part of parts) {
    accumulatedPath += `/${part}`
    
    // 查找对应的路由元数据
    const routeMeta = Object.values(ROUTES).find(route => 
      matchPath(route as RoutePath, accumulatedPath)
    )
    
    if (routeMeta) {
      breadcrumbs.push({
        path: routeMeta as RoutePath,
        title: getRouteTitle(routeMeta as RoutePath)
      })
    }
  }
  
  return breadcrumbs
}

/**
 * 获取路由标题（从 routes.ts 导入）
 */
function getRouteTitle(path: RoutePath): string {
  // 获取对应的主路由路径
  const getMainRoutePath = (routePath: RoutePath): string => {
    // 如果是主路由，直接返回
    if (Object.values(ROUTES).includes(routePath as any)) {
      return routePath
    }
    
    // 如果是子路由，找到对应的主路由
    for (const mainPath of Object.values(ROUTES).slice(0, 9)) { // 前9个是主路由
      if (routePath.startsWith(mainPath)) {
        return mainPath
      }
    }
    
    // 默认返回首页
    return ROUTES.HOME
  }
  
  const mainPath = getMainRoutePath(path)
   return (ROUTE_META as Record<string, RouteMeta>)[mainPath]?.title || path
}

/**
 * 路由历史管理
 */
export class RouteHistory {
  private static instance: RouteHistory
  private history: RoutePath[] = []
  private maxSize = 50
  
  private constructor() {}
  
  static getInstance(): RouteHistory {
    if (!RouteHistory.instance) {
      RouteHistory.instance = new RouteHistory()
    }
    return RouteHistory.instance
  }
  
  push(path: RoutePath) {
    this.history.push(path)
    
    // 限制历史记录大小
    if (this.history.length > this.maxSize) {
      this.history.shift()
    }
  }
  
  pop(): RoutePath | undefined {
    return this.history.pop()
  }
  
  getPrevious(): RoutePath | undefined {
    if (this.history.length < 2) {
      return undefined
    }
    return this.history[this.history.length - 2]
  }
  
  clear() {
    this.history = []
  }
  
  getHistory(): RoutePath[] {
    return [...this.history]
  }
}