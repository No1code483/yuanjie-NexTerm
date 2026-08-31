import { t } from "i18next";
import { useState, useEffect, useRef, useCallback } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { ipc } from '@/lib/ipc';
import { intelligence } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';
import { useFloatingOrbStore } from '@/stores/floatingOrbStore';
import { useNotifStore } from '@/stores/notifStore';
import styles from './FloatingBall.module.css';

// ===== 页面静态建议（原有功能）=====
interface Suggestion {
  icon: string;
  text: string;
  action?: string;
  route?: string;
}
function getPageSuggestions(path: string): Suggestion[] {
  if (path.startsWith('/home')) return [{
    icon: '📝',
    text: t("components.FloatingBall.k1"),
    action: 'add_todo'
  }, {
    icon: '⏱️',
    text: t("components.FloatingBall.k2"),
    action: 'start_timer'
  }, {
    icon: '📰',
    text: t("components.FloatingBall.k3"),
    action: 'refresh_news'
  }];
  if (path.startsWith('/ai')) return [{
    icon: '🤖',
    text: t("components.FloatingBall.k4"),
    action: 'switch_model'
  }, {
    icon: '💬',
    text: t("components.FloatingBall.k5"),
    action: 'group_chat'
  }];
  if (path.startsWith('/knowledge')) return [{
    icon: '📂',
    text: t("components.FloatingBall.k6"),
    action: 'import_folder'
  }, {
    icon: '🔍',
    text: t("components.FloatingBall.k7"),
    action: 'search_kb'
  }, {
    icon: '🏷️',
    text: t("components.FloatingBall.k8"),
    action: 'add_tag'
  }];
  if (path.startsWith('/terminal')) return [{
    icon: '⌨️',
    text: t("components.FloatingBall.k9"),
    action: 'show_help'
  }, {
    icon: '📊',
    text: t("components.FloatingBall.k10"),
    action: 'sys_info'
  }];
  if (path.startsWith('/xin')) return [{
    icon: '💡',
    text: t("components.FloatingBall.k11"),
    action: 'switch_persona'
  }, {
    icon: '🧠',
    text: t("components.FloatingBall.k12"),
    action: 'view_memory'
  }];
  if (path.startsWith('/game')) return [{
    icon: '🎮',
    text: t("components.FloatingBall.k13"),
    action: 'new_world'
  }, {
    icon: '🏗️',
    text: t("components.FloatingBall.k14"),
    action: 'place_building'
  }];
  if (path.startsWith('/search')) return [{
    icon: '🔎',
    text: t("components.FloatingBall.k15"),
    action: 'smart_search'
  }, {
    icon: '⭐',
    text: t("components.FloatingBall.k16"),
    action: 'add_bookmark'
  }];
  if (path.startsWith('/profile')) return [{
    icon: '📄',
    text: t("components.FloatingBall.k17"),
    action: 'edit_resume'
  }, {
    icon: '🔒',
    text: t("components.FloatingBall.k18"),
    action: 'check_security'
  }];
  if (path.startsWith('/spyglass')) return [{
    icon: '📊',
    text: t("components.FloatingBall.k19"),
    action: 'view_dashboard'
  }, {
    icon: '💡',
    text: t("components.FloatingBall.k20"),
    action: 'view_suggestions'
  }];
  return [{
    icon: '🏠',
    text: t("components.FloatingBall.k21"),
    action: 'go_home',
    route: '/home'
  }, {
    icon: '🧠',
    text: t("components.FloatingBall.k22"),
    action: 'go_intelligence',
    route: '/spyglass?tab=dashboard'
  }];
}
type PanelTab = 'ai' | 'suggestions' | 'chat';
const AUTO_COLLAPSE_MS = 3000; // 3秒自动收起

// 根据建议分类返回图标
function getCategoryIcon(category: string): string {
  const iconMap: Record<string, string> = {
    productivity: '📊',
    focus: '🎯',
    break: '☕',
    learning: '📚',
    efficiency: '⚡',
    health: '💪',
    workflow: '🔄',
    analytics: '📈',
    behavior: '🧠',
    schedule: '⏰'
  };
  return iconMap[category] || '💡';
}

