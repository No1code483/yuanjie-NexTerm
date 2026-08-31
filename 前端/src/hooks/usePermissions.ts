/**
 * usePermissions Hook - RBAC 权限检查
 * 
 * v1.01 新增：基于后端 permissions 表的 RBAC 权限模型
 * 替代原有的硬编码 permissions 数组方案
 * 
 * 使用方式:
 * const { can, role, permissions } = usePermissions()
 * if (can('home', 'write')) { ... }
 */

import { useState, useEffect } from 'react'
import { useAuthStore } from '@/stores/authStore'
import { auth } from '@/lib/ipc'
import type { 
  Role, 
  ResourceName, 
  PermissionAction,
  PermissionsMatrix,
  PermissionDetail 
} from '@/types'

interface UsePermissionsReturn {
  /** 当前用户角色 */
  role: Role | null
  /** 完整权限矩阵 */
  permissions: PermissionsMatrix | null
  /** 是否已加载权限 */
  isLoading: boolean
  
  /**
   * 检查是否有指定资源的指定权限
   * @param resource 资源名称 (如 'home', 'terminal')
   * @param action 操作类型 ('read' | 'write' | 'delete' | 'modify')
   * @returns 是否有权限
   */
  can: (resource: ResourceName, action: PermissionAction) => boolean
  
  /**
   * 检查是否是管理员
   */
  isAdmin: () => boolean
  
  /**
   * 检查是否是临时账号 (guest)
   */
  isGuest: () => boolean
  
  /**
   * 获取指定资源的完整权限详情
   */
  getResourcePermission: (resource: ResourceName) => PermissionDetail | null
  
  /**
   * 刷新权限数据 (重新从后端获取)
   */
  refreshPermissions: () => Promise<void>
}

/**
 * 默认权限矩阵 (用于 Mock 模式或降级场景)
 * 对齐 07_安全体系.md 中的默认值
 * 
 * v2.0 更新：整合路由权限系统、临时账号权限系统
 * - 覆盖所有主模块和子功能资源
 * - 对齐 ROUTE_META 中的 allowedForTemp 配置
 * - 对齐 utils/permission.ts 中的临时账号权限规则
 */
