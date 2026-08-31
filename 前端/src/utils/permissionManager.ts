import { t } from "i18next";
/**
 * ⚠️ @deprecated 权限管理系统 (v1.0 旧版本)
 * 
 * 废弃时间：2026-05-26
 * 替代方案：使用 `@/hooks/usePermissions` hook（统一的 RBAC 权限系统）
 * 
 * 废弃原因：
 * - 过度设计：提供了过于复杂的权限管理能力（20+ 权限类型、4 级权限级别）
 * - 未被使用：项目实际未采用此系统，造成代码冗余
 * - 类型不兼容：使用 PermissionType 枚举，与项目的 ResourceName 类型不匹配
 * - 架构冲突：与已建立的 RBAC 权限模型（Role × Resource × Action）重复
 * - 维护成本高：复杂的类结构增加理解和维护难度
 * 
 * 系统对比：
 * 
 * ❌ 本文件 (permissionManager.ts)：
 * - PermissionType 枚举（20+ 权限类型）
 * - PermissionLevel 枚举（5 个级别：NONE/READ/WRITE/EXECUTE/ADMIN）
 * - PermissionManager 类（复杂的状态管理和持久化）
 * - 基于本地存储的权限策略
 * 
 * ✅ 新系统 (usePermissions hook)：
 * - ResourceName 类型（18 个资源，类型安全）
 * - PermissionAction 类型（4 个操作：read/write/delete/modify）
 * - Role 类型（3 个角色：admin/user/guest）
 * - PermissionsMatrix 矩阵（简单直观的权限配置）
 * - 与后端 permissions 表对齐
 * 
 * 迁移指南：
 * 
 * 如果您确实需要细粒度的文件/网络/系统权限控制：
 * 1. 扩展 ResourceName 类型，添加 file_read, network_access 等资源
 * 2. 在 DEFAULT_PERMISSIONS 中配置对应权限
 * 3. 使用 usePermissions().can('file_read', 'read') 检查权限
 * 
 * 如果不需要此类细粒度控制：
 * - 直接删除对此文件的引用
 * - 使用新的 RBAC 权限系统即可
 * 
 * 注意事项：
 * - 此文件将在 v3.0 中完全移除
 * - 当前保留仅为了向后兼容
 * - 新代码请勿使用此文件中的任何导出
 */

// ==================== 已废弃的类型和类 ====================

// 权限管理系统

/**
 * @deprecated 使用 `ResourceName` from '@/types' 替代
 * 权限类型定义
 */
export enum PermissionType {
  // 文件系统权限
  FILE_READ = 'file:read',
  FILE_WRITE = 'file:write',
  FILE_DELETE = 'file:delete',
  FILE_EXECUTE = 'file:execute',
  // 网络权限
  NETWORK_ACCESS = 'network:access',
  NETWORK_DOWNLOAD = 'network:download',
  NETWORK_UPLOAD = 'network:upload',
  // 系统权限
  SYSTEM_INFO = 'system:info',
  SYSTEM_PROCESS = 'system:process',
  SYSTEM_COMMAND = 'system:command',
  // 扩展权限
  EXTENSION_INSTALL = 'extension:install',
  EXTENSION_UNINSTALL = 'extension:uninstall',
  EXTENSION_MANAGE = 'extension:manage',
  // 用户权限
  USER_PROFILE = 'user:profile',
  USER_SETTINGS = 'user:settings',
  USER_DATA = 'user:data',
  // 应用权限
  APP_CONFIG = 'app:config',
  APP_STORAGE = 'app:storage',
  APP_NOTIFICATION = 'app:notification'
}

/**
 * @deprecated 使用 `PermissionAction` from '@/types' 替代
 * 权限级别
 */
export enum PermissionLevel {
  NONE = 'none',
  READ = 'read',
  WRITE = 'write',
  EXECUTE = 'execute',
  ADMIN = 'admin'
}

// 权限请求接口
export interface PermissionRequest {
  permission: PermissionType;
  level: PermissionLevel;
  reason?: string;
  context?: any;
}

// 权限状态接口
export interface PermissionStatus {
  permission: PermissionType;
  level: PermissionLevel;
  granted: boolean;
  grantedAt?: number;
  expiresAt?: number;
  reason?: string;
}

