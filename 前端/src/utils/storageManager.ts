import { t } from "i18next";
// 存储管理系统

// 存储类型定义
export enum StorageType {
  LOCAL = 'local',
  // 本地存储
  SESSION = 'session',
  // 会话存储
  INDEXED_DB = 'indexed_db',
  // IndexedDB
  FILE_SYSTEM = 'file_system',
  // 文件系统
  REMOTE = 'remote' // 远程存储
}

// 存储配置接口
export interface StorageConfig {
  type: StorageType;
  name?: string;
  version?: number;
  size?: number;
  encrypt?: boolean;
  compress?: boolean;
  backup?: boolean;
}

// 存储项接口
export interface StorageItem<T = any> {
  key: string;
  value: T;
  type: string;
  size: number;
  createdAt: number;
  updatedAt: number;
  expiresAt?: number;
  tags?: string[];
  metadata?: Record<string, any>;
}

// 存储统计接口
export interface StorageStats {
  totalItems: number;
  totalSize: number;
  usedSize: number;
  freeSize: number;
  itemTypes: Record<string, number>;
}

// 存储查询选项
export interface StorageQueryOptions {
  prefix?: string;
  tags?: string[];
  type?: string;
  limit?: number;
  offset?: number;
  sortBy?: 'key' | 'createdAt' | 'updatedAt' | 'size';
  sortOrder?: 'asc' | 'desc';
  includeExpired?: boolean;
}

// 存储管理器类
export class StorageManager {
  private config: StorageConfig;
  private storages: Map<StorageType, any> = new Map();
  private listeners: Map<string, Function[]> = new Map();
  constructor(config: StorageConfig) {
    this.config = {
      ...config
    };
    this.initializeStorages();
  }

  // 初始化存储系统
  private async initializeStorages(): Promise<void> {
    try {
      // 初始化本地存储
      if (this.config.type === StorageType.LOCAL || this.config.type === StorageType.SESSION) {
        await this.initializeWebStorage();
      }

      // 初始化IndexedDB
      if (this.config.type === StorageType.INDEXED_DB) {
        await this.initializeIndexedDB();
      }

      // 初始化文件系统存储
      if (this.config.type === StorageType.FILE_SYSTEM) {
        await this.initializeFileSystem();
      }
      console.log('存储系统初始化完成');
    } catch (error) {
      console.error('存储系统初始化失败:', error);
    }
  }

  // 初始化Web存储（localStorage/sessionStorage）
  private async initializeWebStorage(): Promise<void> {
    const storage = this.config.type === StorageType.LOCAL ? localStorage : sessionStorage;
    this.storages.set(this.config.type, storage);
  }

