// boards.xin L1 壳向各功能域子插件注入的核心共享契约（只读数据 + 回调）。
// 各 L2 插件（xin.chat / memory / mood / wellness / briefing / compaction / dream /
// checkpoint / search / review / skill / tool / evolution）经 core 对象接收跨 tab 共享状态，
// 禁止直接修改 L1 核心 state；需要刷新走此处回调（同板块 L1↔L2 例外，见插件开发规范 §1.4）。
import type { Dispatch, SetStateAction } from 'react';
import type { Persona, ChatMessage, Conversation, CompactionConfig, AiModelOption } from './xin/types';

/** 小欣各功能域共享的核心契约（L1 壳持有，L2 只读消费 + 经回调写回） */
export interface XinCore {
  /** 人格：当前激活人格 + 全量人格 + 切换 */
  activePersona: Persona;
  personas: Persona[];
  switchPersona: (p: Persona) => void;
  /** 云端模型：可用列表 + 选中项 + 选中名 */
  availableModels: AiModelOption[];
  selectedModel: AiModelOption | null;
  selectedModelName: string;
  setSelectedModelName: (v: string) => void;
  /** 会话：列表 + 当前会话 + 增删切换 */
  conversations: Conversation[];
  activeConversationId: string | null;
  setActiveConversationId: (v: string | null) => void;
  loadConversations: () => void;
  createConversation: () => void;
  switchConversation: (id: string) => void;
  deleteConversation: (id: string) => void;
  /** 消息与 token 统计 */
  messages: ChatMessage[];
  setMessages: Dispatch<SetStateAction<ChatMessage[]>>;
  tokenStats: { current: number; limit: number; ratio: number };
  setTokenStats: Dispatch<SetStateAction<{ current: number; limit: number; ratio: number }>>;
  /** 上下文压缩 */
  needsCompaction: boolean;
  triggerCompactionAuto: () => void;
  checkCompactionNeed: () => void;
  compactionConfig: CompactionConfig | null;
  updateCompactionConfig: (p: Partial<CompactionConfig>) => void;
  /** 网络状态与心情 */
  isOnline: boolean;
  mood: any;
  setMood: (m: any) => void;
  /** 实时语音会话 */
  realtimeSession: ReturnType<typeof import('./features/realtime/useRealtimeSession').useRealtimeSession>;
  toggleRealtime: () => void;
  /** 板块内 tab 跳转（setActiveTab + navigate） */
  goToTab: (tab: string) => void;
}
