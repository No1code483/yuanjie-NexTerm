import { t } from "i18next";
import { useRef, useEffect, useState, useCallback } from 'react';
import { listen } from '@tauri-apps/api/event';
import type { AIModel, BackendAgent, Conversation, Message, Participant } from './types';
import { formatRelativeTime, getSenderDisplayName } from './utils';
import MarkdownRenderer from '@/components/MarkdownRenderer';
import styles from '../AI.module.css';
interface PaneStreamingState {
  isStreaming: boolean;
  content: string;
}
interface Props {
  activeChat: number;
  conversations: Conversation[];
  models: AIModel[];
  agents: BackendAgent[];
  participants: Map<number, Participant[]>;
  messages: Message[];
  messageInput: string;
  setMessageInput: React.Dispatch<React.SetStateAction<string>>;
  onSendMessage: () => void;
  onRefreshMessages: () => void;
}
export default function MultiModelChatPanel({
  activeChat,
  conversations,
  models,
  agents,
  participants,
  messages,
  messageInput,
  setMessageInput,
  onSendMessage,
  onRefreshMessages
}: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const parts = participants.get(activeChat) || [];
  const msgEndRef = useRef<HTMLDivElement>(null);
  const modelParts = parts.filter(p => p.model_id);
  const paneCount = modelParts.length;
  const [streamingMap, setStreamingMap] = useState<Map<number, PaneStreamingState>>(new Map());
  const [anyStreaming, setAnyStreaming] = useState(false);
  const [ratios, setRatios] = useState<number[]>(() => Array(paneCount).fill(100 / paneCount));
  const [dragging, setDragging] = useState<number | null>(null);
  const dragStartX = useRef(0);
  const dragStartRatios = useRef<number[]>([]);
  useEffect(() => {
    if (paneCount > 0) {
      setRatios(Array(paneCount).fill(100 / paneCount));
    }
  }, [paneCount]);

  // 监听多模型流式事件
  const unlistenRef = useRef<(() => void) | null>(null);
  const activeChatRef = useRef(activeChat);
  const modelPartsRef = useRef(modelParts);
  const onRefreshMessagesRef = useRef(onRefreshMessages);
  useEffect(() => {
    activeChatRef.current = activeChat;
  }, [activeChat]);
  useEffect(() => {
    modelPartsRef.current = modelParts;
  }, [modelParts]);
  useEffect(() => {
    onRefreshMessagesRef.current = onRefreshMessages;
  }, [onRefreshMessages]);
  useEffect(() => {
    let cancelled = false;
    const handler = (event: any) => {
      if (cancelled) return;
      const {
        conversation_id: convId,
        chunk,
        event: evt,
        sender_id,
        multi_model
      } = event.payload;
      if (convId !== activeChatRef.current) return;
      if (!multi_model && evt !== 'chunk' && evt !== 'model_generation_start' && evt !== 'model_generation_complete' && evt !== 'model_generation_error') {
        return;
      }
      const parts = modelPartsRef.current;
      if (evt === 'generation_start' && multi_model) {
        const newMap = new Map<number, PaneStreamingState>();
        parts.forEach(p => {
          newMap.set(p.id, {
            isStreaming: true,
            content: ''
          });
        });
        setStreamingMap(newMap);
        setAnyStreaming(true);
        return;
      }
      if (evt === 'model_generation_start' && sender_id) {
        setStreamingMap(prev => {
          const next = new Map(prev);
          next.set(sender_id, {
            isStreaming: true,
            content: ''
          });
          return next;
        });
        setAnyStreaming(true);
        return;
      }
      if (evt === 'chunk' && chunk && sender_id) {
        setStreamingMap(prev => {
          const next = new Map(prev);
          const existing = next.get(sender_id) || {
            isStreaming: true,
            content: ''
          };
          next.set(sender_id, {
            ...existing,
            content: existing.content + chunk
          });
          return next;
        });
        return;
      }
      if ((evt === 'model_generation_complete' || evt === 'generation_complete') && sender_id) {
        setStreamingMap(prev => {
          const next = new Map(prev);
          next.set(sender_id, {
            isStreaming: false,
            content: ''
          });
          return next;
        });
        onRefreshMessagesRef.current();
        return;
      }
      if (evt === 'model_generation_error' && sender_id) {
        setStreamingMap(prev => {
          const next = new Map(prev);
          next.set(sender_id, {
            isStreaming: false,
            content: ''
          });
          return next;
        });
        onRefreshMessagesRef.current();
        return;
      }
      if (evt === 'generation_complete' && multi_model) {
        setStreamingMap(new Map());
        setAnyStreaming(false);
        onRefreshMessagesRef.current();
        return;
      }
      if (evt === 'generation_error') {
        setStreamingMap(new Map());
        setAnyStreaming(false);
        onRefreshMessagesRef.current();
      }
    };
    listen<{
      conversation_id: number;
      chunk?: string;
      event?: string;
      error?: string;
      user_message?: string;
      sender_id?: number;
      model_id?: number;
      multi_model?: boolean;
      total_models?: number;
    }>('ai-stream', handler).then(fn => {
      if (cancelled) {
        fn();
      } else {
        unlistenRef.current = fn;
      }
    });
    return () => {
      cancelled = true;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);
  const handleMouseDown = useCallback((index: number, e: React.MouseEvent) => {
    e.preventDefault();
    setDragging(index);
    dragStartX.current = e.clientX;
    dragStartRatios.current = [...ratios];
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [ratios]);
  useEffect(() => {
    if (dragging === null) return;
    const handleMouseMove = (e: MouseEvent) => {
      const container = containerRef.current;
      if (!container) return;
      const containerWidth = container.getBoundingClientRect().width;
      const deltaX = e.clientX - dragStartX.current;
      const deltaPct = deltaX / containerWidth * 100;
      const newRatios = [...dragStartRatios.current];
      const leftIdx = dragging;
      const rightIdx = dragging + 1;
      const minRatio = 15;
      let newLeft = newRatios[leftIdx] + deltaPct;
      let newRight = newRatios[rightIdx] - deltaPct;
      if (newLeft < minRatio) {
        newLeft = minRatio;
        newRight = 100 - newRatios.reduce((s, r, i) => s + (i === leftIdx || i === rightIdx ? 0 : r), 0) - minRatio;
      }
      if (newRight < minRatio) {
        newRight = minRatio;
        newLeft = 100 - newRatios.reduce((s, r, i) => s + (i === leftIdx || i === rightIdx ? 0 : r), 0) - minRatio;
      }
      newRatios[leftIdx] = newLeft;
      newRatios[rightIdx] = newRight;
      setRatios([...newRatios]);
    };
    const handleMouseUp = () => {
      setDragging(null);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [dragging]);
  const getActiveConvName = () => {
    const conv = conversations.find(c => c.id === activeChat);
    return conv?.title || t("ai.ChatPanel.k1", {
      activeChat: activeChat
    });
  };
  const handleSend = () => {
    if (anyStreaming) return;
    onSendMessage();
    setTimeout(() => {
      inputRef.current?.focus();
    }, 0);
  };
  if (paneCount === 0) {
    return <div className={styles.emptyState}>
        <div className={styles.emptyStateIcon}>🤖</div>
        <div className={styles.emptyStateTitle}>{t("ai.MultiModelChatPanel.k1")}</div>
        <div className={styles.emptyStateDesc}>{t("ai.MultiModelChatPanel.k2")}</div>
      </div>;
  }
  const startedCount = streamingMap.size;
  return <div className={styles.multiModelContainer}>
      <div className={styles.chatInterfaceHeader}>
        <span className={styles.chatInterfaceTitle}>{getActiveConvName()}</span>
        <div className={styles.headerActions}>
          <span className={styles.multiModelBadge}>
            {paneCount} {t("ai.MultiModelChatPanel.k3")}
            {anyStreaming && <span className={styles.badgeProgress}> · {startedCount}/{paneCount} {t("ai.MultiModelChatPanel.k4")}</span>}
          </span>
        </div>
      </div>

      <div className={styles.modelLabelBar}>
        {modelParts.map((part, i) => {
        const m = models.find(mo => mo.id === part.model_id);
        const name = m?.name || t("ai.AgentManager.k9", {
          model_id: part.model_id
        });
        const provider = m?.provider || 'unknown';
        const state = streamingMap.get(part.id);
        const isActive = state?.isStreaming;
        return <div key={part.id} className={styles.modelLabelItem} style={{
          width: `${ratios[i]}%`
        }}>
              <span className={`${styles.modelLabelDot} ${isActive ? styles.modelLabelDotActive : ''}`} />
              <span className={styles.modelLabelName}>{name}</span>
              <span className={`${styles.providerTag} ${styles[`provider${provider}` as keyof typeof styles] || ''}`}>
                {provider}
              </span>
            </div>;
      })}
      </div>

      <div className={styles.multiModelPanes} ref={containerRef}>
        {modelParts.map((part, i) => {
        const m = models.find(mo => mo.id === part.model_id);
        const modelName = m?.name || t("ai.AgentManager.k9", {
          model_id: part.model_id
        });
        const state = streamingMap.get(part.id);
        const isPaneStreaming = state?.isStreaming || false;
        const paneStreamingContent = state?.content || '';
        const paneMessages = messages.filter(msg => msg.sender_type === 'user' || msg.sender_id === part.id);
        return <div key={part.id} className={styles.multiPane} style={{
          width: `${ratios[i]}%`
        }}>
              <div className={styles.paneContent}>
                <div className={styles.paneMessages}>
                  {paneMessages.length > 0 ? paneMessages.map(msg => {
                const isError = msg.sender_type === 'error';
                const displayName = getSenderDisplayName(msg, parts, models, agents);
                return <div key={msg.id} className={`${styles.message} ${msg.sender_type === 'user' ? styles.messageUser : isError ? styles.messageError : styles.messageAi}`}>
                          <div className={styles.senderName}>{displayName}</div>
                          {isError ? <div className={styles.errorBubble}>
                              <div className={styles.errorIcon}>⚠</div>
                              <div className={styles.errorContent}>
                                <div className={styles.errorTitle}>{msg.content.replace(/^\[错误\]\s*/, '')}</div>
                              </div>
                            </div> : <div className={styles.messageBubble}>
                              <MarkdownRenderer content={msg.content} />
                            </div>}
                          <div className={styles.messageMeta}>
                            <span>{formatRelativeTime(msg.created_at)}</span>
                          </div>
                        </div>;
              }) : <div className={styles.paneEmpty}>
                      <span className={styles.paneModelIcon}>🤖</span>
                      <span className={styles.paneModelName}>{modelName}</span>
                      <span className={styles.paneHint}>{t("ai.MultiModelChatPanel.k5")}</span>
                    </div>}

                  {isPaneStreaming && paneStreamingContent && <div className={`${styles.message} ${styles.messageAi}`}>
                      <div className={styles.senderName}>{modelName}</div>
                      <div className={styles.messageBubble}>
                        <MarkdownRenderer content={paneStreamingContent} showCursor={true} />
                      </div>
                      <div className={styles.messageMeta}>
                        <span className={styles.generating}>{t("components.intelligence.DashboardPanel.k67")}</span>
                      </div>
                    </div>}
                  <div ref={msgEndRef} />
                </div>
              </div>
              {i < paneCount - 1 && <div className={`${styles.paneDivider} ${dragging === i ? styles.paneDividerActive : ''}`} onMouseDown={e => handleMouseDown(i, e)}>
                  <div className={styles.paneDividerHandle} />
                </div>}
            </div>;
      })}
      </div>

      <div className={styles.inputArea}>
        <div className={styles.inputRow}>
          <textarea ref={inputRef} placeholder={t("ai.MultiModelChatPanel.k6", {
          paneCount: paneCount
        })} value={messageInput} onChange={e => {
          setMessageInput(e.target.value);
          const el = e.target;
          el.style.height = 'auto';
          el.style.height = Math.min(el.scrollHeight, 200) + 'px';
        }} onKeyDown={e => {
          if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            handleSend();
          }
        }} className={styles.messageInput} rows={1} disabled={anyStreaming} />
          <button onClick={handleSend} className={styles.sendButton} disabled={anyStreaming || !messageInput.trim()}>
            {anyStreaming ? '...' : '↑'}
          </button>
        </div>
        <div className={styles.inputHint}>
          <kbd>Enter</kbd> {t("ai.ChatPanel.k22")} <kbd>Shift+Enter</kbd> {t("ai.MultiModelChatPanel.k7")} {paneCount} {t("ai.MultiModelChatPanel.k8")}
        </div>
      </div>
    </div>;
}