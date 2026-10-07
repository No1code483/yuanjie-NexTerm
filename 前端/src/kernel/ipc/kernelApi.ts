/** 内核保留命令客户端（契约：阶段1 F7；后端：tauri_glue.rs 三命令）
 *  owner == "kernel" 时可信直查、免 check_ipc */
import { invoke } from '@tauri-apps/api/core';
import { KERNEL_DISPATCH } from './namespace';
import type { PluginLevel } from '../types';

/** 与 crates/kernel/src/tauri_glue.rs PluginInfo 严格对齐（serde camelCase）——
 *  任何一侧改字段，两侧同改 */
export interface KernelPluginInfo {
  id: string;
  name: string;
  level: PluginLevel;
  version: string;
  parent?: string;
  slot?: string;
  /** "registered"/"enabled"/"error"/...（PluginState::as_str） */
  state: string;
}

/** 与 crates/kernel/src/plugin.rs StartupReportItem 严格对齐 */
export interface KernelStartupReportItem {
  plugin: string;
  /** 状态字符串（"enabled"/"error"/...） */
  state: string;
  note?: string;
}

/** kernel:plugin:get_enabled -> Vec<PluginInfo> */
export function getEnabledPlugins(): Promise<KernelPluginInfo[]> {
  return invoke(KERNEL_DISPATCH, { cmd: 'kernel:plugin:get_enabled', args: {} });
}

/** kernel:plugin:set_enabled -> (id, enabled) -> null */
export function setEnabled(id: string, enabled: boolean): Promise<null> {
  return invoke(KERNEL_DISPATCH, { cmd: 'kernel:plugin:set_enabled', args: { id, enabled } });
}

/** kernel:startup_report -> Vec<StartupReportItem> */
export function getStartupReport(): Promise<KernelStartupReportItem[]> {
  return invoke(KERNEL_DISPATCH, { cmd: 'kernel:startup_report', args: {} });
}

/** 内核 API 客户端（内核级 UI 如 PluginManagerPage 使用） */
export const kernelApi = { getEnabledPlugins, setEnabled, getStartupReport };
