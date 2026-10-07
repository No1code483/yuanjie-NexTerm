import { invoke } from '@tauri-apps/api/core';
import type { IpcMethodSpec } from '../registry/definePlugin';

/** 统一传输命令（B7 落地决策：内核 Tauri 插件唯一命令，cmd 字段承载契约逻辑名） */
export const KERNEL_DISPATCH = 'plugin:kernel|kernel_dispatch';

/** 命名空间 IPC 客户端定义（插件 ipc/ 目录使用，契约：07_TypeScript代码契约.md §八）
 *  ⭐ B7 落地决策：统一经内核 kernel_dispatch 分发——
 *  传输层唯一命令 'plugin:kernel|kernel_dispatch'，cmd 字段承载契约逻辑名 `${ns}:plugin:${spec.cmd}`；
 *  后端入口在 check_ipc 前将 cmd 规范化为 `${ns}_${spec.cmd}` 做白名单匹配（见 06 契约 §十一） */
export function defineIpcNamespace(ns: string, methods: Record<string, IpcMethodSpec>) {
  const client: Record<string, (args?: unknown) => Promise<unknown>> = {};
  for (const [key, spec] of Object.entries(methods)) {
    client[key] = (args) => invoke(KERNEL_DISPATCH, { cmd: `${ns}:plugin:${spec.cmd}`, args: args ?? {} });
  }
  return client;
}
