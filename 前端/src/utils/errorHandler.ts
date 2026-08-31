import { t } from "i18next";
// React导入用于错误边界组件
import React from 'react';
import { random } from '@/lib/utils';

// 错误处理系统

// 错误类型定义
export enum ErrorType {
  NETWORK_ERROR = 'network_error',
  VALIDATION_ERROR = 'validation_error',
  PERMISSION_ERROR = 'permission_error',
  STORAGE_ERROR = 'storage_error',
  SYSTEM_ERROR = 'system_error',
  EXTENSION_ERROR = 'extension_error',
  UI_ERROR = 'ui_error',
  UNKNOWN_ERROR = 'unknown_error'
}

// 错误级别定义
export enum ErrorLevel {
  DEBUG = 'debug',
  INFO = 'info',
  WARNING = 'warning',
  ERROR = 'error',
  FATAL = 'fatal'
}

// 错误信息接口
export interface ErrorInfo {
  id: string;
  type: ErrorType;
  level: ErrorLevel;
  message: string;
  stack?: string;
  timestamp: number;
  context?: Record<string, any>;
  userInfo?: {
    userId?: string;
    userRole?: string;
    userAgent?: string;
  };
  systemInfo?: {
    platform?: string;
    version?: string;
    memory?: number;
  };
}

// 错误处理选项
export interface ErrorHandlerOptions {
  autoReport?: boolean;
  maxErrors?: number;
  ignorePatterns?: RegExp[];
  notifyUser?: boolean;
  logToConsole?: boolean;
  logToFile?: boolean;
  logToRemote?: boolean;
}

// 错误监听器接口
export interface ErrorListener {
  onError: (error: ErrorInfo) => void;
  onWarning?: (error: ErrorInfo) => void;
  onFatal?: (error: ErrorInfo) => void;
}

// 错误处理管理器类
export class ErrorHandler {
  private options: ErrorHandlerOptions;
  private errors: ErrorInfo[] = [];
  private listeners: ErrorListener[] = [];
  private isInitialized = false;
  constructor(options: ErrorHandlerOptions = {}) {
    this.options = {
      autoReport: true,
      maxErrors: 1000,
      ignorePatterns: [],
      notifyUser: true,
      logToConsole: true,
      logToFile: false,
      logToRemote: false,
      ...options
    };
  }

  // 初始化错误处理系统
  initialize(): void {
    if (this.isInitialized) return;

    // 捕获全局错误
    window.addEventListener('error', this.handleGlobalError.bind(this));
    window.addEventListener('unhandledrejection', this.handleUnhandledRejection.bind(this));

    // 捕获React错误
    if ((window as any).React) {
      this.setupReactErrorBoundary();
    }
    this.isInitialized = true;
    console.log('错误处理系统初始化完成');
  }

  // 处理错误
  handleError(error: Error | string, context?: Record<string, any>): string {
    const errorInfo = this.createErrorInfo(error, ErrorLevel.ERROR, context);
    return this.processError(errorInfo);
  }

  // 处理警告
  handleWarning(error: Error | string, context?: Record<string, any>): string {
    const errorInfo = this.createErrorInfo(error, ErrorLevel.WARNING, context);
    return this.processError(errorInfo);
  }

  // 处理致命错误
  handleFatal(error: Error | string, context?: Record<string, any>): string {
    const errorInfo = this.createErrorInfo(error, ErrorLevel.FATAL, context);
    const errorId = this.processError(errorInfo);

    // 致命错误需要特殊处理
    this.handleFatalError(errorInfo);
    return errorId;
  }

  // 添加错误监听器
  addListener(listener: ErrorListener): void {
    this.listeners.push(listener);
  }

  // 移除错误监听器
  removeListener(listener: ErrorListener): void {
    const index = this.listeners.indexOf(listener);
    if (index > -1) {
      this.listeners.splice(index, 1);
    }
  }

  // 获取错误统计
  getErrorStats(): {
    total: number;
    byType: Record<ErrorType, number>;
    byLevel: Record<ErrorLevel, number>;
    recent: ErrorInfo[];
  } {
    const byType: Record<ErrorType, number> = {} as any;
    const byLevel: Record<ErrorLevel, number> = {} as any;
    this.errors.forEach(error => {
      byType[error.type] = (byType[error.type] || 0) + 1;
      byLevel[error.level] = (byLevel[error.level] || 0) + 1;
    });
    return {
      total: this.errors.length,
      byType,
      byLevel,
      recent: this.errors.slice(-10)
    };
  }

