/**
 * 自定义主题 IPC 封装（C1.5 / v1.51.7）
 *
 * 封装 5 个后端 Tauri 命令，提供类型安全的调用接口。
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.5
 */

import { invoke } from '@tauri-apps/api/core';

/** 后端统一响应 */
interface ApiResponse<T> {
  code: number;
  message: string;
  data?: T;
}

/** 自定义主题记录（对应后端 CustomTheme 结构体） */
export interface CustomTheme {
  id: number;
  name: string;
  base_theme: string;
  variables: string; // JSON 字符串：{ "--theme-bg-primary": "#xxx", ... }
  created_at: number;
  updated_at: number;
}

/** UPSERT 请求体 */
export interface UpsertCustomThemeRequest {
  name: string;
  base_theme?: string;
  variables: string; // JSON 字符串
}

/**
 * 调用后端命令的通用封装，提取 data 并抛错
 */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const res = await invoke<ApiResponse<T>>(cmd, args);
  if (res.code !== 0) {
    throw new Error(res.message || `命令 ${cmd} 调用失败`);
  }
  if (res.data === undefined || res.data === null) {
    throw new Error(`命令 ${cmd} 返回空数据`);
  }
  return res.data;
}

/**
 * 列出所有自定义主题
 * @returns 主题数组（按 updated_at 倒序）
 */
export async function customThemeList(): Promise<CustomTheme[]> {
  return call<CustomTheme[]>('custom_theme_list');
}

/**
 * 按 id 获取单个自定义主题
 * @returns 主题对象，若不存在返回 null
 */
export async function customThemeGet(id: number): Promise<CustomTheme | null> {
  return call<CustomTheme | null>('custom_theme_get', { id });
}

/**
 * 新增或更新自定义主题（UPSERT，按 name 唯一约束）
 * @param request 名称 + 可选基础主题 + variables JSON 字符串
 * @returns 写入后的完整记录（含 id）
 */
export async function customThemeUpsert(request: UpsertCustomThemeRequest): Promise<CustomTheme> {
  return call<CustomTheme>('custom_theme_upsert', { request });
}

/**
 * 按 id 删除自定义主题
 * @returns 受影响行数（0 表示未找到）
 */
export async function customThemeDelete(id: number): Promise<number> {
  return call<number>('custom_theme_delete', { id });
}

/**
 * 批量从 localStorage 迁移到 DB
 * @param themesJson 旧 localStorage 中的 JSON 字符串（数组）
 * @returns 成功迁移的主题数量
 */
export async function customThemeMigrateLocal(themesJson: string): Promise<number> {
  return call<number>('custom_theme_migrate_local', { themesJson });
}