// ===== 1.3 智能推荐模式：基于时间段自动切换 =====
type RecommendMode = 'morning' | 'work' | 'noon' | 'afternoon' | 'evening' | 'night';
interface ModeConfig {
  key: RecommendMode;
  label: string;
  icon: string;
  color: string;
  tips: Suggestion[];
}
const MODE_CONFIGS: Record<RecommendMode, ModeConfig> = {
  morning: {
    key: 'morning',
    label: t("components.FloatingBall.k23"),
    icon: '🌅',
    color: '#FFB84D',
    tips: [{
      icon: '📋',
      text: t("components.FloatingBall.k24"),
      action: 'go_home',
      route: '/home'
    }, {
      icon: '☕',
      text: t("components.FloatingBall.k25")
    }]
  },
  work: {
    key: 'work',
    label: t("components.FloatingBall.k26"),
    icon: '💼',
    color: '#00F0FF',
    tips: [{
      icon: '🎯',
      text: t("components.FloatingBall.k27"),
      action: 'start_timer'
    }, {
      icon: '🔇',
      text: t("components.FloatingBall.k28")
    }]
  },
  noon: {
    key: 'noon',
    label: t("components.FloatingBall.k29"),
    icon: '🍽️',
    color: '#FF80A0',
    tips: [{
      icon: '🍲',
      text: t("components.FloatingBall.k30")
    }, {
      icon: '😴',
      text: t("components.FloatingBall.k31")
    }]
  },
  afternoon: {
    key: 'afternoon',
    label: t("components.FloatingBall.k32"),
    icon: '⚡',
    color: '#B026FF',
    tips: [{
      icon: '🔄',
      text: t("components.FloatingBall.k33"),
      action: 'go_home',
      route: '/home'
    }, {
      icon: '💧',
      text: t("components.FloatingBall.k34")
    }]
  },
  evening: {
    key: 'evening',
    label: t("components.FloatingBall.k35"),
    icon: '🌙',
    color: '#7A8BE8',
    tips: [{
      icon: '📖',
      text: t("components.FloatingBall.k36")
    }, {
      icon: '📚',
      text: t("components.FloatingBall.k37")
    }]
  },
  night: {
    key: 'night',
    label: t("components.FloatingBall.k38"),
    icon: '💤',
    color: '#FF5050',
    tips: [{
      icon: '🛏️',
      text: t("components.FloatingBall.k39")
    }, {
      icon: '🌙',
      text: t("components.FloatingBall.k40")
    }]
  }
};
function getModeByHour(hour: number): RecommendMode {
  if (hour >= 6 && hour < 9) return 'morning';
  if (hour >= 9 && hour < 12) return 'work';
  if (hour >= 12 && hour < 14) return 'noon';
  if (hour >= 14 && hour < 18) return 'afternoon';
  if (hour >= 18 && hour < 23) return 'evening';
  return 'night';
}

