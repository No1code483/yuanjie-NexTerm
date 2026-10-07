// plugins/boards/home/features/todo/ipc.ts — home.todo 插件 IPC 客户端（短码 td）。
// 契约：统一经内核 dispatcher，逻辑名 `td:plugin:<旧命令名>`；参数形状与迁移前
// 旧 Tauri 命令逐字一致（06_Rust代码契约 §8.1 / 后端 commands.rs 的 dispatch_legacy）。
import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const TODO_IPC_METHODS = {
  getTodos: { cmd: 'get_todos' },
  getTodosPaginated: { cmd: 'get_todos_paginated' },
  addTodo: { cmd: 'add_todo' },
  toggleTodo: { cmd: 'toggle_todo' },
  updateTodo: { cmd: 'update_todo' },
  deleteTodo: { cmd: 'delete_todo' },
} satisfies Record<string, IpcMethodSpec>;

const dispatcherTodo = defineIpcNamespace('td', TODO_IPC_METHODS);
type TodoMethod = keyof typeof TODO_IPC_METHODS;

// 未标注泛型时与旧 `ipc.invoke<T = any>` 的推断一致
function invokeTodo<T = any>(method: TodoMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherTodo[method](args) as Promise<ApiResponse<T>>;
}

export const todo = {
  getTodos: (date: string) => invokeTodo<any[]>('getTodos', { date }),
  // 零消费命令（前端无调用点），参数键沿用旧 Tauri 命令的 camelCase 绑定
  getTodosPaginated: (page?: number, pageSize?: number) =>
    invokeTodo('getTodosPaginated', { page: page ?? null, pageSize: pageSize ?? null }),
  addTodo: (title: string, date: string) => invokeTodo('addTodo', { request: { title, date } }),
  toggleTodo: (id: number) => invokeTodo('toggleTodo', { id }),
  updateTodo: (request: {
    id: number;
    title: string;
    description?: string;
    priority: string;
    due_date?: string;
    version: number;
  }) => invokeTodo('updateTodo', { request }),
  deleteTodo: (id: number) => invokeTodo('deleteTodo', { id }),
};
