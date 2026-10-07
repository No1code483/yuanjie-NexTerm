// plugins/customs/sync/ipc/sync.ts — customs.sync L1 IPC 客户端（短码 sy，批次6b）。
// 契约：统一经内核 dispatcher，逻辑名 `sy:plugin:<旧命令名>`；参数形状与迁移前
// 旧封装逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 21 条 alias：sync_enqueue / sync_fetch_pending / sync_mark_synced / sync_mark_failed /
// sync_get_queue_stats / sync_register_device / sync_list_devices / sync_unregister_device /
// sync_test_transport / sync_run_once / sync_get_status / sync_list_conflicts /
// sync_resolve_conflict / sync_ecdh_generate_keypair / sync_e2ee_encrypt / sync_e2ee_validate /
// sync_get_network_status / sync_check_network_now / sync_record_config_change /
// sync_flush_config_queue / sync_get_pending_config_count
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

// ============================================================================
// 类型定义（对齐后端 sync_service.rs / sync/scheduler.rs / sync/e2ee.rs）
// ============================================================================

export type SyncOperation = 'INSERT' | 'UPDATE' | 'DELETE';

export type SyncStatus = 'pending' | 'syncing' | 'synced' | 'failed' | 'conflict';

export interface SyncQueueItem {
  id: number;
  table_name: string;
  record_id: number;
  operation: string;
  payload: string;
  device_id: string | null;
  sync_status: string;
  retry_count: number;
  last_error: string | null;
  created_at: string;
  synced_at: string | null;
}

export interface SyncQueueStats {
  pending: number;
  syncing: number;
  synced: number;
  failed: number;
  conflict: number;
}

export interface SyncDevice {
  id: string;
  device_name: string;
  device_type: string;
  device_os: string;
  public_key: string;
  registered_at: string;
  last_seen_at: string | null;
  last_sync_at: string | null;
  is_current_device: number;
}

export interface SyncRunResult {
  total_pending: number;
  succeeded: number;
  failed: number;
  conflicts: number;
  retried: number;
  duration_ms: number;
  errors: string[];
}

export interface EncryptedPayload {
  ephemeral_public_key: string;
  nonce: string;
  ciphertext: string;
}

export interface SerializedKeyPair {
  private_key_b64: string;
  public_key_b64: string;
}

export interface NetworkStatus {
  is_online: boolean;
  last_checked: number;
}

export const SYNC_IPC_METHODS = {
  // Phase 1 队列管理
  enqueue: { cmd: 'sync_enqueue' },
  fetchPending: { cmd: 'sync_fetch_pending' },
  markSynced: { cmd: 'sync_mark_synced' },
  markFailed: { cmd: 'sync_mark_failed' },
  getQueueStats: { cmd: 'sync_get_queue_stats' },

  // 设备管理
  registerDevice: { cmd: 'sync_register_device' },
  listDevices: { cmd: 'sync_list_devices' },
  unregisterDevice: { cmd: 'sync_unregister_device' },

  // Phase 2-4 新增
  testTransport: { cmd: 'sync_test_transport' },
  runOnce: { cmd: 'sync_run_once' },
  getStatus: { cmd: 'sync_get_status' },
  listConflicts: { cmd: 'sync_list_conflicts' },
  resolveConflict: { cmd: 'sync_resolve_conflict' },

  // Phase 4 ECDH / E2EE
  ecdhGenerateKeypair: { cmd: 'sync_ecdh_generate_keypair' },
  e2eeEncrypt: { cmd: 'sync_e2ee_encrypt' },
  e2eeValidate: { cmd: 'sync_e2ee_validate' },

  // Phase 3 网络状态检测
  getNetworkStatus: { cmd: 'sync_get_network_status' },
  checkNetworkNow: { cmd: 'sync_check_network_now' },

  // Phase 3 Task 5 配置同步降级
  recordConfigChange: { cmd: 'sync_record_config_change' },
  flushConfigQueue: { cmd: 'sync_flush_config_queue' },
  getPendingConfigCount: { cmd: 'sync_get_pending_config_count' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherSy = defineIpcNamespace('sy', SYNC_IPC_METHODS);
type SyncMethod = keyof typeof SYNC_IPC_METHODS;

function invokeSy<T>(method: SyncMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherSy[method](args) as Promise<ApiResponse<T>>;
}

export const sync = {
  // Phase 1 队列管理
  enqueue: (
    table_name: string,
    record_id: number,
    operation: SyncOperation,
    payload: string,
    device_id?: string,
  ) =>
    invokeSy<number>('enqueue', {
      table_name,
      record_id,
      operation,
      payload,
      device_id: device_id ?? null,
    }),

  fetchPending: (limit: number = 100) =>
    invokeSy<SyncQueueItem[]>('fetchPending', { limit }),

  markSynced: (id: number) => invokeSy('markSynced', { id }),

  markFailed: (id: number, error_msg: string) =>
    invokeSy('markFailed', { id, error_msg }),

  getQueueStats: () => invokeSy<SyncQueueStats>('getQueueStats'),

  // 设备管理
  registerDevice: (
    id: string,
    device_name: string,
    device_type: string,
    device_os: string,
    public_key: string,
    is_current: boolean,
  ) =>
    invokeSy('registerDevice', {
      id,
      device_name,
      device_type,
      device_os,
      public_key,
      is_current,
    }),

  listDevices: () => invokeSy<SyncDevice[]>('listDevices'),

  unregisterDevice: (device_id: string) =>
    invokeSy('unregisterDevice', { device_id }),

  // Phase 2-4 新增
  testTransport: (backend_type: 'webdav' | 's3', config: Record<string, unknown>) =>
    invokeSy<boolean>('testTransport', {
      backend_type,
      config_json: JSON.stringify(config),
    }),

  runOnce: () => invokeSy<SyncRunResult>('runOnce'),

  getStatus: () => invokeSy<SyncQueueStats>('getStatus'),

  listConflicts: () => invokeSy<SyncQueueItem[]>('listConflicts'),

  resolveConflict: (
    id: number,
    resolution: 'local' | 'remote' | 'merged',
    resolved_payload?: string,
  ) =>
    invokeSy('resolveConflict', {
      id,
      resolution,
      resolved_payload: resolved_payload ?? null,
    }),

  // Phase 4 ECDH / E2EE
  ecdhGenerateKeypair: () =>
    invokeSy<SerializedKeyPair>('ecdhGenerateKeypair'),

  e2eeEncrypt: (payload: string, peer_public_key_b64: string) =>
    invokeSy<EncryptedPayload>('e2eeEncrypt', {
      payload,
      peer_public_key_b64,
    }),

  e2eeValidate: (encrypted: EncryptedPayload) =>
    invokeSy<boolean>('e2eeValidate', { encrypted }),

  // Phase 3 网络状态检测
  getNetworkStatus: () =>
    invokeSy<NetworkStatus>('getNetworkStatus'),

  checkNetworkNow: () =>
    invokeSy<NetworkStatus>('checkNetworkNow'),

  // Phase 3 Task 5 配置同步降级
  recordConfigChange: (config_key: string, config_value: string) =>
    invokeSy<void>('recordConfigChange', {
      config_key,
      config_value,
    }),

  flushConfigQueue: () =>
    invokeSy<number>('flushConfigQueue'),

  getPendingConfigCount: () =>
    invokeSy<number>('getPendingConfigCount'),
};
