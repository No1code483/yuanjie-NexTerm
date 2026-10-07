// boards.xin L1 壳 + 核心数据层（小欣「一切皆插件」拆分）。
// 功能域已物理拆分至 features/<dir>/ 各 L2 插件（chat/memory/mood/wellness/briefing/compaction/
// dream/checkpoint/search/review/skill/tool/evolution），本文件仅保留：路由页骨架、tab 解析（?tab=）、
// 子插件启停门控、核心共享状态与数据加载，以及按 displayTab 分发各功能域渲染（停用回退 chat）。
import { t } from "i18next";
import { useState, useEffect, useMemo } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import { xin, xinOrchestration } from './ipc';
import { aiModels } from '@/plugins/boards/ai/features/models/ipc';
import { random } from '@/lib/utils';
import { useRealtimeSession } from './features/realtime/useRealtimeSession';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import styles from './Xin.module.css';
import type { Persona, ChatMessage, Conversation, CompactionConfig, AiModelOption } from './xin/types';
import { BUILTIN_PERSONAS } from './xin/types';
import type { XinCore } from './core';
import { useChat } from './features/chat/useChat';
import { useMemory } from './features/memory/useMemory';
import { useMood } from './features/mood/useMood';
import { useWellness } from './features/wellness/useWellness';
import { useBriefing } from './features/briefing/useBriefing';
import { useCompaction } from './features/compaction/useCompaction';
import { useDream } from './features/dream/useDream';
import { useCheckpoint } from './features/checkpoint/useCheckpoint';
import { useSearch } from './features/search/useSearch';
import { useReview } from './features/review/useReview';
import { useSkill } from './features/skill/useSkill';
import { useTool } from './features/tool/useTool';
import { useEvolution } from './features/evolution/useEvolution';

/** tab → 对应子插件 id（用于启停门控：停用时回退 chat） */
const TAB_PLUGIN_ID: Record<string, string> = {
  chat: 'xin.chat',
  memory: 'xin.memory',
  mood: 'xin.mood',
  productivity: 'xin.wellness',
  briefing: 'xin.briefing',
  compaction: 'xin.compaction',
  dream: 'xin.dream',
  checkpoint: 'xin.checkpoint',
  search: 'xin.search',
  review: 'xin.review',
  skill: 'xin.skill',
  tool: 'xin.tool',
  evolution: 'xin.evolution'
};

