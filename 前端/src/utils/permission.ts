import { t } from "i18next";
/**
 * ⚠️ @deprecated 权限检查工具函数 (v1.0 旧版本)
 * 
 * 废弃时间：2026-05-26
 * 替代方案：使用 `@/hooks/usePermissions` hook（统一的 RBAC 权限系统）
 * 
 * 废弃原因：
 * - 使用硬编码的模块权限规则，难以维护和扩展
 * - 参数类型不安全（使用 string 而非枚举类型）
 * - 与项目其他部分的权限系统（AuthGuard、路由守卫）不一致
 * - 无法支持细粒度的资源级权限控制
 * 
 * 迁移指南：
 * 
 * ❌ 旧代码：
 * ```typescript
 * import { usePermission } from '@/utils/permission'
 * const { hasPermission } = usePermission('read', 'home')
 * ```
 * 
 * ✅ 新代码：
 * ```typescript
 * import { usePermissions } from '@/hooks/usePermissions'
 * import type { ResourceName, PermissionAction } from '@/types'
 * 
 * const { can } = usePermissions()
 * const hasPermission = can('home', 'read') // 类型安全！
 * ```
 * 
 * 资源名称映射（旧 → 新）：
 * - 'home' → 'home'
 * - 'ai' → 'ai_chat'
 * - 'knowledge' → 'knowledge'
 * - 'terminal' → 'terminal'
 * - 'game' → 'game'
 * - 'profile' → 'profile_settings'
 * - 'recycle' → 'recycle_bin'
 * 
 * 注意事项：
 * - 此文件将在 v3.0 中完全移除
 * - 请尽快迁移到新的权限系统
 * - 如有疑问，请参考 `@/hooks/usePermissions.ts` 的文档
 */

// ==================== 已废弃的函数 ====================

/**
 * @deprecated 使用 `usePermissions().isGuest()` 替代
 * 检查当前用户是否为临时账号
 */
export function isTempAccount(): boolean {
  console.warn('[Permission] ⚠️ isTempAccount() 已废弃，请使用 usePermissions().isGuest()');
  return localStorage.getItem('nt_temp_account') === 'true';
}

/**
 * @deprecated 使用 `usePermissions().can(resource, action)` 替代
 * 检查临时账号是否有权限执行操作
 * @param action 操作类型：'read' | 'write' | 'create' | 'delete' | 'modify'
 * @param module 模块名称
 */
export function hasPermission(action: 'read' | 'write' | 'create' | 'delete' | 'modify', module: string): boolean {
  console.warn(`[Permission] ⚠️ hasPermission() 已废弃，请使用 usePermissions().can('${module}', '${action}')`);
  const temp = isTempAccount();
  if (!temp) {
    return true;
  }
  switch (module) {
    case 'home':
      return action === 'read';
    case 'knowledge':
      return action === 'read';
    case 'ai':
      return action === 'read' || action === 'write';
    case 'game':
      return action === 'read';
    case 'terminal':
      return false;
    case 'profile':
      return action === 'read';
    case 'recycle':
      return false;
    default:
      return false;
  }
}

/**
 * @deprecated 使用 PermissionRestricted 组件的自动提示功能替代
 * 获取权限提示信息
 */
export function getPermissionMessage(action: string, module: string): string {
  console.warn('[Permission] ⚠️ getPermissionMessage() 已废弃，请使用 PermissionRestricted 组件');
  if (!isTempAccount()) {
    return '';
  }
  if (!hasPermission(action as any, module)) {
    return t("utils.permission.k1");
  }
  return '';
}

/**
 * @deprecated 使用 `usePermissions` hook 替代
 * 权限检查钩子
 */
export function usePermission(action: 'read' | 'write' | 'create' | 'delete' | 'modify', module: string) {
  console.warn('[Permission] ⚠️ usePermission() 已废弃，请使用 usePermissions() hook');
  const hasPerm = hasPermission(action, module);
  const message = getPermissionMessage(action, module);
  return {
    hasPermission: hasPerm,
    message,
    isTempAccount: isTempAccount()
  };
}