// ===== 迷你对话 =====
interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
}
export default function FloatingBall() {
  const [expanded, setExpanded] = useState(false);
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [dynamicSuggestions, setDynamicSuggestions] = useState<Suggestion[]>([]);
  const [position, setPosition] = useState({
    x: 0,
    y: 200
  });
  const [dragging, setDragging] = useState(false);
  const [visible, setVisible] = useState(false);
  const [panelTab, setPanelTab] = useState<PanelTab>('ai');
  const [collapsedOrbs, setCollapsedOrbs] = useState<Set<string>>(new Set());
  const [exitingOrbs, setExitingOrbs] = useState<Set<string>>(new Set());
  // 迷你对话状态
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [chatInput, setChatInput] = useState('');
  const [chatLoading, setChatLoading] = useState(false);
  // 1.3 智能推荐模式：manualOverride 用于手动切换；不设置时按时间自动判断
  const [manualMode, setManualMode] = useState<RecommendMode | null>(null);
  const [currentHour, setCurrentHour] = useState(() => new Date().getHours());
  const chatScrollRef = useRef<HTMLDivElement>(null);
  const ballRef = useRef<HTMLDivElement>(null);
  const dragStart = useRef({
    x: 0,
    y: 0,
    ballX: 0,
    ballY: 0
  });
  const collapseTimers = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());
  // 1.4 通知与提醒：防止重复通知
  const nightNotifiedRef = useRef(false); // 深夜提醒每会话只触发一次
  const notifiedSuggestionIdsRef = useRef<Set<number>>(new Set()); // 已通知的高优先级建议
  const location = useLocation();
  const navigate = useNavigate();

  // 从 store 读取 AI orbs
  const {
    orbs,
    removeOrb,
    toggleFavorite,
    touchOrb,
    cleanupExpired
  } = useFloatingOrbStore();
  const hasAiOrbs = orbs.length > 0;
  const {
    user
  } = useAuthStore();
  const addToast = useNotifStore(s => s.addToast);

  // 1.3 当前模式：手动覆盖优先，否则按小时判断
  const autoMode = getModeByHour(currentHour);
  const currentMode: RecommendMode = manualMode ?? autoMode;
  const modeConfig = MODE_CONFIGS[currentMode];

  // 1.3 每分钟更新一次当前小时（用于自动切换模式）
  useEffect(() => {
    const updateHour = () => setCurrentHour(new Date().getHours());
    const iv = setInterval(updateHour, 60 * 1000);
    return () => clearInterval(iv);
  }, []);

  // 1.3 手动切换模式（点击徽章切换到下一个模式）
  const cycleMode = useCallback(() => {
    const order: RecommendMode[] = ['morning', 'work', 'noon', 'afternoon', 'evening', 'night'];
    const idx = order.indexOf(currentMode);
    const next = order[(idx + 1) % order.length];
    setManualMode(next);
    addToast({
      type: 'info',
      title: t("components.FloatingBall.k41", {
        label: MODE_CONFIGS[next].label
      }),
      duration: 2500
    });
  }, [currentMode, addToast]);

  // 1.4 深夜提醒：进入 night 模式时触发一次 Toast
  useEffect(() => {
    if (!visible) return;
    if (currentMode === 'night' && !nightNotifiedRef.current) {
      nightNotifiedRef.current = true;
      addToast({
        type: 'warning',
        title: t("components.FloatingBall.k42"),
        message: t("components.FloatingBall.k43"),
        duration: 6000
      });
    }
    // 离开 night 模式后重置，下次再进入可再次提醒
    if (currentMode !== 'night' && nightNotifiedRef.current) {
      nightNotifiedRef.current = false;
    }
  }, [currentMode, visible, addToast]);

  // 动态获取智能建议（V4 基于活动数据生成）
  useEffect(() => {
    const fetchDynamicSuggestions = async () => {
      if (!user?.id) return;
      try {
        const res = await intelligence.getSuggestions(String(user.id), 'pending');
        if (res?.code === 0 && res?.data?.suggestions) {
          const mapped: Suggestion[] = res.data.suggestions.map(s => ({
            icon: getCategoryIcon(s.category),
            text: s.title,
            action: 'dynamic_suggestion',
            route: s.source ? undefined : undefined
          }));
          setDynamicSuggestions(mapped);

          // 1.4 高优先级建议主动通知：检测 priority=high 的新建议
          for (const s of res.data.suggestions) {
            if (s.priority === 'high' && !notifiedSuggestionIdsRef.current.has(s.id)) {
              notifiedSuggestionIdsRef.current.add(s.id);
              addToast({
                type: 'info',
                title: `⚡ ${s.title}`,
                message: s.description || t("components.FloatingBall.k44"),
                duration: 6000,
                action: {
                  label: t("common.view"),
                  onClick: () => {
                    setExpanded(true);
                    setPanelTab('suggestions');
                  }
                }
              });
            }
          }
        }
      } catch {
        // 静默失败，使用 fallback
      }
    };
    fetchDynamicSuggestions();
    const iv = setInterval(fetchDynamicSuggestions, 60000); // 每分钟刷新一次
    return () => clearInterval(iv);
  }, [user?.id, location.pathname, addToast]);

  // 4.2 跨模块智能推荐：基于当前页面和实时活动数据生成跨模块建议
  const [crossModuleSuggestions, setCrossModuleSuggestions] = useState<Suggestion[]>([]);
  useEffect(() => {
    const generateCrossModuleSuggestions = async () => {
      if (!user?.id) return;
      try {
        const res = await intelligence.getRealtimeStats(String(user.id));
        if (res?.code === 0 && res?.data) {
          const rt = res.data;
          const path = location.pathname;
          const crossSugs: Suggestion[] = [];

          // 场景1: 知识库学习超过30分钟 → 推荐写日志总结
          if (path.startsWith('/knowledge') && rt.current_focus_module === 'knowledge_base' && rt.active_duration_today_secs >= 1800) {
            crossSugs.push({
              icon: '📖',
              text: t("components.FloatingBall.k45"),
              action: 'go_journal',
              route: '/home?tab=journal'
            });
          }
          // 场景2: 待办完成率低 → 推荐查看待办
          if (!path.startsWith('/home') && rt.modules_used_today >= 3 && rt.operations_today >= 50) {
            crossSugs.push({
              icon: '📋',
              text: t("components.FloatingBall.k46"),
              action: 'go_todo',
              route: '/home?tab=todo'
            });
          }
          // 场景3: 长时间专注 → 推荐查看仪表盘
          if (rt.active_duration_today_secs >= 3600 && !path.startsWith('/spyglass')) {
            crossSugs.push({
              icon: '📊',
              text: t("components.FloatingBall.k47"),
              action: 'go_spyglass',
              route: '/spyglass?tab=dashboard'
            });
          }
          // 场景4: 终端使用频繁 → 推荐查看活动感知
          if (rt.current_focus_module === 'terminal' && rt.operations_today >= 30) {
            crossSugs.push({
              icon: '🗂️',
              text: t("components.FloatingBall.k48"),
              action: 'go_activity',
              route: '/spyglass?tab=activity'
            });
          }
          // 场景5: 搜索模块使用 → 推荐将搜索结果归档到知识库
          if (path.startsWith('/search') && rt.current_focus_module === 'search') {
            crossSugs.push({
              icon: '📚',
              text: t("components.FloatingBall.k49"),
              action: 'go_kb',
              route: '/knowledge'
            });
          }
          setCrossModuleSuggestions(crossSugs.slice(0, 2)); // 最多2条，避免过多
        }
      } catch {
        // 静默失败
      }
    };
    generateCrossModuleSuggestions();
    const iv = setInterval(generateCrossModuleSuggestions, 120000); // 2分钟刷新
    return () => clearInterval(iv);
  }, [user?.id, location.pathname]);

  // 定期清理过期 orb
  useEffect(() => {
    const interval = setInterval(cleanupExpired, 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, [cleanupExpired]);

  // 新 orb 自动收起计时器
  useEffect(() => {
    orbs.forEach(orb => {
      if (!collapsedOrbs.has(orb.id)) {
        // 清除旧的（如果有）
        const old = collapseTimers.current.get(orb.id);
        if (old) clearTimeout(old);
        const timer = setTimeout(() => {
          setCollapsedOrbs(prev => new Set(prev).add(orb.id));
          collapseTimers.current.delete(orb.id);
        }, AUTO_COLLAPSE_MS);
        collapseTimers.current.set(orb.id, timer);
      }
    });
    return () => {
      collapseTimers.current.forEach(t => clearTimeout(t));
      collapseTimers.current.clear();
    };
  }, [orbs.map(o => o.id).join(',')]); // 只在 orb ID 变化时触发

  // 检查开关状态
  const checkEnabled = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('get_system_config', {
        key: 'intelligence_frontend_settings'
      });
      if (res?.code === 0 && res?.data?.config_value) {
        const parsed = JSON.parse(res.data.config_value);
        const masterOn = parsed.toggles?.master_switch !== false;
        const ballOn = parsed.toggles?.floating_ball === true;
        setVisible(masterOn && ballOn);
      }
    } catch {
      setVisible(false);
    }
  }, []);
  useEffect(() => {
    checkEnabled();
  }, [checkEnabled, location.pathname]);
  useEffect(() => {
    const iv = setInterval(checkEnabled, 3000);
    return () => clearInterval(iv);
  }, [checkEnabled]);

  // 更新页面建议
  useEffect(() => {
    setSuggestions(getPageSuggestions(location.pathname));
  }, [location.pathname]);

  // 初始位置
  useEffect(() => {
    const updatePosition = () => setPosition(prev => ({
      x: Math.max(0, Math.min(window.innerWidth - 36, prev.x)),
      y: Math.max(0, Math.min(window.innerHeight - 36, prev.y))
    }));
    updatePosition();
    window.addEventListener('resize', updatePosition);
    return () => window.removeEventListener('resize', updatePosition);
  }, []);

  // 拖拽
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return;
    setDragging(true);
    dragStart.current = {
      x: e.clientX,
      y: e.clientY,
      ballX: position.x,
      ballY: position.y
    };
    e.preventDefault();
  }, [position]);
  useEffect(() => {
    if (!dragging) return;
    const handleMouseMove = (e: MouseEvent) => {
      const dx = e.clientX - dragStart.current.x;
      const dy = e.clientY - dragStart.current.y;
      setPosition({
        x: Math.max(0, Math.min(window.innerWidth - 36, dragStart.current.ballX + dx)),
        y: Math.max(0, Math.min(window.innerHeight - 36, dragStart.current.ballY + dy))
      });
    };
    const handleMouseUp = () => setDragging(false);
    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
    };
  }, [dragging]);

  // 点击球体：有AI内容时展开AI面板，否则展开对话面板
  const handleBallClick = () => {
    if (!dragging) {
      if (hasAiOrbs && !expanded) {
        setPanelTab('ai');
      } else if (!hasAiOrbs && !expanded) {
        setPanelTab('chat');
      }
      setExpanded(prev => !prev);
    }
  };

  // 点击 orb 切换展开/收起
  const handleOrbClick = (id: string) => {
    touchOrb(id);
    const t = collapseTimers.current.get(id);
    if (t) {
      clearTimeout(t);
      collapseTimers.current.delete(id);
    }
    setCollapsedOrbs(prev => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  };

  // 删除 orb
  const handleDeleteOrb = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setExitingOrbs(prev => new Set(prev).add(id));
    setTimeout(() => removeOrb(id), 280);
  };

  // 收藏 orb
  const handleFavOrb = (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    touchOrb(id);
    toggleFavorite(id);
  };

  // 建议项点击
  const handleSuggestionClick = (s: Suggestion) => {
    if (s.route) navigate(s.route);
    setExpanded(false);
  };

  // 迷你对话：发送消息
  const handleChatSend = async () => {
    const text = chatInput.trim();
    if (!text || chatLoading) return;
    const userMsg: ChatMessage = {
      role: 'user',
      content: text
    };
    setChatMessages(prev => [...prev, userMsg]);
    setChatInput('');
    setChatLoading(true);

    // 滚动到底部
    setTimeout(() => {
      chatScrollRef.current?.scrollTo({
        top: 999999,
        behavior: 'smooth'
      });
    }, 50);
    try {
      const res = await ipc.invoke<string>('intelligence_query_local_llm', {
        prompt: text
      });
      const reply = res?.code === 0 && res?.data ? res.data : t("components.FloatingBall.k50");
      setChatMessages(prev => [...prev, {
        role: 'assistant',
        content: reply
      }]);
    } catch (err) {
      setChatMessages(prev => [...prev, {
        role: 'assistant',
        content: t("components.FloatingBall.k51") + (err instanceof Error ? err.message : t("errors.unknown"))
      }]);
    } finally {
      setChatLoading(false);
      setTimeout(() => {
        chatScrollRef.current?.scrollTo({
          top: 999999,
          behavior: 'smooth'
        });
      }, 50);
    }
  };
  const handleChatKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleChatSend();
    }
  };

  // 合并建议：跨模块优先（场景化），动态次之，模式建议次之，静态补充
  const modeTips = modeConfig.tips;
  const displaySuggestions = crossModuleSuggestions.length > 0 ? [...crossModuleSuggestions, ...(dynamicSuggestions.length > 0 ? dynamicSuggestions.slice(0, 1) : modeTips.slice(0, 1)), ...suggestions.slice(0, 1)] : dynamicSuggestions.length > 0 ? [...dynamicSuggestions, ...modeTips.slice(0, 1), ...suggestions.slice(0, 1)] : [...modeTips, ...suggestions];
  if (!visible) return null;

  // 计算面板位置
  const isLeftSide = position.x < window.innerWidth / 2;
  return <>
      {/* 遮罩 */}
      {expanded && <div className={styles.backdrop} onClick={() => setExpanded(false)} />}

      {/* 展开面板 */}
      {expanded && <div className={styles.panel} style={{
      right: isLeftSide ? undefined : '30px',
      left: isLeftSide ? '30px' : undefined,
      top: Math.min(position.y, window.innerHeight - 400),
      width: hasAiOrbs ? '340px' : '280px'
    }}>
          {/* Tab 栏 */}
          {hasAiOrbs && <div className={styles.tabBar}>
              <button className={`${styles.tabBtn} ${panelTab === 'ai' ? styles.tabActive : ''}`} onClick={() => setPanelTab('ai')}>
                {t("components.FloatingBall.k52")}{orbs.length > 0 && <span className={styles.tabBadge}>{orbs.length}</span>}
              </button>
              <button className={`${styles.tabBtn} ${panelTab === 'chat' ? styles.tabActive : ''}`} onClick={() => setPanelTab('chat')}>{t("components.FloatingBall.k53")}</button>
              <button className={`${styles.tabBtn} ${panelTab === 'suggestions' ? styles.tabActive : ''}`} onClick={() => setPanelTab('suggestions')}>{t("components.FloatingBall.k54")}</button>
            </div>}

          {/* 无 AI orbs 时的简化 Tab 栏 */}
          {!hasAiOrbs && <div className={styles.tabBar}>
              <button className={`${styles.tabBtn} ${panelTab === 'chat' ? styles.tabActive : ''}`} onClick={() => setPanelTab('chat')}>{t("components.FloatingBall.k53")}</button>
              <button className={`${styles.tabBtn} ${panelTab === 'suggestions' ? styles.tabActive : ''}`} onClick={() => setPanelTab('suggestions')}>{t("components.FloatingBall.k54")}</button>
            </div>}

          {/* ===== AI Orb 内容区 ===== */}
          {panelTab === 'ai' && <div className={styles.aiContent}>
              {orbs.length === 0 ? <div className={styles.emptyHint}>{t("components.FloatingBall.k55")}</div> : orbs.map(orb => {
          const isCollapsed = collapsedOrbs.has(orb.id);
          const isExiting = exitingOrbs.has(orb.id);
          return <div key={orb.id} className={`${styles.orbCard} ${isExiting ? styles.orbExiting : ''}`} style={{
            '--orb-color': orb.color || '#00F0FF'
          } as React.CSSProperties}>
                      {!isCollapsed ? (/* 展开态 */
            <div className={styles.orbExpanded}>
                          <div className={styles.orbHeader}>
                            <span>{orb.icon}</span>
                            <span className={styles.orbTitle}>{orb.title}</span>
                            <div className={styles.orbActions}>
                              <button className={`${styles.orbActBtn} ${orb.favorited ? styles.favorited : ''}`} onClick={e => handleFavOrb(orb.id, e)} title={orb.favorited ? t("components.FloatingBall.k56") : t("components.FloatingBall.k57")}>{orb.favorited ? '★' : '☆'}</button>
                              <button className={styles.orbActBtn} onClick={e => handleDeleteOrb(orb.id, e)} title={t("common.delete")}>×</button>
                              <button className={styles.orbActBtn} onClick={() => handleOrbClick(orb.id)} title={t("common.collapse")}>−</button>
                            </div>
                          </div>
                          <div className={styles.orbBody}>
                            {orb.content && <p className={styles.orbText}>{orb.content}</p>}
                            {orb.keyPoints && orb.keyPoints.length > 0 && <ul className={styles.orbPoints}>
                                {orb.keyPoints.map((p, i) => <li key={i}>{p}</li>)}
                              </ul>}
                          </div>
                          <div className={styles.timerBar} />
                        </div>) : (/* 收起态 */
            <div className={styles.orbMini} onClick={() => handleOrbClick(orb.id)}>
                          <span className={styles.orbMiniIcon}>{orb.icon}</span>
                          <span className={styles.orbMiniText}>{orb.title}</span>
                          {orb.favorited && <span className={styles.favStar}>★</span>}
                        </div>)}
                    </div>;
        })}
            </div>}

          {/* ===== 迷你对话区 ===== */}
          {panelTab === 'chat' && <div className={styles.chatContainer}>
              <div className={styles.chatMessages} ref={chatScrollRef}>
                {chatMessages.length === 0 ? <div className={styles.chatEmpty}>
                    <span>{t("components.FloatingBall.k58")}</span>
                    <p>{t("components.FloatingBall.k59")}</p>
                    <p className={styles.chatEmptyHint}>{t("components.FloatingBall.k60")}</p>
                  </div> : chatMessages.map((msg, i) => <div key={i} className={`${styles.chatMsg} ${msg.role === 'user' ? styles.chatMsgUser : styles.chatMsgAI}`}>
                      <span className={styles.chatMsgIcon}>{msg.role === 'user' ? t("components.FloatingBall.k61") : 'AI'}</span>
                      <div className={styles.chatMsgContent}>{msg.content}</div>
                    </div>)}
                {chatLoading && <div className={`${styles.chatMsg} ${styles.chatMsgAI}`}>
                    <span className={styles.chatMsgIcon}>AI</span>
                    <div className={styles.chatMsgContent}>
                      <span className={styles.chatTyping}>{t("components.FloatingBall.k62")}</span>
                    </div>
                  </div>}
              </div>
              <div className={styles.chatInputBar}>
                <textarea className={styles.chatInput} value={chatInput} onChange={e => setChatInput(e.target.value)} onKeyDown={handleChatKeyDown} placeholder={t("components.FloatingBall.k63")} rows={1} disabled={chatLoading} />
                <button className={styles.chatSendBtn} onClick={handleChatSend} disabled={!chatInput.trim() || chatLoading}>{t("components.FloatingBall.k64")}</button>
              </div>
            </div>}

          {/* ===== 页面建议区 ===== */}
          {(!hasAiOrbs || panelTab === 'suggestions') && <>
              <div className={styles.panelHeader}>
                <span>{t("components.FloatingBall.k65")}</span>
                <button className={styles.modeBadge} onClick={cycleMode} title={t("components.FloatingBall.k66")} style={{
            '--mode-color': modeConfig.color
          } as React.CSSProperties}>
                  <span className={styles.modeIcon}>{modeConfig.icon}</span>
                  <span className={styles.modeLabel}>{modeConfig.label}</span>
                  {manualMode && <span className={styles.modeManual}>{t("components.FloatingBall.k67")}</span>}
                </button>
                <button className={styles.panelClose} onClick={() => setExpanded(false)}>×</button>
              </div>
              <div className={styles.panelContent}>
                {displaySuggestions.map((s, i) => <div key={i} className={styles.suggestionItem} onClick={() => handleSuggestionClick(s)}>
                    <span className={styles.suggestionIcon}>{s.icon}</span>
                    <span className={styles.suggestionText}>{s.text}</span>
                  </div>)}
                {dynamicSuggestions.length > 0 && <div className={styles.dynamicBadge}>{t("components.FloatingBall.k68")}</div>}
              </div>
            </>}
        </div>}

      {/* 球体 */}
      <div ref={ballRef} className={`${styles.ball} ${expanded ? styles.ballActive : ''} ${hasAiOrbs ? styles.ballHasContent : ''}`} style={{
      left: position.x,
      top: position.y
    }} onMouseDown={handleMouseDown} onClick={handleBallClick} title={hasAiOrbs ? t("components.FloatingBall.k69", {
      length: orbs.length
    }) : t("components.FloatingBall.k70")}>
        <span className={styles.ballIcon}>{expanded ? '×' : '🤖'}</span>
        {hasAiOrbs && <span className={styles.ballBadge}>{orbs.length}</span>}
      </div>
    </>;
}