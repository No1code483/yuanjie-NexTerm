// xin.chat L2 —— 对话功能域（局部 state + handlers + JSX）。
// 物理迁入本目录：会话侧栏 + 消息区 + 输入区 + 文末人格面板；跨 tab 共享状态经 core（XinCore）注入。
import { t } from "i18next";
import { useState, useRef, useEffect } from 'react';
import { xin, xinOrchestration, type XinAttachment } from '../../ipc';
import { random } from '@/lib/utils';
import { PERSONA_KNOWLEDGE } from '../../xinChatEngine';
import styles from '../../Xin.module.css';
import { MOOD_EMOJI } from '../../xin/types';
import type { ChatMessage } from '../../xin/types';
import type { XinCore } from '../../core';

export function useChat(core: XinCore, _active: boolean) {
  const {
    activePersona,
    personas,
    switchPersona,
    availableModels,
    selectedModel,
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
    isOnline,
    mood,
    setMood,
    realtimeSession,
    toggleRealtime
  } = core;

  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const [streamingContent, setStreamingContent] = useState('');
  // D3.2 TTS：正在合成 / 正在播放的消息 id
  const [ttsLoadingId, setTtsLoadingId] = useState<string | null>(null);
  const [ttsPlayingId, setTtsPlayingId] = useState<string | null>(null);
  const ttsAudioRef = useRef<HTMLAudioElement | null>(null);

  // 语音输入
  const [voiceRecording, setVoiceRecording] = useState(false);
  const voiceRecognitionRef = useRef<any>(null);
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const audioChunksRef = useRef<Blob[]>([]);

  // D3.4 图片上传附件（多模态输入）
  const [attachments, setAttachments] = useState<XinAttachment[]>([]);
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const [showingConversationSidebar, setShowingConversationSidebar] = useState(true);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const chatContainerRef = useRef<HTMLDivElement>(null);
  const streamUnlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({
      behavior: 'smooth'
    });
  }, [messages, streamingContent]);

  useEffect(() => {
    return () => {
      if (streamUnlistenRef.current) streamUnlistenRef.current();
    };
  }, []);
  useEffect(() => {
    return () => {
      if (voiceRecognitionRef.current) voiceRecognitionRef.current.abort();
    };
  }, []);

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
      const res = await xinOrchestration.dialogueSend(activeConversationId || '', text, {
        model_id: selectedModel?.id,
        persona_id: activePersona.id,
        attachments: sentAttachments.length > 0 ? sentAttachments : undefined,
      });
      if (res?.data) {
        if (res.data.conversation_id && !activeConversationId) {
          setActiveConversationId(res.data.conversation_id);
          loadConversations();
        }
        const xinMsg: ChatMessage = {
          id: random.uid(),
          role: 'xin',
          content: res.data.content,
          timestamp: new Date().toISOString()
        };
        setMessages(prev => [...prev, xinMsg]);
        if (res.data.token_usage) {
          setTokenStats({
            current: res.data.token_usage.total_tokens || 0,
            limit: 128000,
            ratio: (res.data.token_usage.total_tokens || 0) / 128000
          });
        }
        if (res.data.mood) setMood(res.data.mood);
      }
      estimateContextTokens();
    } catch (e) {
      try {
        const {
          xinChat
        } = await import('../../xinChatEngine');
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
      const res = await xinOrchestration.estimateTokens(allText);
      if (typeof res?.data === 'number') {
        setTokenStats(prev => ({
          ...prev,
          current: res.data ?? prev.current,
          ratio: (res.data ?? 0) / prev.limit
        }));
      }
    } catch {/* silent */}
  };
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  // ===== 语音输入（D3.3: MediaRecorder 录音 → 后端 Whisper API 转写） =====
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

  const renderChat = () => <>
      <div className={`${styles.tabContent} ${styles.chatLayout}`}>
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
              await xinOrchestration.dialogueStop(activeConversationId);
            } catch {/* silent */}
          }
        }}>
                {t("Xin.k38")}
              </button>}
          </div>
        </div>
      </div>

      {/* PERSONA PANEL (Shown in Chat Tab when sidebar collapsed) */}
      {!showingConversationSidebar && <div className={styles.personaPanel}>
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
    </>;

  return { renderChat };
}
