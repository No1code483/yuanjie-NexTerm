// ipc/ai.ts — ai 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';

export const ai = {
  getModels: () => ipc.invoke<any[]>('get_ai_models'),
  createModel: (model: any) => ipc.invoke('add_ai_model', {
    request: model
  }),
  updateModel: (model: any) => ipc.invoke('update_ai_model', {
    request: model
  }),
  deleteModel: (id: number) => ipc.invoke('delete_ai_model', {
    id
  }),
  getAgents: () => ipc.invoke<any[]>('get_ai_agents'),
  createAgent: (agent: any) => ipc.invoke('add_ai_agent', {
    request: agent
  }),
  deleteAgent: (id: number) => ipc.invoke('delete_ai_agent', {
    id
  }),
  createConversation: (request: any) => ipc.invoke('create_conversation', {
    request
  }),
  getSessions: () => ipc.invoke<any[]>('get_conversations'),
  getMessages: (conversationId: number) => ipc.invoke<any[]>('get_messages', {
    conversationId
  }),
  getParticipants: (conversationId: number) => ipc.invoke<any[]>('get_participants', {
    conversationId
  }),
  sendMessage: (sessionId: number, content: string) => ipc.invoke('send_message', {
    request: {
      conversation_id: sessionId,
      content
    }
  }),
  startGroupChat: (conversationId: number, userMessage: string, config?: any) => ipc.invoke('run_orchestrator', {
    conversationId,
    userMessage,
    ...config
  }),
  getOrchestrationStatus: (sessionId: number) => ipc.invoke('ai_get_orchestration_status', {
    conversationId: sessionId
  }),
  endGroupChat: (sessionId: number) => ipc.invoke('ai_end_group_chat', {
    conversationId: sessionId
  }),
  deleteConversation: (id: number) => ipc.invoke('delete_conversation', {
    id
  }),
  deleteMessage: (id: number) => ipc.invoke('delete_message', {
    id
  }),
  searchConversations: (query: string) => ipc.invoke<any[]>('search_conversations', {
    request: {
      query
    }
  }),
  toggleStar: (id: number) => ipc.invoke<boolean>('toggle_star_conversation', {
    id
  }),
  branchConversation: (conversationId: number, messageId: number) => ipc.invoke<any>('branch_conversation', {
    request: {
      conversation_id: conversationId,
      message_id: messageId
    }
  }),
  exportConversation: (conversationId: number, format: string) => ipc.invoke<string>('export_conversation', {
    request: {
      conversation_id: conversationId,
      format
    }
  }),
  getPromptTemplates: () => ipc.invoke<any[]>('prompt_template_list'),
  createPromptTemplate: (title: string, category: string, content: string) => ipc.invoke<any>('prompt_template_create', {
    request: {
      title,
      category,
      content
    }
  }),
  updatePromptTemplate: (id: number, title?: string, category?: string, content?: string) => ipc.invoke('prompt_template_update', {
    request: {
      id,
      title,
      category,
      content
    }
  }),
  deletePromptTemplate: (id: number) => ipc.invoke('prompt_template_delete', {
    id
  }),
  // spec ai-chat-enhancement Phase 1 Task 5: 模型健康检测
  // 参数名采用 camelCase，Tauri 2.x 默认转换为后端 snake_case（model_id）
  // 事件监听由 Task 6 在 AI.tsx 中实现（ai-model-health-changed）
  checkModelHealth: (modelId: number) => ipc.invoke<any>('check_model_health', {
    modelId
  }),
  checkAllModelsHealth: () => ipc.invoke<any[]>('check_all_models_health'),
  getModelHealthStatus: (modelId: number) => ipc.invoke<any>('get_model_health_status', {
    modelId
  }),
  setHealthCheckInterval: (minutes: number) => ipc.invoke('set_health_check_interval', {
    minutes
  }),
  // spec ai-chat-enhancement Phase 2 Task 9/12: 会话列表增强 IPC
  // 参数名 camelCase，Tauri 2.x 自动转 snake_case（conversation_id / items）
  markConversationRead: (conversationId: number) => ipc.invoke('mark_conversation_read', {
    conversationId
  }),
  reorderConversations: (items: Array<{
    id: number;
    sort_order: number;
  }>) => ipc.invoke('reorder_conversations', {
    items
  }),
  // Task 11 重命名需要：复用后端 update_conversation 命令（UpdateConversationRequest { id, title, token_budget }）
  updateConversation: (request: {
    id: number;
    title?: string;
    token_budget?: number;
  }) => ipc.invoke('update_conversation', {
    request
  })
};
