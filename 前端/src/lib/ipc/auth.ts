// ipc/auth.ts — auth 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';
import type { UserInfo, BackendPermission } from '@/types';

export const auth = {
  login: (username: string, password: string) => ipc.invoke<{
    user: UserInfo;
    token: string;
  }>('login', {
    request: {
      username,
      password
    }
  }),
  register: (username: string, password: string, _recoveryPhrase?: string[]) => ipc.invoke<{
    user: UserInfo;
    token: string;
  }>('register', {
    username,
    password,
    isPermanent: true
  }),
  verifyToken: () => ipc.invoke<UserInfo>('auth_verify_token'),
  logout: () => ipc.invoke('logout'),
  getPermissions: () => ipc.invoke<BackendPermission[]>('auth_get_permissions'),
  verifyRecoveryPhrase: (username: string, words: string[], newPassword: string) => ipc.invoke<{
    user: UserInfo;
    token: string;
  }>('recover_by_phrase', {
    username,
    recovery_phrase: words.join(' '),
    new_password: newPassword
  }),
  resetPassword: (oldPassword: string, newPassword: string) => ipc.invoke('auth_reset_password', {
    request: {
      old_password: oldPassword,
      new_password: newPassword
    }
  }),
  // P2: 会话管理
  sessionList: () => ipc.invoke<any[]>('session_list'),
  sessionRevoke: (sessionId: string) => ipc.invoke<any>('session_revoke', {
    session_id: sessionId
  }),
  // P2: 2FA 双因素认证
  auth2faSetup: () => ipc.invoke<{
    secret: string;
    qr_code_url: string;
  }>('auth_2fa_setup'),
  auth2faVerify: (code: string) => ipc.invoke<{
    verified: boolean;
  }>('auth_2fa_verify', {
    code
  }),
  auth2faEnable: (code: string) => ipc.invoke<any>('auth_2fa_enable', {
    code
  }),
  auth2faDisable: (code: string) => ipc.invoke<any>('auth_2fa_disable', {
    code
  }),
  auth2faStatus: () => ipc.invoke<{
    enabled: boolean;
  }>('auth_2fa_status'),
  auth2faLoginVerify: (token: string, code: string) => ipc.invoke<{
    user: UserInfo;
    token: string;
  }>('auth_2fa_login_verify', {
    token,
    code
  })
};