// 用户角色定义
export enum UserRole {
  GUEST = 'guest',
  USER = 'user',
  DEVELOPER = 'developer',
  ADMIN = 'admin'
}

// 权限策略接口
export interface PermissionPolicy {
  role: UserRole;
  permissions: Record<PermissionType, PermissionLevel>;
}

// 创建完整的权限记录，为所有权限类型提供默认值
const createFullPermissions = (defaultLevel: PermissionLevel = PermissionLevel.NONE): Record<PermissionType, PermissionLevel> => {
  const permissions: Partial<Record<PermissionType, PermissionLevel>> = {};

  // 为所有权限类型设置默认值
  Object.values(PermissionType).forEach(type => {
    permissions[type] = defaultLevel;
  });
  return permissions as Record<PermissionType, PermissionLevel>;
};

// 默认权限策略
const defaultPolicies: PermissionPolicy[] = [{
  role: UserRole.GUEST,
  permissions: {
    ...createFullPermissions(PermissionLevel.NONE),
    [PermissionType.FILE_READ]: PermissionLevel.READ,
    [PermissionType.USER_PROFILE]: PermissionLevel.READ,
    [PermissionType.APP_NOTIFICATION]: PermissionLevel.READ
  }
}, {
  role: UserRole.USER,
  permissions: {
    ...createFullPermissions(PermissionLevel.NONE),
    [PermissionType.FILE_READ]: PermissionLevel.READ,
    [PermissionType.FILE_WRITE]: PermissionLevel.WRITE,
    [PermissionType.NETWORK_ACCESS]: PermissionLevel.READ,
    [PermissionType.SYSTEM_INFO]: PermissionLevel.READ,
    [PermissionType.USER_PROFILE]: PermissionLevel.WRITE,
    [PermissionType.USER_SETTINGS]: PermissionLevel.WRITE,
    [PermissionType.USER_DATA]: PermissionLevel.WRITE,
    [PermissionType.APP_CONFIG]: PermissionLevel.READ,
    [PermissionType.APP_STORAGE]: PermissionLevel.WRITE,
    [PermissionType.APP_NOTIFICATION]: PermissionLevel.WRITE
  }
}, {
  role: UserRole.DEVELOPER,
  permissions: {
    ...createFullPermissions(PermissionLevel.NONE),
    [PermissionType.FILE_READ]: PermissionLevel.EXECUTE,
    [PermissionType.FILE_WRITE]: PermissionLevel.EXECUTE,
    [PermissionType.FILE_DELETE]: PermissionLevel.EXECUTE,
    [PermissionType.FILE_EXECUTE]: PermissionLevel.EXECUTE,
    [PermissionType.NETWORK_ACCESS]: PermissionLevel.EXECUTE,
    [PermissionType.NETWORK_DOWNLOAD]: PermissionLevel.EXECUTE,
    [PermissionType.NETWORK_UPLOAD]: PermissionLevel.EXECUTE,
    [PermissionType.SYSTEM_INFO]: PermissionLevel.EXECUTE,
    [PermissionType.SYSTEM_PROCESS]: PermissionLevel.EXECUTE,
    [PermissionType.SYSTEM_COMMAND]: PermissionLevel.EXECUTE,
    [PermissionType.EXTENSION_INSTALL]: PermissionLevel.WRITE,
    [PermissionType.EXTENSION_UNINSTALL]: PermissionLevel.WRITE,
    [PermissionType.EXTENSION_MANAGE]: PermissionLevel.WRITE,
    [PermissionType.USER_PROFILE]: PermissionLevel.EXECUTE,
    [PermissionType.USER_SETTINGS]: PermissionLevel.EXECUTE,
    [PermissionType.USER_DATA]: PermissionLevel.EXECUTE,
    [PermissionType.APP_CONFIG]: PermissionLevel.EXECUTE,
    [PermissionType.APP_STORAGE]: PermissionLevel.EXECUTE,
    [PermissionType.APP_NOTIFICATION]: PermissionLevel.EXECUTE
  }
}, {
  role: UserRole.ADMIN,
  permissions: {
    ...createFullPermissions(PermissionLevel.ADMIN)
  }
}];

