// boards.profile L1 壳向各功能域子插件注入的核心共享契约（只读数据 + 回调）。
// 各 L2 插件（profile.account / resume / quote / settings）经 props 或 core 对象接收，
// 禁止直接修改 L1 核心 state；需要刷新走此处回调（同板块 L1↔L2 例外，见插件开发规范 §1.4）。
import type { Dispatch, SetStateAction } from 'react';
import type { UserInfo } from '@/types';

/** 统一确认弹窗状态（多场景复用） */
export interface ConfirmDialogState {
  isOpen: boolean;
  targetName: string;
  onConfirm: () => void | Promise<void>;
}

export interface ProfileCore {
  /** 当前登录用户（未登录时为 null；各面板仅在鉴权通过后渲染） */
  user: UserInfo | null;
  /** 是否为临时账号（user.is_permanent === false） */
  isTempAccount: boolean;
  /** 统一通知：桥接旧 showNotification → 全局 notifStore */
  showNotify: (type: 'success' | 'error' | 'info', message: string) => void;
  /** 统一确认弹窗（多场景复用） */
  confirmDialog: ConfirmDialogState;
  setConfirmDialog: Dispatch<SetStateAction<ConfirmDialogState>>;
  aiOn: boolean;
  featureOn: (feature: string) => boolean;
  llmConfigured: boolean;
  formatTime: (ts: number) => string;
  /** 概览统计计数：L1 挂载即加载，各功能域刷新后经此回写 */
  setResumeCount: Dispatch<SetStateAction<number>>;
  setQuoteCount: Dispatch<SetStateAction<number>>;
}
