import { t } from "i18next";
import { create } from 'zustand';
import type { ChatMessage, ChatSession, GroupChatOrchestration } from '@/types';
interface ChatState {
  sessions: ChatSession[];
  activeSessionId: number | null;
  isLoading: boolean;
  isSending: boolean;
  messages: Map<number, ChatMessage[]>;
  orchestration: Map<number, GroupChatOrchestration>;
  createSession: (session: Omit<ChatSession, 'id' | 'created_at' | 'updated_at'>) => number;
  deleteSession: (id: number) => void;
  setActiveSession: (id: number | null) => void;
  updateSession: (id: number, updates: Partial<ChatSession>) => void;
  sendMessage: (sessionId: number, content: string) => Promise<void>;
  addMessage: (sessionId: number, message: Omit<ChatMessage, 'id' | 'timestamp'>) => void;
  clearMessages: (sessionId: number) => void;
  startOrchestration: (sessionId: number, config: {
    max_rounds: number;
    total_token_budget: number;
  }) => void;
  updateOrchestration: (sessionId: number, updates: Partial<GroupChatOrchestration>) => void;
  endOrchestration: (sessionId: number, summary: string, summarizer: string) => void;
  getOrchestration: (sessionId: number) => GroupChatOrchestration | undefined;
  setLoading: (loading: boolean) => void;
  setSending: (sending: boolean) => void;
}
export const useChatStore = create<ChatState>()((set, get) => ({
  sessions: [],
  activeSessionId: null,
  isLoading: false,
  isSending: false,
  messages: new Map(),
  orchestration: new Map(),
  createSession: sessionData => {
    const id = Date.now();
    const now = new Date().toISOString();
    const session: ChatSession = {
      ...sessionData,
      id,
      created_at: now,
      updated_at: now
    };
    set(state => ({
      sessions: [...state.sessions, session],
      activeSessionId: id
    }));
    return id;
  },
  deleteSession: id => set(state => ({
    sessions: state.sessions.filter(s => s.id !== id),
    activeSessionId: state.activeSessionId === id ? null : state.activeSessionId,
    orchestration: (() => {
      const newMap = new Map(state.orchestration);
      newMap.delete(id);
      return newMap;
    })()
  })),
  setActiveSession: id => set({
    activeSessionId: id
  }),
  updateSession: (id, updates) => set(state => ({
    sessions: state.sessions.map(s => s.id === id ? {
      ...s,
      ...updates,
      updated_at: new Date().toISOString()
    } : s)
  })),
  sendMessage: async (sessionId, content) => {
    const {
      addMessage,
      setSending
    } = get();
    addMessage(sessionId, {
      session_id: sessionId,
      role: 'user',
      content
    });
    setSending(true);
    try {
      setTimeout(() => {
        addMessage(sessionId, {
          session_id: sessionId,
          role: 'assistant',
          content: t("stores.chatStore.k1", {
            content: content
          }),
          model: 'GPT-4'
        });
        setSending(false);
      }, 1000);
    } catch (error) {
      console.error('发送消息失败:', error);
      setSending(false);
    }
  },
  addMessage: (sessionId, messageData) => {
    const id = Date.now();
    const message: ChatMessage = {
      ...messageData,
      id,
      timestamp: new Date().toISOString()
    };
    set(state => {
      const newMessages = new Map(state.messages);
      const sessionMessages = newMessages.get(sessionId) || [];
      newMessages.set(sessionId, [...sessionMessages, message]);
      return {
        messages: newMessages,
        sessions: state.sessions.map(s => {
          if (s.id !== sessionId) return s;
          const last_message = message.content.slice(0, 50) + (message.content.length > 50 ? '...' : '');
          return {
            ...s,
            last_message: last_message,
            last_message_at: message.timestamp,
            updated_at: new Date().toISOString()
          };
        })
      };
    });
  },
  clearMessages: sessionId => set(state => {
    const newMessages = new Map(state.messages);
    newMessages.set(sessionId, []);
    return {
      messages: newMessages
    };
  }),
  startOrchestration: (sessionId, config) => {
    const orchestration: GroupChatOrchestration = {
      session_id: sessionId,
      current_round: 0,
      max_rounds: config.max_rounds,
      total_token_budget: config.total_token_budget,
      used_tokens: 0,
      status: 'discussing',
      timeout_models: [],
      started_at: new Date().toISOString()
    };
    set(state => {
      const newMap = new Map(state.orchestration);
      newMap.set(sessionId, orchestration);
      return {
        orchestration: newMap
      };
    });
  },
  updateOrchestration: (sessionId, updates) => set(state => {
    const newMap = new Map(state.orchestration);
    const current = newMap.get(sessionId);
    if (current) {
      newMap.set(sessionId, {
        ...current,
        ...updates
      });
    }
    return {
      orchestration: newMap
    };
  }),
  endOrchestration: (sessionId, summary, summarizer) => set(state => {
    const newMap = new Map(state.orchestration);
    const current = newMap.get(sessionId);
    if (current) {
      newMap.set(sessionId, {
        ...current,
        status: 'completed',
        summary,
        summarizer,
        ended_at: new Date().toISOString()
      });
    }
    return {
      orchestration: newMap
    };
  }),
  getOrchestration: sessionId => get().orchestration.get(sessionId),
  setLoading: loading => set({
    isLoading: loading
  }),
  setSending: sending => set({
    isSending: sending
  })
}));