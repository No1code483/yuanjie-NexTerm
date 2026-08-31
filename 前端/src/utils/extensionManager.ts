import { t } from "i18next";
import { NexTermAdapter, ExtensionManager, ExtensionRegistry, ExtensionConfig, EXTENSION_EVENTS } from '../types/extensions';

/**
 * 扩展管理器实现
 */
export class ExtensionManagerImpl implements ExtensionManager {
  private registry: ExtensionRegistry = {};
  private mountedExtensions: Set<string> = new Set();
  private eventListeners: Map<string, Function[]> = new Map();
  async register(adapter: NexTermAdapter): Promise<void> {
    if (this.registry[adapter.id]) {
      throw new Error(t("utils.extensionManager.k1", {
        id: adapter.id
      }));
    }

    // 验证扩展格式
    this.validateAdapter(adapter);

    // 注册扩展
    this.registry[adapter.id] = adapter;

    // 触发注册事件
    this.emit('extensionRegistered', adapter);
    console.log(`扩展注册成功: ${adapter.name} (${adapter.id})`);
  }
  async unregister(id: string): Promise<void> {
    const adapter = this.registry[id];
    if (!adapter) {
      throw new Error(t("utils.extensionManager.k2", {
        id: id
      }));
    }

    // 如果已挂载，先卸载
    if (this.mountedExtensions.has(id)) {
      await this.unmount(id);
    }

    // 从注册表中移除
    delete this.registry[id];

    // 触发注销事件
    this.emit('extensionUnregistered', id);
    console.log(`扩展注销成功: ${id}`);
  }
  get(id: string): NexTermAdapter | undefined {
    return this.registry[id];
  }
  getAll(): NexTermAdapter[] {
    return Object.values(this.registry);
  }
  async mount(id: string): Promise<void> {
    const adapter = this.get(id);
    if (!adapter) {
      throw new Error(t("utils.extensionManager.k2", {
        id: id
      }));
    }
    if (this.mountedExtensions.has(id)) {
      console.warn(`扩展 ${id} 已挂载`);
      return;
    }
    try {
      // 执行挂载钩子
      if (adapter.onMount) {
        await adapter.onMount();
      }
      this.mountedExtensions.add(id);
      this.emit('extensionMounted', id);
      console.log(`扩展挂载成功: ${adapter.name}`);
    } catch (error) {
      console.error(`扩展挂载失败: ${adapter.name}`, error);
      throw error;
    }
  }
  async unmount(id: string): Promise<void> {
    const adapter = this.get(id);
    if (!adapter) {
      throw new Error(t("utils.extensionManager.k2", {
        id: id
      }));
    }
    if (!this.mountedExtensions.has(id)) {
      console.warn(`扩展 ${id} 未挂载`);
      return;
    }
    try {
      // 执行卸载钩子
      if (adapter.onUnmount) {
        await adapter.onUnmount();
      }
      this.mountedExtensions.delete(id);
      this.emit('extensionUnmounted', id);
      console.log(`扩展卸载成功: ${adapter.name}`);
    } catch (error) {
      console.error(`扩展卸载失败: ${adapter.name}`, error);
      throw error;
    }
  }
  async sendMessage(id: string, message: any): Promise<void> {
    const adapter = this.get(id);
    if (!adapter) {
      throw new Error(t("utils.extensionManager.k2", {
        id: id
      }));
    }
    if (!this.mountedExtensions.has(id)) {
      throw new Error(t("utils.extensionManager.k3", {
        id: id
      }));
    }
    if (adapter.onMessage) {
      await adapter.onMessage(message);
    }
  }
  async updateConfig(id: string, config: Partial<ExtensionConfig>): Promise<void> {
    const adapter = this.get(id);
    if (!adapter) {
      throw new Error(t("utils.extensionManager.k2", {
        id: id
      }));
    }

    // 更新配置
    adapter.config = {
      ...adapter.config,
      ...config
    };

    // 触发配置变更事件
    this.emit(EXTENSION_EVENTS.EXTENSION_CONFIG_CHANGED, {
      id,
      config: adapter.config
    });
  }

  // 事件系统
  on(event: string, listener: Function): void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, []);
    }
    this.eventListeners.get(event)!.push(listener);
  }
  off(event: string, listener: Function): void {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      const index = listeners.indexOf(listener);
      if (index > -1) {
        listeners.splice(index, 1);
      }
    }
  }
  private emit(event: string, data?: any): void {
    const listeners = this.eventListeners.get(event);
    if (listeners) {
      listeners.forEach(listener => {
        try {
          listener(data);
        } catch (error) {
          console.error(`事件监听器执行失败: ${event}`, error);
        }
      });
    }
  }
  private validateAdapter(adapter: NexTermAdapter): void {
    const required = ['id', 'name', 'type', 'version', 'description', 'data'];
    const missing = required.filter(field => !adapter[field as keyof NexTermAdapter]);
    if (missing.length > 0) {
      throw new Error(t("utils.extensionManager.k4", {
        arg0: missing.join(', ')
      }));
    }

    // 验证ID格式
    if (!/^[a-zA-Z0-9_-]+$/.test(adapter.id)) {
      throw new Error(t("utils.extensionManager.k5"));
    }

    // 验证版本格式
    if (!/^\d+\.\d+\.\d+$/.test(adapter.version)) {
      throw new Error(t("utils.extensionManager.k6"));
    }
  }

  // 获取已挂载的扩展列表
  getMountedExtensions(): string[] {
    return Array.from(this.mountedExtensions);
  }

  // 检查扩展是否挂载
  isMounted(id: string): boolean {
    return this.mountedExtensions.has(id);
  }

  // 批量挂载扩展
  async mountAll(): Promise<void> {
    const extensions = this.getAll();
    for (const extension of extensions) {
      if (extension.config?.autoStart) {
        try {
          await this.mount(extension.id);
        } catch (error) {
          console.error(`自动挂载扩展失败: ${extension.name}`, error);
        }
      }
    }
  }

  // 批量卸载扩展
  async unmountAll(): Promise<void> {
    const mountedIds = this.getMountedExtensions();
    for (const id of mountedIds) {
      try {
        await this.unmount(id);
      } catch (error) {
        console.error(`卸载扩展失败: ${id}`, error);
      }
    }
  }
}

// 创建全局扩展管理器实例
export const extensionManager = new ExtensionManagerImpl();