const DEFAULT_PERMISSIONS: Record<Role, PermissionsMatrix> = {
  admin: {
    // ===== 主模块 (管理员拥有所有权限) =====
    home:             { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    ai_chat:          { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    knowledge:        { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    terminal:         { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    game:             { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    profile_settings: { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    recycle_bin:      { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    
    // ===== 首页子功能 =====
    home_news:        { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    home_todo:        { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    home_log:         { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    home_timer:       { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },

    // ===== 终端子功能 =====
    terminal_manual:  { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 命令手册为只读参考
    terminal_yuancode:{ can_read: true,  can_write: true,  can_delete: true,  can_modify: true },
    terminal_linux:   { can_read: true,  can_write: true,  can_delete: true,  can_modify: true },

    // ===== 其他独立模块 =====
    search:           { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    xin:              { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    spyglass:         { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
  },
  user: {
    // ===== 主模块 (普通用户权限) =====
    home:             { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    ai_chat:          { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    knowledge:        { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    terminal:         { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    game:             { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    profile_settings: { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    recycle_bin:      { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    
    // ===== 首页子功能 =====
    home_news:        { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    home_todo:        { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    home_log:         { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    home_timer:       { can_read: true,  can_write: true,  can_delete: false, can_modify: true },

    // ===== 终端子功能 =====
    terminal_manual:  { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 命令手册为只读参考
    terminal_yuancode:{ can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    terminal_linux:   { can_read: true,  can_write: true,  can_delete: false, can_modify: true },

    // ===== 其他独立模块 =====
    search:           { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    xin:              { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
    spyglass:         { can_read: true,  can_write: true,  can_delete: false, can_modify: true },
  },
  guest: {
    // ===== 主模块 (临时账号权限 - 整合原有硬编码规则) =====
    // 对齐 utils/permission.ts: home=read, ai=read+write, terminal=none, knowledge=read, game=read
    // 对齐 routes.ts ROUTE_META: allowedForTemp 配置
    home:             { can_read: true,  can_write: false, can_delete: false, can_modify: false },
    ai_chat:          { can_read: true,  can_write: true,  can_delete: false, can_modify: false }, // AI会话允许读写
    knowledge:        { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 知识库只读
    terminal:         { can_read: false, can_write: false, can_delete: false, can_modify: false }, // 无终端权限
    game:             { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 游戏只读
    profile_settings: { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 个人中心只读（受限）
    recycle_bin:      { can_read: false, can_write: false, can_delete: false, can_modify: false }, // 无回收站权限
    
    // ===== 首页子功能 (临时账号只能查看) =====
    home_news:        { can_read: true,  can_write: false, can_delete: false, can_modify: false },
    home_todo:        { can_read: true,  can_write: false, can_delete: false, can_modify: false },
    home_log:         { can_read: true,  can_write: false, can_delete: false, can_modify: false },
    home_timer:       { can_read: true,  can_write: false, can_delete: false, can_modify: false },

    // ===== 终端子功能 (特殊白名单) =====
    // 对齐 AuthGuard.tsx 和 routes.ts 的白名单逻辑：
    // - /terminal/manual → 允许访问（纯参考页面）
    // - /terminal/yuancode → 允许访问（展示性占位页面）
    // - /terminal/linux → 允许访问（展示性占位页面）
    terminal_manual:  { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 白名单：命令手册
    terminal_yuancode:{ can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 白名单：YuanCode
    terminal_linux:   { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 白名单：Linux

    // ===== 其他独立模块 (临时账号权限) =====
    search:           { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 搜索只读
    xin:              { can_read: true,  can_write: false, can_delete: false, can_modify: false }, // 小欣只读
    spyglass:         { can_read: false, can_write: false, can_delete: false, can_modify: false }, // 无底层智能权限
  }
}

export function usePermissions(): UsePermissionsReturn {
  const { user } = useAuthStore()
  
  const [permissions, setPermissions] = useState<PermissionsMatrix | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  // 从 authStore 获取角色
  const role: Role | null = user?.role ?? null

  useEffect(() => {
    if (!role || !user) {
      setPermissions(null)
      return
    }

    loadPermissions(role)
  }, [role, user?.id])

  /**
   * 从后端加载权限矩阵
   */
  const loadPermissions = async (currentRole: Role) => {
    setIsLoading(true)

    try {
      // 以 DEFAULT_PERMISSIONS 为基底，确保所有前端资源名都存在
      const basePermissions = { ...DEFAULT_PERMISSIONS[currentRole] }
      const response = await auth.getPermissions()

      if (response.code === 0 && response.data) {
        console.log(`[Permissions] ✅ 加载权限成功 (角色: ${currentRole})`)
        
        // 后端返回 Vec<Permission> (数组) → 转为 Record → 覆盖默认值
        const backendRecord = convertBackendPermissions(response.data)
        const merged = mergeBackendOverrides(basePermissions, backendRecord)
        setPermissions(merged)
      } else {
        console.warn(`[Permissions] ⚠️ 后端返回错误 (code=${response.code})，使用默认权限`)
        setPermissions(basePermissions)
      }
    } catch (error) {
      console.error('[Permissions] ❌ 加载权限失败:', error)
      setPermissions(DEFAULT_PERMISSIONS[currentRole])
    } finally {
      setIsLoading(false)
    }
  }

  /**
   * 将后端返回的权限数组转换为前端期望的 Record 格式
   * 
   * 后端: Vec<Permission> = [ { role, resource, can_read, ... }, ... ]
   * 前端: PermissionsMatrix = { [resource]: { can_read, can_write, ... } }
   */
  const convertBackendPermissions = (data: any): Partial<PermissionsMatrix> => {
    if (!Array.isArray(data)) {
      console.warn('[Permissions] ⚠️ 后端返回非数组格式权限数据')
      return {}
    }

    const record: any = {}
    for (const item of data) {
      if (item?.resource) {
        record[item.resource] = {
          can_read: item.can_read ?? false,
          can_write: item.can_write ?? false,
          can_delete: item.can_delete ?? false,
          can_modify: item.can_modify ?? false,
        }
      }
    }

    console.log(`[Permissions] 📋 后端返回 ${data.length} 条权限记录 → 转换为 ${Object.keys(record).length} 个资源`)
    return record
  }

  /**
   * 以后端数据覆盖 DEFAULT_PERMISSIONS 中的对应资源
   * 
   * 策略：
   * - 前端资源名与后端 resource 完全匹配的 → 用后端数据覆盖
   * - 后端不存在但前端需要的资源 → 保留 DEFAULT_PERMISSIONS 默认值
   * - 这样无论后端资源命名如何都能正确定权
   */
  const mergeBackendOverrides = (
    base: PermissionsMatrix,
    overrides: Partial<PermissionsMatrix>
  ): PermissionsMatrix => {
    return Object.assign({}, base, overrides)
  }

  /**
   * 核心权限检查方法
   */
  const can = (resource: ResourceName, action: PermissionAction): boolean => {
    if (!permissions || !role) {
      console.warn(`[Permissions] ⚠️ 权限未加载，拒绝访问: ${resource}.${action}`)
      return false
    }

    const resourcePermission = permissions[resource]
    
    if (!resourcePermission) {
      console.warn(`[Permissions] ⚠️ 未知资源: ${resource}`)
      console.log('[Permissions] 已知资源列表:', Object.keys(permissions))
      return false
    }

    const permissionKey = `can_${action}` as keyof PermissionDetail
    const hasPermission = resourcePermission[permissionKey]

    if (!hasPermission) {
      console.log(
        `[Permissions] 🔒 权限拒绝: ${resource}.${action} (角色: ${role})`
      )
    }

    return hasPermission
  }

  const isAdmin = (): boolean => role === 'admin'
  
  const isGuest = (): boolean => user?.is_permanent === false

  const getResourcePermission = (resource: ResourceName): PermissionDetail | null => {
    return permissions?.[resource] ?? null
  }

  const refreshPermissions = async () => {
    if (role) {
      await loadPermissions(role)
    }
  }

  return {
    role,
    permissions,
    isLoading,
    can,
    isAdmin,
    isGuest,
    getResourcePermission,
    refreshPermissions
  }
}

export default usePermissions
