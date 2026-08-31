import { t } from "i18next";
/**
 * 权限控制组件集合
 * 
 * v2.0 重构：统一使用 RBAC 权限系统
 * 
 * 重构前的问题：
 * - 使用旧的 utils/permission.ts 中的 usePermission hook
 * - 参数类型为 action (string) + module (string)，类型不安全
 * - 与 AuthGuard、路由守卫使用的权限系统不一致
 * 
 * 重构后的优势：
 * - 统一使用 usePermissions hook（与 AuthGuard 一致）
 * - 参数类型为 ResourceName + PermissionAction，类型安全
 * - 支持细粒度的资源级权限控制
 * - 自动生成友好的权限提示信息
 * 
 * 包含组件：
 * - PermissionRestricted：权限限制容器组件
 * - PermissionButton：权限受限按钮组件
 * - PermissionInput：权限受限输入框组件
 */

import React from 'react';
import { usePermissions } from '../hooks/usePermissions';
import type { ResourceName, PermissionAction } from '@/types';

// ==================== 类型定义 ====================

/**
 * 基础权限组件 Props（所有权限组件的通用属性）
 */
interface BasePermissionProps {
  /** 资源名称（如 'home', 'terminal', 'knowledge'） */
  resource: ResourceName;
  /** 操作类型 ('read' | 'write' | 'delete' | 'modify') */
  action: PermissionAction;
}

// ==================== PermissionRestricted 组件 ====================

interface PermissionRestrictedProps extends BasePermissionProps {
  children: React.ReactNode;
  /** 无权限时显示的备用内容 */
  fallback?: React.ReactNode;
  /** 自定义无权限提示信息 */
  customMessage?: string;
}

/**
 * 权限限制容器组件
 * 
 * 当用户有权限时渲染 children，否则显示权限不足提示或 fallback
 * 
 * @example
 * ```tsx
 * // 知识库写入权限检查
 * <PermissionRestricted resource="knowledge" action="write">
 *   <EditButton>编辑</EditButton>
 * </PermissionRestricted>
 * 
 * // 终端访问权限检查（带自定义提示）
 * <PermissionRestricted resource="terminal" action="read" customMessage="终端功能仅对永久账号开放">
 *   <TerminalComponent />
 * </PermissionRestricted>
 * ```
 */
export const PermissionRestricted: React.FC<PermissionRestrictedProps> = ({
  resource,
  action,
  children,
  fallback,
  customMessage
}) => {
  // 使用统一的 RBAC 权限系统
  const {
    can,
    role,
    isLoading
  } = usePermissions();
  if (isLoading) {
    return <>{children}</>;
  }
  const hasPermission = can(resource, action);
  if (hasPermission) {
    return <>{children}</>;
  }
  if (fallback) {
    return <>{fallback}</>;
  }
  return <div className="nt-permission-restricted">
      <div className="nt-permission-message">
        <div className="nt-permission-icon">🔒</div>
        <div className="nt-permission-text">
          <div className="nt-permission-title">{t("components.PermissionRestricted.k1")}</div>
          <div className="nt-permission-desc">
            {customMessage || generatePermissionMessage(resource, action, role)}
          </div>
        </div>
      </div>
    </div>;
};

// ==================== PermissionButton 组件 ====================

interface PermissionButtonProps extends BasePermissionProps {
  onClick?: () => void;
  children: React.ReactNode;
  className?: string;
  disabled?: boolean;
  /** 自定义无权限时的 tooltip 提示 */
  disabledTooltip?: string;
}

/**
 * 权限受限按钮组件
 * 
 * 当用户无权限时，按钮自动禁用并显示锁图标
 * 点击禁用按钮时显示权限提示
 * 
 * @example
 * ```tsx
 * // 删除按钮（需要 delete 权限）
 * <PermissionButton resource="home_todo" action="delete" onClick={handleDelete}>
 *   删除待办
 * </PermissionButton>
 * ```
 */
export const PermissionButton: React.FC<PermissionButtonProps> = ({
  resource,
  action,
  onClick,
  children,
  className = '',
  disabled = false,
  disabledTooltip
}) => {
  const {
    can,
    role,
    isLoading
  } = usePermissions();
  if (isLoading) {
    return <button className={`nt-button ${className}`} onClick={onClick} disabled={disabled}>
        {children}
      </button>;
  }
  const hasPermission = can(resource, action);
  const message = disabledTooltip || generatePermissionMessage(resource, action, role);
  const handleClick = (e: React.MouseEvent) => {
    if (!hasPermission) {
      e.preventDefault();
      e.stopPropagation();
      alert(message);
      return;
    }
    if (onClick) {
      onClick();
    }
  };
  const isDisabled = disabled || !hasPermission;
  return <button className={`nt-button ${isDisabled ? 'nt-disabled' : ''} ${className}`} onClick={handleClick} disabled={isDisabled} title={!hasPermission ? message : undefined}>
      {children}
      {!hasPermission && <span className="nt-permission-badge">🔒</span>}
    </button>;
};

