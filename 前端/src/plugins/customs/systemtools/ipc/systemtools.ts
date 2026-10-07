// plugins/customs/systemtools/ipc/systemtools.ts — customs.systemtools L1 IPC 客户端（短码 st，批次4c）。
// 契约：统一经内核 dispatcher，逻辑名 `st:plugin:<旧命令名>`；参数形状与迁移前
// 旧封装逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 10 条 alias：system 10 条命令。
// extension 2 + adapter 3 裁定零消费判删 —— 不在前端 client。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';
import type { SystemConfig } from '@/types';

export const SYSTEMTOOLS_IPC_METHODS = {
  // ===== 系统配置（3）=====
  getInfo: { cmd: 'get_system_config' },
  setConfig: { cmd: 'set_system_config' },
  getAllConfigs: { cmd: 'get_all_system_configs' },
  // ===== 系统操作（3）=====
  openFile: { cmd: 'system_open_file' },
  openUrl: { cmd: 'system_open_url' },
  getAppInfo: { cmd: 'system_get_app_info' },
  // ===== 剪贴板（2）=====
  clipboardWriteText: { cmd: 'clipboard_write_text' },
  clipboardReadText: { cmd: 'clipboard_read_text' },
  // ===== 窗口（2）=====
  setWindowAlwaysOnTop: { cmd: 'window_set_always_on_top' },
  screenshot: { cmd: 'window_screenshot' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherSt = defineIpcNamespace('st', SYSTEMTOOLS_IPC_METHODS);
type StMethod = keyof typeof SYSTEMTOOLS_IPC_METHODS;

function invokeSt<T>(method: StMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherSt[method](args) as Promise<ApiResponse<T>>;
}

/** 参数键与迁移前旧封装逐字一致；方法命名沿用旧 `system.*` 门面。 */
export const systemtools = {
  // === 系统配置（3）===
  getInfo: (key: string) => invokeSt<SystemConfig | null>('getInfo', { key }),
  setConfig: (key: string, value: string) =>
    invokeSt<SystemConfig>('setConfig', { key, value }),
  getAllConfigs: () => invokeSt<SystemConfig[]>('getAllConfigs'),
  // === 系统操作（3）===
  openFile: (path: string) => invokeSt<void>('openFile', { path }),
  openUrl: (url: string) => invokeSt<void>('openUrl', { url }),
  getAppInfo: () =>
    invokeSt<{ name: string; version: string; platform: string; arch: string }>('getAppInfo'),
  // === 剪贴板（2）===
  clipboardWriteText: (text: string) => invokeSt<void>('clipboardWriteText', { text }),
  clipboardReadText: () => invokeSt<string>('clipboardReadText'),
  // === 窗口（2）===
  setWindowAlwaysOnTop: (alwaysOnTop: boolean) =>
    invokeSt<void>('setWindowAlwaysOnTop', { alwaysOnTop }),
  screenshot: () => invokeSt<string>('screenshot'),
};
