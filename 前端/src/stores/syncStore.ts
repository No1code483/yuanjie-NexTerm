// syncStore.ts — A5 离线与同步机制前端状态管理（Phase 2-4）
//
// 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.6
//
// 职责：
// - 维护同步队列状态（pending/syncing/synced/failed/conflict 计数）
// - 维护已注册设备列表
// - 维护冲突记录列表
// - 提供 sync IPC 调用封装（refresh / runOnce / resolveConflict / registerDevice / revokeDevice）
//
// 注：本 store 不维护密钥状态——ECDH 私钥需在内存中保持（每次启动重新生成），
//     由调用方（设备注册流程）持有，本 store 仅传递公钥。

import { create } from 'zustand';
import { sync as syncIpc } from '@/lib/ipc';
import type {
  SyncQueueStats,
  SyncDevice,
  SyncQueueItem,
  SyncRunResult,
} from '@/lib/ipc';

export interface SyncState {
  // 状态
  stats: SyncQueueStats | null;
  devices: SyncDevice[];
  conflicts: SyncQueueItem[];
  isRunning: boolean;
  lastRun: SyncRunResult | null;
  lastError: string | null;

  // 派生状态
  hasConflicts: () => boolean;
  hasPending: () => boolean;
  currentDevice: () => SyncDevice | null;

  // 操作
  refreshStatus: () => Promise<void>;
  refreshDevices: () => Promise<void>;
  refreshConflicts: () => Promise<void>;
  runOnce: () => Promise<SyncRunResult | null>;
  resolveConflict: (
    id: number,
    resolution: 'local' | 'remote' | 'merged',
    resolvedPayload?: string,
  ) => Promise<boolean>;
  registerCurrentDevice: (
    id: string,
    deviceName: string,
    // 设备形态（desktop/laptop/phone/tablet/server）——BUG-018 起拆分
    deviceType: string,
    // 操作系统（windows/macos/linux/other）——BUG-018 新增
    deviceOs: string,
    publicKey: string,
  ) => Promise<boolean>;
  revokeDevice: (deviceId: string) => Promise<boolean>;
}

export const useSyncStore = create<SyncState>((set, get) => ({
  stats: null,
  devices: [],
  conflicts: [],
  isRunning: false,
  lastRun: null,
  lastError: null,

  hasConflicts: () => (get().stats?.conflict ?? 0) > 0,
  hasPending: () => (get().stats?.pending ?? 0) > 0,
  currentDevice: () => get().devices.find((d) => d.is_current_device === 1) ?? null,

  refreshStatus: async () => {
    const res = await syncIpc.getStatus();
    if (res.code === 0 && res.data) {
      set({ stats: res.data });
    } else {
      set({ lastError: res.message });
    }
  },

  refreshDevices: async () => {
    const res = await syncIpc.listDevices();
    if (res.code === 0 && res.data) {
      set({ devices: res.data });
    } else {
      set({ lastError: res.message });
    }
  },

  refreshConflicts: async () => {
    const res = await syncIpc.listConflicts();
    if (res.code === 0 && res.data) {
      set({ conflicts: res.data });
    } else {
      set({ lastError: res.message });
    }
  },

  runOnce: async () => {
    set({ isRunning: true, lastError: null });
    try {
      const res = await syncIpc.runOnce();
      if (res.code === 0 && res.data) {
        set({ lastRun: res.data, isRunning: false });
        // 同步后刷新状态
        await Promise.all([get().refreshStatus(), get().refreshConflicts()]);
        return res.data;
      }
      set({ isRunning: false, lastError: res.message });
      return null;
    } catch (e) {
      set({
        isRunning: false,
        lastError: e instanceof Error ? e.message : String(e),
      });
      return null;
    }
  },

  resolveConflict: async (id, resolution, resolvedPayload) => {
    const res = await syncIpc.resolveConflict(id, resolution, resolvedPayload);
    if (res.code === 0) {
      await Promise.all([get().refreshStatus(), get().refreshConflicts()]);
      return true;
    }
    set({ lastError: res.message });
    return false;
  },

  registerCurrentDevice: async (id, deviceName, deviceType, deviceOs, publicKey) => {
    const res = await syncIpc.registerDevice(id, deviceName, deviceType, deviceOs, publicKey, true);
    if (res.code === 0) {
      await get().refreshDevices();
      return true;
    }
    set({ lastError: res.message });
    return false;
  },

  revokeDevice: async (deviceId) => {
    const res = await syncIpc.unregisterDevice(deviceId);
    if (res.code === 0) {
      await get().refreshDevices();
      return true;
    }
    set({ lastError: res.message });
    return false;
  },
}));
