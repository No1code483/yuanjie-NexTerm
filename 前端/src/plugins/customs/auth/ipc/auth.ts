// plugins/customs/auth/ipc/auth.ts — 认证插件 IPC 客户端
// 阶段3 批次1a-1：由 lib/ipc/auth.ts 迁入，命名空间 'au'（短码表 05_数据层 §2.1）。
// 其中 changePassword / changeUsername / updateProfile / createTempAccount
// 4 条命令原挂在 boards.profile，按「跨插件命令归属唯一」（插件化重构期规则 §三.5）
// 收归本插件；命令名保持旧名以便后端 alias 一对一。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';
import type { UserInfo, BackendPermission } from '@/types';

export const AUTH_IPC_METHODS = {
  login: { cmd: 'login' },
  register: { cmd: 'register' },
  verifyToken: { cmd: 'auth_verify_token' },
  logout: { cmd: 'logout' },
  getPermissions: { cmd: 'auth_get_permissions' },
  verifyRecoveryPhrase: { cmd: 'recover_by_phrase' },
  resetPassword: { cmd: 'auth_reset_password' },
  restoreSession: { cmd: 'auth_restore_session' },
  sessionList: { cmd: 'session_list' },
  sessionRevoke: { cmd: 'session_revoke' },
  auth2faSetup: { cmd: 'auth_2fa_setup' },
  auth2faVerify: { cmd: 'auth_2fa_verify' },
  auth2faEnable: { cmd: 'auth_2fa_enable' },
  auth2faDisable: { cmd: 'auth_2fa_disable' },
  auth2faStatus: { cmd: 'auth_2fa_status' },
  auth2faLoginVerify: { cmd: 'auth_2fa_login_verify' },
  changePassword: { cmd: 'profile_change_password' },
  changeUsername: { cmd: 'profile_change_username' },
  updateProfile: { cmd: 'profile_update_profile' },
  createTempAccount: { cmd: 'create_temp_account' }
} satisfies Record<string, IpcMethodSpec>;

const dispatcherAuth = defineIpcNamespace('au', AUTH_IPC_METHODS);
type AuthMethod = keyof typeof AUTH_IPC_METHODS;

function invokeAuth<T>(method: AuthMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherAuth[method](args) as Promise<ApiResponse<T>>;
}

export const auth = {
  login: (username: string, password: string) => invokeAuth<{
    user: UserInfo;
    token: string;
  }>('login', {
    request: {
      username,
      password
    }
  }),
  register: (username: string, password: string, _recoveryPhrase?: string[]) => invokeAuth<{
    user: UserInfo;
    token: string;
    recovery_phrase: string;
  }>('register', {
    username,
    password,
    isPermanent: true
  }),
  verifyToken: () => invokeAuth<UserInfo>('verifyToken'),
  logout: () => invokeAuth('logout'),
  getPermissions: () => invokeAuth<BackendPermission[]>('getPermissions'),
  verifyRecoveryPhrase: (username: string, words: string[], newPassword: string) => invokeAuth<{
    user: UserInfo;
    token: string;
  }>('verifyRecoveryPhrase', {
    username,
    recoveryPhrase: words.join(' '),
    newPassword
  }),
  resetPassword: (oldPassword: string, newPassword: string) => invokeAuth('resetPassword', {
    request: {
      old_password: oldPassword,
      new_password: newPassword
    }
  }),
  restoreSession: (token: string) => invokeAuth<{
    user: UserInfo;
    token: string;
  }>('restoreSession', {
    token
  }),
  // P2: 会话管理
  sessionList: () => invokeAuth<any[]>('sessionList'),
  sessionRevoke: (sessionId: string) => invokeAuth<any>('sessionRevoke', {
    sessionId
  }),
  // P2: 2FA 双因素认证
  auth2faSetup: () => invokeAuth<{
    secret: string;
    qr_code_url: string;
  }>('auth2faSetup'),
  auth2faVerify: (code: string) => invokeAuth<{
    verified: boolean;
  }>('auth2faVerify', {
    code
  }),
  auth2faEnable: (code: string) => invokeAuth<any>('auth2faEnable', {
    code
  }),
  auth2faDisable: (code: string) => invokeAuth<any>('auth2faDisable', {
    code
  }),
  auth2faStatus: () => invokeAuth<{
    enabled: boolean;
  }>('auth2faStatus'),
  auth2faLoginVerify: (token: string, code: string) => invokeAuth<{
    user: UserInfo;
    token: string;
  }>('auth2faLoginVerify', {
    token,
    code
  }),
  // 由 boards.profile 收归（批次1a-1）
  changePassword: (oldPassword: string, newPassword: string) => invokeAuth('changePassword', {
    request: { old_password: oldPassword, new_password: newPassword }
  }),
  changeUsername: (newUsername: string) => invokeAuth('changeUsername', {
    request: { new_username: newUsername }
  }),
  updateProfile: (data: {
    avatar_url?: string | null;
    bio?: string | null;
    display_name?: string | null;
  }) => invokeAuth('updateProfile', { request: data }),
  createTempAccount: (username: string, duration: '1h' | '24h' | '7d') => invokeAuth<{
    username: string;
    password: string;
    expires_at: number;
  }>('createTempAccount', {
    durationHours: duration === '1h' ? 1 : duration === '24h' ? 24 : 168,
    username
  })
};
