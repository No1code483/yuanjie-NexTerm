import { t } from "i18next";
import { useState, useEffect, useRef, useCallback } from 'react';
import { ipc } from '@/lib/ipc';
import styles from './SelectionToolbar.module.css';
type ActionType = 'translate' | 'summarize' | 'rewrite' | 'explain';
interface ActionConfig {
  icon: string;
  label: string;
  prompt: (text: string) => string;
}
const ACTIONS: Record<ActionType, ActionConfig> = {
  translate: {
    icon: t("components.SelectionToolbar.k1"),
    label: t("components.SelectionToolbar.k2"),
    prompt: text => t("components.SelectionToolbar.k3", {
      text: text
    })
  },
  summarize: {
    icon: t("components.SelectionToolbar.k4"),
    label: t("components.SelectionToolbar.k5"),
    prompt: text => t("components.SelectionToolbar.k6", {
      text: text
    })
  },
  rewrite: {
    icon: t("components.SelectionToolbar.k7"),
    label: t("components.SelectionToolbar.k8"),
    prompt: text => t("components.SelectionToolbar.k9", {
      text: text
    })
  },
  explain: {
    icon: t("components.SelectionToolbar.k10"),
    label: t("components.SelectionToolbar.k11"),
    prompt: text => t("components.SelectionToolbar.k12", {
      text: text
    })
  }
};
export default function SelectionToolbar() {
  const [visible, setVisible] = useState(false);
  const [position, setPosition] = useState({
    x: 0,
    y: 0
  });
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<string | null>(null);
  const [resultVisible, setResultVisible] = useState(false);
  const [currentAction, setCurrentAction] = useState<ActionType | null>(null);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  // 检查智能开关是否开启
  const [enabled, setEnabled] = useState(false);
  useEffect(() => {
    const checkEnabled = async () => {
      try {
        const res = await ipc.invoke<any>('get_system_config', {
          key: 'intelligence_frontend_settings'
        });
        if (res?.code === 0 && res?.data?.config_value) {
          const parsed = JSON.parse(res.data.config_value);
          const masterOn = parsed.toggles?.master_switch !== false;
          const ballOn = parsed.toggles?.floating_ball === true;
          setEnabled(masterOn && ballOn);
        }
      } catch {
        setEnabled(false);
      }
    };
    checkEnabled();
    const iv = setInterval(checkEnabled, 5000);
    return () => clearInterval(iv);
  }, []);

  // 监听选中文本事件
  const handleSelectionChange = useCallback(() => {
    if (!enabled || loading) return;
    const selection = window.getSelection();
    if (!selection || selection.isCollapsed || selection.rangeCount === 0) {
      if (!loading) {
        setVisible(false);
        setResultVisible(false);
      }
      return;
    }
    const text = selection.toString().trim();
    if (text.length < 2 || text.length > 5000) {
      setVisible(false);
      return;
    }

    // 确保选区在 DOM 中
    const range = selection.getRangeAt(0);
    const rect = range.getBoundingClientRect();
    if (rect.width === 0 && rect.height === 0) return;

    // 隐藏结果面板，显示工具栏
    setResultVisible(false);
    setPosition({
      x: rect.left + rect.width / 2,
      y: rect.top - 8
    });
    setVisible(true);

    // 重置隐藏计时器
    if (hideTimer.current) clearTimeout(hideTimer.current);
  }, [enabled, loading]);

  // 延迟监听，避免拖拽时频繁触发
  useEffect(() => {
    let debounced: ReturnType<typeof setTimeout>;
    const handler = () => {
      clearTimeout(debounced);
      debounced = setTimeout(handleSelectionChange, 200);
    };
    document.addEventListener('selectionchange', handler);
    return () => {
      document.removeEventListener('selectionchange', handler);
      clearTimeout(debounced);
    };
  }, [handleSelectionChange]);

  // 鼠标离开时延迟隐藏
  const handleMouseLeave = () => {
    hideTimer.current = setTimeout(() => {
      if (!loading) {
        setVisible(false);
        setResultVisible(false);
      }
    }, 500);
  };
  const handleMouseEnter = () => {
    if (hideTimer.current) clearTimeout(hideTimer.current);
  };

  // 执行 AI 操作
  const handleAction = async (action: ActionType) => {
    const selection = window.getSelection();
    if (!selection) return;
    const text = selection.toString().trim();
    if (!text) return;
    setLoading(true);
    setCurrentAction(action);
    setResult(null);
    setResultVisible(true);
    try {
      const res = await ipc.invoke<string>('intelligence_query_local_llm', {
        prompt: ACTIONS[action].prompt(text)
      });
      if (res?.code === 0 && res?.data) {
        setResult(res.data);
      } else {
        setResult(t("components.FloatingBall.k50"));
      }
    } catch (err) {
      setResult(t("components.SelectionToolbar.k13") + (err instanceof Error ? err.message : t("errors.unknown")));
    } finally {
      setLoading(false);
    }
  };

  // 关闭结果
  const handleCloseResult = () => {
    setResultVisible(false);
    setVisible(false);
    window.getSelection()?.removeAllRanges();
  };
  if (!enabled) return null;
  return <>
      {/* 选中文本工具栏 */}
      {visible && !resultVisible && <div className={styles.toolbar} style={{
      left: Math.max(60, Math.min(window.innerWidth - 200, position.x - 80)),
      top: Math.max(40, position.y - 44)
    }} onMouseLeave={handleMouseLeave} onMouseEnter={handleMouseEnter}>
          {(Object.keys(ACTIONS) as ActionType[]).map(key => <button key={key} className={styles.toolBtn} onClick={() => handleAction(key)} disabled={loading} title={ACTIONS[key].label}>
              <span className={styles.toolIcon}>{ACTIONS[key].icon}</span>
              <span className={styles.toolLabel}>{ACTIONS[key].label}</span>
            </button>)}
        </div>}

      {/* AI 结果面板 */}
      {resultVisible && <div className={styles.resultPanel} style={{
      left: Math.max(20, Math.min(window.innerWidth - 360, position.x - 160)),
      top: Math.max(40, position.y - 200)
    }} onMouseLeave={handleMouseLeave} onMouseEnter={handleMouseEnter}>
          <div className={styles.resultHeader}>
            <span className={styles.resultTitle}>
              {loading ? t("components.SelectionToolbar.k14") : t("components.SelectionToolbar.k15", {
            arg0: ACTIONS[currentAction!]?.icon || '',
            arg1: ACTIONS[currentAction!]?.label || ''
          })}
            </span>
            <button className={styles.resultClose} onClick={handleCloseResult}>×</button>
          </div>
          <div className={styles.resultBody}>
            {loading ? <div className={styles.loadingDots}>
                <span></span><span></span><span></span>
              </div> : <pre className={styles.resultText}>{result}</pre>}
          </div>
        </div>}
    </>;
}