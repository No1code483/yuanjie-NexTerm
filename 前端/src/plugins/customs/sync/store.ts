// store.ts — customs.sync L1 前端插件入口（批次6b）。
// 同步状态管理 store，迁入自 src/stores/syncStore.ts（D6 收编）。
import { create } from 'zustand';
import { sync as syncIpc } from '@/lib/ipc';
import type {
  SyncQueueStats,
  SyncDevice,
  SyncQueueItem,
  SyncRunResult,
} from '@/lib/ipc';

export interface SyncState {
  stats: SyncQueueStats | null;
  devices: SyncDevice[];
  conflicts: SyncQueueItem[];
  isRunning: boolean;
  lastRun: SyncRunResult | null;
  lastError: string | null;

  hasConflicts: () => boolean;
  hasPending: () => boolean;
  currentDevice: () => SyncDevice | null;

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
    deviceType: string,
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
