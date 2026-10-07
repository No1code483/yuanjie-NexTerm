// customs.auth L1 壳向各功能域子插件注入的核心共享契约（纯类型，无 JSX）。
// 各 L2 插件（auth.login / register / recovery / temp）经 core 对象接收共享表单状态与回调，
// 禁止直接修改 L1 核心 state（同板块 L1↔L2 例外，见插件开发规范 §1.4）。
import type { Dispatch, SetStateAction } from 'react';

/** 认证视图模式（与迁移前 AuthMode 逐字一致） */
export type AuthMode =
  | 'login' | 'register' | 'show_recovery_phrase' | 'forgot_password'
  | 'verify_recovery' | 'reset_password' | 'temp_login' | '2fa_verify';

/** 各认证视图共享的 L1 契约（登录/注册表单共用 username/password；全视图共用 error/isLoading） */
export interface AuthCore {
  username: string;
  setUsername: Dispatch<SetStateAction<string>>;
  password: string;
  setPassword: Dispatch<SetStateAction<string>>;
  error: string;
  setError: Dispatch<SetStateAction<string>>;
  isLoading: boolean;
  setIsLoading: Dispatch<SetStateAction<boolean>>;
  clearError: () => void;
  /** 切换视图（L1 持有 mode，各 L2 经此回写） */
  goToMode: (mode: AuthMode) => void;
  /** 完成登录（透传 App.tsx 的 onLogin） */
  onLogin: (user?: any, token?: string) => void;
}
