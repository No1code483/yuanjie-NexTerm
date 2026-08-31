// ipc/system.ts — system 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';

export const recycleBin = {
  getItems: () => ipc.invoke<any[]>('recycle_list'),
  moveToRecycle: (itemType: string, itemIds: number[]) => ipc.invoke('recycle_move_to', {
    itemType,
    itemIds
  }),
  restore: (ids: number[]) => ipc.invoke('recycle_restore', {
    ids
  }),
  permanentDelete: (ids: number[]) => ipc.invoke('recycle_delete_permanently', {
    ids
  }),
  cleanupExpired: () => ipc.invoke('cleanup_expired_recycle'),
  empty: () => ipc.invoke('recycle_empty_all'),
  stats: () => ipc.invoke<any>('recycle_stats')
};

export const newsSource = {
  getNewsSources: () => ipc.invoke<any[]>('get_news_sources'),
  addNewsSource: (name: string, url: string, category: string, feedType: string) => ipc.invoke('add_news_source', {
    name,
    url,
    category,
    feed_type: feedType
  }),
  deleteNewsSource: (id: number) => ipc.invoke('delete_news_source', {
    id
  })
};

export const system = {
  getInfo: () => ipc.invoke<any>('get_system_config'),
  setConfig: (key: string, value: string) => ipc.invoke('set_system_config', {
    key,
    value
  }),
  getAllConfigs: () => ipc.invoke<any[]>('get_all_system_configs'),
  openFile: (path: string) => ipc.invoke('system_open_file', {
    path
  }),
  openUrl: (url: string) => ipc.invoke('system_open_url', {
    url
  }),
  getAppInfo: () => ipc.invoke<any>('system_get_app_info')
};

export const extension = {
  getEntry: (moduleName: string) => ipc.invoke<any>('extension_get_entry', {
    module_name: moduleName
  }),
  listModules: () => ipc.invoke<any[]>('extension_list_modules')
};