/**
 * @deprecated 使用 `usePermissions` hook from '@/hooks/usePermissions' 替代
 * 权限管理器类
 */
export class PermissionManager {
  private userRole: UserRole = UserRole.GUEST;
  private customPolicies: PermissionPolicy[] = [];
  private grantedPermissions: Map<PermissionType, PermissionStatus> = new Map();
  private listeners: Map<PermissionType, Function[]> = new Map();
  constructor() {
    this.loadUserRole();
    this.loadCustomPolicies();
    this.loadGrantedPermissions();
  }

  // 设置用户角色
  setUserRole(role: UserRole): void {
    this.userRole = role;
    this.saveUserRole();
    this.notifyRoleChange(role);
  }

  // 获取用户角色
  getUserRole(): UserRole {
    return this.userRole;
  }

  // 检查权限
  checkPermission(permission: PermissionType, requiredLevel: PermissionLevel = PermissionLevel.READ): boolean {
    // 检查已授予的权限
    const grantedStatus = this.grantedPermissions.get(permission);
    if (grantedStatus && grantedStatus.granted) {
      return this.compareLevels(grantedStatus.level, requiredLevel) >= 0;
    }

    // 检查策略权限
    const policy = this.getPolicyForRole(this.userRole);
    const policyLevel = policy.permissions[permission] || PermissionLevel.NONE;
    return this.compareLevels(policyLevel, requiredLevel) >= 0;
  }

  // 请求权限
  async requestPermission(request: PermissionRequest): Promise<boolean> {
    const {
      permission,
      level,
      reason
    } = request;

    // 检查是否已有足够权限
    if (this.checkPermission(permission, level)) {
      return true;
    }

    // 检查策略权限
    const policy = this.getPolicyForRole(this.userRole);
    const policyLevel = policy.permissions[permission] || PermissionLevel.NONE;
    if (this.compareLevels(policyLevel, level) >= 0) {
      // 策略允许，自动授予
      this.grantPermission(permission, level, reason);
      return true;
    }

    // 需要用户确认
    const granted = await this.showPermissionDialog();
    if (granted) {
      this.grantPermission(permission, level, reason);
    }
    return granted;
  }

  // 授予权限
  grantPermission(permission: PermissionType, level: PermissionLevel, reason?: string): void {
    const status: PermissionStatus = {
      permission,
      level,
      granted: true,
      grantedAt: Date.now(),
      reason
    };
    this.grantedPermissions.set(permission, status);
    this.saveGrantedPermissions();
    this.notifyPermissionChange(permission, status);
  }

  // 撤销权限
  revokePermission(permission: PermissionType): void {
    this.grantedPermissions.delete(permission);
    this.saveGrantedPermissions();
    this.notifyPermissionChange(permission, {
      permission,
      level: PermissionLevel.NONE,
      granted: false
    });
  }

  // 获取权限状态
  getPermissionStatus(permission: PermissionType): PermissionStatus | null {
    return this.grantedPermissions.get(permission) || null;
  }

  // 获取所有权限状态
  getAllPermissions(): PermissionStatus[] {
    return Array.from(this.grantedPermissions.values());
  }

  // 添加自定义策略
  addCustomPolicy(policy: PermissionPolicy): void {
    this.customPolicies.push(policy);
    this.saveCustomPolicies();
  }

  // 移除自定义策略
  removeCustomPolicy(role: UserRole): void {
    this.customPolicies = this.customPolicies.filter(p => p.role !== role);
    this.saveCustomPolicies();
  }

  // 监听权限变化
  onPermissionChange(permission: PermissionType, callback: (status: PermissionStatus) => void): void {
    if (!this.listeners.has(permission)) {
      this.listeners.set(permission, []);
    }
    this.listeners.get(permission)!.push(callback);
  }

  // 移除监听器
  offPermissionChange(permission: PermissionType, callback: Function): void {
    const callbacks = this.listeners.get(permission);
    if (callbacks) {
      const index = callbacks.indexOf(callback);
      if (index > -1) {
        callbacks.splice(index, 1);
      }
    }
  }

