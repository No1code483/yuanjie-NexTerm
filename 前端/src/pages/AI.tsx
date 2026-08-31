import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import { ai } from '@/lib/ipc';
import { listen } from '@tauri-apps/api/event';
import { GroupChatOrchestrationPanel } from '@/components/GroupChatOrchestrationPanel';
import type { AIModel, Conversation, Message, BackendAgent, Participant, GroupProgress, ContextMenuState } from './ai/types';
import { getSenderDisplayName } from './ai/utils';
import ModelManager from './ai/ModelManager';
import AgentManager from './ai/AgentManager';
import ChatPanel from './ai/ChatPanel';
import MultiModelChatPanel from './ai/MultiModelChatPanel';
import ConversationList from './ai/ConversationList';
import GroupChatList from './ai/GroupChatList';
import ConversationModal from './ai/ConversationModal';
import GroupModal from './ai/GroupModal';
import { ModelDetail } from './ai/ModelManager';
import styles from './AI.module.css';
export default function AI() {
  const location = useLocation();
  const getCurrentTab = useCallback(() => {
    const searchParams = new URLSearchParams(location.search);
    return searchParams.get('tab') || 'model';
  }, [location.search]);
  const currentTab = getCurrentTab();

  // ===== 共享状态 =====
  const [models, setModels] = useState<AIModel[]>([]);
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [agents, setAgents] = useState<BackendAgent[]>([]);
  const [selectedModel, setSelectedModel] = useState<AIModel | null>(null);
  const [activeChat, setActiveChat] = useState<number | null>(null);
  const [messagesMap, setMessagesMap] = useState<Record<number, Message[]>>({});
  const messages = activeChat ? messagesMap[activeChat] || [] : [];
  const [participants, setParticipants] = useState<Map<number, Participant[]>>(new Map());
  const [loadingModels, setLoadingModels] = useState(false);
  const [loadingConversations, setLoadingConversations] = useState(false);
  const [loadingMessages, setLoadingMessages] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  const [isStreaming, setIsStreaming] = useState(false);
  const [messageInput, setMessageInput] = useState('');
  const [errorMsg, setErrorMsg] = useState('');
  const [successMsg, setSuccessMsg] = useState('');

  // 群聊进度
  const [groupProgress, setGroupProgress] = useState<GroupProgress>({
    round: 0,
    maxRounds: 10,
    responded: [],
    totalParticipants: 0,
    status: 'idle'
  });

  // 右键菜单
  const [contextMenu, setContextMenu] = useState<ContextMenuState>({
    visible: false,
    x: 0,
    y: 0,
    messageId: null
  });

  // 多选模式
  const [multiSelectMode, setMultiSelectMode] = useState(false);
  const [selectedMessageIds, setSelectedMessageIds] = useState<Set<number>>(new Set());

  // 模态框控制
  const [showConvModal, setShowConvModal] = useState(false);
  const [showGroupModal, setShowGroupModal] = useState(false);
  const loadingConvIdRef = useRef<number | null>(null);
  const orchestratingConvRef = useRef<number | null>(null);

  // ===== 工具函数 =====
  const showError = (msg: string) => {
    setErrorMsg(msg);
    setTimeout(() => setErrorMsg(''), 4000);
  };
  const showSuccess = (msg: string) => {
    setSuccessMsg(msg);
    setTimeout(() => setSuccessMsg(''), 3000);
  };

  // ===== 数据获取 =====
  const fetchModels = async () => {
    setLoadingModels(true);
    try {
      setModels((await ai.getModels()).data || []);
    } catch (e) {
      showError(t("AI.k1", {
        e: e
      }));
    } finally {
      setLoadingModels(false);
    }
  };
  const fetchConversations = async () => {
    setLoadingConversations(true);
    try {
      const convs = (await ai.getSessions()).data || [];
      setConversations(convs);
      const partsMap = new Map<number, Participant[]>();
      await Promise.all(convs.map(async conv => {
        try {
          const pRes = await ai.getParticipants(conv.id);
          if (pRes.data && pRes.data.length > 0) partsMap.set(conv.id, pRes.data);
        } catch {/* ignore */}
      }));
      if (partsMap.size > 0) setParticipants(prev => {
        const m = new Map(prev);
        partsMap.forEach((v, k) => m.set(k, v));
        return m;
      });
    } catch (e) {
      showError(t("AI.k2", {
        e: e
      }));
    } finally {
      setLoadingConversations(false);
    }
  };
  const fetchAgents = async () => {
    try {
      setAgents((await ai.getAgents()).data || []);
    } catch (e) {
      showError(t("AI.k3", {
        e: e
      }));
    }
  };
  const fetchParticipants = async (conversationId: number): Promise<Participant[]> => {
    try {
      const res = await ai.getParticipants(conversationId);
      const parts = res.data || [];
      setParticipants(prev => new Map(prev).set(conversationId, parts));
      return parts;
    } catch {
      return [];
    }
  };
  const fetchMessages = async (conversationId: number): Promise<Message[] | null> => {
    const thisConvId = conversationId;
    loadingConvIdRef.current = thisConvId;
    setLoadingMessages(true);
    try {
      const res = await ai.getMessages(conversationId);
      if (loadingConvIdRef.current !== thisConvId) return null;
      const msgs = res.data || [];
      setMessagesMap(prev => {
        const existing = prev[conversationId] || [];
        const dbHasUserMsg = msgs.some(m => m.sender_type === 'user' && m.id > 0);
        const cleaned = dbHasUserMsg ? existing.filter(m => !(m.sender_type === 'user' && m.id < 0)) : existing;
        const existingIds = new Set(cleaned.map(m => m.id));
        const newFromDb = msgs.filter(m => !existingIds.has(m.id));
        if (newFromDb.length === 0 && cleaned === existing) return prev;
        return {
          ...prev,
          [conversationId]: [...cleaned, ...newFromDb].sort((a, b) => a.created_at - b.created_at)
        };
      });
      return msgs;
    } catch (e) {
      showError(t("AI.k4", {
        e: e
      }));
      return null;
    } finally {
      if (loadingConvIdRef.current === thisConvId) {
        setLoadingMessages(false);
        loadingConvIdRef.current = null;
      }
    }
  };

  // ===== 初始化 =====
  useEffect(() => {
    fetchModels();
    fetchConversations();
    fetchAgents();
  }, []);

  // ===== activeChat 变化：加载参与者和消息 =====
  useEffect(() => {
    if (activeChat) {
      fetchParticipants(activeChat);
    }
  }, [activeChat, conversations]);
  useEffect(() => {
    if (!activeChat) return;
    const conv = conversations.find(c => c.id === activeChat);
    if (!conv) return;
    const isGroupTab = currentTab === 'group';
    const convIsGroup = conv.type === 'group';
    if (isGroupTab && !convIsGroup || !isGroupTab && convIsGroup) {
      setActiveChat(null);
      setStreamingContent('');
      setIsStreaming(false);
    }
  }, [currentTab, conversations]);
  useEffect(() => {
    if (activeChat) {
      setStreamingContent('');
      if (orchestratingConvRef.current !== activeChat) setIsStreaming(false);
      fetchMessages(activeChat);
      // spec ai-chat-enhancement Phase 2 Task 9.3: 打开会话时清零未读数（乐观更新）
      // 注：ConversationList.handleSelect 也会调用一次（双重调用幂等，后端仅 SET unread_count=0）
      ai.markConversationRead(activeChat).catch(() => {/* ignore */});
      setConversations(prev => prev.map(c => c.id === activeChat ? {
        ...c,
        unread_count: 0
      } : c));
    } else {
      setStreamingContent('');
      setIsStreaming(false);
    }
  }, [activeChat]);

  // ===== 事件监听 =====
  const unlistenAiRef = useRef<(() => void) | null>(null);
  const unlistenOrchRef = useRef<(() => void) | null>(null);
  // spec ai-chat-enhancement Phase 1 Task 6: 模型健康状态变化事件（后台 ModelHealthMonitor 推送）
  const unlistenHealthRef = useRef<(() => void) | null>(null);
  useEffect(() => {
    let cancelled = false;
    const handleAiStream = (event: any) => {
      if (cancelled) return;
      const {
        conversation_id: convId,
        chunk,
        event: evt,
        error,
        user_message
      } = event.payload;
      if (convId !== activeChat) return;
      if (evt === 'generation_start') {
        setIsStreaming(true);
        setStreamingContent('');
      } else if (evt === 'chunk' && chunk) {
        setStreamingContent(prev => prev + chunk);
      } else if (evt === 'generation_complete') {
        setIsStreaming(false);
        setTimeout(() => {
          fetchMessages(convId).then(() => setStreamingContent(''));
        }, 300);
      } else if (evt === 'generation_error') {
        setIsStreaming(false);
        setStreamingContent('');
        showError(user_message || error || t("errors.unknown"));
        fetchMessages(convId);
      }
    };
    const handleOrch = (event: any) => {
      if (cancelled) return;
      const p = event.payload;
      const {
        conversation_id: convId,
        event: evt,
        round,
        participant,
        tokens
      } = p;
      const isActiveConv = convId === activeChat;
      if (evt === 'message_complete' && p.content) {
        const newMsg: Message = {
          id: p.message_id ?? Date.now(),
          conversation_id: convId,
          sender_type: 'model',
          sender_id: p.sender_id ?? null,
          content: p.content,
          round: round ?? 0,
          created_at: p.created_at ?? Date.now()
        };
        setMessagesMap(prev => ({
          ...prev,
          [convId]: [...(prev[convId] || []), newMsg]
        }));
        if (isActiveConv) {
          setStreamingContent(t("AI.k5", {
            participant: participant,
            tokens: tokens
          }));
          if (participant) setGroupProgress(prev => ({
            ...prev,
            status: 'responding',
            responded: prev.responded.includes(participant) ? prev.responded : [...prev.responded, participant]
          }));
        }
        return;
      }
      if (evt === 'round_start') {
        if (isActiveConv) {
          setStreamingContent(t("AI.k6", {
            round: round
          }));
          const parts = participants.get(convId) || [];
          setGroupProgress(prev => ({
            ...prev,
            round: round ?? prev.round + 1,
            responded: [],
            totalParticipants: parts.length,
            status: 'round_start'
          }));
        }
        return;
      }
      if (evt === 'converged' || evt === 'round_limit' || evt === 'token_exhausted' || evt === 'force_stopped') {
        if (convId === orchestratingConvRef.current) orchestratingConvRef.current = null;
        if (isActiveConv) {
          setIsStreaming(false);
          setStreamingContent(evt === 'force_stopped' ? t("AI.k7") : '');
          setGroupProgress(prev => ({
            ...prev,
            status: evt === 'force_stopped' ? 'stopped' : 'converged',
            responded: prev.totalParticipants > 0 ? Array.from({
              length: prev.totalParticipants
            }, (_, i) => `P${i + 1}`) : prev.responded
          }));
          if (evt !== 'force_stopped') {
            setTimeout(() => {
              if (convId === activeChat) fetchMessages(convId).then(() => {
                setStreamingContent('');
                setGroupProgress(p => ({
                  ...p,
                  status: 'idle'
                }));
              });
            }, 500);
          }
        }
        return;
      }
      if (evt === 'token_warning' && isActiveConv) {
        setStreamingContent(t("AI.k8", {
          arg0: Math.round((tokens || 0) * 100)
        }));
      }
    };
    listen<{
      conversation_id: number;
      chunk?: string;
      event?: string;
      error?: string;
      user_message?: string;
      suggestion?: string;
      error_code?: number;
      category?: string;
    }>('ai-stream', handleAiStream).then(fn => {
      if (cancelled) fn();else unlistenAiRef.current = fn;
    });
    listen<{
      conversation_id: number;
      event: string;
      round?: number;
      participant?: string;
      tokens?: number;
      message_id?: number;
      sender_id?: number | null;
      content?: string;
      created_at?: number;
    }>('ai-orchestrator', handleOrch).then(fn => {
      if (cancelled) fn();else unlistenOrchRef.current = fn;
    });
    return () => {
      cancelled = true;
      if (unlistenAiRef.current) {
        unlistenAiRef.current();
        unlistenAiRef.current = null;
      }
      if (unlistenOrchRef.current) {
        unlistenOrchRef.current();
        unlistenOrchRef.current = null;
      }
    };
  }, [activeChat]);

  // spec ai-chat-enhancement Phase 1 Task 6: 监听 ai-model-health-changed 事件
  // 后台 ModelHealthMonitor 每 15 分钟批量检测后通过此事件推送状态变化，
  // 前端仅更新对应模型的字段，不重新拉取全量模型列表。
  useEffect(() => {
    let cancelled = false;
    const handleHealthChanged = (event: any) => {
      if (cancelled) return;
      const {
        model_id,
        status,
        latency_ms,
        last_health_check
      } = event.payload;
      setModels(prev => prev.map(m => m.id === model_id ? {
        ...m,
        status,
        latency_ms,
        last_health_check
      } : m));
    };
    listen<{
      model_id: number;
      status: string;
      latency_ms: number | null;
      last_health_check: number | null;
    }>('ai-model-health-changed', handleHealthChanged).then(fn => {
      if (cancelled) fn();
      else unlistenHealthRef.current = fn;
    });
    return () => {
      cancelled = true;
      if (unlistenHealthRef.current) {
        unlistenHealthRef.current();
        unlistenHealthRef.current = null;
      }
    };
  }, []);

  // 关闭右键菜单
  useEffect(() => {
    const closeMenu = () => setContextMenu(prev => ({
      ...prev,
      visible: false
    }));
    if (contextMenu.visible) {
      window.addEventListener('click', closeMenu);
      return () => window.removeEventListener('click', closeMenu);
    }
  }, [contextMenu.visible]);

  // ===== 操作处理 =====
  const handleDeleteModel = async (id: number) => {
    try {
      await ai.deleteModel(id);
      if (selectedModel?.id === id) setSelectedModel(null);
      await fetchModels();
      showSuccess(t("AI.k9"));
    } catch (e) {
      showError(t("AI.k10", {
        e: e
      }));
    }
  };
  const handleDeleteConversation = async (id: number) => {
    try {
      await ai.deleteConversation(id);
      if (activeChat === id) setActiveChat(null);
      setMessagesMap(prev => {
        const n = {
          ...prev
        };
        delete n[id];
        return n;
      });
      await fetchConversations();
      showSuccess(t("AI.k11"));
    } catch (e) {
      showError(t("AI.k12", {
        e: e
      }));
    }
  };
  const handleDeleteMessage = async (messageId: number) => {
    setContextMenu(prev => ({
      ...prev,
      visible: false
    }));
    try {
      await ai.deleteMessage(messageId);
      if (activeChat) setMessagesMap(prev => ({
        ...prev,
        [activeChat]: (prev[activeChat] || []).filter(m => m.id !== messageId)
      }));
      showSuccess(t("AI.k13"));
    } catch (e) {
      showError(t("AI.k14", {
        e: e
      }));
    }
  };
  const handleCopyMessage = (content: string) => {
    setContextMenu(prev => ({
      ...prev,
      visible: false
    }));
    const text = content.replace(/<[^>]+>/g, '').replace(/&nbsp;/g, ' ').trim();
    navigator.clipboard.writeText(text).then(() => showSuccess(t("AI.k15"))).catch(() => showError(t("AI.k16")));
  };
  const handleQuoteMessage = (msg: Message) => {
    setContextMenu(prev => ({
      ...prev,
      visible: false
    }));
    const text = msg.content.replace(/<[^>]+>/g, '').replace(/&nbsp;/g, ' ').trim();
    const parts = participants.get(activeChat!) || [];
    const displayName = getSenderDisplayName(msg, parts, models, agents);
    setMessageInput(prev => prev + `\n> ${displayName}: ${text}\n`);
  };
  const handleToggleMultiSelect = () => {
    setContextMenu(prev => ({
      ...prev,
      visible: false
    }));
    setMultiSelectMode(true);
    if (contextMenu.messageId) setSelectedMessageIds(new Set([contextMenu.messageId]));
  };
  const handleToggleMessageSelect = (messageId: number) => {
    setSelectedMessageIds(prev => {
      const next = new Set(prev);
      if (next.has(messageId)) {
        next.delete(messageId);
        if (next.size === 0) setMultiSelectMode(false);
      } else next.add(messageId);
      return next;
    });
  };
  const handleBatchDelete = async () => {
    if (selectedMessageIds.size === 0) return;
    try {
      for (const id of selectedMessageIds) await ai.deleteMessage(id);
      if (activeChat) setMessagesMap(prev => ({
        ...prev,
        [activeChat]: (prev[activeChat] || []).filter(m => !selectedMessageIds.has(m.id))
      }));
      showSuccess(t("AI.k17", {
        size: selectedMessageIds.size
      }));
      setSelectedMessageIds(new Set());
      setMultiSelectMode(false);
    } catch (e) {
      showError(t("AI.k18", {
        e: e
      }));
    }
  };
  const handleCancelMultiSelect = () => {
    setSelectedMessageIds(new Set());
    setMultiSelectMode(false);
  };
  const handleBranchConversation = async (messageId: number) => {
    setContextMenu(prev => ({
      ...prev,
      visible: false
    }));
    if (!activeChat) return;
    try {
      const res = await ai.branchConversation(activeChat, messageId);
      const newConv = res.data;
      if (newConv) {
        showSuccess(t("AI.k19"));
        await fetchConversations();
        setActiveChat(newConv.id);
      }
    } catch (e) {
      showError(t("AI.k20", {
        e: e
      }));
    }
  };
  const handleSendMessage = async () => {
    if (!messageInput.trim() || !activeChat) return;
    const content = messageInput.trim();
    setMessageInput('');
    const conv = conversations.find(c => c.id === activeChat);
    const isGroup = conv?.type === 'group';
    const tempUserMsg: Message = {
      id: -Date.now(),
      conversation_id: activeChat,
      sender_type: 'user',
      sender_id: null,
      content,
      round: 0,
      created_at: Date.now()
    };
    setMessagesMap(prev => ({
      ...prev,
      [activeChat]: [...(prev[activeChat] || []), tempUserMsg]
    }));
    try {
      if (isGroup) {
        orchestratingConvRef.current = activeChat;
        setIsStreaming(true);
        setStreamingContent(t("AI.k21"));
        const parts = participants.get(activeChat!) || [];
        setGroupProgress({
          round: 1,
          maxRounds: 10,
          responded: [],
          totalParticipants: parts.length,
          status: 'round_start'
        });
        ai.startGroupChat(activeChat, content).then(() => {
          setIsStreaming(false);
          setStreamingContent('');
          orchestratingConvRef.current = null;
        }).catch(e => {
          showError(t("AI.k22", {
            e: e
          }));
          setIsStreaming(false);
          setStreamingContent('');
          orchestratingConvRef.current = null;
          setGroupProgress(p => ({
            ...p,
            status: 'idle'
          }));
        });
      } else {
        await ai.sendMessage(activeChat, content);
      }
    } catch (e) {
      showError(t("AI.k23", {
        e: e
      }));
      setIsStreaming(false);
      setStreamingContent('');
      fetchMessages(activeChat);
    }
  };
  const handleStopGroupChat = async () => {
    const convId = orchestratingConvRef.current;
    if (!convId) return;
    try {
      await ai.endGroupChat(convId);
      setIsStreaming(false);
      setStreamingContent(t("AI.k7"));
      orchestratingConvRef.current = null;
      setTimeout(() => fetchMessages(convId), 500);
    } catch (e) {
      showError(t("AI.k24", {
        e: e
      }));
    }
  };

  // ===== 渲染 =====
  const renderRightPanel = () => {
    if (currentTab === 'chat') {
      // 检查是否为多模型会话
      const convParts = activeChat ? participants.get(activeChat) || [] : [];
      const modelParts = convParts.filter(p => p.model_id);
      const isMultiModel = modelParts.length > 1;
      return <div className={styles.contentArea} style={{
        padding: 0
      }}>
          {activeChat ? isMultiModel ? <MultiModelChatPanel activeChat={activeChat} conversations={conversations} models={models} agents={agents} participants={participants} messages={messages} messageInput={messageInput} setMessageInput={setMessageInput} onSendMessage={handleSendMessage} onRefreshMessages={() => fetchMessages(activeChat)} /> : <ChatPanel activeChat={activeChat} conversations={conversations} models={models} agents={agents} participants={participants} messages={messages} loadingMessages={loadingMessages} isStreaming={isStreaming} streamingContent={streamingContent} messageInput={messageInput} setMessageInput={setMessageInput} onSendMessage={handleSendMessage} onStopGroupChat={handleStopGroupChat} orchestratingConvRef={orchestratingConvRef} groupProgress={groupProgress} contextMenu={contextMenu} setContextMenu={setContextMenu} multiSelectMode={multiSelectMode} selectedMessageIds={selectedMessageIds} onToggleMessageSelect={handleToggleMessageSelect} onDeleteMessage={handleDeleteMessage} onCopyMessage={handleCopyMessage} onQuoteMessage={handleQuoteMessage} onToggleMultiSelect={handleToggleMultiSelect} onBatchDelete={handleBatchDelete} onCancelMultiSelect={handleCancelMultiSelect} onBranchConversation={handleBranchConversation} /> : <div className={styles.emptyState}>
              <div className={styles.emptyStateIcon}>💬</div>
              <div className={styles.emptyStateTitle}>{t("AI.k25")}</div>
              <div className={styles.emptyStateDesc}>{t("AI.k26")}</div>
            </div>}
        </div>;
    }
    if (currentTab === 'group') {
      const activeConv = conversations.find(c => c.id === activeChat);
      const isGroup = activeConv?.type === 'group';
      return <div className={styles.contentArea} style={{
        padding: 0
      }}>
          {!activeChat ? <div className={styles.emptyState}>
              <div className={styles.emptyStateIcon}>👥</div>
              <div className={styles.emptyStateTitle}>{t("AI.k27")}</div>
              <div className={styles.emptyStateDesc}>{t("AI.k28")}</div>
            </div> : !isGroup ? <div className={styles.emptyState}>
              <div className={styles.emptyStateIcon}>👤</div>
              <div className={styles.emptyStateTitle}>{t("AI.k29")}</div>
              <div className={styles.emptyStateDesc}>{t("AI.k30")}</div>
            </div> : <div className={styles.chatContainerFull}>
              <GroupChatOrchestrationPanel sessionId={activeChat} />
              {(() => {
            const parts = participants.get(activeChat) || [];
            const modelNames = parts.filter(p => p.model_id).map(p => {
              const m = models.find(mo => mo.id === p.model_id);
              return {
                name: m?.name || t("ai.AgentManager.k9", {
                  model_id: p.model_id
                }),
                provider: m?.provider || 'unknown'
              };
            });
            return modelNames.length > 0 ? <div className={styles.groupMemberBar}>
                    <span className={styles.groupMemberBarLabel}>{t("AI.k31")}</span>
                    <div className={styles.groupMemberBarList}>
                      {modelNames.map((m, i) => <span key={i} className={`${styles.providerTag} ${styles[`provider${m.provider}` as keyof typeof styles] || ''}`}>{m.name}</span>)}
                    </div>
                  </div> : null;
          })()}
              <ChatPanel activeChat={activeChat} conversations={conversations} models={models} agents={agents} participants={participants} messages={messages} loadingMessages={loadingMessages} isStreaming={isStreaming} streamingContent={streamingContent} messageInput={messageInput} setMessageInput={setMessageInput} onSendMessage={handleSendMessage} onStopGroupChat={handleStopGroupChat} orchestratingConvRef={orchestratingConvRef} groupProgress={groupProgress} contextMenu={contextMenu} setContextMenu={setContextMenu} multiSelectMode={multiSelectMode} selectedMessageIds={selectedMessageIds} onToggleMessageSelect={handleToggleMessageSelect} onDeleteMessage={handleDeleteMessage} onCopyMessage={handleCopyMessage} onQuoteMessage={handleQuoteMessage} onToggleMultiSelect={handleToggleMultiSelect} onBatchDelete={handleBatchDelete} onCancelMultiSelect={handleCancelMultiSelect} onBranchConversation={handleBranchConversation} />
            </div>}
        </div>;
    }

    // model 或 agent tab
    return <div className={styles.contentArea}>
        {currentTab === 'model' ? selectedModel ? <ModelDetail model={selectedModel} onEdit={() => {}} onDelete={() => handleDeleteModel(selectedModel.id)} /> : <div className={styles.emptyState}>
              <div className={styles.emptyStateIcon}>🤖</div>
              <div className={styles.emptyStateTitle}>{t("AI.k32")}</div>
              <div className={styles.emptyStateDesc}>{t("AI.k33")}</div>
            </div> : <div className={styles.emptyState}>
            <div className={styles.emptyStateIcon}>🤖</div>
            <div className={styles.emptyStateTitle}>{t("AI.k34")}</div>
            <div className={styles.emptyStateDesc}>{t("AI.k35")}</div>
          </div>}
      </div>;
  };
  return <div className={styles.container}>
      {successMsg && <div className={styles.toast}>{successMsg}</div>}

      {showConvModal && <ConversationModal models={models} errorMsg={errorMsg} showError={showError} showSuccess={showSuccess} onCreated={() => {
      fetchConversations();
      setShowConvModal(false);
    }} onClose={() => setShowConvModal(false)} />}
      {showGroupModal && <GroupModal models={models} errorMsg={errorMsg} showError={showError} showSuccess={showSuccess} onCreated={() => {
      fetchConversations();
      setShowGroupModal(false);
    }} onClose={() => setShowGroupModal(false)} />}

      <main className={styles.main}>
        <div className={styles.contentGrid}>
          <div className={styles.chatListSection}>
            <h3 className={styles.sectionTitle}>
              {currentTab === 'model' && t("layout.k2")}
              {currentTab === 'agent' && t("layout.k3")}
              {currentTab === 'chat' && t("layout.k4")}
              {currentTab === 'group' && t("layout.k5")}
            </h3>
            <div className={styles.contentArea}>
              {currentTab === 'model' && <ModelManager models={models} loadingModels={loadingModels} selectedModel={selectedModel} onSelectModel={setSelectedModel} onDeleteModel={handleDeleteModel} errorMsg={errorMsg} showError={showError} showSuccess={showSuccess} onModelsChanged={fetchModels} />}
              {currentTab === 'chat' && <ConversationList conversations={conversations} loadingConversations={loadingConversations} activeChat={activeChat} models={models} participants={participants} onSelect={setActiveChat} onDelete={handleDeleteConversation} onCreate={() => setShowConvModal(true)} onRefresh={fetchConversations} />}
              {currentTab === 'group' && <GroupChatList conversations={conversations} loadingConversations={loadingConversations} activeChat={activeChat} models={models} participants={participants} onSelect={setActiveChat} onDelete={handleDeleteConversation} onCreate={() => setShowGroupModal(true)} />}
              {currentTab === 'agent' && <AgentManager agents={agents} models={models} errorMsg={errorMsg} showError={showError} showSuccess={showSuccess} onAgentsChanged={fetchAgents} />}
            </div>
          </div>

          <div className={styles.mainContent}>
            {renderRightPanel()}
          </div>
        </div>
      </main>
    </div>;
}