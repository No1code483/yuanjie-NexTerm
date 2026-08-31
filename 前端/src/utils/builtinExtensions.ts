import { t } from "i18next";
import { NexTermAdapter, BUILTIN_EXTENSION_TYPES } from '../types/extensions';

/**
 * 内置扩展示例
 */

export const builtinExtensions: NexTermAdapter[] = [
// 系统工具扩展
{
  id: 'system-monitor',
  name: t("utils.builtinExtensions.k1"),
  type: BUILTIN_EXTENSION_TYPES.SYSTEM,
  version: '1.0.0',
  description: t("utils.builtinExtensions.k2"),
  author: 'NexTerm Team',
  icon: 'icon-monitor',
  category: t("components.AddExtensionModal.k15"),
  tags: [t("utils.builtinExtensions.k3"), t("utils.builtinExtensions.k4"), t("components.GroupChatOrchestrationPanel.k2")],
  data: {
    cpuUsage: 0,
    memoryUsage: 0,
    diskUsage: 0,
    networkUsage: 0
  },
  config: {
    autoStart: true,
    enabled: true,
    settings: {
      refreshInterval: 2000,
      showNotifications: true
    }
  },
  onMount: async () => {
    console.log('系统监控扩展已挂载');
    // 这里可以启动监控任务
  },
  onUnmount: async () => {
    console.log('系统监控扩展已卸载');
    // 这里可以停止监控任务
  },
  onMessage: async msg => {
    console.log('收到系统监控消息:', msg);
  }
},
// 文件管理扩展
{
  id: 'file-explorer',
  name: t("utils.builtinExtensions.k5"),
  type: BUILTIN_EXTENSION_TYPES.UTILITY,
  version: '1.0.0',
  description: t("utils.builtinExtensions.k6"),
  author: 'NexTerm Team',
  icon: 'icon-folder',
  category: t("utils.builtinExtensions.k7"),
  tags: [t("knowledge.GraphView.k1"), t("utils.builtinExtensions.k8"), t("utils.builtinExtensions.k9")],
  data: {
    currentPath: '/',
    fileList: [],
    selectedFiles: []
  },
  config: {
    autoStart: false,
    enabled: true,
    settings: {
      defaultPath: '/',
      showHiddenFiles: false
    }
  },
  onMount: async () => {
    console.log('文件管理器扩展已挂载');
  },
  onUnmount: async () => {
    console.log('文件管理器扩展已卸载');
  },
  onMessage: async msg => {
    console.log('收到文件管理器消息:', msg);
  }
},
// 网络工具扩展
{
  id: 'network-tools',
  name: t("utils.builtinExtensions.k10"),
  type: BUILTIN_EXTENSION_TYPES.TOOL,
  version: '1.0.0',
  description: t("utils.builtinExtensions.k11"),
  author: 'NexTerm Team',
  icon: 'icon-network',
  category: t("utils.builtinExtensions.k10"),
  tags: [t("linux.types.k10"), t("utils.builtinExtensions.k12"), t("utils.builtinExtensions.k3")],
  data: {
    pingResults: [],
    portScanResults: [],
    networkStatus: 'online'
  },
  config: {
    autoStart: false,
    enabled: true,
    settings: {
      pingTimeout: 5000,
      scanPortRange: '1-1000'
    }
  },
  onMount: async () => {
    console.log('网络工具扩展已挂载');
  },
  onUnmount: async () => {
    console.log('网络工具扩展已卸载');
  },
  onMessage: async msg => {
    console.log('收到网络工具消息:', msg);
  }
},
// 开发工具扩展
{
  id: 'developer-tools',
  name: t("components.AddExtensionModal.k10"),
  type: BUILTIN_EXTENSION_TYPES.TOOL,
  version: '1.0.0',
  description: t("utils.builtinExtensions.k13"),
  author: 'NexTerm Team',
  icon: 'icon-code',
  category: t("components.AddExtensionModal.k10"),
  tags: [t("utils.builtinExtensions.k14"), t("utils.builtinExtensions.k15"), t("layout.k27")],
  data: {
    codeSnippets: [],
    apiDocs: [],
    debugTools: []
  },
  config: {
    autoStart: false,
    enabled: true,
    settings: {
      autoFormat: true,
      lintOnSave: true
    }
  },
  onMount: async () => {
    console.log('开发工具扩展已挂载');
  },
  onUnmount: async () => {
    console.log('开发工具扩展已卸载');
  },
  onMessage: async msg => {
    console.log('收到开发工具消息:', msg);
  }
},
// 占位扩展（用于展示）
{
  id: 'placeholder-extension',
  name: t("layout.k31"),
  type: BUILTIN_EXTENSION_TYPES.UTILITY,
  version: '1.0.0',
  description: t("utils.builtinExtensions.k16"),
  author: 'NexTerm Team',
  icon: 'icon-placeholder',
  category: t("utils.builtinExtensions.k17"),
  tags: [t("utils.builtinExtensions.k18"), t("utils.builtinExtensions.k17")],
  data: {
    message: t("components.NexTermPlaceholder.k1")
  },
  config: {
    autoStart: false,
    enabled: true
  },
  onMount: async () => {
    console.log('占位扩展已挂载');
  },
  onUnmount: async () => {
    console.log('占位扩展已卸载');
  },
  onMessage: async msg => {
    console.log('收到占位扩展消息:', msg);
  }
}];

/**
 * 注册所有内置扩展
 */
export async function registerBuiltinExtensions(manager: any): Promise<void> {
  for (const extension of builtinExtensions) {
    try {
      const existing = manager.get(extension.id);
      if (existing) {
        console.log(`内置扩展已存在，跳过: ${extension.name}`);
        continue;
      }
      await manager.register(extension);
      console.log(`内置扩展注册成功: ${extension.name}`);
    } catch (error) {
      console.error(`内置扩展注册失败: ${extension.name}`, error);
    }
  }
}

/**
 * 根据类型筛选扩展
 */
export function getExtensionsByType(type: string): NexTermAdapter[] {
  return builtinExtensions.filter(ext => ext.type === type);
}

/**
 * 根据分类筛选扩展
 */
export function getExtensionsByCategory(category: string): NexTermAdapter[] {
  return builtinExtensions.filter(ext => ext.category === category);
}

/**
 * 搜索扩展
 */
export function searchExtensions(query: string): NexTermAdapter[] {
  const lowerQuery = query.toLowerCase();
  return builtinExtensions.filter(ext => ext.name.toLowerCase().includes(lowerQuery) || ext.description.toLowerCase().includes(lowerQuery) || ext.tags?.some(tag => tag.toLowerCase().includes(lowerQuery)));
}