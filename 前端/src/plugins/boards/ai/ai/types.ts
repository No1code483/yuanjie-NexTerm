export interface BackendModel {
  id: number
  name: string
  provider: string
  api_endpoint: string | null
  model_name: string
  model_type: string
  // spec ai-chat-enhancement Phase 1 Task 5.1: 5 种健康检测状态
  // 兼容旧值 'active'/'inactive'（保留 | string 以兼容历史数据）
  status: 'online' | 'offline' | 'checking' | 'error' | 'unknown' | string;
  has_api_key: boolean
  created_at: number
  updated_at: number
  // spec ai-chat-enhancement Phase 1 Task 5.1: 健康检测元数据
  latency_ms: number | null
  last_health_check: number | null
}

export interface BackendConversation {
  id: number
  title: string | null
  type: string
  is_temp: boolean
  token_budget: number
  starred: boolean | null
  created_at: number
  updated_at: number
  // spec ai-chat-enhancement Phase 2 Task 9.1: 列表预览/未读数/排序字段
  // 对齐后端 ConversationWithPreview（migration 0115 新增 unread_count / sort_order，
  // 联表查最后一条消息得 last_message_*）
  last_message_preview?: string | null
  last_message_sender_type?: string | null
  last_message_at?: number | null
  unread_count?: number
  sort_order?: number
}

export interface BackendMessage {
  id: number
  conversation_id: number
  sender_type: string
  sender_id: number | null
  content: string
  round: number
  created_at: number
}

export interface NewModelForm {
  name: string
  provider: string
  api_url: string
  api_key: string
  api_format: string
  model_name: string
  display_name: string
  is_local: boolean
  multimodal: boolean
  system_prompt: string
  context_window: number
  temperature: number
}

export interface NewConversationForm {
  title: string
  model_id: number
  model_ids: number[]
}

export interface BackendAgent {
  id: number
  name: string
  description: string | null
  system_prompt: string | null
  model_id: number | null
  created_at: number
  updated_at: number
}

export interface AgentForm {
  name: string
  description: string
  system_prompt: string
  model_id: number
}

export interface GroupConversationForm {
  title: string
  model_ids: number[]
}

export interface Participant {
  id: number
  conversation_id: number
  model_id: number | null
  agent_id: number | null
  role: string
}

export interface GroupProgress {
  round: number
  maxRounds: number
  responded: string[]
  totalParticipants: number
  status: 'idle' | 'round_start' | 'responding' | 'converged' | 'stopped'
}

export interface ContextMenuState {
  visible: boolean
  x: number
  y: number
  messageId: number | null
}

export type AIModel = BackendModel
export type Conversation = BackendConversation
export type Message = BackendMessage