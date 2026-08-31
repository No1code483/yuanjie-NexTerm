import { t } from "i18next";
import { useRef, useEffect, useState, useCallback } from 'react';
import { createPortal } from 'react-dom';
import type { AIModel, BackendAgent, Conversation, Message, Participant, GroupProgress, ContextMenuState } from './types';
import { formatRelativeTime, getSenderDisplayName } from './utils';
import MarkdownRenderer from '@/components/MarkdownRenderer';
import { ai } from '@/lib/ipc';
import PromptTemplates from './PromptTemplates';
import styles from '../AI.module.css';
interface Props {
  activeChat: number;
  conversations: Conversation[];
  models: AIModel[];
  agents: BackendAgent[];
  participants: Map<number, Participant[]>;
  messages: Message[];
  loadingMessages: boolean;
  isStreaming: boolean;
  streamingContent: string;
  messageInput: string;
  setMessageInput: React.Dispatch<React.SetStateAction<string>>;
  onSendMessage: () => void;
  onStopGroupChat: () => void;
  orchestratingConvRef: React.MutableRefObject<number | null>;
  groupProgress: GroupProgress;
  contextMenu: ContextMenuState;
  setContextMenu: (v: ContextMenuState) => void;
  multiSelectMode: boolean;
  selectedMessageIds: Set<number>;
  onToggleMessageSelect: (id: number) => void;
  onDeleteMessage: (id: number) => void;
  onCopyMessage: (content: string) => void;
  onQuoteMessage: (msg: Message) => void;
  onToggleMultiSelect: () => void;
  onBatchDelete: () => void;
  onCancelMultiSelect: () => void;
  onBranchConversation: (messageId: number) => void;
}
export default function ChatPanel({
  activeChat,
  conversations,
  models,
  agents,
  participants,
  messages,
  loadingMessages,
  isStreaming,
  streamingContent,
  messageInput,
  setMessageInput,
  onSendMessage,
  onStopGroupChat,
  orchestratingConvRef,
  groupProgress,
  contextMenu,
  setContextMenu,
  multiSelectMode,
  selectedMessageIds,
  onToggleMessageSelect,
  onDeleteMessage,
  onCopyMessage,
  onQuoteMessage,
  onToggleMultiSelect,
  onBatchDelete,
  onCancelMultiSelect,
  onBranchConversation
}: Props) {
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const [showTemplates, setShowTemplates] = useState(false);
  const [tokenUsage, setTokenUsage] = useState(0);
  const tokenLimit = 32768;
  
  // Tab 补全状态
  const [tabGhostText, setTabGhostText] = useState('');
  const tabDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const tabSuggestionsRef = useRef<string[]>([]);
  const tabSuggestionIndexRef = useRef(0);

  // Tab 补全：从历史消息中获取建议
  const fetchMessageSuggestions = useCallback((text: string) => {
    if (!text.trim() || text.length < 2) return [];
    const prefix = text.toLowerCase();
    return messages
      .filter(m => m.sender_type === 'user' && m.content.toLowerCase().startsWith(prefix) && m.content !== text)
      .map(m => m.content)
      .slice(0, 5);
  }, [messages]);

  // 输入变化时触发补全
  const handleMessageInputChange = useCallback((value: string) => {
    setMessageInput(value);
    setTabGhostText('');
    
    if (tabDebounceRef.current) clearTimeout(tabDebounceRef.current);
    tabDebounceRef.current = setTimeout(() => {
      const suggestions = fetchMessageSuggestions(value);
      tabSuggestionsRef.current = suggestions;
      tabSuggestionIndexRef.current = 0;
      if (suggestions.length > 0 && suggestions[0].toLowerCase().startsWith(value.toLowerCase())) {
        setTabGhostText(suggestions[0].slice(value.length));
      }
    }, 150);
  }, [fetchMessageSuggestions, setMessageInput]);

  // 清理定时器
  useEffect(() => {
    return () => {
      if (tabDebounceRef.current) clearTimeout(tabDebounceRef.current);
    };
  }, []);
  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({
      behavior: 'instant' as ScrollBehavior
    });
  };
  useEffect(() => {
    scrollToBottom();
  }, [messages, streamingContent]);

  // 估算 token 使用量
  useEffect(() => {
    const totalChars = messages.reduce((sum, m) => sum + m.content.length, 0);
    setTokenUsage(Math.round(totalChars * 0.5));
  }, [messages]);
  const getActiveConvName = () => {
    const conv = conversations.find(c => c.id === activeChat);
    return conv?.title || t("ai.ChatPanel.k1", {
      activeChat: activeChat
    });
  };
  const conv = conversations.find(c => c.id === activeChat);
  const isGroupConv = conv?.type === 'group';
  const tokenPct = Math.min(100, Math.round(tokenUsage / tokenLimit * 100));
  const tokenColor = tokenPct < 50 ? '#00FF41' : tokenPct < 80 ? '#FFD700' : '#FF0000';
  const handleExport = async (format: 'md' | 'json') => {
    try {
      const res = await ai.exportConversation(activeChat, format);
      const content = res.data || '';
      const ext = format === 'md' ? 'md' : 'json';
      const mimeType = format === 'md' ? 'text/markdown' : 'application/json';
      const filename = `${getActiveConvName()}.${ext}`;
      const blob = new Blob([content], {
        type: mimeType
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      a.click();
      URL.revokeObjectURL(url);
    } catch {/* ignore */}
  };
  const handleTemplateApply = (content: string) => {
    setMessageInput(prev => prev + content);
    setShowTemplates(false);
  };
  return <div className={styles.chatContainer}>
      <div className={styles.chatInterfaceHeader}>
        <span className={styles.chatInterfaceTitle}>{getActiveConvName()}</span>
        <div className={styles.headerActions}>
          <button className={styles.headerActionBtn} onClick={() => setShowTemplates(!showTemplates)} title={t("ai.ChatPanel.k2")}>
            📋
          </button>
          <div className={styles.exportDropdown}>
            <button className={styles.headerActionBtn} title={t("common.export")}>📥</button>
            <div className={styles.exportMenu}>
              <button className={styles.exportMenuItem} onClick={() => handleExport('md')}>Markdown (.md)</button>
              <button className={styles.exportMenuItem} onClick={() => handleExport('json')}>JSON (.json)</button>
            </div>
          </div>
        </div>
      </div>

      {/* Prompt 模板库面板 */}
      {showTemplates && <PromptTemplates onApply={handleTemplateApply} onClose={() => setShowTemplates(false)} />}

      <div className={styles.chatMessages}>
        {messages.length > 0 ? messages.map(msg => {
        const isError = msg.sender_type === 'error';
        const parts = participants.get(activeChat) || [];
        const displayName = getSenderDisplayName(msg, parts, models, agents);
        return <div key={msg.id} className={`${styles.message} ${msg.sender_type === 'user' ? styles.messageUser : isError ? styles.messageError : styles.messageAi} ${multiSelectMode ? styles.messageSelectable : ''} ${selectedMessageIds.has(msg.id) ? styles.messageSelected : ''}`} onClick={() => {
          if (multiSelectMode) onToggleMessageSelect(msg.id);
        }} onContextMenu={e => {
          e.preventDefault();
          e.stopPropagation();
          setContextMenu({
            visible: true,
            x: e.clientX,
            y: e.clientY,
            messageId: msg.id
          });
        }}>
                <div className={styles.senderName}>{displayName}</div>
                {isError ? <div className={styles.errorBubble}>
                    <div className={styles.errorIcon}>⚠</div>
                    <div className={styles.errorContent}>
                      <div className={styles.errorTitle}>{msg.content.replace(/^\[错误\]\s*/, '')}</div>
                      {msg.content.includes('💡') && <div className={styles.errorSuggestion}>{msg.content.split('💡')[1]?.trim() || ''}</div>}
                    </div>
                  </div> : <div className={styles.messageBubble}>
                    <MarkdownRenderer content={msg.content} />
                  </div>}
                <div className={styles.messageMeta}>
                  <span>{formatRelativeTime(msg.created_at)}</span>
                </div>
              </div>;
      }) : loadingMessages ? <div className={styles.loadingHint}>{t("ai.ChatPanel.k3")}</div> : !isStreaming ? <div className={styles.welcomeGuide}>
            <div className={styles.welcomeTitle}>{t("ai.ChatPanel.k4")}</div>
            <div className={styles.welcomeCards}>
              {[t("ai.ChatPanel.k5"), t("ai.ChatPanel.k6"), t("ai.ChatPanel.k7"), t("ai.ChatPanel.k8")].map((s, i) => <button key={i} className={styles.welcomeCard} onClick={() => {
            setMessageInput(s);
            inputRef.current?.focus();
          }}>
                  {s}
                </button>)}
            </div>
            <div className={styles.welcomeHint}>{t("ai.ChatPanel.k9")}</div>
          </div> : null}

        {/* 群聊进度条 / 流式输出 */}
        {(() => {
        if (isGroupConv && groupProgress.status !== 'idle') {
          const pct = Math.min(100, Math.round(groupProgress.round / groupProgress.maxRounds * 100));
          return <div className={styles.groupProgressBar}>
                <div className={styles.groupProgressHeader}>
                  <span className={styles.groupProgressText}>
                    {groupProgress.status === 'stopped' ? t("ai.ChatPanel.k10") : groupProgress.status === 'converged' ? t("ai.ChatPanel.k11") : t("ai.ChatPanel.k12", {
                  round: groupProgress.round
                })}
                  </span>
                  <span className={styles.groupProgressPct}>{pct}%</span>
                </div>
                <div className={styles.groupProgressTrack}>
                  <div className={styles.groupProgressFill} style={{
                width: `${pct}%`
              }} />
                </div>
                {groupProgress.totalParticipants > 0 && <div className={styles.groupParticipants}>
                    {(participants.get(activeChat) || []).map((p, i) => {
                const name = p.model_id ? models.find(m => m.id === p.model_id)?.name || t("ai.AgentManager.k9", {
                  model_id: p.model_id
                }) : p.role || t("components.GroupChatOrchestrationPanel.k35");
                const hasResponded = groupProgress.responded.includes(name) || groupProgress.status === 'converged' || groupProgress.status === 'stopped';
                return <div key={i} className={`${styles.participantDot} ${hasResponded ? styles.dotDone : styles.dotPending}`} title={name}>
                          <span className={styles.dotLabel}>{name}</span>
                        </div>;
              })}
                  </div>}
              </div>;
        }
        if (streamingContent) {
          return <div className={`${styles.message} ${styles.messageAi}`}>
                <div className={styles.messageBubble}>
                  <MarkdownRenderer content={streamingContent} showCursor={isStreaming} />
                </div>
                <div className={styles.messageMeta}>
                  <span className={styles.messageAuthor}>AI</span>
                  {isStreaming && <span className={styles.generating}>{t("components.intelligence.DashboardPanel.k67")}</span>}
                </div>
              </div>;
        }
        if (isStreaming && !streamingContent) {
          return <div className={`${styles.message} ${styles.messageAi}`}>
                <div className={styles.messageBubble}><span className={styles.thinking}>{t("components.FloatingBall.k62")}</span></div>
              </div>;
        }
        return null;
      })()}
        <div ref={messagesEndRef} />
      </div>

      {/* 右键菜单 */}
      {contextMenu.visible && createPortal(<div className={styles.contextMenu} style={{
      left: contextMenu.x,
      top: contextMenu.y
    }} onClick={e => e.stopPropagation()} onContextMenu={e => e.preventDefault()}>
          <button className={styles.contextMenuItem} onClick={() => {
        const msg = messages.find(m => m.id === contextMenu.messageId);
        if (msg) onCopyMessage(msg.content);
      }}>{t("ai.ChatPanel.k13")}</button>
          <button className={styles.contextMenuItem} onClick={() => {
        const msg = messages.find(m => m.id === contextMenu.messageId);
        if (msg) onQuoteMessage(msg);
      }}>{t("ai.ChatPanel.k14")}</button>
          <button className={styles.contextMenuItem} onClick={() => {
        const msg = messages.find(m => m.id === contextMenu.messageId);
        if (msg) onBranchConversation(msg.id);
      }}>{t("ai.ChatPanel.k15")}</button>
          <button className={styles.contextMenuItem} onClick={onToggleMultiSelect}>{t("ai.ChatPanel.k16")}</button>
          <div className={styles.contextMenuDivider} />
          <button className={`${styles.contextMenuItem} ${styles.contextMenuItemDanger}`} onClick={() => contextMenu.messageId && onDeleteMessage(contextMenu.messageId)}>{t("ai.ChatPanel.k17")}</button>
        </div>, document.body)}

      {/* 多选工具栏 */}
      {multiSelectMode && createPortal(<div className={styles.multiSelectBar}>
          <span className={styles.multiSelectCount}>{t("ai.ChatPanel.k18")} {selectedMessageIds.size} {t("ai.ChatPanel.k19")}</span>
          <div className={styles.multiSelectActions}>
            <button className={styles.multiSelectBtnDanger} onClick={onBatchDelete} disabled={selectedMessageIds.size === 0}>{t("ai.ChatPanel.k20")}</button>
            <button className={styles.multiSelectBtn} onClick={onCancelMultiSelect}>{t("common.cancel")}</button>
          </div>
        </div>, document.body)}

      <div className={styles.inputArea}>
        {/* Token 用量 */}
        <div className={styles.tokenBar}>
          <div className={styles.tokenTrack}>
            <div className={styles.tokenFill} style={{
            width: `${tokenPct}%`,
            background: tokenColor
          }} />
          </div>
          <span className={styles.tokenText} style={{
          color: tokenColor
        }}>
            {tokenUsage.toLocaleString()} / {tokenLimit.toLocaleString()} tokens
          </span>
        </div>
        <div className={styles.inputRow}>
          <textarea ref={inputRef} placeholder={t("ai.ChatPanel.k21")} value={messageInput} onChange={e => {
          handleMessageInputChange(e.target.value);
          const el = e.target;
          el.style.height = 'auto';
          el.style.height = Math.min(el.scrollHeight, 200) + 'px';
        }} onKeyDown={e => {
          if (e.key === 'Tab' && tabGhostText) {
            e.preventDefault();
            const fullText = messageInput + tabGhostText;
            setMessageInput(fullText);
            setTabGhostText('');
            tabSuggestionsRef.current = [];
          } else if (e.key === 'Escape') {
            setTabGhostText('');
            tabSuggestionsRef.current = [];
          } else if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            onSendMessage();
          }
        }} className={styles.messageInput} rows={1} disabled={isStreaming} />
          <button onClick={onSendMessage} className={styles.sendButton} disabled={isStreaming || !messageInput.trim()}>
            {isStreaming ? '...' : '↑'}
          </button>
          {orchestratingConvRef.current && <button onClick={onStopGroupChat} className={styles.sendButton} style={{
          background: '#e53e3e'
        }}>⏹</button>}
        </div>
        <div className={styles.inputHint}><kbd>Enter</kbd> {t("ai.ChatPanel.k22")} <kbd>Shift+Enter</kbd> {t("ai.ChatPanel.k23")}</div>
      </div>
    </div>;
}