// ==================== PermissionInput 组件 ====================

interface PermissionInputProps extends BasePermissionProps {
  value: string;
  onChange?: (value: string) => void;
  placeholder?: string;
  type?: string;
  className?: string;
}

/**
 * 权限受限输入框组件
 * 
 * 当用户无写权限时，输入框自动禁用并显示遮罩层
 * 用户仍可查看内容（read 权限），但无法修改
 * 
 * @example
 * ```tsx
 * // 可编辑输入框（需要 write 权限）
 * <PermissionInput 
 *   resource="knowledge" 
 *   action="write"
 *   value={title}
 *   onChange={setTitle}
 *   placeholder="请输入标题"
 * />
 * ```
 */
export const PermissionInput: React.FC<PermissionInputProps> = ({
  resource,
  action,
  value,
  onChange,
  placeholder,
  type = 'text',
  className = ''
}) => {
  const {
    can,
    role,
    isLoading
  } = usePermissions();
  if (isLoading) {
    return <input type={type} value={value} onChange={e => onChange?.(e.target.value)} placeholder={placeholder} className={`nt-input ${className}`} />;
  }
  const hasPermission = can(resource, action);
  const message = generatePermissionMessage(resource, action, role);
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    if (!hasPermission) {
      return;
    }
    if (onChange) {
      onChange(e.target.value);
    }
  };
  return <div className="nt-permission-input-wrapper">
      <input type={type} value={value} onChange={handleChange} placeholder={placeholder} className={`nt-input ${!hasPermission ? 'nt-disabled' : ''} ${className}`} disabled={!hasPermission} title={!hasPermission ? message : undefined} />
      {!hasPermission && <div className="nt-permission-overlay">
          <span className="nt-permission-overlay-text">🔒 {message}</span>
        </div>}
    </div>;
};

// ==================== 辅助函数 ====================

/**
 * 生成友好的权限提示信息
 * 
 * @param resource 资源名称
 * @param action 操作类型
 * @param role 当前用户角色
 * @returns 友好的中文提示信息
 */
function generatePermissionMessage(resource: ResourceName, action: PermissionAction, role: string | null): string {
  // 资源名称映射（中文显示名称）
  const resourceNames: Record<ResourceName, string> = {
    home: t("components.intelligence.DashboardPanel.k106"),
    ai_chat: t("components.PermissionRestricted.k2"),
    knowledge: t("components.intelligence.ActivityPanel.k1"),
    terminal: t("components.intelligence.ActivityPanel.k8"),
    game: t("components.AddExtensionModal.k11"),
    profile_settings: t("components.PermissionRestricted.k3"),
    recycle_bin: t("components.PermissionRestricted.k4"),
    home_news: t("components.PermissionRestricted.k5"),
    home_todo: t("components.PermissionRestricted.k6"),
    home_log: t("components.PermissionRestricted.k7"),
    home_timer: t("components.PermissionRestricted.k8"),
    terminal_manual: t("components.PermissionRestricted.k9"),
    terminal_yuancode: 'YuanCode',
    terminal_linux: 'Linux',
    search: t("common.search"),
    xin: t("components.FloatingXin.k26"),
    spyglass: t("components.intelligence.DashboardPanel.k103")
  };

  // 操作类型映射（中文显示名称）
  const actionNames: Record<PermissionAction, string> = {
    read: t("common.view"),
    write: t("common.edit"),
    delete: t("common.delete"),
    modify: t("common.edit")
  };
  const resourceName = resourceNames[resource] || resource;
  const actionName = actionNames[action] || action;
  if (role === 'guest') {
    return t("components.PermissionRestricted.k10", {
      actionName: actionName,
      resourceName: resourceName
    });
  } else if (role === 'user') {
    return t("components.PermissionRestricted.k11", {
      resourceName: resourceName,
      actionName: actionName
    });
  } else {
    return t("components.PermissionRestricted.k12", {
      resourceName: resourceName,
      actionName: actionName
    });
  }
}