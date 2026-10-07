// plugins/boards/terminal/ipc.ts — boards.terminal L1 IPC 客户端（短码 tm，批次3a）。
// 契约：统一经内核 dispatcher，逻辑名 `tm:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
// 封装层口径（2a-1 裁定 35-A 同源）：「有消费才迁」——仅收编前端实际消费的 10 条方法；
// 其余 19 条（SSH 整族 / mux / history 系 / config 系等，裁定 T7 判「留 + alias」）
// 有后端 alias 无前端方法，待后续消费时补录。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const TERMINAL_IPC_METHODS = {
  createSession: { cmd: 'terminal_create_session' },
  createWslSession: { cmd: 'terminal_create_wsl_session' },
  detectWsl: { cmd: 'terminal_detect_wsl' },
  writeInput: { cmd: 'terminal_write_input' },
  executeBuiltin: { cmd: 'terminal_execute_builtin' },
  resize: { cmd: 'terminal_resize' },
  killSession: { cmd: 'terminal_kill_session' },
  saveLayout: { cmd: 'terminal_save_layout' },
  loadLayout: { cmd: 'terminal_load_layout' },
  getThemes: { cmd: 'terminal_get_themes' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherTerminal = defineIpcNamespace('tm', TERMINAL_IPC_METHODS);
type TerminalMethod = keyof typeof TERMINAL_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeTerminal<T = any>(method: TerminalMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherTerminal[method](args) as Promise<ApiResponse<T>>;
}

/** 方法名与参数键与迁移前旧封装（lib/ipc/terminal.ts）逐字一致；
 *  sessionType 放宽为 string（后端 session_type 实际支持 terminal/cmd/powershell/wsl）。 */
export const terminal = {
  createSession: (type: string) => invokeTerminal<string>('createSession', { session_type: type }),
  createWslSession: (distro?: string, shell?: string) =>
    invokeTerminal<string>('createWslSession', { distro, shell }),
  detectWsl: () => invokeTerminal<any>('detectWsl'),
  /** 旧封装名 executeCommand（命令 terminal_write_input） */
  executeCommand: (sessionId: string, command: string) =>
    invokeTerminal('writeInput', { session_id: sessionId, input: command }),
  executeBuiltin: (command: string) => invokeTerminal<any>('executeBuiltin', { command }),
  resize: (sessionId: string, rows: number, cols: number) =>
    invokeTerminal('resize', { session_id: sessionId, rows, cols }),
  closeSession: (sessionId: string) => invokeTerminal<number>('killSession', { session_id: sessionId }),
  saveLayout: (tabs: unknown) => invokeTerminal('saveLayout', { tabs }),
  loadLayout: () => invokeTerminal<unknown[]>('loadLayout'),
  getThemes: () => invokeTerminal<any[]>('getThemes'),
};
