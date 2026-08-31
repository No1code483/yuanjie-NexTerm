import { t } from "i18next";
import { useState, useEffect, useRef, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { ipc } from '@/lib/ipc';
import styles from './FloatingXin.module.css';

// ===== 类型定义 =====
interface ChatLine {
  id: string;
  role: 'user' | 'xin';
  content: string;
  timestamp: string;
}
interface Toast {
  id: string;
  type: 'greeting' | 'break' | 'mood' | 'info';
  icon: string;
  text: string;
  actions?: {
    label: string;
    action: string;
  }[];
}

// ===== 语音识别类型 =====
interface SpeechRecognitionEvent extends Event {
  readonly resultIndex: number;
  readonly results: SpeechRecognitionResultList;
}
interface SpeechRecognitionResultList {
  readonly length: number;
  [index: number]: SpeechRecognitionResult;
}
interface SpeechRecognitionResult {
  readonly isFinal: boolean;
  readonly length: number;
  [index: number]: SpeechRecognitionAlternative;
}
interface SpeechRecognitionAlternative {
  readonly transcript: string;
  readonly confidence: number;
}
interface SpeechRecognition extends EventTarget {
  continuous: boolean;
  interimResults: boolean;
  lang: string;
  start(): void;
  stop(): void;
  abort(): void;
  onresult: ((event: SpeechRecognitionEvent) => void) | null;
  onerror: ((event: Event) => void) | null;
  onend: (() => void) | null;
}

// ===== 常量 =====
const WIN_W = 360;
const WIN_H = 520;
const COLLAPSED_W = 48;
const MARGIN = 16;
const GREETINGS: Record<string, string> = {
  morning: t("components.FloatingXin.k1"),
  afternoon: t("components.FloatingXin.k2"),
  evening: t("components.FloatingXin.k3"),
  night: t("components.FloatingXin.k4")
};
const BREAK_MESSAGES = [t("components.FloatingXin.k5"), t("components.FloatingXin.k6"), t("components.FloatingXin.k7")];
const MOOD_CHECK_MESSAGES = [t("components.FloatingXin.k8"), t("components.FloatingXin.k9"), t("components.FloatingXin.k10")];

// ===== 组件 =====
export default function FloatingXin() {
  const navigate = useNavigate();

  // 窗口状态
  const [visible, setVisible] = useState(true);
  const [collapsed, setCollapsed] = useState(false);
  const [docked, setDocked] = useState(false);
  const [position, setPosition] = useState({
    x: 0,
    y: 0
  });
  const [dragging, setDragging] = useState(false);
  const dragStart = useRef({
    x: 0,
    y: 0,
    winX: 0,
    winY: 0
  });

  // 聊天
  const [messages, setMessages] = useState<ChatLine[]>([]);
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);

  // 语音
  const [recording, setRecording] = useState(false);
  const recognitionRef = useRef<SpeechRecognition | null>(null);

  // 提醒
  const [toasts, setToasts] = useState<Toast[]>([]);

  // 右键菜单
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
  } | null>(null);

  // 容器 ref
  const containerRef = useRef<HTMLDivElement>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // ===== 初始化位置 =====
  useEffect(() => {
    const x = window.innerWidth - WIN_W - MARGIN;
    const y = window.innerHeight - WIN_H - MARGIN;
    setPosition({
      x: Math.max(MARGIN, x),
      y: Math.max(MARGIN, y)
    });
  }, []);

  // 窗口大小变化时调整位置
  useEffect(() => {
    const handleResize = () => {
      setPosition(prev => ({
        x: Math.max(0, Math.min(window.innerWidth - (collapsed ? COLLAPSED_W : WIN_W), prev.x)),
        y: Math.max(0, Math.min(window.innerHeight - (collapsed ? COLLAPSED_W : WIN_H), prev.y))
      }));
    };
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [collapsed]);

  // 滚动到底部
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({
      behavior: 'smooth'
    });
  }, [messages]);

  // ===== 拖拽 =====
  const handleHeaderMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return;
    setDragging(true);
    dragStart.current = {
      x: e.clientX,
      y: e.clientY,
      winX: position.x,
      winY: position.y
    };
    e.preventDefault();
  }, [position]);
  useEffect(() => {
    if (!dragging) return;
    const handleMouseMove = (e: MouseEvent) => {
      const dx = e.clientX - dragStart.current.x;
      const dy = e.clientY - dragStart.current.y;
      const w = collapsed ? COLLAPSED_W : WIN_W;
      const h = collapsed ? COLLAPSED_W : WIN_H;
      setPosition({
        x: Math.max(0, Math.min(window.innerWidth - w, dragStart.current.winX + dx)),
        y: Math.max(0, Math.min(window.innerHeight - h, dragStart.current.winY + dy))
      });
    };
    const handleMouseUp = () => setDragging(false);
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [dragging, collapsed]);

  // ===== 最小化 / 停靠 =====
  const minimize = () => {
    setCollapsed(true);
    setDocked(false);
  };
  const restore = () => {
    setCollapsed(false);
    setDocked(false);
  };
  const dockToEdge = () => {
    setDocked(true);
    setCollapsed(false);
  };

  // ===== 右键菜单 =====
  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    setContextMenu({
      x: e.clientX,
      y: e.clientY
    });
  };
  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    window.addEventListener('click', close);
    window.addEventListener('contextmenu', close);
    return () => {
      window.removeEventListener('click', close);
      window.removeEventListener('contextmenu', close);
    };
  }, [contextMenu]);

  // ===== 发送消息 =====
  const sendMessage = async () => {
    const text = input.trim();
    if (!text || sending) return;
    const userMsg: ChatLine = {
      id: `u${Date.now()}`,
      role: 'user',
      content: text,
      timestamp: new Date().toISOString()
    };
    setMessages(prev => [...prev, userMsg]);
    setInput('');
    setSending(true);
    try {
      const res = await ipc.invoke<any>('xin_v3_dialogue_send', {
        conversation_id: null,
        content: text,
        persona_id: 'caring_friend',
        stream: false
      });
      if (res?.data?.content) {
        const xinMsg: ChatLine = {
          id: `x${Date.now()}`,
          role: 'xin',
          content: res.data.content,
          timestamp: new Date().toISOString()
        };
        setMessages(prev => [...prev, xinMsg]);
      } else {
        // 本地 fallback
        const fallback = getLocalResponse(text);
        const xinMsg: ChatLine = {
          id: `x${Date.now()}`,
          role: 'xin',
          content: fallback,
          timestamp: new Date().toISOString()
        };
        setMessages(prev => [...prev, xinMsg]);
      }
    } catch {
      const fallback = getLocalResponse(text);
      const xinMsg: ChatLine = {
        id: `x${Date.now()}`,
        role: 'xin',
        content: fallback,
        timestamp: new Date().toISOString()
      };
      setMessages(prev => [...prev, xinMsg]);
    } finally {
      setSending(false);
    }
  };
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  };

  // ===== 语音输入 =====
  const toggleRecording = () => {
    if (recording) {
      stopRecording();
    } else {
      startRecording();
    }
  };
  const startRecording = () => {
    // 检查 Web Speech API
    const SpeechRecognitionCtor = (window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
    if (SpeechRecognitionCtor) {
      const recognition = new SpeechRecognitionCtor() as SpeechRecognition;
      recognition.continuous = false;
      recognition.interimResults = false;
      recognition.lang = 'zh-CN';
      recognition.onresult = (event: SpeechRecognitionEvent) => {
        const transcript = event.results[0]?.[0]?.transcript || '';
        if (transcript) {
          setInput(prev => prev + transcript);
        }
      };
      recognition.onerror = () => {
        setRecording(false);
      };
      recognition.onend = () => {
        setRecording(false);
      };
      recognition.start();
      recognitionRef.current = recognition;
      setRecording(true);
    } else {
      // Fallback: 调用后端
      fallbackVoiceInput();
    }
  };
  const stopRecording = () => {
    if (recognitionRef.current) {
      recognitionRef.current.stop();
      recognitionRef.current = null;
    }
    setRecording(false);
  };
  const fallbackVoiceInput = async () => {
    // D3.3 后端 STT 需要音频文件路径，浏览器端 fallback 暂不可用
    // 完整 STT 入口在 Xin.tsx 主页面（用 MediaRecorder 录音 → 写临时文件 → 调用后端）
    setRecording(false);
  };
  useEffect(() => {
    return () => {
      if (recognitionRef.current) {
        recognitionRef.current.abort();
      }
    };
  }, []);

  // ===== 主动提醒 =====
  useEffect(() => {
    // 定时问候
    const checkGreeting = () => {
      const now = new Date();
      const hour = now.getHours();
      const lastGreeting = localStorage.getItem('xin_last_greeting_date');
      const today = now.toDateString();
      if (lastGreeting !== today) {
        let greeting: string;
        if (hour < 9) greeting = GREETINGS.morning;else if (hour < 12) greeting = GREETINGS.morning;else if (hour < 18) greeting = GREETINGS.afternoon;else if (hour < 22) greeting = GREETINGS.evening;else greeting = GREETINGS.night;
        addToast({
          id: `greeting-${Date.now()}`,
          type: 'greeting',
          icon: '👋',
          text: greeting,
          actions: [{
            label: t("components.FloatingXin.k11"),
            action: 'reply'
          }, {
            label: t("components.FloatingXin.k12"),
            action: 'dismiss'
          }]
        });
        localStorage.setItem('xin_last_greeting_date', today);
      }
    };

    // 休息提醒（每2小时）
    const checkBreak = () => {
      const lastInteraction = localStorage.getItem('xin_last_interaction');
      const now = Date.now();
      if (lastInteraction && now - parseInt(lastInteraction) > 2 * 60 * 60 * 1000) {
        const msg = BREAK_MESSAGES[Math.floor(Math.random() * BREAK_MESSAGES.length)];
        addToast({
          id: `break-${Date.now()}`,
          type: 'break',
          icon: '💡',
          text: msg,
          actions: [{
            label: t("components.FloatingXin.k13"),
            action: 'dismiss'
          }]
        });
      }
    };

    // 心情关怀（每4小时）
    const checkMood = () => {
      const lastMoodCheck = localStorage.getItem('xin_last_mood_check');
      const now = Date.now();
      if (!lastMoodCheck || now - parseInt(lastMoodCheck) > 4 * 60 * 60 * 1000) {
        const msg = MOOD_CHECK_MESSAGES[Math.floor(Math.random() * MOOD_CHECK_MESSAGES.length)];
        addToast({
          id: `mood-${Date.now()}`,
          type: 'mood',
          icon: '💭',
          text: msg,
          actions: [{
            label: t("components.FloatingXin.k14"),
            action: 'reply'
          }, {
            label: t("components.FloatingXin.k15"),
            action: 'dismiss'
          }]
        });
        localStorage.setItem('xin_last_mood_check', now.toString());
      }
    };

    // 初始检查
    setTimeout(checkGreeting, 2000);
    const greetingTimer = setInterval(checkGreeting, 60 * 1000); // 每分钟检查
    const breakTimer = setInterval(checkBreak, 5 * 60 * 1000); // 每5分钟检查
    const moodTimer = setInterval(checkMood, 10 * 60 * 1000); // 每10分钟检查

    return () => {
      clearInterval(greetingTimer);
      clearInterval(breakTimer);
      clearInterval(moodTimer);
    };
  }, []);

  // 更新最后交互时间
  useEffect(() => {
    localStorage.setItem('xin_last_interaction', Date.now().toString());
  }, [messages]);
  const addToast = (toast: Toast) => {
    setToasts(prev => {
      // 去重
      if (toast.type !== 'info' && prev.some(t => t.type === toast.type)) return prev;
      return [...prev.slice(-3), toast];
    });
  };
  const handleToastAction = (toastId: string, action: string) => {
    if (action === 'reply') {
      restore();
      setInput(t("components.FloatingXin.k16"));
    }
    setToasts(prev => prev.filter(t => t.id !== toastId));
  };

  // ===== 打开主页面 =====
  const openMainPage = () => {
    navigate('/xin?tab=chat');
  };

  // ===== 关闭 =====
  const close = () => {
    setVisible(false);
  };

  // ===== 本地回复 =====
  const getLocalResponse = (text: string): string => {
    const lower = text.toLowerCase();
    if (lower.includes(t("components.FloatingXin.k17")) || lower.includes('hi') || lower.includes('hello')) return t("components.FloatingXin.k18");
    if (lower.includes(t("components.FloatingXin.k19")) || lower.includes('thank')) return t("components.FloatingXin.k20");
    if (lower.includes(t("components.FloatingXin.k21")) || lower.includes('help')) return t("components.FloatingXin.k22");
    if (lower.includes(t("components.FloatingXin.k23")) || lower.includes('bye')) return t("components.FloatingXin.k24");
    return t("components.FloatingXin.k25");
  };

  // ===== 不显示 =====
  if (!visible) return null;

  // ===== 停靠状态 =====
  if (docked) {
    return <>
        <div className={styles.docked} style={{
        right: 0,
        top: '40%'
      }} onClick={restore} onContextMenu={handleContextMenu} title={t("components.FloatingXin.k26")}>
          <span className={styles.dockedIcon}>{t("components.FloatingXin.k26")}</span>
          <span className={styles.dockedLabel}>{t("components.FloatingXin.k26")}</span>
        </div>
        {contextMenu && <div className={styles.contextMenu} style={{
        left: contextMenu.x,
        top: contextMenu.y
      }}>
            <button className={styles.contextMenuItem} onClick={() => {
          restore();
          setContextMenu(null);
        }}>
              {t("components.FloatingXin.k27")}
            </button>
            <div className={styles.contextMenuDivider} />
            <button className={styles.contextMenuItem} onClick={() => {
          close();
          setContextMenu(null);
        }} style={{
          color: 'rgba(255, 100, 100, 0.8)'
        }}>
              {t("components.FloatingXin.k28")}
            </button>
          </div>}
      </>;
  }

  // ===== 折叠状态（圆形图标） =====
  if (collapsed) {
    return <>
        <div ref={containerRef} className={`${styles.container} ${styles.containerCollapsed}`} style={{
        left: position.x,
        top: position.y
      }} onMouseDown={handleHeaderMouseDown} onClick={restore} onContextMenu={handleContextMenu} title={t("components.FloatingXin.k29")}>
          <div className={styles.avatar} style={{
          width: '100%',
          height: '100%',
          fontSize: '14px',
          fontWeight: 'bold'
        }}>
            {t("components.FloatingXin.k26")}
          </div>
        </div>
        {contextMenu && <div className={styles.contextMenu} style={{
        left: contextMenu.x,
        top: contextMenu.y
      }}>
            <button className={styles.contextMenuItem} onClick={() => {
          restore();
          setContextMenu(null);
        }}>
              {t("components.FloatingXin.k30")}
            </button>
            <button className={styles.contextMenuItem} onClick={() => {
          dockToEdge();
          setContextMenu(null);
        }}>
              {t("components.FloatingXin.k31")}
            </button>
            <div className={styles.contextMenuDivider} />
            <button className={styles.contextMenuItem} onClick={() => {
          close();
          setContextMenu(null);
        }} style={{
          color: 'rgba(255, 100, 100, 0.8)'
        }}>
              {t("components.FloatingXin.k28")}
            </button>
          </div>}
      </>;
  }

  // ===== 完整展开状态 =====
  return <>
      <div ref={containerRef} className={styles.container} style={{
      left: position.x,
      top: position.y
    }} onContextMenu={handleContextMenu}>
        {/* 标题栏 */}
        <div className={styles.header} onMouseDown={handleHeaderMouseDown}>
          <div className={styles.headerLeft}>
            <div className={styles.avatar}>{t("components.FloatingXin.k26")}</div>
            <span className={styles.title}>{t("components.FloatingXin.k26")}</span>
          </div>
          <div className={styles.headerActions}>
            <button className={styles.headerBtn} onClick={openMainPage} title={t("components.FloatingXin.k32")}>
              ↗
            </button>
            <button className={styles.headerBtn} onClick={dockToEdge} title={t("components.FloatingXin.k33")}>
              📌
            </button>
            <button className={styles.headerBtn} onClick={minimize} title={t("common.minimize")}>
              −
            </button>
            <button className={styles.headerBtn} onClick={close} title={t("common.close")}>
              ×
            </button>
          </div>
        </div>

        {/* 消息区 */}
        <div className={styles.messages}>
          {toasts.map(toast => <div key={toast.id} className={styles.toast}>
              <div className={styles.toastIcon}>{toast.icon} {toast.text}</div>
              {toast.actions && toast.actions.length > 0 && <div className={styles.toastActions}>
                  {toast.actions.map(a => <button key={a.action} className={`${styles.toastBtn} ${a.action === 'dismiss' ? styles.toastBtnDismiss : ''}`} onClick={() => handleToastAction(toast.id, a.action)}>
                      {a.label}
                    </button>)}
                </div>}
            </div>)}

          {messages.length === 0 && toasts.length === 0 && <div className={styles.msgEmpty}>
              {t("components.FloatingXin.k34")}<br />
              <span style={{
            fontSize: '10px'
          }}>{t("components.FloatingXin.k35")}</span>
            </div>}

          {messages.map(m => <div key={m.id} className={`${styles.msg} ${m.role === 'user' ? styles.msgUser : styles.msgXin}`}>
              {m.content}
            </div>)}

          {sending && <div className={styles.msgThinking}>
              {t("components.FloatingXin.k36")}
            </div>}

          <div ref={messagesEndRef} />
        </div>

        {/* 输入区域 */}
        <div className={styles.inputArea}>
          <button className={`${styles.micBtn} ${recording ? styles.micBtnRecording : ''}`} onClick={toggleRecording} title={recording ? t("components.FloatingXin.k37") : t("components.FloatingXin.k38")}>
            🎤
            {recording && <span className={styles.recordingDot} />}
          </button>
          <input className={styles.chatInput} value={input} onChange={e => setInput(e.target.value)} onKeyDown={handleKeyDown} placeholder={t("components.FloatingXin.k39")} disabled={sending} />
          <button className={styles.sendBtn} onClick={sendMessage} disabled={sending || !input.trim()}>
            ▶
          </button>
        </div>

        {/* 状态栏 */}
        <div className={styles.statusBar}>
          <span className={`${styles.statusDot} ${!visible ? styles.statusDotOff : ''}`} />
          <span>{t("components.FloatingXin.k40")}</span>
        </div>
      </div>

      {/* 右键菜单 */}
      {contextMenu && <div className={styles.contextMenu} style={{
      left: contextMenu.x,
      top: contextMenu.y
    }}>
          <button className={styles.contextMenuItem} onClick={() => {
        openMainPage();
        setContextMenu(null);
      }}>
            {t("components.FloatingXin.k41")}
          </button>
          <button className={styles.contextMenuItem} onClick={() => {
        dockToEdge();
        setContextMenu(null);
      }}>
            {t("components.FloatingXin.k31")}
          </button>
          <button className={styles.contextMenuItem} onClick={() => {
        minimize();
        setContextMenu(null);
      }}>
            {t("components.FloatingXin.k42")}
          </button>
          <div className={styles.contextMenuDivider} />
          <button className={styles.contextMenuItem} onClick={() => {
        close();
        setContextMenu(null);
      }} style={{
        color: 'rgba(255, 100, 100, 0.8)'
      }}>
            {t("components.FloatingXin.k28")}
          </button>
        </div>}
    </>;
}