export default function Xin() {
  const [searchParams] = useSearchParams();
  const tabParam = searchParams.get('tab') || 'chat';
  const [activeTab, setActiveTab] = useState(tabParam);
  const navigate = useNavigate();
  useEffect(() => {
    const paramTab = searchParams.get('tab') || 'chat';
    setActiveTab(paramTab);
  }, [searchParams]);

  const [activePersona, setActivePersona] = useState<Persona>(BUILTIN_PERSONAS[0]);
  const [personas, setPersonas] = useState<Persona[]>(BUILTIN_PERSONAS);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConversationId, setActiveConversationId] = useState<string | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [tokenStats, setTokenStats] = useState<{
    current: number;
    limit: number;
    ratio: number;
  }>({
    current: 0,
    limit: 128000,
    ratio: 0
  });
  const [compactionConfig, setCompactionConfig] = useState<CompactionConfig | null>(null);
  const [needsCompaction, setNeedsCompaction] = useState(false);

  // 从 AI会话板块加载的模型列表
  const [availableModels, setAvailableModels] = useState<AiModelOption[]>([]);
  const [selectedModelName, setSelectedModelName] = useState<string>('');
  // 从 AI会话模型列表中查找选中的模型名
  const selectedModel = useMemo(() => {
    return availableModels.find(m => m.name === selectedModelName) || availableModels[0] || null;
  }, [availableModels, selectedModelName]);

  const [mood, setMood] = useState<any>(null);

  // D3.6 实时对话会话（Web Audio API 采集 PCM → 后端流式 STT/LLM/TTS）
  const realtimeSession = useRealtimeSession();

  // A5 Phase 3 Task 2: 云端 AI 离线降级（小欣走云端 API，离线时禁用发送）
  const { isOnline } = useNetworkStatus();

  // 主动提醒
  const [toasts, setToasts] = useState<{
    id: string;
    text: string;
    icon: string;
  }[]>([]);

  useEffect(() => {
    const timer = setInterval(() => {
      const now = new Date();
      const icons = ['✦', '◈', '◇', '◆'];
      const icon = icons[Math.floor(now.getSeconds() / 15)];
      document.title = t("Xin.k1", {
        icon: icon
      });
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  // 主动提醒
  useEffect(() => {
    const addToast = (id: string, text: string, icon: string) => {
      setToasts(prev => {
        if (prev.some(t => t.id === id)) return prev;
        return [...prev.slice(-4), {
          id,
          text,
          icon
        }];
      });
      setTimeout(() => setToasts(prev => prev.filter(t => t.id !== id)), 8000);
    };
    const checkGreetings = () => {
      const now = new Date();
      const hour = now.getHours();
      const today = now.toDateString();
      const lastDate = localStorage.getItem('xin_greet_date');
      if (lastDate !== today) {
        if (hour >= 8 && hour < 10) {
          addToast(`greet-${today}`, t("Xin.k2"), '☀️');
          localStorage.setItem('xin_greet_date', today);
        }
      }

      // 晚间问候
      if (hour >= 19 && hour < 22) {
        const lastEvening = localStorage.getItem('xin_evening_date');
        if (lastEvening !== today) {
          addToast(`evening-${today}`, t("Xin.k3"), '🌙');
          localStorage.setItem('xin_evening_date', today);
        }
      }
    };
    const checkBreak = () => {
      const lastBreak = localStorage.getItem('xin_last_break');
      const now = Date.now();
      if (!lastBreak || now - parseInt(lastBreak) > 2 * 60 * 60 * 1000) {
        addToast(`break-${now}`, t("Xin.k4"), '☕');
        localStorage.setItem('xin_last_break', now.toString());
      }
    };
    setTimeout(checkGreetings, 3000);
    setTimeout(checkBreak, 5000);
    const greetTimer = setInterval(checkGreetings, 60 * 1000);
    const breakTimer = setInterval(checkBreak, 10 * 60 * 1000);
    return () => {
      clearInterval(greetTimer);
      clearInterval(breakTimer);
    };
  }, []);
  useEffect(() => {
    loadPersonas();
    loadConversations();
    loadCompactionConfig();
    loadAvailableModels();
  }, []);
  useEffect(() => {
    if (activeConversationId) {
      checkCompactionNeed();
    }
  }, [activeConversationId, messages.length]);

  // 子插件启停门控（参考 Home.tsx focusEnabled 写法；null 为加载中，放行）
  const [enabledIds, setEnabledIds] = useState<string[] | null>(null);
  useEffect(() => {
    invoke<Array<{ id: string }>>('plugin:kernel|kernel_dispatch', {
      cmd: 'kernel:plugin:get_enabled',
      args: {}
    }).then((list) => {
      setEnabledIds(list.map((p) => p.id));
    }).catch(() => setEnabledIds(null));
  }, []);
  // 对应子插件被停用时回退到 chat（默认必备）
  const displayTab = enabledIds !== null && TAB_PLUGIN_ID[activeTab] && !enabledIds.includes(TAB_PLUGIN_ID[activeTab]) ? 'chat' : activeTab;

  // 从 AI会话板块加载可用模型列表
  const loadAvailableModels = async () => {
    try {
      const res = await aiModels.getModels();
      if (res?.data && Array.isArray(res.data)) {
        const models: AiModelOption[] = res.data.map((m: any) => ({
          id: m.id,
          name: m.name,
          provider: m.provider,
          model_name: m.model_name
        }));
        setAvailableModels(models);
        // 自动选中第一个模型
        if (models.length > 0 && !selectedModelName) {
          setSelectedModelName(models[0].name);
        }
      }
    } catch {/* 后端不可用，保留空列表 */}
  };
  const loadPersonas = async () => {
    try {
      const res = await xin.getPersonas();
      if (res?.data && Array.isArray(res.data) && res.data.length > 0) {
        const mapped: Persona[] = res.data.map((p: any) => ({
          id: p.id,
          name: p.name,
          description: p.description || '',
          emoji: p.avatar_emoji || '🤖',
          traits: (p.traits || []).map((t: any) => ({
            name: t.name,
            value: t.value
          })),
          style: p.speaking_style || {
            formality: 0.5,
            verbosity: 0.5,
            humor: 0.3,
            technical_depth: 0.5,
            empathy: 0.5
          }
        }));
        setPersonas(mapped);
        try {
          const activeRes = await xin.getActivePersona();
          if (activeRes?.data) {
            const found = mapped.find(p => p.id === activeRes.data.id);
            if (found) setActivePersona(found);
          }
        } catch {/* fallback */}
      }
    } catch {/* keep builtin */}
  };
  const switchPersona = async (persona: Persona) => {
    setActivePersona(persona);
    try {
      await xin.setActivePersona(persona.id);
    } catch {/* silent */}
  };
  const loadConversations = async () => {
    try {
      const res = await xinOrchestration.dialogueList(50, 0);
      if (res?.data) {
        setConversations(res.data);
      }
    } catch {/* silent */}
  };
  const createConversation = async () => {
    try {
      const res = await xinOrchestration.dialogueCreate({ personaId: activePersona.id });
      if (res?.data) {
        const conv: Conversation = res.data;
        setConversations(prev => [conv, ...prev]);
        setActiveConversationId(conv.id);
        setMessages([]);
        setTokenStats({
          current: 0,
          limit: 128000,
          ratio: 0
        });
      }
    } catch {/* silent */}
  };
  const switchConversation = async (convId: string) => {
    setActiveConversationId(convId);
    setMessages([]);
    try {
      const res = await xinOrchestration.dialogueGet(convId);
      if (res?.data) {
        const ctx = res.data.context_json ? JSON.parse(res.data.context_json) : null;
        if (ctx?.messages) {
          const msgs: ChatMessage[] = ctx.messages.map((m: any) => ({
            id: random.uid(),
            role: m.role === 'assistant' ? 'xin' : 'user',
            content: m.content,
            timestamp: m.timestamp || new Date().toISOString()
          }));
          setMessages(msgs);
        }
        setTokenStats({
          current: res.data.total_tokens || 0,
          limit: 128000,
          ratio: (res.data.total_tokens || 0) / 128000
        });
      }
    } catch {/* silent */}
  };
  const deleteConversation = async (convId: string) => {
    try {
      await xinOrchestration.dialogueDelete(convId);
      setConversations(prev => prev.filter(c => c.id !== convId));
      if (activeConversationId === convId) {
        setActiveConversationId(null);
        setMessages([]);
      }
    } catch {/* silent */}
  };
  const loadCompactionConfig = async () => {
    try {
      const res = await xinOrchestration.compactionGetConfig();
      if (res?.data) setCompactionConfig(res.data);
    } catch {/* silent */}
  };
  const checkCompactionNeed = async () => {
    if (!activeConversationId) return;
    try {
      const contextJson = JSON.stringify({
        conversation_id: activeConversationId,
        messages: messages.map(m => ({
          id: m.id,
          role: m.role === 'user' ? 'user' : 'assistant',
          content: m.content,
        })),
      });
      const res = await xinOrchestration.compactionNeedsCheck(contextJson);
      if (res?.data?.needs_compaction) setNeedsCompaction(true);else setNeedsCompaction(false);
    } catch {/* silent */}
  };
  const updateCompactionConfig = async (partial: Partial<CompactionConfig>) => {
    try {
      await xinOrchestration.compactionUpdateConfig({
        ...compactionConfig,
        ...partial
      });
      setCompactionConfig(prev => prev ? {
        ...prev,
        ...partial
      } : prev);
    } catch {/* silent */}
  };
  const triggerCompactionAuto = async () => {
    if (!activeConversationId) return;
    try {
      const contextJson = JSON.stringify(messages.map(m => ({
        id: m.id,
        role: m.role === 'user' ? 'user' : 'assistant',
        content: m.content,
      })));
      await xinOrchestration.compactionAuto(activeConversationId, contextJson, String(selectedModel?.id ?? ''));
      setNeedsCompaction(false);
      loadConversations();
    } catch {/* silent */}
  };
  const toggleRealtime = async () => {
    if (realtimeSession.isActive) {
      await realtimeSession.stop();
    } else {
      if (!selectedModel?.id) {
        console.warn('[realtime] 未选择模型，无法启动实时对话');
        return;
      }
      await realtimeSession.start({
        stt_model_id: selectedModel.id,
        llm_model_id: selectedModel.id,
        tts_model_id: null,
        persona_id: activePersona.id,
        vad_enabled: true
      });
    }
  };
  const goToTab = (tab: string) => {
    setActiveTab(tab);
    navigate(`/xin?tab=${tab}`, {
      replace: true
    });
  };

  // ===== 核心共享契约（注入各功能域插件） =====
  const core: XinCore = {
    activePersona,
    personas,
    switchPersona,
    availableModels,
    selectedModel,
    selectedModelName,
    setSelectedModelName,
    conversations,
    activeConversationId,
    setActiveConversationId,
    loadConversations,
    createConversation,
    switchConversation,
    deleteConversation,
    messages,
    setMessages,
    tokenStats,
    setTokenStats,
    needsCompaction,
    triggerCompactionAuto,
    checkCompactionNeed,
    compactionConfig,
    updateCompactionConfig,
    isOnline,
    mood,
    setMood,
    realtimeSession,
    toggleRealtime,
    goToTab
  };

  // ===== 功能域 hook =====
  const chat = useChat(core, displayTab === 'chat');
  const memory = useMemory(core, displayTab === 'memory');
  const moodTab = useMood(core, displayTab === 'mood');
  const wellness = useWellness(core, displayTab === 'productivity');
  const briefing = useBriefing(core, displayTab === 'briefing');
  const compaction = useCompaction(core, displayTab === 'compaction');
  const dream = useDream(core, displayTab === 'dream');
  const checkpoint = useCheckpoint(core, displayTab === 'checkpoint');
  const search = useSearch(core, displayTab === 'search');
  const review = useReview(core, displayTab === 'review');
  const skill = useSkill(core, displayTab === 'skill');
  const tool = useTool(core, displayTab === 'tool');
  const evolution = useEvolution(core, displayTab === 'evolution');

  return <div className={styles.page}>
      <div className={styles.header}>
        <h2 className={styles.title}>{t("Xin.k21")}</h2>
      </div>

      {/* Toast 通知 */}
      <div className={styles.toastContainer}>
        {toasts.map(t => <div key={t.id} className={styles.toastItem}>
            <span>{t.icon}</span>
            <span>{t.text}</span>
          </div>)}
      </div>

      <div className={styles.content}>
        {displayTab === 'chat' && chat.renderChat()}
        {displayTab === 'memory' && memory.renderMemory()}
        {displayTab === 'mood' && moodTab.renderMood()}
        {displayTab === 'productivity' && wellness.renderWellness()}
        {displayTab === 'briefing' && briefing.renderBriefing()}
        {displayTab === 'compaction' && compaction.renderCompaction()}
        {displayTab === 'dream' && dream.renderDream()}
        {displayTab === 'checkpoint' && checkpoint.renderCheckpoint()}
        {displayTab === 'search' && search.renderSearch()}
        {displayTab === 'review' && review.renderReview()}
        {displayTab === 'skill' && skill.renderSkill()}
        {displayTab === 'tool' && tool.renderTool()}
        {displayTab === 'evolution' && evolution.renderEvolution()}
      </div>
    </div>;
}
