// ipc/sync.ts — A5 离线与同步机制 IPC 封装（Phase 2-4）
// 命令名与后端 src/commands/sync_commands.rs 一致
import { ipc } from './core';

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

// Phase 3 网络状态检测
// 对齐后端 sync_get_network_status / sync_check_network_now 返回的 data
export interface NetworkStatus {
  is_online: boolean;
  last_checked: number; // 毫秒时间戳（与 JS Date.now() 一致）
}

// ============================================================================
// IPC 命令封装
// ============================================================================

export const sync = {
  // Phase 1 队列管理
  enqueue: (
    table_name: string,
    record_id: number,
    operation: SyncOperation,
    payload: string,
    device_id?: string,
  ) =>
    ipc.invoke<number>('sync_enqueue', {
      table_name,
      record_id,
      operation,
      payload,
      device_id: device_id ?? null,
    }),

  fetchPending: (limit: number = 100) =>
    ipc.invoke<SyncQueueItem[]>('sync_fetch_pending', { limit }),

  markSynced: (id: number) => ipc.invoke('sync_mark_synced', { id }),

  markFailed: (id: number, error_msg: string) =>
    ipc.invoke('sync_mark_failed', { id, error_msg }),

  getQueueStats: () => ipc.invoke<SyncQueueStats>('sync_get_queue_stats'),

  // 设备管理
  registerDevice: (
    id: string,
    device_name: string,
    device_type: string,
    public_key: string,
    is_current: boolean,
  ) =>
    ipc.invoke('sync_register_device', {
      id,
      device_name,
      device_type,
      public_key,
      is_current,
    }),

  listDevices: () => ipc.invoke<SyncDevice[]>('sync_list_devices'),

  unregisterDevice: (device_id: string) =>
    ipc.invoke('sync_unregister_device', { device_id }),

  // Phase 2-4 新增
  testTransport: (backend_type: 'webdav' | 's3', config: Record<string, unknown>) =>
    ipc.invoke<boolean>('sync_test_transport', {
      backend_type,
      config_json: JSON.stringify(config),
    }),

  runOnce: () => ipc.invoke<SyncRunResult>('sync_run_once'),

  getStatus: () => ipc.invoke<SyncQueueStats>('sync_get_status'),

  listConflicts: () => ipc.invoke<SyncQueueItem[]>('sync_list_conflicts'),

  resolveConflict: (
    id: number,
    resolution: 'local' | 'remote' | 'merged',
    resolved_payload?: string,
  ) =>
    ipc.invoke('sync_resolve_conflict', {
      id,
      resolution,
      resolved_payload: resolved_payload ?? null,
    }),

  // Phase 4 ECDH / E2EE
  ecdhGenerateKeypair: () =>
    ipc.invoke<SerializedKeyPair>('sync_ecdh_generate_keypair'),

  e2eeEncrypt: (payload: string, peer_public_key_b64: string) =>
    ipc.invoke<EncryptedPayload>('sync_e2ee_encrypt', {
      payload,
      peer_public_key_b64,
    }),

  e2eeValidate: (encrypted: EncryptedPayload) =>
    ipc.invoke<boolean>('sync_e2ee_validate', { encrypted }),

  // Phase 3 网络状态检测
  getNetworkStatus: () =>
    ipc.invoke<NetworkStatus>('sync_get_network_status'),

  checkNetworkNow: () =>
    ipc.invoke<NetworkStatus>('sync_check_network_now'),

  // Phase 3 Task 5 配置同步降级
  /** 记录配置变更到 sync_queue（配置本地立即生效由前端处理） */
  recordConfigChange: (config_key: string, config_value: string) =>
    ipc.invoke<void>('sync_record_config_change', {
      config_key,
      config_value,
    }),

  /** 网络恢复时 flush 配置队列（将 failed 重置为 pending），返回受影响行数 */
  flushConfigQueue: () =>
    ipc.invoke<number>('sync_flush_config_queue'),

  /** 获取待发配置条数（pending + failed） */
  getPendingConfigCount: () =>
    ipc.invoke<number>('sync_get_pending_config_count'),
};