  // 清除错误记录
  clearErrors(): void {
    this.errors = [];
  }

  // 导出错误报告
  exportReport(): Blob {
    const report = {
      timestamp: Date.now(),
      version: '1.0.0',
      errors: this.errors,
      stats: this.getErrorStats()
    };
    return new Blob([JSON.stringify(report, null, 2)], {
      type: 'application/json'
    });
  }

  // 私有方法
  private createErrorInfo(error: Error | string, level: ErrorLevel, context?: Record<string, any>): ErrorInfo {
    const isErrorObject = error instanceof Error;
    const message = isErrorObject ? error.message : String(error);
    const stack = isErrorObject ? error.stack : new Error().stack;

    // 确定错误类型
    const type = this.determineErrorType(message, stack, context);
    return {
      id: this.generateErrorId(),
      type,
      level,
      message,
      stack,
      timestamp: Date.now(),
      context,
      userInfo: this.getUserInfo(),
      systemInfo: this.getSystemInfo()
    };
  }
  private processError(errorInfo: ErrorInfo): string {
    // 检查是否应该忽略此错误
    if (this.shouldIgnoreError(errorInfo)) {
      return errorInfo.id;
    }

    // 添加到错误记录
    this.errors.push(errorInfo);

    // 限制错误记录数量
    if (this.errors.length > this.options.maxErrors!) {
      this.errors.shift();
    }

    // 记录到控制台
    if (this.options.logToConsole) {
      this.logToConsole(errorInfo);
    }

    // 记录到文件
    if (this.options.logToFile) {
      this.logToFile(errorInfo);
    }

    // 通知用户
    if (this.options.notifyUser && errorInfo.level !== ErrorLevel.DEBUG) {
      this.notifyUser(errorInfo);
    }

    // 通知监听器
    this.notifyListeners(errorInfo);

    // 自动报告
    if (this.options.autoReport && errorInfo.level >= ErrorLevel.ERROR) {
      this.reportError(errorInfo);
    }
    return errorInfo.id;
  }
  private determineErrorType(message: string, _stack?: string, _context?: Record<string, any>): ErrorType {
    const lowerMessage = message.toLowerCase();
    if (lowerMessage.includes('network') || lowerMessage.includes('fetch') || lowerMessage.includes('http')) {
      return ErrorType.NETWORK_ERROR;
    }
    if (lowerMessage.includes('permission') || lowerMessage.includes('access denied')) {
      return ErrorType.PERMISSION_ERROR;
    }
    if (lowerMessage.includes('storage') || lowerMessage.includes('localstorage') || lowerMessage.includes('indexeddb')) {
      return ErrorType.STORAGE_ERROR;
    }
    if (lowerMessage.includes('extension') || lowerMessage.includes('plugin')) {
      return ErrorType.EXTENSION_ERROR;
    }
    if (lowerMessage.includes('validation') || lowerMessage.includes('invalid')) {
      return ErrorType.VALIDATION_ERROR;
    }
    if (lowerMessage.includes('ui') || lowerMessage.includes('component') || lowerMessage.includes('render')) {
      return ErrorType.UI_ERROR;
    }
    if (lowerMessage.includes('system') || lowerMessage.includes('os') || lowerMessage.includes('memory')) {
      return ErrorType.SYSTEM_ERROR;
    }
    return ErrorType.UNKNOWN_ERROR;
  }
  private generateErrorId(): string {
    return `err_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
  private getUserInfo() {
    return {
      userId: localStorage.getItem('nt_user_id') || undefined,
      userRole: localStorage.getItem('nt_user_role') || undefined,
      userAgent: navigator.userAgent
    };
  }
  private getSystemInfo() {
    return {
      platform: navigator.platform,
      version: navigator.appVersion,
      memory: (performance as any).memory ? (performance as any).memory.usedJSHeapSize : undefined
    };
  }
  private shouldIgnoreError(errorInfo: ErrorInfo): boolean {
    // 检查忽略模式
    for (const pattern of this.options.ignorePatterns || []) {
      if (pattern.test(errorInfo.message)) {
        return true;
      }
    }

    // 忽略某些浏览器特定的错误
    const ignoredMessages = ['ResizeObserver loop limit exceeded', 'Script error', 'Loading chunk', 'Failed to fetch'];
    return ignoredMessages.some(msg => errorInfo.message.includes(msg));
  }
  private logToConsole(errorInfo: ErrorInfo): void {
    const {
      level,
      message,
      stack,
      context
    } = errorInfo;
    const logMethod = {
      [ErrorLevel.DEBUG]: console.debug,
      [ErrorLevel.INFO]: console.info,
      [ErrorLevel.WARNING]: console.warn,
      [ErrorLevel.ERROR]: console.error,
      [ErrorLevel.FATAL]: console.error
    }[level];
    const prefix = `[${level.toUpperCase()}] ${errorInfo.type}:`;
    if (context) {
      logMethod(prefix, message, '\nContext:', context, '\nStack:', stack);
    } else {
      logMethod(prefix, message, '\nStack:', stack);
    }
  }
  private logToFile(errorInfo: ErrorInfo): void {
    // 在实际应用中，这里会将错误记录到文件
    // 目前仅记录到控制台
    console.log('记录错误到文件:', errorInfo);
  }
  private notifyUser(errorInfo: ErrorInfo): void {
    const {
      level
    } = errorInfo;

    // 根据错误级别显示不同的用户通知
    let notificationType: string | undefined;
    switch (level) {
      case ErrorLevel.WARNING:
        notificationType = 'warning';
        break;
      case ErrorLevel.ERROR:
      case ErrorLevel.FATAL:
        notificationType = 'error';
        break;
      default:
        // DEBUG和INFO级别不显示用户通知
        return;
    }
    if (notificationType) {
      // 在实际应用中，这里会显示用户友好的错误通知
      // 使用更友好的方式显示错误信息
      const userMessage = this.getUserFriendlyMessage(errorInfo);
      console.log(`用户通知 [${notificationType}]:`, userMessage);

      // 在实际应用中，这里可以显示弹窗或通知组件
      this.showUserNotification(notificationType, userMessage);
    }
  }
  private getUserFriendlyMessage(errorInfo: ErrorInfo): string {
    const {
      type,
      message
    } = errorInfo;
    const friendlyMessages = {
      [ErrorType.NETWORK_ERROR]: t("utils.errorHandler.k1"),
      [ErrorType.VALIDATION_ERROR]: t("utils.errorHandler.k2"),
      [ErrorType.PERMISSION_ERROR]: t("utils.errorHandler.k3"),
      [ErrorType.STORAGE_ERROR]: t("utils.errorHandler.k4"),
      [ErrorType.SYSTEM_ERROR]: t("utils.errorHandler.k5"),
      [ErrorType.EXTENSION_ERROR]: t("utils.errorHandler.k6"),
      [ErrorType.UI_ERROR]: t("utils.errorHandler.k7"),
      [ErrorType.UNKNOWN_ERROR]: t("utils.errorHandler.k8")
    };
    return friendlyMessages[type] || message;
  }
  private showUserNotification(type: string, message: string): void {
    // 在实际应用中，这里会显示真实的用户通知
    // 例如使用 toast 通知、模态框等UI组件
    console.log(`显示用户通知 [${type}]:`, message);
  }
  private notifyListeners(errorInfo: ErrorInfo): void {
    this.listeners.forEach(listener => {
      try {
        if (errorInfo.level === ErrorLevel.FATAL && listener.onFatal) {
          listener.onFatal(errorInfo);
        } else if (errorInfo.level === ErrorLevel.WARNING && listener.onWarning) {
          listener.onWarning(errorInfo);
        } else if (listener.onError) {
          listener.onError(errorInfo);
        }
      } catch (error) {
        console.error('错误监听器执行失败:', error);
      }
    });
  }
  private reportError(errorInfo: ErrorInfo): void {
    // 在实际应用中，这里会将错误报告到远程服务器
    // 目前仅记录到控制台
    if (this.options.logToRemote) {
      console.log('远程错误报告:', errorInfo);
    }
  }
  private handleGlobalError(event: ErrorEvent): void {
    const errorInfo = this.createErrorInfo(event.error || event.message, ErrorLevel.ERROR, {
      filename: event.filename,
      lineno: event.lineno,
      colno: event.colno
    });
    this.processError(errorInfo);

    // 阻止默认错误处理
    event.preventDefault();
  }
  private handleUnhandledRejection(event: PromiseRejectionEvent): void {
    const errorInfo = this.createErrorInfo(event.reason, ErrorLevel.ERROR, {
      promise: event.promise
    });
    this.processError(errorInfo);

    // 阻止默认错误处理
    event.preventDefault();
  }
  private setupReactErrorBoundary(): void {
    // 在实际应用中，这里会设置React错误边界
    // 目前仅记录到控制台
    console.log('React错误边界已设置');
  }
  private handleFatalError(errorInfo: ErrorInfo): void {
    // 致命错误处理逻辑
    console.error('致命错误发生:', errorInfo);

    // 在实际应用中，这里会进行应用重启或数据恢复
    // 目前仅记录到控制台
  }
}

// 创建全局错误处理器实例
export const errorHandler = new ErrorHandler();

// 错误边界组件（React）
export class ErrorBoundary extends React.Component<{
  children: React.ReactNode;
  fallback?: React.ComponentType<{
    error: ErrorInfo;
  }>;
  onError?: (error: ErrorInfo) => void;
}, {
  hasError: boolean;
  errorInfo?: ErrorInfo;
}> {
  constructor(props: any) {
    super(props);
    this.state = {
      hasError: false
    };
  }
  static getDerivedStateFromError(_error: Error) {
    return {
      hasError: true
    };
  }
  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    const errorData = {
      id: random.uid('err_'),
      type: ErrorType.UI_ERROR,
      level: ErrorLevel.ERROR,
      message: error.message,
      stack: error.stack,
      timestamp: Date.now(),
      context: {
        componentStack: errorInfo.componentStack
      }
    };
    errorHandler.handleError(error);
    this.setState({
      errorInfo: errorData
    });
    if (this.props.onError) {
      this.props.onError(errorData);
    }
  }
  render() {
    if (this.state.hasError) {
      if (this.props.fallback && this.state.errorInfo) {
        const FallbackComponent = this.props.fallback;
        return React.createElement(FallbackComponent, {
          error: this.state.errorInfo
        });
      }
      return React.createElement('div', {
        style: {
          padding: '20px',
          textAlign: 'center',
          color: '#FF0000',
          backgroundColor: '#000000',
          border: '1px solid #FF0000'
        }
      }, [React.createElement('h3', {
        key: 'title'
      }, t("utils.errorHandler.k9")), React.createElement('p', {
        key: 'message'
      }, t("utils.errorHandler.k10")), this.state.errorInfo && React.createElement('details', {
        key: 'details',
        style: {
          textAlign: 'left',
          marginTop: '10px'
        }
      }, [React.createElement('summary', {
        key: 'summary'
      }, t("utils.errorHandler.k11")), React.createElement('pre', {
        key: 'pre',
        style: {
          fontSize: '12px',
          color: '#888888',
          overflow: 'auto'
        }
      }, this.state.errorInfo.message)])]);
    }
    return this.props.children;
  }
}

// 错误处理工具函数
export function withErrorHandler<T extends Function>(fn: T, context?: Record<string, any>): T {
  return ((...args: any[]) => {
    try {
      return fn(...args);
    } catch (error) {
      errorHandler.handleError(error as Error, context);
      throw error;
    }
  }) as unknown as T;
}
export function safeExecute<T>(fn: () => T, defaultValue?: T, context?: Record<string, any>): T {
  try {
    return fn();
  } catch (error) {
    errorHandler.handleError(error as Error, context);
    return defaultValue as T;
  }
}
export async function safeExecuteAsync<T>(fn: () => Promise<T>, defaultValue?: T, context?: Record<string, any>): Promise<T> {
  try {
    return await fn();
  } catch (error) {
    errorHandler.handleError(error as Error, context);
    return defaultValue as T;
  }
}

// 错误处理Hooks（React）
export function useErrorHandler() {
  const [error, setError] = React.useState<ErrorInfo | null>(null);
  const handleError = React.useCallback((error: Error | string, context?: Record<string, any>) => {
    const errorInfo = errorHandler.handleError(error, context);
    setError(errorHandler.getErrorStats().recent[0] || null);
    return errorInfo;
  }, []);
  const clearError = React.useCallback(() => {
    setError(null);
  }, []);
  return {
    error,
    handleError,
    clearError
  };
}