  // 私有方法
  private getPolicyForRole(role: UserRole): PermissionPolicy {
    // 首先检查自定义策略
    const customPolicy = this.customPolicies.find(p => p.role === role);
    if (customPolicy) {
      return customPolicy;
    }

    // 返回默认策略
    const defaultPolicy = defaultPolicies.find(p => p.role === role);
    return defaultPolicy || {
      role,
      permissions: createFullPermissions()
    };
  }
  private compareLevels(level1: PermissionLevel, level2: PermissionLevel): number {
    const levels = [PermissionLevel.NONE, PermissionLevel.READ, PermissionLevel.WRITE, PermissionLevel.EXECUTE, PermissionLevel.ADMIN];
    return levels.indexOf(level1) - levels.indexOf(level2);
  }
  private async showPermissionDialog(): Promise<boolean> {
    // 在实际应用中，这里会显示一个权限请求对话框
    // 目前返回false，需要用户手动确认
    return false;
  }
  private notifyPermissionChange(permission: PermissionType, status: PermissionStatus): void {
    const callbacks = this.listeners.get(permission);
    if (callbacks) {
      callbacks.forEach(callback => callback(status));
    }
  }
  private notifyRoleChange(_role: UserRole): void {
    // 通知所有权限监听器角色变化
    this.grantedPermissions.forEach((status, permission) => {
      this.notifyPermissionChange(permission, status);
    });
  }

  // 持久化方法
  private loadUserRole(): void {
    try {
      const saved = localStorage.getItem('nt_user_role');
      if (saved) {
        this.userRole = saved as UserRole;
      }
    } catch (error) {
      console.warn('加载用户角色失败:', error);
    }
  }
  private saveUserRole(): void {
    try {
      localStorage.setItem('nt_user_role', this.userRole);
    } catch (error) {
      console.warn('保存用户角色失败:', error);
    }
  }
  private loadCustomPolicies(): void {
    try {
      const saved = localStorage.getItem('nt_custom_policies');
      if (saved) {
        this.customPolicies = JSON.parse(saved);
      }
    } catch (error) {
      console.warn('加载自定义策略失败:', error);
    }
  }
  private saveCustomPolicies(): void {
    try {
      localStorage.setItem('nt_custom_policies', JSON.stringify(this.customPolicies));
    } catch (error) {
      console.warn('保存自定义策略失败:', error);
    }
  }
  private loadGrantedPermissions(): void {
    try {
      const saved = localStorage.getItem('nt_granted_permissions');
      if (saved) {
        const permissions = JSON.parse(saved);
        permissions.forEach((p: any) => {
          this.grantedPermissions.set(p.permission, p);
        });
      }
    } catch (error) {
      console.warn('加载已授予权限失败:', error);
    }
  }
  private saveGrantedPermissions(): void {
    try {
      const permissions = Array.from(this.grantedPermissions.values());
      localStorage.setItem('nt_granted_permissions', JSON.stringify(permissions));
    } catch (error) {
      console.warn('保存已授予权限失败:', error);
    }
  }
}

/**
 * @deprecated 使用 `usePermissions` hook 替代
 * 创建全局权限管理器实例
 */
export const permissionManager = new PermissionManager();

/**
 * @deprecated 使用组件级权限检查替代（如 PermissionRestricted 组件）
 * 权限装饰器（用于函数级别的权限控制）
 */
export function requirePermission(permission: PermissionType, level: PermissionLevel = PermissionLevel.READ) {
  return function (_target: any, _propertyKey: string, descriptor: PropertyDescriptor) {
    const originalMethod = descriptor.value;
    descriptor.value = function (...args: any[]) {
      if (!permissionManager.checkPermission(permission, level)) {
        throw new Error(t("utils.permissionManager.k1", {
          permission: permission,
          level: level
        }));
      }
      return originalMethod.apply(this, args);
    };
    return descriptor;
  };
}

/**
 * @deprecated 使用 `usePermissions().can(resource, action)` 替代
 * 权限检查工具函数
 */
export function hasPermission(permission: PermissionType, level: PermissionLevel = PermissionLevel.READ): boolean {
  return permissionManager.checkPermission(permission, level);
}
export function requirePermissionAsync(permission: PermissionType, level: PermissionLevel = PermissionLevel.READ): Promise<boolean> {
  return permissionManager.requestPermission({
    permission,
    level
  });
}