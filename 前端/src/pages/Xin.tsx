import { t } from "i18next";
import { useState, useEffect, useRef, useMemo } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { xin, ai, type XinAttachment } from '@/lib/ipc';
import { time, random } from '@/lib/utils';
import { PERSONA_KNOWLEDGE } from '@/lib/xinChatEngine';
import { useRealtimeSession } from '@/hooks/useRealtimeSession';
import { useNetworkStatus } from '@/hooks/useNetworkStatus';
import styles from './Xin.module.css';
import type { Persona, ChatMessage, Conversation, Memory, MoodEntry, Reminder, Habit, CompactionRecord, CompactionConfig, DreamConfig, DreamResult, DreamState, CheckpointSummary, SearchResult, ConversationReview, TopicTrend, GrowthTrajectory, HeatmapData, SkillInfo, SkillResult, ToolDef, ToolCall, ToolExecResult, FusionResult, AiModelOption } from './xin/types';
import { BUILTIN_PERSONAS, MOOD_EMOJI, CATEGORY_LABELS } from './xin/types';
import { getMockMemories, getMockMoodTimeline, getMockBriefing } from './xin/mockData';
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
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  // D3.2 TTS：正在合成 / 正在播放的消息 id
  const [ttsLoadingId, setTtsLoadingId] = useState<string | null>(null);
  const [ttsPlayingId, setTtsPlayingId] = useState<string | null>(null);
  const ttsAudioRef = useRef<HTMLAudioElement | null>(null);
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
  const [compactionRecords, setCompactionRecords] = useState<CompactionRecord[]>([]);
  const [needsCompaction, setNeedsCompaction] = useState(false);

  // 从 AI会话板块加载的模型列表
  const [availableModels, setAvailableModels] = useState<AiModelOption[]>([]);
  const [selectedModelName, setSelectedModelName] = useState<string>('');
  // 从 AI会话模型列表中查找选中的模型名
  const selectedModel = useMemo(() => {
    return availableModels.find(m => m.name === selectedModelName) || availableModels[0] || null;
  }, [availableModels, selectedModelName]);
  const [dreamConfig, setDreamConfig] = useState<DreamConfig | null>(null);
  const [dreamState, setDreamState] = useState<DreamState | null>(null);
  const [dreamResult, setDreamResult] = useState<DreamResult | null>(null);
  const [dreamLoading, setDreamLoading] = useState(false);
  const [checkpoints, setCheckpoints] = useState<CheckpointSummary[]>([]);
  const [checkpointLoading, setCheckpointLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [searchTotal, setSearchTotal] = useState(0);
  const [searchLoading, setSearchLoading] = useState(false);
  const [review, setReview] = useState<ConversationReview | null>(null);
  const [reviewPeriod, setReviewPeriod] = useState('week');
  const [topicTrends, setTopicTrends] = useState<TopicTrend[]>([]);
  const [growthTrajectory, setGrowthTrajectory] = useState<GrowthTrajectory | null>(null);
  const [heatmap, setHeatmap] = useState<HeatmapData | null>(null);
  const [reviewLoading, setReviewLoading] = useState(false);
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [skillInput, setSkillInput] = useState('');
  const [skillResult, setSkillResult] = useState<SkillResult | null>(null);
  const [skillExecLoading, setSkillExecLoading] = useState(false);
  const [tools, setTools] = useState<ToolDef[]>([]);
  const [toolInputText, setToolInputText] = useState('');
  const [parsedCalls, setParsedCalls] = useState<ToolCall[]>([]);
  const [toolResults, setToolResults] = useState<ToolExecResult[]>([]);
  const [toolExecLoading, setToolExecLoading] = useState(false);
  const [fusionQuery, setFusionQuery] = useState('');
  const [fusionResults, setFusionResults] = useState<FusionResult[]>([]);
  const [fusionKeywords, setFusionKeywords] = useState<string[]>([]);
  const [fusionLoading, setFusionLoading] = useState(false);
  const [memories, setMemories] = useState<Memory[]>([]);
  const [memorySearch, setMemorySearch] = useState('');
  const [memoryCategory, setMemoryCategory] = useState('all');
  const [memoryLoading, setMemoryLoading] = useState(false);
  const [mood, setMood] = useState<any>(null);
  const [moodTimeline, setMoodTimeline] = useState<MoodEntry[]>([]);
  const [moodLoading, setMoodLoading] = useState(false);
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [habits, setHabits] = useState<Habit[]>([]);
  const [pomodoroRunning, setPomodoroRunning] = useState(false);
  const [pomodoroTime, setPomodoroTime] = useState(25 * 60);
  const [pomodoroTask, setPomodoroTask] = useState('');
  const pomodoroRef = useRef<ReturnType<typeof setInterval> | null>(null);

  // 语音输入
  const [voiceRecording, setVoiceRecording] = useState(false);
  const voiceRecognitionRef = useRef<any>(null);

  // D3.4 图片上传附件（多模态输入）
  const [attachments, setAttachments] = useState<XinAttachment[]>([]);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  // D3.6 实时对话会话（Web Audio API 采集 PCM → 后端流式 STT/LLM/TTS）
  const realtimeSession = useRealtimeSession();

  // A5 Phase 3 Task 2: 云端 AI 离线降级（小欣走云端 API，离线时禁用发送）
  const { isOnline } = useNetworkStatus();

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

  // 主动提醒
  const [toasts, setToasts] = useState<{
    id: string;
    text: string;
    icon: string;
  }[]>([]);

  // 人格进化
  const [evolutionTab, setEvolutionTab] = useState<'favorability' | 'radar'>('favorability');
  const [briefing, setBriefing] = useState<any>(null);
  const [briefingLoading, setBriefingLoading] = useState(false);
  const [showingConversationSidebar, setShowingConversationSidebar] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const chatContainerRef = useRef<HTMLDivElement>(null);
  const streamUnlistenRef = useRef<(() => void) | null>(null);
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
    loadMoodData();
    loadConversations();
    loadCompactionConfig();
    loadAvailableModels();
  }, []);
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({
      behavior: 'smooth'
    });
  }, [messages, streamingContent]);
  useEffect(() => {
    if (activeTab === 'memory' && memories.length === 0) loadMemories();
    if (activeTab === 'productivity' && reminders.length === 0) loadProductivity();
    if (activeTab === 'briefing' && !briefing) loadBriefing();
    if (activeTab === 'compaction' && compactionRecords.length === 0) loadCompactionRecords();
    if (activeTab === 'dream' && !dreamConfig) loadDream();
    if (activeTab === 'checkpoint' && checkpoints.length === 0) loadCheckpoints();
    if (activeTab === 'review' && !review) loadReview();
    if (activeTab === 'skill' && skills.length === 0) loadSkills();
    if (activeTab === 'tool' && tools.length === 0) loadTools();
  }, [activeTab]);
  useEffect(() => {
    return () => {
      if (pomodoroRef.current) clearInterval(pomodoroRef.current);
      if (streamUnlistenRef.current) streamUnlistenRef.current();
    };
  }, []);
  useEffect(() => {
    if (activeConversationId) {
      checkCompactionNeed();
    }
  }, [activeConversationId, messages.length]);

  // 从 AI会话板块加载可用模型列表
  const loadAvailableModels = async () => {
    try {
      const res = await ai.getModels();
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
      const res = await xin.dialogueList(50, 0);
      if (res?.data) {
        setConversations(res.data);
      }
    } catch {/* silent */}
  };
  const createConversation = async () => {
    try {
      const res = await xin.dialogueCreate(activePersona.id);
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
      const res = await xin.dialogueGet(convId);
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
      await xin.dialogueDelete(convId);
      setConversations(prev => prev.filter(c => c.id !== convId));
      if (activeConversationId === convId) {
        setActiveConversationId(null);
        setMessages([]);
      }
    } catch {/* silent */}
  };
  const loadCompactionConfig = async () => {
    try {
      const res = await xin.compactionGetConfig();
      if (res?.data) setCompactionConfig(res.data);
    } catch {/* silent */}
  };
  const loadCompactionRecords = async () => {
    try {
      const res = await xin.compactionGetRecords(activeConversationId || '', 30);
      if (res?.data) setCompactionRecords(res.data);
    } catch {/* silent */}
  };
  const checkCompactionNeed = async () => {
    if (!activeConversationId) return;
    try {
      const res = await xin.compactionNeedsCheck(activeConversationId);
      if (res?.data?.needs_compaction) setNeedsCompaction(true);else setNeedsCompaction(false);
    } catch {/* silent */}
  };
  const updateCompactionConfig = async (partial: Partial<CompactionConfig>) => {
    try {
      await xin.compactionUpdateConfig({
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
      await xin.compactionAuto(activeConversationId);
      setNeedsCompaction(false);
      loadCompactionRecords();
      loadConversations();
    } catch {/* silent */}
  };
  const triggerCompactionManual = async () => {
    if (!activeConversationId) return;
    try {
      await xin.compactionManual(activeConversationId, messages.map(m => ({
        id: m.id,
        role: m.role,
        content: m.content
      })));
      loadCompactionRecords();
      loadConversations();
    } catch {/* silent */}
  };
  // D3.2 TTS：合成并播放消息语音
  const playTts = async (msg: ChatMessage) => {
    if (ttsLoadingId || ttsPlayingId) return;
    if (msg.role !== 'xin') return;
    try {
      setTtsLoadingId(msg.id);
      // 停止当前正在播放的音频
      if (ttsAudioRef.current) {
        ttsAudioRef.current.pause();
        ttsAudioRef.current = null;
      }
      const res = await xin.tts(msg.content);
      if (!res?.data?.audio_path) return;
      // Tauri convertFileSrc：将本地文件路径转为 webview 可访问的 URL
      const { convertFileSrc } = await import('@tauri-apps/api/core');
      const audioUrl = convertFileSrc(res.data.audio_path);
      const audio = new Audio(audioUrl);
      ttsAudioRef.current = audio;
      setTtsPlayingId(msg.id);
      audio.onended = () => {
        setTtsPlayingId(null);
        ttsAudioRef.current = null;
      };
      audio.onerror = () => {
        setTtsPlayingId(null);
        ttsAudioRef.current = null;
      };
      await audio.play();
    } catch (e) {
      console.error('TTS 播放失败:', e);
    } finally {
      setTtsLoadingId(null);
    }
  };
  const stopTts = () => {
    if (ttsAudioRef.current) {
      ttsAudioRef.current.pause();
      ttsAudioRef.current = null;
    }
    setTtsPlayingId(null);
  };
  // D3.4 图片选择：读取为 base64 加入 attachments（后端 parse_attachment 支持 image_url 多模态）
  const handleImageSelect = (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = e.target.files;
    if (!files || files.length === 0) return;
    Array.from(files).forEach(file => {
      if (!file.type.startsWith('image/')) return;
      // 限制 20MB（与后端 validate_image_b64 一致）
      if (file.size > 20 * 1024 * 1024) {
        console.warn(`[D3.4] 图片 ${file.name} 超过 20MB，跳过`);
        return;
      }
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        // reader.result 格式 "data:image/png;base64,XXXX"，提取纯 base64
        const base64 = result.includes(',') ? result.split(',')[1] : result;
        setAttachments(prev => [...prev, {
          name: file.name,
          mime_type: file.type,
          data_b64: base64,
          label: file.name,
        }]);
      };
      reader.readAsDataURL(file);
    });
    // 清空 input value 以便重复选择同一文件
    e.target.value = '';
  };
  const removeAttachment = (index: number) => {
    setAttachments(prev => prev.filter((_, i) => i !== index));
  };

  const sendMessage = async () => {
    const text = input.trim();
    if ((!text && attachments.length === 0) || sending) return;
    // A5 Phase 3 Task 2: 离线时直接显示提示，不调用云端 AI
    if (!isOnline) {
      const errMsg: ChatMessage = {
        id: random.uid(),
        role: 'xin',
        content: t('Xin.offlineHint', { defaultValue: '📴 AI 服务不可用，请连接网络后重试。历史会话仍可查看。' }),
        timestamp: new Date().toISOString()
      };
      setMessages(prev => [...prev, errMsg]);
      return;
    }
    const userMsg: ChatMessage = {
      id: random.uid(),
      role: 'user',
      content: attachments.length > 0 ? `${text}${text ? '\n' : ''}[📷 图片×${attachments.length}]` : text,
      timestamp: new Date().toISOString()
    };
    setMessages(prev => [...prev, userMsg]);
    setInput('');
    const sentAttachments = attachments;
    setAttachments([]);
    setSending(true);
    setStreamingContent('');
    try {
      const result = await xin.dialogueSend(activeConversationId || '', text, {
        model_id: selectedModel?.id,
        persona_id: activePersona.id,
        attachments: sentAttachments.length > 0 ? sentAttachments : undefined,
      });
      if (result?.data) {
        if (result.data.conversation_id && !activeConversationId) {
          setActiveConversationId(result.data.conversation_id);
          loadConversations();
        }
        const xinMsg: ChatMessage = {
          id: random.uid(),
          role: 'xin',
          content: result.data.content,
          timestamp: new Date().toISOString()
        };
        setMessages(prev => [...prev, xinMsg]);
        if (result.data.token_usage) {
          setTokenStats({
            current: result.data.token_usage.total_tokens || 0,
            limit: 128000,
            ratio: (result.data.token_usage.total_tokens || 0) / 128000
          });
        }
        if (result.data.mood) setMood(result.data.mood);
      }
      estimateContextTokens();
    } catch (e) {
      try {
        const {
          xinChat
        } = await import('@/lib/xinChatEngine');
        const history = messages.map(m => ({
          role: m.role === 'user' ? 'user' : 'assistant',
          content: m.content
        }));
        const fallback = await xinChat(text, {
          personaId: activePersona.id,
          history,
          config: activePersona.style
        });
        const xinMsg: ChatMessage = {
          id: random.uid(),
          role: 'xin',
          content: fallback.content,
          timestamp: new Date().toISOString()
        };
        setMessages(prev => [...prev, xinMsg]);
        if (fallback.mood) setMood(fallback.mood);
      } catch (fallbackErr) {
        const errMsg: ChatMessage = {
          id: random.uid(),
          role: 'xin',
          content: t("Xin.k5"),
          timestamp: new Date().toISOString()
        };
        setMessages(prev => [...prev, errMsg]);
      }
    } finally {
      setSending(false);
      setStreamingContent('');
    }
  };
  const estimateContextTokens = async () => {
    const allText = messages.map(m => m.content).join('\n') + input;
    try {
      const res = await xin.estimateTokens(allText);
      if (typeof res?.data === 'number') {
        setTokenStats(prev => ({
          ...prev,
          current: res.data ?? prev.current,
          ratio: (res.data ?? 0) / prev.limit
        }));
      }
    } catch {/* silent */}
  };
  const loadDream = async () => {
    setDreamLoading(true);
    try {
      const [cfgRes, stateRes] = await Promise.all([xin.dreamDefaultConfig(), xin.dreamCalcHealth()]);
      if (cfgRes?.data) setDreamConfig(cfgRes.data);
      if (stateRes?.data) setDreamState(stateRes.data);
    } catch {/* silent */} finally {
      setDreamLoading(false);
    }
  };
  const runDream = async (phase: 'light' | 'deep' | 'rem') => {
    setDreamLoading(true);
    try {
      let res: any;
      if (phase === 'light') res = await xin.dreamRunLight();else if (phase === 'deep') res = await xin.dreamRunDeep(20);else res = await xin.dreamRunRem();
      if (res?.data) setDreamResult(res.data);
      await loadDream();
    } catch {/* silent */} finally {
      setDreamLoading(false);
    }
  };
  const loadCheckpoints = async () => {
    if (!activeConversationId) return;
    setCheckpointLoading(true);
    try {
      const res = await xin.checkpointList(activeConversationId);
      if (res?.data) setCheckpoints(res.data);
    } catch {/* silent */} finally {
      setCheckpointLoading(false);
    }
  };
  const saveCheckpoint = async () => {
    if (!activeConversationId) return;
    try {
      await xin.checkpointSave(activeConversationId, t("Xin.k6", {
        arg0: new Date().toLocaleTimeString('zh-CN')
      }));
      loadCheckpoints();
    } catch {/* silent */}
  };
  const restoreCheckpoint = async (checkpointId: string) => {
    try {
      await xin.checkpointRestore(checkpointId);
      if (activeConversationId) switchConversation(activeConversationId);
      loadCheckpoints();
    } catch {/* silent */}
  };
  const deleteCheckpoint = async (checkpointId: string) => {
    try {
      await xin.checkpointDelete(checkpointId);
      setCheckpoints(prev => prev.filter(c => c.id !== checkpointId));
    } catch {/* silent */}
  };
  const doSearch = async () => {
    if (!searchQuery.trim()) return;
    setSearchLoading(true);
    try {
      const res = await xin.dialogueSearch(searchQuery, 20);
      if (res?.data) {
        setSearchResults(res.data);
        setSearchTotal(res.data.length);
      }
    } catch {/* silent */} finally {
      setSearchLoading(false);
    }
  };
  const loadReview = async () => {
    setReviewLoading(true);
    try {
      const [reviewRes, trendRes, growthRes, heatmapRes] = await Promise.all([xin.reviewGenerate(reviewPeriod, activePersona.id), xin.reviewTopicTrends(activePersona.id, reviewPeriod, 7), xin.reviewGrowthTrajectory(activePersona.id, reviewPeriod, 7), xin.reviewHeatmap(activePersona.id, reviewPeriod)]);
      if (reviewRes?.data) setReview(reviewRes.data);
      if (trendRes?.data?.trends) setTopicTrends(trendRes.data.trends);
      if (growthRes?.data) setGrowthTrajectory(growthRes.data);
      if (heatmapRes?.data) setHeatmap(heatmapRes.data);
    } catch {/* silent */} finally {
      setReviewLoading(false);
    }
  };
  const loadSkills = async () => {
    try {
      const res = await xin.listSkills();
      if (res?.data) setSkills(res.data);
    } catch {/* silent */}
  };
  const executeSkill = async (skillId: string) => {
    if (!skillInput.trim()) return;
    setSkillExecLoading(true);
    try {
      const res = await xin.executeSkill(skillId, skillInput);
      if (res?.data) setSkillResult(res.data);
    } catch {/* silent */} finally {
      setSkillExecLoading(false);
    }
  };
  const loadTools = async () => {
    try {
      const res = await xin.listTools();
      if (res?.data) setTools(Array.isArray(res.data) ? res.data : []);
    } catch {/* silent */}
  };
  const parseToolCalls = async () => {
    if (!toolInputText.trim()) return;
    try {
      const res = await xin.parseToolCalls(toolInputText);
      if (res?.data) setParsedCalls(Array.isArray(res.data) ? res.data : []);
    } catch {/* silent */}
  };
  const executeToolCalls = async () => {
    if (parsedCalls.length === 0) return;
    setToolExecLoading(true);
    try {
      const res = await xin.executeTool(JSON.stringify(parsedCalls));
      if (res?.data) setToolResults(Array.isArray(res.data) ? res.data : []);
    } catch {/* silent */} finally {
      setToolExecLoading(false);
    }
  };
  const doFusionQuery = async () => {
    if (!fusionQuery.trim()) return;
    setFusionLoading(true);
    try {
      const [memoryRes, keywordRes] = await Promise.all([xin.fusionMemoryQuery(fusionQuery, 10), xin.fusionExtractKeywords(fusionQuery)]);
      if (memoryRes?.data) setFusionResults(Array.isArray(memoryRes.data) ? memoryRes.data : []);
      if (keywordRes?.data) setFusionKeywords(keywordRes.data);
    } catch {/* silent */} finally {
      setFusionLoading(false);
    }
  };
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  // ===== 语音输入（D3.3: MediaRecorder 录音 → 后端 Whisper API 转写） =====
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);
  const startVoiceInput = async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const mime = MediaRecorder.isTypeSupported('audio/webm;codecs=opus')
        ? 'audio/webm;codecs=opus'
        : 'audio/webm';
      const recorder = new MediaRecorder(stream, { mimeType: mime });
      audioChunksRef.current = [];
      recorder.ondataavailable = (e) => {
        if (e.data.size > 0) audioChunksRef.current.push(e.data);
      };
      recorder.onstop = async () => {
        stream.getTracks().forEach(t => t.stop());
        const blob = new Blob(audioChunksRef.current, { type: mime });
        await transcribeAudio(blob);
      };
      recorder.start();
      mediaRecorderRef.current = recorder;
      setVoiceRecording(true);
    } catch (e) {
      console.error('录音启动失败:', e);
      // 降级到浏览器原生 SpeechRecognition（若可用）
      const SpeechRecognitionCtor = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
      if (SpeechRecognitionCtor) {
        const recognition = new SpeechRecognitionCtor();
        recognition.continuous = false;
        recognition.interimResults = false;
        recognition.lang = 'zh-CN';
        recognition.onresult = (event: any) => {
          const transcript = event.results[0]?.[0]?.transcript || '';
          if (transcript) setInput(prev => prev + transcript);
        };
        recognition.onerror = () => setVoiceRecording(false);
        recognition.onend = () => setVoiceRecording(false);
        recognition.start();
        voiceRecognitionRef.current = recognition;
        setVoiceRecording(true);
      }
    }
  };
  const stopVoiceInput = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
      mediaRecorderRef.current.stop();
      mediaRecorderRef.current = null;
    }
    if (voiceRecognitionRef.current) {
      voiceRecognitionRef.current.stop();
      voiceRecognitionRef.current = null;
    }
    setVoiceRecording(false);
  };
  // D3.3: 将录音 blob 写入临时文件，调用后端 Whisper API 转写
  const transcribeAudio = async (blob: Blob) => {
    try {
      // 优先尝试写入临时文件，再传路径给后端
      const arrayBuffer = await blob.arrayBuffer();
      const uint8 = new Uint8Array(arrayBuffer);
      // 通过 Tauri 文件系统插件写入临时文件
      const { writeFile } = await import('@tauri-apps/plugin-fs');
      const { tempDir } = await import('@tauri-apps/api/path');
      const tmpDir = await tempDir();
      const fileName = `xin_voice_${Date.now()}.webm`;
      const filePath = `${tmpDir}/${fileName}`;
      await writeFile(filePath, uint8);
      const res = await xin.voiceInput(filePath, 'zh', selectedModel?.id);
      const text = res?.data?.text;
      if (text) setInput(prev => prev + text);
    } catch (e) {
      console.error('语音转写失败:', e);
    }
  };
  useEffect(() => {
    return () => {
      if (voiceRecognitionRef.current) voiceRecognitionRef.current.abort();
    };
  }, []);
  const loadMemories = async () => {
    setMemoryLoading(true);
    try {
      const res = await xin.getMemories(50);
      if (res?.data && Array.isArray(res.data)) {
        setMemories(res.data);
      }
      if (memories.length === 0) {
        setMemories(getMockMemories());
      }
    } catch {
      if (memories.length === 0) setMemories(getMockMemories());
    } finally {
      setMemoryLoading(false);
    }
  };
  const loadMoodData = async () => {
    setMoodLoading(true);
    try {
      const res = await xin.getMood();
      if (res?.data) setMood(res.data);
      setMoodTimeline(getMockMoodTimeline());
    } catch {
      setMood({
        category: 'curious',
        intensity: 0.75,
        trigger: t("Xin.k7"),
        updated_at: new Date().toISOString()
      });
      setMoodTimeline(getMockMoodTimeline());
    } finally {
      setMoodLoading(false);
    }
  };
  const loadProductivity = () => {
    setReminders([{
      id: 'r1',
      title: t("Xin.k8"),
      description: t("Xin.k9"),
      time: t("Xin.k10"),
      repeat: 'work',
      active: true
    }, {
      id: 'r2',
      title: t("Xin.k11"),
      description: t("Xin.k12"),
      time: t("Xin.k13"),
      repeat: 'hourly',
      active: true
    }, {
      id: 'r3',
      title: t("Xin.k14"),
      description: t("Xin.k15"),
      time: t("Xin.k16"),
      repeat: 'daily',
      active: false
    }]);
    setHabits([{
      id: 'h1',
      name: t("Xin.k17"),
      category: t("home.utils.k6"),
      streak: 12,
      total: 45,
      checked: true
    }, {
      id: 'h2',
      name: t("Xin.k18"),
      category: t("Xin.k19"),
      streak: 5,
      total: 30,
      checked: false
    }, {
      id: 'h3',
      name: t("Xin.k20"),
      category: t("layout.k20"),
      streak: 8,
      total: 22,
      checked: true
    }]);
  };
  const loadBriefing = async () => {
    setBriefingLoading(true);
    try {
      const res = await xin.dailyBriefing();
      if (res?.data) {
        setBriefing(res.data);
        return;
      }
    } catch {/* fallback to mock */}
    setBriefing(getMockBriefing());
    setBriefingLoading(false);
  };
  const togglePomodoro = () => {
    if (pomodoroRunning) {
      if (pomodoroRef.current) clearInterval(pomodoroRef.current);
      pomodoroRef.current = null;
      setPomodoroRunning(false);
    } else {
      if (!pomodoroTask.trim()) {
        setPomodoroTask(t("components.FloatingBall.k26"));
      }
      setPomodoroRunning(true);
      pomodoroRef.current = setInterval(() => {
        setPomodoroTime(prev => {
          if (prev <= 1) return 0;
          return prev - 1;
        });
      }, 1000);
    }
  };
  const resetPomodoro = () => {
    if (pomodoroRef.current) clearInterval(pomodoroRef.current);
    pomodoroRef.current = null;
    setPomodoroRunning(false);
    setPomodoroTime(25 * 60);
  };
  const formatPomodoro = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  };
  const filteredMemories = memories.filter(m => {
    if (memorySearch && !m.key.includes(memorySearch) && !m.value.includes(memorySearch)) return false;
    if (memoryCategory !== 'all' && m.category !== memoryCategory) return false;
    return true;
  });
  const uniqueCategories = Array.from(new Set(memories.map(m => m.category)));

  // tabs 定义（供 UI 渲染使用）

  return <div className={styles.page}>
      <div className={styles.header}>
        <h2 className={styles.title}>{t("Xin.k21")}</h2>
        <div className={styles.tabBar}>
          {['chat', 'memory', 'mood', 'productivity', 'briefing', 'compaction', 'dream', 'checkpoint', 'search', 'review', 'skill', 'tool', 'evolution'].map(tab => <button key={tab} className={`${styles.tab} ${activeTab === tab ? styles.tabActive : ''}`} onClick={() => {
          setActiveTab(tab);
          navigate(`/xin?tab=${tab}`, {
            replace: true
          });
        }}>
              {tab === 'chat' ? t("layout.k17") : tab === 'memory' ? t("layout.k18") : tab === 'mood' ? t("layout.k19") : tab === 'productivity' ? t("layout.k20") : tab === 'briefing' ? t("layout.k21") : tab === 'compaction' ? t("layout.k22") : tab === 'dream' ? t("layout.k23") : tab === 'checkpoint' ? t("layout.k24") : tab === 'search' ? t("common.search") : tab === 'review' ? t("layout.k25") : tab === 'skill' ? t("layout.k26") : tab === 'tool' ? t("layout.k27") : t("Xin.k22")}
            </button>)}
        </div>
      </div>

      {/* Toast 通知 */}
      <div className={styles.toastContainer}>
        {toasts.map(t => <div key={t.id} className={styles.toastItem}>
            <span>{t.icon}</span>
            <span>{t.text}</span>
          </div>)}
      </div>

      <div className={styles.content}>
        {/* ========== CHAT TAB ========== */}
        {activeTab === 'chat' && <div className={`${styles.tabContent} ${styles.chatLayout}`}>
            {/* Conversation Sidebar */}
            <div className={`${styles.convSidebar} ${!showingConversationSidebar ? styles.convSidebarCollapsed : ''}`}>
              <div className={styles.convSidebarHeader}>
                <span className={styles.convSidebarTitle}>{t("layout.k4")}</span>
                <div className={styles.convSidebarActions}>
                  <button className={styles.btnIcon} onClick={createConversation} title={t("ai.ConversationModal.k5")}>＋</button>
                  <button className={styles.btnIcon} onClick={() => setShowingConversationSidebar(!showingConversationSidebar)} title={t("Xin.k23")}>
                    {showingConversationSidebar ? '◀' : '▶'}
                  </button>
                </div>
              </div>

              {showingConversationSidebar && <div className={styles.convList}>
                  {conversations.length === 0 && <div className={styles.convEmpty}>
                      <p>{t("ai.ConversationList.k8")}</p>
                      <button className={styles.btnSmall} onClick={createConversation}>{t("ai.ConversationModal.k5")}</button>
                    </div>}
                  {conversations.map(conv => <div key={conv.id} className={`${styles.convItem} ${activeConversationId === conv.id ? styles.convItemActive : ''}`} onClick={() => switchConversation(conv.id)}>
                      <div className={styles.convItemMain}>
                        <span className={styles.convItemTitle}>{conv.title}</span>
                        <span className={styles.convItemMeta}>
                          {conv.message_count}{t("Xin.k24")} {conv.total_tokens} tokens
                        </span>
                      </div>
                      <button className={styles.btnDangerSmall} onClick={e => {
                e.stopPropagation();
                deleteConversation(conv.id);
              }}>
                        ✕
                      </button>
                    </div>)}
                </div>}
            </div>

            {/* Chat Main Area */}
            <div className={styles.chatMain}>
              {/* Token Bar */}
              <div className={styles.tokenBar}>
                <div className={styles.tokenInfo}>
                  <span className={styles.tokenLabel}>📊 Token: {tokenStats.current.toLocaleString()} / {tokenStats.limit.toLocaleString()}</span>
                  <span className={styles.tokenPercent}>({Math.round(tokenStats.ratio * 100)}%)</span>
                </div>
                <div className={styles.tokenTrack}>
                  <div className={`${styles.tokenFill} ${tokenStats.ratio > 0.8 ? styles.tokenFillDanger : tokenStats.ratio > 0.6 ? styles.tokenFillWarn : ''}`} style={{
                width: `${Math.min(tokenStats.ratio * 100, 100)}%`
              }} />
                </div>
                {needsCompaction && <button className={styles.compactionHint} onClick={triggerCompactionAuto}>
                    {t("Xin.k25")}
                  </button>}
              </div>

              {/* Chat Header */}
              <div className={styles.chatHeader}>
                <div className={styles.chatPersonaInfo}>
                  <span className={styles.chatPersonaEmoji}>{activePersona.emoji}</span>
                  <span>{activePersona.name}</span>
                </div>
                {/* A5 Phase 3 Task 2: 离线降级提示（小欣走云端 API，离线时显示） */}
                {!isOnline && <span className={styles.offlineHint} title={t('Xin.offlineHint', { defaultValue: '📴 AI 服务不可用，请连接网络后重试。历史会话仍可查看。' })}>
                    📴 {t('Xin.offlineBadge', { defaultValue: 'AI 离线' })}
                  </span>}
                <div className={styles.chatStatusArea}>
                  {/* 模型选择器 */}
                  {availableModels.length > 0 && <select className={styles.modelSelect} value={selectedModel?.name || ''} onChange={e => setSelectedModelName(e.target.value)} title={t("Xin.k26")}>
                      {availableModels.map(m => <option key={m.id} value={m.name}>{m.name} ({m.provider})</option>)}
                    </select>}
                  {availableModels.length === 0 && <span className={styles.modelSelectHint} title={t("Xin.k27")}>{t("Xin.k28")}</span>}
                  {activeConversationId && <span className={styles.convIdBadge}>
                      {t("Xin.k29")} {activeConversationId.slice(0, 8)}...
                    </span>}
                  {mood && <span className={styles.chatPersonaMood}>
                      {MOOD_EMOJI[mood.category] || '🤖'} {mood.category}
                    </span>}
                  {sending && <span className={styles.streamingIndicator}>
                      <span className={styles.streamDot} />
                      {t("Xin.k30")}
                    </span>}
                </div>
              </div>

              {/* Messages */}
              <div className={styles.chatMessages} ref={chatContainerRef}>
                {messages.length === 0 && !sending && <div className={styles.chatEmpty}>
                    <div className={styles.chatEmptyIcon}>💬</div>
                    <p>{t("Xin.k31")}{activePersona.name}{t("Xin.k32")}</p>
                    <p className={styles.chatEmptyHint}>
                      {t("Xin.k33")}
                    </p>
                    <p className={styles.chatEmptyHint}>
                      {t("Xin.k34")}
                    </p>
                  </div>}

                {messages.map(m => <div key={m.id} className={`${styles.chatMsg} ${m.role === 'user' ? styles.msgUser : styles.msgXin}`}>
                    <span className={styles.msgRole}>
                      {m.role === 'user' ? t("Xin.k35") : `${activePersona.emoji} ${activePersona.name}`}
                    </span>
                    <pre className={styles.msgContent}>{m.content}</pre>
                    {m.role === 'xin' && <div className={styles.msgActions}>
                      {ttsPlayingId === m.id
                        ? <button type="button" className={styles.ttsBtn} onClick={stopTts} title={t("Xin.k26") || '停止播放'}>⏹</button>
                        : <button type="button" className={styles.ttsBtn} onClick={() => playTts(m)} disabled={ttsLoadingId === m.id || ttsPlayingId !== null} title={t("Xin.k26") || '语音播放'}>
                            {ttsLoadingId === m.id ? '⏳' : '🔊'}
                          </button>}
                    </div>}
                  </div>)}

                {sending && streamingContent && <div className={`${styles.chatMsg} ${styles.msgXin}`}>
                    <span className={styles.msgRole}>{activePersona.emoji} {activePersona.name}</span>
                    <pre className={styles.msgContent}>{streamingContent}<span className={styles.cursor}>▌</span></pre>
                  </div>}

                {sending && !streamingContent && <div className={`${styles.chatMsg} ${styles.msgXin}`}>
                    <span className={styles.msgRole}>{activePersona.emoji} {activePersona.name}</span>
                    <span className={styles.msgContent} style={{
                color: '#5a5a5a',
                fontStyle: 'italic'
              }}>
                      {t("Xin.k36")}<span className={styles.dots}>...</span>
                    </span>
                  </div>}
                <div ref={messagesEndRef} />
              </div>

              {/* D3.4 附件预览 */}
              {attachments.length > 0 && <div className={styles.attachmentPreview}>
                  {attachments.map((att, i) => <div key={i} className={styles.attachmentItem}>
                      <img className={styles.attachmentThumb} src={`data:${att.mime_type};base64,${att.data_b64}`} alt={att.name} />
                      <span className={styles.attachmentName}>{att.name}</span>
                      <button className={styles.attachmentRemove} onClick={() => removeAttachment(i)} title={t("common.delete")}>✕</button>
                    </div>)}
                </div>}
              {/* Input Area */}
              <div className={styles.chatInputArea}>
                <input ref={fileInputRef} type="file" accept="image/*" multiple style={{ display: 'none' }} onChange={handleImageSelect} />
                <input className={styles.chatInput} value={input} onChange={e => setInput(e.target.value)} onKeyDown={handleKeyDown} placeholder={t("Xin.k37", {
              name: activePersona.name
            })} disabled={sending} />
                <button className={styles.micBtn} onClick={() => fileInputRef.current?.click()} disabled={sending} title="上传图片">
                  📷
                </button>
                <button className={`${styles.micBtn} ${voiceRecording ? styles.micBtnRecording : ''}`} onClick={() => voiceRecording ? stopVoiceInput() : startVoiceInput()} disabled={sending} title={voiceRecording ? t("components.FloatingXin.k37") : t("components.FloatingXin.k38")}>
                  🎤
                  {voiceRecording && <span className={styles.recordingDot} />}
                </button>
                {/* D3.6 实时对话按钮：点击启动/停止实时会话，状态指示当前 phase */}
                <button
                  className={`${styles.micBtn} ${realtimeSession.isActive ? styles.micBtnRecording : ''}`}
                  onClick={toggleRealtime}
                  disabled={sending}
                  title={realtimeSession.isActive ? `实时对话中：${realtimeSession.state}` : '启动实时对话'}
                  style={realtimeSession.isActive ? { color: realtimeSession.state === 'speaking' ? '#00FF00' : realtimeSession.state === 'thinking' ? '#FFAA00' : '#00AAFF' } : undefined}
                >
                  {realtimeSession.state === 'thinking' ? '💭' : realtimeSession.state === 'speaking' ? '🔊' : '📡'}
                  {realtimeSession.isActive && <span className={styles.recordingDot} />}
                </button>
                {realtimeSession.error && <span style={{ color: '#FF0000', fontSize: '11px', alignSelf: 'center' }}>{realtimeSession.error}</span>}
                <button className={styles.chatSendBtn} onClick={sendMessage} disabled={sending || (!input.trim() && attachments.length === 0)}>
                  {sending ? '···' : t("components.FloatingBall.k64")}
                </button>
                {sending && <button className={styles.chatStopBtn} onClick={async () => {
              if (activeConversationId) {
                try {
                  await xin.dialogueStop(activeConversationId);
                } catch {/* silent */}
              }
            }}>
                    {t("Xin.k38")}
                  </button>}
              </div>
            </div>
          </div>}

        {/* ========== MEMORY TAB ========== */}
        {activeTab === 'memory' && <div className={styles.tabContent}>
            <div className={styles.searchRow}>
              <input className={styles.input} placeholder={t("Xin.k39")} value={memorySearch} onChange={e => setMemorySearch(e.target.value)} />
              <select className={styles.select} value={memoryCategory} onChange={e => setMemoryCategory(e.target.value)}>
                <option value="all">{t("Xin.k40")}</option>
                {uniqueCategories.map(c => <option key={c} value={c}>{CATEGORY_LABELS[c] || c}</option>)}
              </select>
              <button className={styles.btn} onClick={loadMemories} disabled={memoryLoading}>
                {memoryLoading ? t("common.loading") : t("common.refresh")}
              </button>
            </div>

            {memoryLoading && <div className={styles.loading}>{t("Xin.k41")}</div>}

            {!memoryLoading && filteredMemories.length === 0 && <div style={{
          textAlign: 'center',
          padding: '40px',
          color: '#5a5a5a'
        }}>
                {memorySearch ? t("Xin.k42") : t("Xin.k43")}
              </div>}

            {!memoryLoading && <div className={styles.memoryGrid}>
                {filteredMemories.map(m => <div key={m.id} className={styles.memoryItem}>
                    <div className={styles.memoryHeader}>
                      <span className={styles.memoryCategory}>{CATEGORY_LABELS[m.category] || m.category}</span>
                      <span className={styles.memoryImportance}>★ {m.importance.toFixed(1)}</span>
                    </div>
                    <span className={styles.memoryKey}>{m.key}</span>
                    <p className={styles.memoryValue}>{m.value}</p>
                    <div className={styles.memoryMeta}>
                      <span>{time.formatCompact(m.created_at)}</span>
                    </div>
                  </div>)}
              </div>}
          </div>}

        {/* ========== MOOD TAB ========== */}
        {activeTab === 'mood' && <div className={styles.tabContent}>
            {moodLoading ? <div className={styles.loading}>{t("Xin.k44")}</div> : <>
                {mood && <div className={styles.card} style={{
            marginBottom: '16px'
          }}>
                    <div className={styles.moodDisplay}>
                      <span className={styles.moodEmoji}>{MOOD_EMOJI[mood.category] || '🤖'}</span>
                      <div className={styles.moodInfo}>
                        <span className={styles.moodLabel}>{mood.category}</span>
                        <span className={styles.moodIntensity}>{t("Xin.k45")} {((mood.intensity || 0) * 100).toFixed(0)}%</span>
                        {mood.trigger && <span className={styles.moodTrigger}>{t("Xin.k46")} {mood.trigger}</span>}
                      </div>
                    </div>
                  </div>}

                <div className={styles.card}>
                  <div className={styles.panelTitle}>{t("Xin.k47")}</div>
                  <div className={styles.moodTimeline}>
                    {moodTimeline.map(m => <div key={m.id} className={styles.moodTimelineItem}>
                        <span className={styles.moodTimelineEmoji}>{m.emoji}</span>
                        <div className={styles.moodTimelineInfo}>
                          <span className={styles.moodTimelineCategory}>{m.category}</span>
                          <span className={styles.moodTimelineContext}>{m.context}</span>
                        </div>
                        <span className={styles.moodTimelineTime}>{m.time}</span>
                      </div>)}
                  </div>
                </div>
              </>}
          </div>}

        {/* ========== PRODUCTIVITY TAB ========== */}
        {activeTab === 'productivity' && <div className={styles.tabContent}>
            {/* Pomodoro */}
            <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k48")}</div>
              {pomodoroRunning ? <div className={styles.pomodoroActive}>
                  <div className={styles.pomodoroDisplay}>
                    <span className={styles.pomodoroTask}>{pomodoroTask}</span>
                    <span className={styles.pomodoroTime}>{formatPomodoro(pomodoroTime)}</span>
                  </div>
                  <button className={styles.btn} onClick={togglePomodoro}>{t("common.pause")}</button>
                  <button className={styles.btnSmall} onClick={resetPomodoro}>{t("common.reset")}</button>
                </div> : <div className={styles.pomodoroSetup}>
                  <div className={styles.addRow}>
                    <span className={styles.inputLabel}>{t("Xin.k49")}</span>
                    <input className={styles.input} value={pomodoroTask} onChange={e => setPomodoroTask(e.target.value)} placeholder={t("Xin.k50")} />
                    <button className={styles.btn} onClick={togglePomodoro}>{t("common.start")}</button>
                  </div>
                </div>}
            </div>

            {/* Reminders */}
            <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k51")}</div>
              <div className={styles.list}>
                {reminders.map(r => <div key={r.id} className={`${styles.reminderItem} ${!r.active ? styles.reminderInactive : ''}`}>
                    <div className={styles.reminderInfo}>
                      <span className={styles.reminderTitle}>{r.title}</span>
                      <span className={styles.reminderDesc}>{r.description}</span>
                      <span className={styles.reminderMeta}>{r.time}</span>
                    </div>
                  </div>)}
              </div>
            </div>

            {/* Habits */}
            <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k52")}</div>
              <div className={styles.list}>
                {habits.map(h => <div key={h.id} className={styles.habitItem}>
                    <div className={styles.habitInfo}>
                      <span className={styles.habitName}>{h.name}</span>
                      <span className={styles.habitCategory}>{h.category}</span>
                    </div>
                    <div className={styles.habitStats}>
                      <span className={styles.habitStreak}>🔥 {h.streak}{t("components.FocusMode.k9")}</span>
                      <span className={styles.habitTotal}>{t("components.GroupChatOrchestrationPanel.k26")}{h.total}{t("profile.StatsDashboard.k3")}</span>
                      <span className={styles.checkedBadge}>{h.checked ? '✅' : '⬜'}</span>
                    </div>
                  </div>)}
              </div>
            </div>
          </div>}

        {/* ========== BRIEFING TAB ========== */}
        {activeTab === 'briefing' && <div className={styles.tabContent}>
            {briefingLoading ? <div className={styles.loading}>{t("Xin.k53")}</div> : <div className={styles.briefingContent}>
                {briefing && <>
                    <div className={styles.card}>
                      <p className={styles.briefingDate}>{briefing.date || new Date().toLocaleDateString('zh-CN')}</p>
                      <p className={styles.briefingGreeting}>{briefing.greeting || t("Xin.k54")}</p>
                      {briefing.quote && <p className={styles.briefingQuote}>"{briefing.quote}"</p>}
                    </div>

                    {briefing.suggestions && <div className={styles.card}>
                        <div className={styles.panelTitle}>{t("Xin.k55")}</div>
                        <div className={styles.briefingSuggestions}>
                          {(briefing.suggestions as string[]).map((s: string, i: number) => <div key={i} className={styles.suggestionItem}>
                              <span className={styles.suggestionIcon}>✦</span>
                              <span>{s}</span>
                            </div>)}
                        </div>
                      </div>}

                    {briefing.memories && <div className={styles.card}>
                        <div className={styles.panelTitle}>{t("Xin.k56")}</div>
                        <div className={styles.briefingMemories}>
                          {(briefing.memories as any[]).map((m: any, i: number) => <div key={i} className={styles.briefingMemItem}>
                              <span>📌</span>
                              <span>{m.title || m.key || m.content?.slice(0, 60)}</span>
                            </div>)}
                        </div>
                      </div>}
                  </>}
                {!briefing && <div className={styles.card}>
                    <p className={styles.briefingGreeting}>{t("Xin.k57")}</p>
                    <p style={{
              fontSize: '12px',
              color: '#5a5a5a'
            }}>
                      {t("Xin.k58")}
                    </p>
                  </div>}
              </div>}
          </div>}

        {/* ========== COMPACTION TAB ========== */}
        {activeTab === 'compaction' && <div className={styles.tabContent}>
            {compactionConfig && <div className={styles.card}>
                <div className={styles.panelTitle}>{t("Xin.k59")}</div>
                <div className={styles.settingsGrid}>
                  <div className={styles.settingItem}>
                    <span className={styles.inputLabel}>{t("Xin.k60")}</span>
                    <button className={compactionConfig.enabled ? styles.toggleOn : styles.toggleOff} onClick={() => updateCompactionConfig({
                enabled: !compactionConfig.enabled
              })}>
                      {compactionConfig.enabled ? 'ON' : 'OFF'}
                    </button>
                  </div>
                  <div className={styles.settingItem}>
                    <span className={styles.inputLabel}>{t("Xin.k61")}</span>
                    <span>{(compactionConfig.trigger_threshold_ratio * 100).toFixed(0)}%</span>
                  </div>
                  <div className={styles.settingItem}>
                    <span className={styles.inputLabel}>{t("Xin.k62")}</span>
                    <span>{compactionConfig.keep_recent_tokens.toLocaleString()}</span>
                  </div>
                  <div className={styles.settingItem}>
                    <span className={styles.inputLabel}>{t("Xin.k63")}</span>
                    <span className={styles.badgeOn}>
                      {compactionConfig.compaction_mode === 'sliding_window' ? t("Xin.k64") : t("Xin.k65")}
                    </span>
                  </div>
                  <div className={styles.settingItem}>
                    <span className={styles.inputLabel}>{t("Xin.k66")}</span>
                    <button className={compactionConfig.memory_flush_enabled ? styles.toggleOn : styles.toggleOff} onClick={() => updateCompactionConfig({
                memory_flush_enabled: !compactionConfig.memory_flush_enabled
              })}>
                      {compactionConfig.memory_flush_enabled ? 'ON' : 'OFF'}
                    </button>
                  </div>
                  <div className={styles.settingItem}>
                    <span className={styles.inputLabel}>{t("Xin.k67")}</span>
                    <button className={compactionConfig.notify_user ? styles.toggleOn : styles.toggleOff} onClick={() => updateCompactionConfig({
                notify_user: !compactionConfig.notify_user
              })}>
                      {compactionConfig.notify_user ? 'ON' : 'OFF'}
                    </button>
                  </div>
                </div>
              </div>}

            {!compactionConfig && <div className={styles.card}>
                <div className={styles.panelTitle}>{t("Xin.k59")}</div>
                <p style={{
            color: '#5a5a5a'
          }}>{t("Xin.k68")}</p>
              </div>}

            <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k69")}</div>
              <div style={{
            display: 'flex',
            gap: '8px',
            flexWrap: 'wrap'
          }}>
                <button className={styles.btn} onClick={checkCompactionNeed} disabled={!activeConversationId}>
                  {t("Xin.k70")}
                </button>
                <button className={styles.btn} onClick={triggerCompactionAuto} disabled={!activeConversationId}>
                  {t("Xin.k71")}
                </button>
                <button className={styles.btn} onClick={triggerCompactionManual} disabled={!activeConversationId}>
                  {t("Xin.k72")}
                </button>
              </div>
              {!activeConversationId && <p style={{
            fontSize: '10px',
            color: '#6a6a8a',
            marginTop: '8px'
          }}>
                  {t("Xin.k73")}
                </p>}
              {needsCompaction && <p style={{
            fontSize: '10px',
            color: '#FFD700',
            marginTop: '8px'
          }}>
                  {t("Xin.k74")}
                </p>}
            </div>

            <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k75")}</div>
              {compactionRecords.length === 0 && <p style={{
            color: '#5a5a5a',
            textAlign: 'center',
            padding: '20px'
          }}>
                  {t("Xin.k76")}
                </p>}
              {compactionRecords.map(r => <div key={r.id} className={styles.compactionRecord}>
                  <div className={styles.compactionRecordHeader}>
                    <span className={styles.compactionTrigger}>
                      {r.trigger_type === 'auto' ? t("Xin.k77") : t("Xin.k78")}
                    </span>
                    <span className={styles.compactionTime}>{time.formatCompact(r.created_at)}</span>
                  </div>
                  <div className={styles.compactionStats}>
                    <span>{t("Xin.k79")} {r.pre_message_count} → {r.post_message_count}</span>
                    <span>Token: {r.pre_token_count.toLocaleString()} → {r.post_token_count.toLocaleString()}</span>
                    <span>{t("Xin.k80")} {r.memory_flush_count} {t("ai.ChatPanel.k19")}</span>
                  </div>
                  {r.summary_text && <div className={styles.compactionSummary}>
                      <span className={styles.summaryLabel}>{t("Linux.k101")}</span>
                      <span>{r.summary_text.slice(0, 120)}{r.summary_text.length > 120 ? '...' : ''}</span>
                    </div>}
                  {r.guidance_text && <div className={styles.compactionGuidance}>
                      <span className={styles.summaryLabel}>{t("Xin.k81")}</span>
                      <span>"{r.guidance_text}"</span>
                    </div>}
                </div>)}
            </div>
          </div>}

        {/* ========== DREAM TAB ========== */}
        {activeTab === 'dream' && <div className={styles.tabContent}>
            {dreamLoading ? <div className={styles.loading}>{t("Xin.k82")}</div> : <>
                {dreamConfig && <div className={styles.card}>
                    <div className={styles.panelTitle}>{t("Xin.k83")}</div>
                    <div className={styles.settingsGrid}>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k84")}</span>
                        <span className={dreamConfig.enabled ? styles.badgeOn : styles.badgeOff}>
                          {dreamConfig.enabled ? t("Linux.k46") : t("Xin.k85")}
                        </span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k86")}</span>
                        <span>{dreamConfig.phases.light.interval_hours}h</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k87")}</span>
                        <span>{dreamConfig.phases.deep.interval_hours}h</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k88")}</span>
                        <span>{dreamConfig.phases.rem.interval_hours}h</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k89")}</span>
                        <span>{dreamConfig.phases.light.max_candidates}</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k90")}</span>
                        <span>{(dreamConfig.phases.deep.min_score * 100).toFixed(0)}%</span>
                      </div>
                    </div>
                  </div>}

                {dreamState && <div className={styles.card}>
                    <div className={styles.panelTitle}>{t("Xin.k91")}</div>
                    <div className={styles.settingsGrid}>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k92")}</span>
                        <span className={styles.badgeOn}>{dreamState.light_candidates_count}</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k93")}</span>
                        <span className={styles.badgeOn}>{dreamState.deep_promotions_count}</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k94")}</span>
                        <span className={styles.badgeOn}>{dreamState.rem_patterns_count}</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k95")}</span>
                        <span>{dreamState.last_light_at ? time.formatCompact(dreamState.last_light_at) : t("Xin.k96")}</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k97")}</span>
                        <span>{dreamState.last_deep_at ? time.formatCompact(dreamState.last_deep_at) : t("Xin.k96")}</span>
                      </div>
                      <div className={styles.settingItem}>
                        <span className={styles.inputLabel}>{t("Xin.k98")}</span>
                        <span>{dreamState.last_rem_at ? time.formatCompact(dreamState.last_rem_at) : t("Xin.k96")}</span>
                      </div>
                    </div>
                  </div>}

                <div className={styles.card}>
                  <div className={styles.panelTitle}>{t("Xin.k99")}</div>
                  <div style={{
              display: 'flex',
              gap: '8px',
              flexWrap: 'wrap'
            }}>
                    <button className={styles.btn} onClick={() => runDream('light')} disabled={dreamLoading}>
                      {t("Xin.k100")}
                    </button>
                    <button className={styles.btn} onClick={() => runDream('deep')} disabled={dreamLoading}>
                      {t("Xin.k101")}
                    </button>
                    <button className={styles.btn} onClick={() => runDream('rem')} disabled={dreamLoading}>
                      {t("Xin.k102")}
                    </button>
                  </div>
                </div>

                {dreamResult && <div className={styles.card}>
                    <div className={styles.panelTitle}>{t("Xin.k103")}</div>
                    {dreamResult.candidates && <div>
                        <span style={{
                fontSize: '10px',
                color: '#6a6a8a'
              }}>
                          {t("Xin.k104")} {dreamResult.candidates.length} {t("Xin.k105")} {dreamResult.deduped_count}{t("Xin.k106")} {dreamResult.sources_processed}, {dreamResult.duration_ms}ms)
                        </span>
                        <div className={styles.dreamCandidates}>
                          {dreamResult.candidates.slice(0, 10).map((c, i) => <div key={i} className={styles.dreamCandidateItem}>
                              <span className={styles.dreamCandidateKey}>{c.key}</span>
                              <span className={styles.dreamCandidateValue}>{c.value.slice(0, 80)}</span>
                              <div className={styles.dreamCandidateMeta}>
                                <span>{c.category}</span>
                                <span>{t("Xin.k107")} {(c.confidence * 100).toFixed(0)}%</span>
                                <span>{t("Xin.k108")} {c.importance.toFixed(2)}</span>
                              </div>
                            </div>)}
                        </div>
                      </div>}
                    {dreamResult.promoted !== undefined && <div style={{
              fontSize: '11px',
              color: '#c8c8dd'
            }}>
                        {t("Xin.k109")} {dreamResult.promoted} {t("Xin.k110")} {((dreamResult.health_score || 0) * 100).toFixed(0)}{t("Xin.k111")} {dreamResult.health_status}
                      </div>}
                    {dreamResult.pattern_count !== undefined && <div style={{
              fontSize: '11px',
              color: '#c8c8dd'
            }}>
                        {t("Xin.k112")} {dreamResult.pattern_count} {t("Xin.k113")} {dreamResult.cross_domain_links?.length || 0}
                      </div>}
                  </div>}

                {!dreamConfig && !dreamLoading && <div className={styles.card}>
                    <p style={{
              color: '#6a6a8a',
              textAlign: 'center'
            }}>
                      {t("Xin.k114")}
                    </p>
                  </div>}
              </>}
          </div>}

        {/* ========== CHECKPOINT TAB ========== */}
        {activeTab === 'checkpoint' && <div className={styles.tabContent}>
            <div style={{
          display: 'flex',
          gap: '8px',
          marginBottom: '12px'
        }}>
              <button className={styles.btn} onClick={saveCheckpoint} disabled={!activeConversationId}>
                {t("Xin.k115")}
              </button>
              <button className={styles.btnSmall} onClick={loadCheckpoints}>
                {t("common.refresh")}
              </button>
            </div>

            {!activeConversationId && <div className={styles.card}>
                <p style={{
            color: '#6a6a8a',
            textAlign: 'center'
          }}>
                  {t("Xin.k116")}
                </p>
              </div>}

            {activeConversationId && checkpoints.length === 0 && !checkpointLoading && <div className={styles.card}>
                <p style={{
            color: '#6a6a8a',
            textAlign: 'center'
          }}>{t("Xin.k117")}</p>
              </div>}

            {checkpointLoading && <div className={styles.loading}>{t("Xin.k118")}</div>}

            {checkpoints.map(cp => <div key={cp.id} className={styles.card} style={{
          marginBottom: '8px'
        }}>
                <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            marginBottom: '6px'
          }}>
                  <span style={{
              fontSize: '12px',
              color: '#c8c8dd'
            }}>
                    {cp.title || t("Xin.k6", {
                arg0: time.formatCompact(cp.created_at)
              })}
                  </span>
                  <span className={cp.checkpoint_type === 'auto' ? styles.badgeOn : styles.badgeOn} style={cp.checkpoint_type === 'user' ? {
              background: 'rgba(255,180,84,0.1)',
              border: '1px solid #FFD700',
              color: '#FFD700'
            } : {}}>
                    {cp.checkpoint_type === 'auto' ? t("Xin.k119") : t("components.FloatingBall.k67")}
                  </span>
                </div>
                <div style={{
            fontSize: '10px',
            color: '#6a6a8a',
            marginBottom: '8px'
          }}>
                  {t("Xin.k79")} {cp.message_count} | Token: {cp.total_tokens.toLocaleString()} | {time.formatCompact(cp.created_at)}
                </div>
                <div style={{
            display: 'flex',
            gap: '6px'
          }}>
                  <button className={styles.btnSmall} onClick={() => restoreCheckpoint(cp.id)}>
                    {t("Xin.k120")}
                  </button>
                  <button className={styles.btnDangerSmall} onClick={() => deleteCheckpoint(cp.id)}>
                    {t("common.delete")}
                  </button>
                </div>
              </div>)}
          </div>}

        {/* ========== SEARCH TAB ========== */}
        {activeTab === 'search' && <div className={styles.tabContent}>
            <div className={styles.searchRow}>
              <input className={styles.input} placeholder={t("Xin.k121")} value={searchQuery} onChange={e => setSearchQuery(e.target.value)} onKeyDown={e => {
            if (e.key === 'Enter') doSearch();
          }} style={{
            flex: '1'
          }} />
              <button className={styles.btn} onClick={doSearch} disabled={searchLoading}>
                {searchLoading ? t("ai.ConversationList.k4") : t("Xin.k122")}
              </button>
            </div>

            {searchLoading && <div className={styles.loading}>{t("ai.ConversationList.k4")}</div>}

            {!searchLoading && searchResults.length === 0 && searchQuery && <div style={{
          textAlign: 'center',
          padding: '20px',
          color: '#6a6a8a'
        }}>{t("components.Linux.k34")}</div>}

            {searchTotal > 0 && <div style={{
          fontSize: '10px',
          color: '#6a6a8a',
          marginBottom: '8px'
        }}>
                {t("components.GroupChatOrchestrationPanel.k26")} {searchTotal} {t("Xin.k123")}
              </div>}

            {searchResults.map(r => <div key={r.conversation_id} className={styles.card} style={{
          marginBottom: '8px'
        }}>
                <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            marginBottom: '4px'
          }}>
                  <span style={{
              fontSize: '12px',
              color: '#00F0FF'
            }}>{r.title}</span>
                  <span style={{
              fontSize: '10px',
              color: '#FFD700'
            }}>
                    {t("Xin.k124")} {(r.relevance_score * 100).toFixed(0)}%
                  </span>
                </div>
                <div style={{
            fontSize: '10px',
            color: '#6a6a8a',
            marginBottom: '6px'
          }}>
                  {r.message_count}{t("Xin.k125")} {r.total_tokens} tokens | {time.formatCompact(r.created_at)}
                </div>
                {r.matched_snippets.slice(0, 3).map((snippet, i) => <div key={i} className={styles.searchSnippet}>
                    <span>...</span><span>{snippet}</span><span>...</span>
                  </div>)}
                <button className={styles.btnSmall} style={{
            marginTop: '6px'
          }} onClick={() => {
            switchConversation(r.conversation_id);
            setActiveTab('chat');
            navigate('/xin?tab=chat', {
              replace: true
            });
          }}>
                  {t("Xin.k126")}
                </button>
              </div>)}

            {!searchQuery && <div style={{
          textAlign: 'center',
          padding: '40px',
          color: '#6a6a8a'
        }}>
                <p style={{
            fontSize: '13px'
          }}>{t("Xin.k127")}</p>
                <p style={{
            fontSize: '11px'
          }}>{t("Xin.k128")}</p>
              </div>}
          </div>}

        {/* ========== REVIEW TAB ========== */}
        {activeTab === 'review' && <div className={styles.tabContent}>
            <div style={{
          display: 'flex',
          gap: '8px',
          marginBottom: '12px',
          alignItems: 'center'
        }}>
              <span className={styles.inputLabel}>{t("Xin.k129")}</span>
              <select className={styles.select} value={reviewPeriod} onChange={e => {
            setReviewPeriod(e.target.value);
            loadReview();
          }}>
                <option value="day">{t("components.FocusMode.k9")}</option>
                <option value="week">{t("Xin.k130")}</option>
                <option value="month">{t("Xin.k131")}</option>
                <option value="year">{t("Xin.k132")}</option>
              </select>
              <button className={styles.btn} onClick={loadReview} disabled={reviewLoading}>
                {reviewLoading ? t("components.intelligence.DashboardPanel.k67") : t("Xin.k133")}
              </button>
            </div>

            {reviewLoading && <div className={styles.loading}>{t("Xin.k134")}</div>}

            {review && <>
                <div className={styles.card}>
                  <div className={styles.panelTitle}>
                    {t("Xin.k135")} {review.period_label}
                  </div>
                  <p style={{
              fontSize: '11px',
              color: '#6a6a8a',
              marginBottom: '8px'
            }}>
                    {review.date_from} → {review.date_to}
                  </p>
                  <div className={styles.settingsGrid}>
                    <div className={styles.settingItem}>
                      <span className={styles.inputLabel}>{t("Xin.k136")}</span>
                      <span className={styles.badgeOn}>{review.conversation_count}</span>
                    </div>
                    <div className={styles.settingItem}>
                      <span className={styles.inputLabel}>{t("Xin.k137")}</span>
                      <span className={styles.badgeOn}>{review.total_messages}</span>
                    </div>
                    <div className={styles.settingItem}>
                      <span className={styles.inputLabel}>Token</span>
                      <span>{review.total_tokens.toLocaleString()}</span>
                    </div>
                    <div className={styles.settingItem}>
                      <span className={styles.inputLabel}>{t("components.intelligence.BehaviorPanel.k8")}</span>
                      <span>{review.most_active_hours.map(h => `${h}h`).join(', ')}</span>
                    </div>
                  </div>
                  {review.generated_summary && <div style={{
              marginTop: '10px',
              padding: '10px',
              background: 'rgba(0,240,255,0.02)',
              borderRadius: '4px',
              border: '1px solid rgba(0,240,255,0.15)'
            }}>
                      <span style={{
                fontSize: '10px',
                color: '#00F0FF'
              }}>{t("Xin.k138")}</span>
                      <p style={{
                fontSize: '11px',
                color: '#c8c8dd',
                margin: '6px 0 0',
                lineHeight: '1.5'
              }}>
                        {review.generated_summary}
                      </p>
                    </div>}
                  {review.dominant_topics.length > 0 && <div style={{
              marginTop: '8px'
            }}>
                      <span style={{
                fontSize: '10px',
                color: '#6a6a8a'
              }}>{t("Xin.k139")}</span>
                      <div style={{
                display: 'flex',
                gap: '6px',
                flexWrap: 'wrap',
                marginTop: '4px'
              }}>
                        {review.dominant_topics.map((t, i) => <span key={i} style={{
                  fontSize: '10px',
                  padding: '2px 8px',
                  background: 'rgba(255,180,84,0.1)',
                  borderRadius: '3px',
                  color: '#FFD700',
                  border: '1px solid rgba(255,180,84,0.2)'
                }}>
                            {t.topic} ({t.count})
                          </span>)}
                      </div>
                    </div>}
                </div>

                {topicTrends.length > 0 && <div className={styles.card}>
                    <div className={styles.panelTitle}>{t("Xin.k140")}</div>
                    {topicTrends.slice(0, 5).map((trend, i) => <div key={i} style={{
              marginBottom: '8px'
            }}>
                        <span style={{
                fontSize: '10px',
                color: '#00F0FF'
              }}>{trend.topic}</span>
                        <div className={styles.trendLine}>
                          {trend.data_points.map((pt, j) => <div key={j} className={styles.trendPointWrapper} title={`${pt.date_label}: ${pt.value.toFixed(1)}`}>
                              <div className={styles.trendPoint} style={{
                    height: `${Math.max(pt.value * 40, 4)}px`
                  }} />
                              <span className={styles.trendLabel}>{pt.date_label.slice(-2)}</span>
                            </div>)}
                        </div>
                      </div>)}
                  </div>}

                {growthTrajectory && <div className={styles.card}>
                    <div className={styles.panelTitle}>{t("Xin.k141")}</div>
                    <div className={styles.settingsGrid}>
                      {[{
                label: t("Xin.k142"),
                data: growthTrajectory.emotion_trend
              }, {
                label: t("Xin.k143"),
                data: growthTrajectory.empathy_trend
              }, {
                label: t("lib.xinChatEngine.k68"),
                data: growthTrajectory.creativity_trend
              }, {
                label: t("ai.ModelManager.k38"),
                data: growthTrajectory.conversation_frequency
              }, {
                label: 'Token',
                data: growthTrajectory.token_usage_trend
              }].map(({
                label,
                data
              }) => <div key={label} className={styles.settingItem} style={{
                flexDirection: 'column',
                alignItems: 'flex-start',
                gap: '4px'
              }}>
                          <span className={styles.inputLabel}>{label}</span>
                          <div className={styles.miniTrend}>
                            {data.map((pt, j) => <div key={j} className={styles.trendPointWrapper}>
                                <div className={styles.trendPoint} style={{
                      height: `${Math.max(pt.value * 20, 2)}px`
                    }} />
                              </div>)}
                          </div>
                          <span style={{
                  fontSize: '9px',
                  color: '#6a6a8a'
                }}>
                            {data[0]?.date_label} → {data[data.length - 1]?.date_label}
                          </span>
                        </div>)}
                    </div>
                  </div>}

                {heatmap && <div className={styles.card}>
                    <div className={styles.panelTitle}>{t("Xin.k144")}</div>
                    <div className={styles.heatmapGrid}>
                      {[t("Xin.k145"), t("Xin.k146"), t("Xin.k147"), t("Xin.k148"), t("Xin.k149"), t("Xin.k150"), t("Xin.k151")].map((day, d) => <div key={day} className={styles.heatmapRow}>
                          <span className={styles.heatmapDayLabel}>{day}</span>
                          {Array.from({
                  length: 24
                }, (_, h) => {
                  const cell = heatmap.cells.find(c => c.day_of_week === d && c.hour === h);
                  const intensity = cell ? cell.intensity / Math.max(heatmap.max_intensity, 1) : 0;
                  return <div key={h} className={styles.heatmapCell} style={{
                    background: `rgba(0, 240, 255, ${Math.min(intensity * 0.8, 0.8)})`
                  }} title={t("Xin.k152", {
                    day: day,
                    h: h,
                    arg0: cell?.count || 0
                  })} />;
                })}
                        </div>)}
                    </div>
                    <div style={{
              fontSize: '9px',
              color: '#6a6a8a',
              marginTop: '6px',
              textAlign: 'center'
            }}>
                      {t("Xin.k153")} {heatmap.max_intensity}{t("ai.ChatPanel.k19")}
                    </div>
                  </div>}
              </>}

            {!review && !reviewLoading && <div style={{
          textAlign: 'center',
          padding: '40px',
          color: '#6a6a8a'
        }}>
                <p style={{
            fontSize: '13px'
          }}>{t("Xin.k154")}</p>
                <p style={{
            fontSize: '11px'
          }}>{t("Xin.k155")}</p>
              </div>}
          </div>}

        {/* ========== SKILL TAB ========== */}
        {activeTab === 'skill' && <div className={styles.tabContent}>
            {skills.length === 0 ? <div className={styles.loading}>{t("Xin.k156")}</div> : <>
                <div className={styles.card}>
                  <div className={styles.panelTitle}>{t("Xin.k157")}</div>
                  <div style={{
              display: 'flex',
              gap: '8px',
              marginBottom: '12px'
            }}>
                    <input className={styles.input} placeholder={t("Xin.k158")} value={skillInput} onChange={e => setSkillInput(e.target.value)} style={{
                flex: '1'
              }} />
                  </div>
                  <div className={styles.skillGrid}>
                    {skills.map(s => <div key={s.id} className={styles.skillCard} style={{
                opacity: s.available ? 1 : 0.4
              }}>
                        <div style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center'
                }}>
                          <span style={{
                    fontSize: '12px',
                    color: '#00F0FF'
                  }}>{s.name}</span>
                          <span className={styles.skillCategory}>{s.category}</span>
                        </div>
                        <p style={{
                  fontSize: '10px',
                  color: '#8a8aaa',
                  margin: '4px 0'
                }}>{s.description}</p>
                        <button className={styles.btnSmall} onClick={() => executeSkill(s.id)} disabled={!s.available || skillExecLoading}>
                          {skillExecLoading ? t("Xin.k159") : t("Xin.k160")}
                        </button>
                      </div>)}
                  </div>
                </div>

                {skillResult && <div className={styles.card}>
                    <div className={styles.panelTitle}>{t("Xin.k161")}</div>
                    <div style={{
              fontSize: '11px',
              color: '#c8c8dd',
              marginBottom: '4px'
            }}>
                      <span style={{
                color: '#6a6a8a'
              }}>{t("Xin.k162")} {skillResult.skill_id}</span>
                      <span style={{
                marginLeft: '12px',
                color: skillResult.success ? '#00F0FF' : '#FF006E'
              }}>
                        {skillResult.success ? t("Xin.k163") : t("Xin.k164")}
                      </span>
                    </div>
                    {skillResult.output && <pre className={styles.resultPre}>{skillResult.output}</pre>}
                    {skillResult.error && <div style={{
              fontSize: '10px',
              color: '#FF006E'
            }}>{skillResult.error}</div>}
                  </div>}
              </>}
          </div>}

        {/* ========== TOOL TAB ========== */}
        {activeTab === 'tool' && <div className={styles.tabContent}>
            {tools.length === 0 ? <div className={styles.loading}>{t("Xin.k165")}</div> : <>
                <div className={styles.card}>
                  <div className={styles.panelTitle}>{t("Xin.k166")}</div>
                  <div className={styles.toolGrid}>
                    {tools.map((t, i) => <div key={i} className={styles.toolCard}>
                        <div style={{
                  display: 'flex',
                  justifyContent: 'space-between'
                }}>
                          <span style={{
                    fontSize: '12px',
                    color: '#00F0FF'
                  }}>{t.name}</span>
                          <span className={styles.skillCategory}>{t.category}</span>
                        </div>
                        <p style={{
                  fontSize: '10px',
                  color: '#8a8aaa',
                  margin: '4px 0'
                }}>{t.description}</p>
                        {t.parameters.length > 0 && <div style={{
                  display: 'flex',
                  gap: '4px',
                  flexWrap: 'wrap'
                }}>
                            {t.parameters.map((p, j) => <span key={j} style={{
                    fontSize: '8px',
                    padding: '1px 5px',
                    border: `1px solid ${p.required ? '#FFD700' : 'rgba(176,38,255,0.2)'}`,
                    borderRadius: '2px',
                    color: p.required ? '#FFD700' : '#6a6a8a'
                  }}>
                                {p.name}:{p.param_type}{p.required ? '*' : ''}
                              </span>)}
                          </div>}
                      </div>)}
                  </div>
                </div>

                <div className={styles.card}>
                  <div className={styles.panelTitle}>{t("Xin.k167")}</div>
                  <div style={{
              display: 'flex',
              gap: '8px',
              marginBottom: '8px'
            }}>
                    <textarea className={styles.input} placeholder={t("Xin.k168")} value={toolInputText} onChange={e => setToolInputText(e.target.value)} rows={3} style={{
                flex: '1',
                resize: 'vertical',
                fontFamily: 'inherit'
              }} />
                  </div>
                  <div style={{
              display: 'flex',
              gap: '8px'
            }}>
                    <button className={styles.btn} onClick={parseToolCalls}>{t("Xin.k169")}</button>
                    <button className={styles.btn} onClick={executeToolCalls} disabled={parsedCalls.length === 0 || toolExecLoading}>
                      {toolExecLoading ? t("Xin.k159") : t("Xin.k170")}
                    </button>
                  </div>
                  {parsedCalls.length > 0 && <div style={{
              marginTop: '8px'
            }}>
                      <span style={{
                fontSize: '10px',
                color: '#6a6a8a'
              }}>{t("Xin.k171")} {parsedCalls.length} {t("Xin.k172")}</span>
                      {parsedCalls.map((call, i) => <div key={i} style={{
                fontSize: '10px',
                color: '#c8c8dd',
                padding: '4px 8px',
                background: 'rgba(0,240,255,0.02)',
                border: '1px solid rgba(0,240,255,0.15)',
                borderRadius: '3px',
                marginTop: '4px'
              }}>
                          <span style={{
                  color: '#00F0FF'
                }}>{call.tool_name}</span>
                          {Object.entries(call.arguments).map(([k, v]) => <span key={k} style={{
                  marginLeft: '8px',
                  fontSize: '9px',
                  color: '#8a8aaa'
                }}>{k}={v}</span>)}
                        </div>)}
                    </div>}
                </div>

                {toolResults.length > 0 && <div className={styles.card}>
                    <div className={styles.panelTitle}>{t("Xin.k161")}</div>
                    {toolResults.map((r, i) => <div key={i} style={{
              marginBottom: '8px',
              padding: '8px',
              background: 'rgba(0,240,255,0.02)',
              border: `1px solid ${r.success ? 'rgba(0,240,255,0.15)' : 'rgba(255,0,110,0.15)'}`,
              borderRadius: '4px'
            }}>
                        <div style={{
                fontSize: '11px',
                marginBottom: '4px'
              }}>
                          <span style={{
                  color: '#00F0FF'
                }}>{r.tool_name}</span>
                          <span style={{
                  marginLeft: '12px',
                  color: r.success ? '#00F0FF' : '#FF006E'
                }}>
                            {r.success ? '✓' : '✗'}
                          </span>
                          <span style={{
                  marginLeft: '8px',
                  fontSize: '9px',
                  color: '#6a6a8a'
                }}>{r.duration_ms}ms</span>
                        </div>
                        {r.output && <pre className={styles.resultPre} style={{
                maxHeight: '120px'
              }}>{r.output.slice(0, 500)}</pre>}
                        {r.error && <div style={{
                fontSize: '10px',
                color: '#FF006E'
              }}>{r.error}</div>}
                      </div>)}
                  </div>}

                <div className={styles.card}>
                  <div className={styles.panelTitle}>{t("Xin.k173")}</div>
                  <div className={styles.searchRow}>
                    <input className={styles.input} placeholder={t("Xin.k174")} value={fusionQuery} onChange={e => setFusionQuery(e.target.value)} onKeyDown={e => {
                if (e.key === 'Enter') doFusionQuery();
              }} style={{
                flex: '1'
              }} />
                    <button className={styles.btn} onClick={doFusionQuery} disabled={fusionLoading}>
                      {fusionLoading ? t("Xin.k175") : t("Xin.k176")}
                    </button>
                  </div>
                  {fusionKeywords.length > 0 && <div style={{
              display: 'flex',
              gap: '4px',
              flexWrap: 'wrap',
              marginBottom: '8px'
            }}>
                      <span style={{
                fontSize: '9px',
                color: '#6a6a8a'
              }}>{t("Xin.k177")} </span>
                      {fusionKeywords.map((kw, i) => <span key={i} style={{
                fontSize: '9px',
                padding: '1px 6px',
                background: 'rgba(0,240,255,0.08)',
                borderRadius: '2px',
                color: '#00F0FF',
                border: '1px solid rgba(0,240,255,0.2)'
              }}>{kw}</span>)}
                    </div>}
                  {fusionResults.length > 0 && <div>
                      {fusionResults.map((r, i) => <div key={i} style={{
                padding: '8px',
                marginBottom: '6px',
                background: 'rgba(0,240,255,0.02)',
                border: '1px solid rgba(0,240,255,0.15)',
                borderRadius: '4px'
              }}>
                          <div style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  marginBottom: '4px'
                }}>
                            <span style={{
                    fontSize: '10px',
                    color: '#00F0FF'
                  }}>{r.source}</span>
                            <span style={{
                    fontSize: '9px',
                    color: '#FFD700'
                  }}>
                              score: {typeof r.score === 'number' ? r.score.toFixed(2) : r.score}
                            </span>
                          </div>
                          <p style={{
                  fontSize: '10px',
                  color: '#c8c8dd',
                  lineHeight: '1.4'
                }}>{r.content.slice(0, 200)}</p>
                        </div>)}
                    </div>}
                </div>
              </>}
          </div>}

        {/* ========== EVOLUTION TAB ========== */}
        {activeTab === 'evolution' && <div className={styles.tabContent}>
            <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k178")}</div>
              <div style={{
            display: 'flex',
            gap: '8px',
            marginBottom: '12px'
          }}>
                <button className={evolutionTab === 'favorability' ? styles.btn : styles.btnSmall} onClick={() => setEvolutionTab('favorability')}>
                  {t("Xin.k179")}
                </button>
                <button className={evolutionTab === 'radar' ? styles.btn : styles.btnSmall} onClick={() => setEvolutionTab('radar')}>
                  {t("Xin.k180")}
                </button>
              </div>

              {/* 好感度变化曲线 */}
              {evolutionTab === 'favorability' && <div className={styles.evoChartContainer}>
                  <svg viewBox="0 0 400 200" className={styles.evoChartSvg}>
                    {/* 网格线 */}
                    {[0, 1, 2, 3, 4].map(i => <line key={`h${i}`} x1={50} y1={20 + i * 40} x2={380} y2={20 + i * 40} stroke="rgba(0,255,100,0.08)" strokeWidth="0.5" />)}
                    {[0, 1, 2, 3, 4, 5, 6].map(i => <line key={`v${i}`} x1={50 + i * 55} y1={20} x2={50 + i * 55} y2={180} stroke="rgba(0,255,100,0.08)" strokeWidth="0.5" />)}
                    {/* Y轴标签 */}
                    {[100, 75, 50, 25, 0].map((v, i) => <text key={`yl${i}`} x={40} y={24 + i * 40} fill="rgba(0,255,100,0.4)" fontSize="8" textAnchor="end">{v}</text>)}
                    {/* X轴标签 */}
                    {[t("Xin.k181"), t("Xin.k182"), t("Xin.k183"), t("Xin.k184"), t("Xin.k185"), t("Xin.k186"), t("Xin.k187")].map((d, i) => <text key={`xl${i}`} x={50 + i * 55} y={195} fill="rgba(0,255,100,0.4)" fontSize="7" textAnchor="middle">{d}</text>)}
                    {/* 曲线 */}
                    <polyline points="50,140 105,120 160,100 215,80 270,60 325,50 380,40" fill="none" stroke="#00FF64" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                    {/* 发光效果 */}
                    <polyline points="50,140 105,120 160,100 215,80 270,60 325,50 380,40" fill="none" stroke="rgba(0,255,100,0.3)" strokeWidth="6" strokeLinecap="round" strokeLinejoin="round" />
                    {/* 数据点 */}
                    {[[50, 140], [105, 120], [160, 100], [215, 80], [270, 60], [325, 50], [380, 40]].map(([cx, cy], i) => <circle key={`dp${i}`} cx={cx} cy={cy} r="3" fill="#00FF64" />)}
                  </svg>
                  <div className={styles.evoChartLabel}>{t("Xin.k188")}</div>
                </div>}

              {/* 人格特质雷达图 */}
              {evolutionTab === 'radar' && <div className={styles.evoChartContainer}>
                  <svg viewBox="0 0 300 280" className={styles.evoChartSvg}>
                    {/* 同心五边形 */}
                    {[1, 2, 3, 4].map(level => {
                const r = 25 + level * 25;
                const traits = [t("Xin.k189"), t("Xin.k190"), t("Xin.k191"), t("Xin.k192"), t("Xin.k193")];
                const points = traits.map((_, i) => {
                  const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
                  return `${150 + r * Math.cos(angle)},${140 + r * Math.sin(angle)}`;
                }).join(' ');
                return <polygon key={`grid${level}`} points={points} fill="none" stroke="rgba(0,255,100,0.08)" strokeWidth="0.5" />;
              })}
                    {/* 轴线 */}
                    {[0, 0.2, 0.4, 0.6, 0.8, 1.0].map((_, i) => {
                const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
                return <line key={`axis${i}`} x1={150} y1={140} x2={150 + 125 * Math.cos(angle)} y2={140 + 125 * Math.sin(angle)} stroke="rgba(0,255,100,0.1)" strokeWidth="0.5" />;
              })}
                    {/* 数据多边形 */}
                    <polygon points={(() => {
                const traits = [t("Xin.k189"), t("Xin.k190"), t("Xin.k191"), t("Xin.k192"), t("Xin.k193")];
                const values = [0.85, 0.7, 0.6, 0.75, 0.9];
                return traits.map((_, i) => {
                  const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
                  const r = 25 + values[i] * 100;
                  return `${150 + r * Math.cos(angle)},${140 + r * Math.sin(angle)}`;
                }).join(' ');
              })()} fill="rgba(0,255,100,0.1)" stroke="#00FF64" strokeWidth="1.5" />
                    {/* 发光多边形 */}
                    <polygon points={(() => {
                const traits = [t("Xin.k189"), t("Xin.k190"), t("Xin.k191"), t("Xin.k192"), t("Xin.k193")];
                const values = [0.85, 0.7, 0.6, 0.75, 0.9];
                return traits.map((_, i) => {
                  const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
                  const r = 25 + values[i] * 100;
                  return `${150 + r * Math.cos(angle)},${140 + r * Math.sin(angle)}`;
                }).join(' ');
              })()} fill="none" stroke="rgba(0,255,100,0.3)" strokeWidth="5" />
                    {/* 数据点 */}
                    {[0.85, 0.7, 0.6, 0.75, 0.9].map((v, i) => {
                const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
                const r = 25 + v * 100;
                return <circle key={`rp${i}`} cx={150 + r * Math.cos(angle)} cy={140 + r * Math.sin(angle)} r="3" fill="#00FF64" />;
              })}
                    {/* 标签 */}
                    {[t("Xin.k189"), t("Xin.k190"), t("Xin.k191"), t("Xin.k192"), t("Xin.k193")].map((trait, i) => {
                const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
                const lr = 140;
                const x = 150 + lr * Math.cos(angle);
                const y = 140 + lr * Math.sin(angle);
                return <text key={`tl${i}`} x={x} y={y} fill="rgba(0,255,100,0.6)" fontSize="9" textAnchor="middle" dominantBaseline="middle">
                          {trait}
                        </text>;
              })}
                  </svg>
                  <div className={styles.evoChartLabel}>{t("Xin.k180")}</div>
                </div>}
            </div>
          </div>}

        {/* ========== PERSONA PANEL (Shown in Chat Tab when sidebar collapsed) ========== */}
        {activeTab === 'chat' && !showingConversationSidebar && <div className={styles.personaPanel}>
            <div className={styles.panelTitle}>{t("Xin.k194")}</div>
            <div className={styles.personaList}>
              {personas.map(p => <button key={p.id} className={`${styles.personaCard} ${activePersona.id === p.id ? styles.personaActive : ''}`} onClick={() => switchPersona(p)}>
                  <span className={styles.personaEmoji}>{p.emoji}</span>
                  <span className={styles.personaName}>{p.name}</span>
                  {activePersona.id === p.id && <span className={styles.activeBadge}>{t("game3d.KnowledgeMapping.k27")}</span>}
                </button>)}
            </div>

            <div className={styles.activePersonaDetail}>
              <p className={styles.personaDesc}>{activePersona.description}</p>
              <div className={styles.traitBar}>
                {activePersona.traits.map(t => <div key={t.name} className={styles.traitItem}>
                    <span className={styles.traitLabel}>{t.name}</span>
                    <div className={styles.traitTrack}>
                      <div className={styles.traitFill} style={{
                  width: `${t.value}%`
                }} />
                    </div>
                  </div>)}
              </div>
            </div>

            {PERSONA_KNOWLEDGE[activePersona.id] && <div style={{
          padding: '12px 0 0',
          borderTop: '1px solid #1F2A35'
        }}>
                <div style={{
            fontSize: '12px',
            color: '#7A828E',
            marginBottom: '8px'
          }}>{t("Xin.k195")}</div>
                <div style={{
            display: 'flex',
            flexWrap: 'wrap',
            gap: '4px'
          }}>
                  {PERSONA_KNOWLEDGE[activePersona.id].slice(0, 12).map(tag => <span key={tag} style={{
              fontSize: '10px',
              padding: '2px 8px',
              background: 'rgba(255,180,84,0.1)',
              borderRadius: '10px',
              color: '#FFB454'
            }}>
                      {tag}
                    </span>)}
                </div>
              </div>}
          </div>}
      </div>
    </div>;
}