  // 初始化IndexedDB
  private async initializeIndexedDB(): Promise<void> {
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.config.name || 'nexterm_db', this.config.version || 1);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        this.storages.set(StorageType.INDEXED_DB, request.result);
        resolve();
      };
      request.onupgradeneeded = event => {
        const db = (event.target as IDBOpenDBRequest).result;

        // 创建对象存储
        if (!db.objectStoreNames.contains('items')) {
          const store = db.createObjectStore('items', {
            keyPath: 'key'
          });
          store.createIndex('type', 'type', {
            unique: false
          });
          store.createIndex('createdAt', 'createdAt', {
            unique: false
          });
          store.createIndex('updatedAt', 'updatedAt', {
            unique: false
          });
          store.createIndex('expiresAt', 'expiresAt', {
            unique: false
          });
          store.createIndex('tags', 'tags', {
            unique: false,
            multiEntry: true
          });
        }
      };
    });
  }

  // 初始化文件系统存储
  private async initializeFileSystem(): Promise<void> {
    // 文件系统存储需要Tauri环境
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      const {
        fileSystem
      } = await import('../lib/tauri');
      this.storages.set(StorageType.FILE_SYSTEM, fileSystem);
    } else {
      throw new Error(t("utils.storageManager.k1"));
    }
  }

  // 设置存储项
  async set<T>(key: string, value: T, options: {
    type?: string;
    expiresIn?: number;
    tags?: string[];
    metadata?: Record<string, any>;
  } = {}): Promise<void> {
    const now = Date.now();
    const item: StorageItem<T> = {
      key,
      value,
      type: options.type || typeof value,
      size: this.calculateSize(value),
      createdAt: now,
      updatedAt: now,
      expiresAt: options.expiresIn ? now + options.expiresIn : undefined,
      tags: options.tags,
      metadata: options.metadata
    };
    await this.saveItem(item);
    this.notifyChange('set', key, item);
  }

  // 获取存储项
  async get<T>(key: string): Promise<T | null> {
    const item = await this.getItem<T>(key);
    if (!item) {
      return null;
    }

    // 检查是否过期
    if (item.expiresAt && item.expiresAt < Date.now()) {
      await this.remove(key);
      return null;
    }
    return item.value;
  }

  // 移除存储项
  async remove(key: string): Promise<void> {
    await this.deleteItem(key);
    this.notifyChange('remove', key);
  }

  // 检查存储项是否存在
  async has(key: string): Promise<boolean> {
    const item = await this.getItem(key);
    if (!item) return false;

    // 检查是否过期
    if (item.expiresAt && item.expiresAt < Date.now()) {
      await this.remove(key);
      return false;
    }
    return true;
  }

  // 获取所有键
  async keys(): Promise<string[]> {
    return await this.getAllKeys();
  }

  // 清空存储
  async clear(): Promise<void> {
    await this.clearStorage();
    this.notifyChange('clear');
  }

  // 查询存储项
  async query(options: StorageQueryOptions = {}): Promise<StorageItem[]> {
    return await this.queryItems(options);
  }

  // 获取存储统计信息
  async getStats(): Promise<StorageStats> {
    const items = await this.query({
      includeExpired: true
    });
    const totalSize = items.reduce((sum, item) => sum + item.size, 0);
    const itemTypes: Record<string, number> = {};
    items.forEach(item => {
      itemTypes[item.type] = (itemTypes[item.type] || 0) + 1;
    });
    return {
      totalItems: items.length,
      totalSize,
      usedSize: totalSize,
      freeSize: this.config.size ? this.config.size - totalSize : 0,
      itemTypes
    };
  }

  // 备份存储数据
  async backup(): Promise<Blob> {
    const items = await this.query({
      includeExpired: true
    });
    const backupData = {
      version: '1.0.0',
      timestamp: Date.now(),
      config: this.config,
      items
    };
    return new Blob([JSON.stringify(backupData, null, 2)], {
      type: 'application/json'
    });
  }

  // 恢复存储数据
  async restore(backupData: Blob): Promise<void> {
    const text = await backupData.text();
    const data = JSON.parse(text);

    // 清空现有数据
    await this.clear();

    // 恢复数据
    for (const item of data.items) {
      await this.set(item.key, item.value, {
        type: item.type,
        expiresIn: item.expiresAt ? item.expiresAt - item.createdAt : undefined,
        tags: item.tags,
        metadata: item.metadata
      });
    }
    this.notifyChange('restore');
  }

  // 监听存储变化
  on(event: string, callback: Function): void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, []);
    }
    this.listeners.get(event)!.push(callback);
  }

  // 移除监听器
  off(event: string, callback: Function): void {
    const callbacks = this.listeners.get(event);
    if (callbacks) {
      const index = callbacks.indexOf(callback);
      if (index > -1) {
        callbacks.splice(index, 1);
      }
    }
  }

  // 私有方法
  private async saveItem(item: StorageItem): Promise<void> {
    const storage = this.storages.get(this.config.type);
    switch (this.config.type) {
      case StorageType.LOCAL:
      case StorageType.SESSION:
        storage.setItem(item.key, JSON.stringify(item));
        break;
      case StorageType.INDEXED_DB:
        return new Promise((resolve, reject) => {
          const transaction = storage.transaction(['items'], 'readwrite');
          const store = transaction.objectStore('items');
          const request = store.put(item);
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve();
        });
      case StorageType.FILE_SYSTEM:
        await storage.writeTextFile(`data/${item.key}.json`, JSON.stringify(item));
        break;
      default:
        throw new Error(t("utils.storageManager.k2", {
          type: this.config.type
        }));
    }
  }
  private async getItem<T>(key: string): Promise<StorageItem<T> | null> {
    const storage = this.storages.get(this.config.type);
    switch (this.config.type) {
      case StorageType.LOCAL:
      case StorageType.SESSION:
        const itemStr = storage.getItem(key);
        return itemStr ? JSON.parse(itemStr) : null;
      case StorageType.INDEXED_DB:
        return new Promise((resolve, reject) => {
          const transaction = storage.transaction(['items'], 'readonly');
          const store = transaction.objectStore('items');
          const request = store.get(key);
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve(request.result || null);
        });
      case StorageType.FILE_SYSTEM:
        try {
          const content = await storage.readTextFile(`data/${key}.json`);
          return JSON.parse(content);
        } catch {
          return null;
        }
      default:
        throw new Error(t("utils.storageManager.k2", {
          type: this.config.type
        }));
    }
  }
  private async deleteItem(key: string): Promise<void> {
    const storage = this.storages.get(this.config.type);
    switch (this.config.type) {
      case StorageType.LOCAL:
      case StorageType.SESSION:
        storage.removeItem(key);
        break;
      case StorageType.INDEXED_DB:
        return new Promise((resolve, reject) => {
          const transaction = storage.transaction(['items'], 'readwrite');
          const store = transaction.objectStore('items');
          const request = store.delete(key);
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve();
        });
      case StorageType.FILE_SYSTEM:
        await storage.remove(`data/${key}.json`);
        break;
      default:
        throw new Error(t("utils.storageManager.k2", {
          type: this.config.type
        }));
    }
  }
  private async getAllKeys(): Promise<string[]> {
    const storage = this.storages.get(this.config.type);
    switch (this.config.type) {
      case StorageType.LOCAL:
      case StorageType.SESSION:
        return Object.keys(storage);
      case StorageType.INDEXED_DB:
        return new Promise((resolve, reject) => {
          const transaction = storage.transaction(['items'], 'readonly');
          const store = transaction.objectStore('items');
          const request = store.getAllKeys();
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve(request.result.map(String));
        });
      case StorageType.FILE_SYSTEM:
        // 文件系统存储需要特殊处理
        return [];
      default:
        throw new Error(t("utils.storageManager.k2", {
          type: this.config.type
        }));
    }
  }
  private async clearStorage(): Promise<void> {
    const storage = this.storages.get(this.config.type);
    switch (this.config.type) {
      case StorageType.LOCAL:
      case StorageType.SESSION:
        storage.clear();
        break;
      case StorageType.INDEXED_DB:
        return new Promise((resolve, reject) => {
          const transaction = storage.transaction(['items'], 'readwrite');
          const store = transaction.objectStore('items');
          const request = store.clear();
          request.onerror = () => reject(request.error);
          request.onsuccess = () => resolve();
        });
      case StorageType.FILE_SYSTEM:
        // 文件系统存储需要特殊处理
        break;
      default:
        throw new Error(t("utils.storageManager.k2", {
          type: this.config.type
        }));
    }
  }
  private async queryItems(options: StorageQueryOptions): Promise<StorageItem[]> {
    const allKeys = await this.getAllKeys();
    const items: StorageItem[] = [];
    for (const key of allKeys) {
      const item = await this.getItem(key);
      if (item) {
        items.push(item);
      }
    }

    // 过滤过期项
    const now = Date.now();
    let filteredItems = options.includeExpired ? items : items.filter(item => !item.expiresAt || item.expiresAt > now);

    // 应用前缀过滤
    if (options.prefix) {
      filteredItems = filteredItems.filter(item => item.key.startsWith(options.prefix!));
    }

    // 应用标签过滤
    if (options.tags && options.tags.length > 0) {
      filteredItems = filteredItems.filter(item => item.tags && options.tags!.some(tag => item.tags!.includes(tag)));
    }

    // 应用类型过滤
    if (options.type) {
      filteredItems = filteredItems.filter(item => item.type === options.type);
    }

    // 应用排序
    if (options.sortBy) {
      filteredItems.sort((a, b) => {
        const aVal = a[options.sortBy!];
        const bVal = b[options.sortBy!];
        const order = options.sortOrder === 'desc' ? -1 : 1;
        if (aVal < bVal) return -1 * order;
        if (aVal > bVal) return 1 * order;
        return 0;
      });
    }

    // 应用分页
    if (options.limit) {
      const offset = options.offset || 0;
      filteredItems = filteredItems.slice(offset, offset + options.limit);
    }
    return filteredItems;
  }
  private calculateSize(value: any): number {
    return JSON.stringify(value).length;
  }
  private notifyChange(event: string, key?: string, item?: StorageItem): void {
    const callbacks = this.listeners.get(event);
    if (callbacks) {
      callbacks.forEach(callback => callback({
        key,
        item
      }));
    }
  }
}

// 创建默认存储管理器实例
export const defaultStorage = new StorageManager({
  type: StorageType.LOCAL,
  name: 'nexterm',
  encrypt: true
});

// 存储工具函数
export async function createStorage(config: StorageConfig): Promise<StorageManager> {
  return new StorageManager(config);
}
export async function getStorageStats(storage: StorageManager): Promise<StorageStats> {
  return await storage.getStats();
}
export async function exportStorageData(storage: StorageManager): Promise<Blob> {
  return await storage.backup();
}
export async function importStorageData(storage: StorageManager, backupData: Blob): Promise<void> {
  await storage.restore(backupData);
}