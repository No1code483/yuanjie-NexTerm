import { t } from "i18next";
import { useState, useEffect, useRef } from 'react';
import { ipc, home, intelligence } from '@/lib/ipc';
import { useIntelligence } from '@/hooks/useIntelligence';
import type { TodoItem, TodoEnhanceResult } from './types';
import { SkeletonList } from '@/components/ui/Skeleton';
import { getPriorityConfig, isOverdue, formatDateChinese } from './utils';
import styles from '../Home.module.css';
export default function TodoPanel() {
  const {
    aiOn,
    featureOn
  } = useIntelligence();
  const [todos, setTodos] = useState<TodoItem[]>([]);
  const [loadingTodos, setLoadingTodos] = useState(false);
  const [today] = useState(() => new Date().toISOString().split('T')[0]);
  const [aiTodoGhost, setAiTodoGhost] = useState('');
  const [aiTodoGhostLoading, setAiTodoGhostLoading] = useState(false);
  const [todoEnhanceMap, setTodoEnhanceMap] = useState<Record<number, {
    loading: boolean;
    result?: TodoEnhanceResult;
  }>>({});
  const todoAnalysisTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Task 5.3: AI 智能补全建议
  const [todoSuggestions, setTodoSuggestions] = useState<Record<number, string[]>>({});
  const [todoSuggestLoading, setTodoSuggestLoading] = useState<Record<number, boolean>>({});
  const todoSuggestTimerRef = useRef<Record<number, ReturnType<typeof setTimeout>>>({});
  const loadTodos = async () => {
    setLoadingTodos(true);
    try {
      const result = await home.getTodos(today);
      if (result.code === 0 && result.data) {
        setTodos(result.data.map((item: any) => ({
          id: item.id,
          title: item.title,
          description: item.description,
          priority: item.priority,
          due_date: item.due_date,
          completed: item.completed,
          createdAt: new Date(item.created_at).toISOString().split('T')[0]
        })));
      }
    } catch (error) {
      console.error('加载待办事项失败:', error);
    } finally {
      setLoadingTodos(false);
    }
  };
  useEffect(() => {
    loadTodos();
  }, [today]);
  const addTodo = async () => {
    try {
      const result = await home.createTodo('', today);
      if (result.code === 0 && result.data) {
        setTodos(prev => [...prev, {
          id: result.data.id,
          title: result.data.title,
          priority: result.data.priority || 'medium',
          due_date: today,
          completed: result.data.completed,
          createdAt: new Date().toISOString().split('T')[0]
        }]);
      }
    } catch (error) {
      console.error('添加待办事项失败:', error);
    }
  };
  const toggleTodo = async (id: number) => {
    try {
      const result = await home.updateTodo(id);
      if (result.code === 0) {
        setTodos(prev => prev.map(todo => todo.id === id ? {
          ...todo,
          completed: !todo.completed
        } : todo));
      }
    } catch (error) {
      console.error('切换待办状态失败:', error);
    }
  };
  const deleteTodo = async (id: number) => {
    try {
      const result = await home.deleteTodo(id);
      if (result.code === 0) setTodos(prev => prev.filter(todo => todo.id !== id));
    } catch (error) {
      console.error('删除待办事项失败:', error);
    }
  };
  const updateTodoDetails = async (id: number, updates: Partial<TodoItem>) => {
    const todo = todos.find(t => t.id === id);
    if (!todo) return;
    try {
      const result = await ipc.invoke('update_todo', {
        id,
        title: updates.title || todo.title,
        description: updates.description !== undefined ? updates.description : todo.description,
        priority: updates.priority || todo.priority,
        due_date: updates.due_date !== undefined ? updates.due_date : todo.due_date
      });
      if (result.code === 0) {
        setTodos(prev => prev.map(t => t.id === id ? {
          ...t,
          ...updates
        } : t));
      }
    } catch (error) {
      console.error('更新待办事项失败:', error);
    }
  };
  const handleTodoEnhance = async (todoId: number, title: string, description?: string) => {
    setTodoEnhanceMap(prev => ({
      ...prev,
      [todoId]: {
        loading: true
      }
    }));
    try {
      const res = await intelligence.todoEnhance(title, description);
      if (res?.data) {
        setTodoEnhanceMap(prev => ({
          ...prev,
          [todoId]: {
            loading: false,
            result: res.data!
          }
        }));
      }
    } catch {
      setTodoEnhanceMap(prev => ({
        ...prev,
        [todoId]: {
          loading: false
        }
      }));
    }
  };
  const dismissTodoEnhance = (todoId: number) => {
    setTodoEnhanceMap(prev => {
      const next = {
        ...prev
      };
      delete next[todoId];
      return next;
    });
  };

  // Task 5.3: AI 智能补全逻辑
  const handleTodoSuggest = async (todoId: number, partialText: string) => {
    if (partialText.trim().length < 2) {
      setTodoSuggestions(prev => {
        const n = {
          ...prev
        };
        delete n[todoId];
        return n;
      });
      return;
    }
    if (todoSuggestTimerRef.current[todoId]) {
      clearTimeout(todoSuggestTimerRef.current[todoId]);
    }
    todoSuggestTimerRef.current[todoId] = setTimeout(async () => {
      setTodoSuggestLoading(prev => ({
        ...prev,
        [todoId]: true
      }));
      try {
        const res = await intelligence.todoEnhance(partialText);
        if (res?.data?.suggestions && res.data.suggestions.length > 0) {
          setTodoSuggestions(prev => ({
            ...prev,
            [todoId]: res.data!.suggestions
          }));
        }
      } catch {
        // ignore
      } finally {
        setTodoSuggestLoading(prev => ({
          ...prev,
          [todoId]: false
        }));
      }
    }, 500);
  };
  const acceptTodoSuggestion = (todoId: number, suggestion: string) => {
    updateTodoDetails(todoId, {
      title: suggestion
    });
    setTodoSuggestions(prev => {
      const n = {
        ...prev
      };
      delete n[todoId];
      return n;
    });
  };
  useEffect(() => {
    if (!aiOn || !featureOn('smart_complete') || todos.length === 0) {
      setAiTodoGhost('');
      return;
    }
    if (todoAnalysisTimerRef.current) clearTimeout(todoAnalysisTimerRef.current);
    setAiTodoGhostLoading(true);
    todoAnalysisTimerRef.current = setTimeout(async () => {
      try {
        const res = await home.aiCompleteTodo();
        if (res.code === 0 && res.data) setAiTodoGhost(res.data!.suggestion || '');else setAiTodoGhost('');
      } catch {
        setAiTodoGhost('');
      } finally {
        setAiTodoGhostLoading(false);
      }
    }, 2000);
    return () => {
      if (todoAnalysisTimerRef.current) clearTimeout(todoAnalysisTimerRef.current);
    };
  }, [todos.length, aiOn, featureOn]);
  const completedCount = todos.filter(t => t.completed).length;
  const totalCount = todos.length;
  return <div className={styles.subpage}>
      <div className={styles.todoStats}>
        <span className={styles.statsText}>
          {t("home.TodoPanel.k1")} <strong style={{
          color: '#00F0FF'
        }}>{completedCount}/{totalCount}</strong> {t("home.TodoPanel.k2")}
        </span>
        {totalCount > 0 && <div className={styles.progressBar}>
            <div className={styles.progressFill} style={{
          width: `${completedCount / totalCount * 100}%`
        }} />
          </div>}
      </div>
      <div className={styles.todoList}>
        {loadingTodos ? <SkeletonList count={5} /> : todos.length === 0 ? <div style={{
        textAlign: 'center',
        padding: '40px',
        color: '#6a6a8a'
      }}>{t("home.TodoPanel.k3")}</div> : todos.map(todo => {
        const priorityConfig = getPriorityConfig(todo.priority);
        const overdue = isOverdue(todo.due_date, today);
        const enhance = todoEnhanceMap[todo.id];
        return <div key={todo.id}>
                <div className={`${styles.todoItem} ${todo.completed ? styles.completed : ''} ${overdue ? styles.overdue : ''}`}>
                  <div className={styles.todoLeft}>
                    <div className={`${styles.todoCheckbox} ${todo.completed ? styles.checked : ''}`} onClick={() => toggleTodo(todo.id)} />
                    <span className={styles.priorityBadge} style={{
                background: priorityConfig.bg,
                color: priorityConfig.color,
                borderColor: priorityConfig.color
              }}>
                      {priorityConfig.label}
                    </span>
                  </div>
                  <div className={styles.todoContent}>
                    <div style={{
                position: 'relative'
              }}>
                      <input type="text" defaultValue={todo.title} placeholder={t("home.TodoPanel.k4")} onBlur={e => updateTodoDetails(todo.id, {
                  title: e.target.value
                })} onChange={e => {
                  if (aiOn && featureOn('smart_complete')) {
                    handleTodoSuggest(todo.id, e.target.value);
                  }
                }} className={styles.todoTitleInput} style={{
                  textDecoration: todo.completed ? 'line-through' : 'none',
                  paddingRight: aiOn && featureOn('smart_complete') ? 28 : undefined
                }} />
                      {aiOn && featureOn('smart_complete') && <span style={{
                  position: 'absolute',
                  right: 6,
                  top: '50%',
                  transform: 'translateY(-50%)',
                  fontSize: 12,
                  color: '#00FF00',
                  pointerEvents: 'none',
                  opacity: todoSuggestLoading[todo.id] ? 0.5 : 0.3
                }}>
                          {todoSuggestLoading[todo.id] ? '⏳' : '🧠'}
                        </span>}
                      {todoSuggestions[todo.id] && todoSuggestions[todo.id].length > 0 && <div style={{
                  position: 'absolute',
                  top: '100%',
                  left: 0,
                  right: 0,
                  zIndex: 10,
                  background: '#000000',
                  border: '1px solid #00FF00',
                  borderRadius: 4,
                  marginTop: 2,
                  overflow: 'hidden'
                }}>
                          {todoSuggestions[todo.id].map((s, i) => <div key={i} onClick={() => acceptTodoSuggestion(todo.id, s)} style={{
                    padding: '4px 8px',
                    fontSize: 11,
                    color: '#00FF00',
                    cursor: 'pointer',
                    fontFamily: 'monospace',
                    borderBottom: i < todoSuggestions[todo.id].length - 1 ? '1px solid rgba(0,255,0,0.15)' : 'none',
                    background: 'transparent'
                  }} onMouseEnter={e => e.currentTarget.style.background = 'rgba(0,255,0,0.08)'} onMouseLeave={e => e.currentTarget.style.background = 'transparent'}>
                              ▸ {s}
                            </div>)}
                        </div>}
                    </div>
                    <input type="text" defaultValue={todo.description || ''} placeholder={t("home.TodoPanel.k5")} onBlur={e => updateTodoDetails(todo.id, {
                description: e.target.value
              })} className={styles.todoDescInput} />
                    <div className={styles.todoMeta}>
                      <div className={styles.todoDueDate}>
                        <input type="date" value={todo.due_date ?? today} onChange={e => {
                    const newDate = e.target.value;
                    setTodos(prev => prev.map(t => t.id === todo.id ? {
                      ...t,
                      due_date: newDate || undefined
                    } : t));
                    updateTodoDetails(todo.id, {
                      due_date: newDate
                    });
                  }} className={styles.dateInput} />
                        <span className={styles.dateLabel}>{formatDateChinese(todo.due_date || today)}</span>
                        {overdue && <span className={styles.overdueBadge}>{t("home.TodoPanel.k6")}</span>}
                      </div>
                      <select value={todo.priority} onChange={e => {
                  const newPriority = e.target.value as 'low' | 'medium' | 'high';
                  setTodos(prev => prev.map(t => t.id === todo.id ? {
                    ...t,
                    priority: newPriority
                  } : t));
                  updateTodoDetails(todo.id, {
                    priority: newPriority
                  });
                }} className={styles.prioritySelect}>
                        <option value="low">{t("home.TodoPanel.k7")}</option>
                        <option value="medium">{t("home.TodoPanel.k8")}</option>
                        <option value="high">{t("home.TodoPanel.k9")}</option>
                      </select>
                    </div>
                  </div>
                  {aiOn && featureOn('smart_complete') && <button onClick={() => handleTodoEnhance(todo.id, todo.title, todo.description)} disabled={enhance?.loading} title={t("components.intelligence.SettingsPanel.k10")} style={{
              marginRight: 8,
              background: 'none',
              border: '1px solid rgba(176,38,255,0.3)',
              borderRadius: 4,
              color: '#B026FF',
              cursor: 'pointer',
              fontSize: 14,
              padding: '2px 6px'
            }}>
                      {enhance?.loading ? '⏳' : '🧠'}
                    </button>}
                  <button className={styles.deleteButton} onClick={() => deleteTodo(todo.id)} title={t("common.delete")}>×</button>
                </div>
                {enhance?.result && <div style={{
            marginTop: 4,
            padding: 10,
            borderRadius: 6,
            border: '1px solid rgba(176,38,255,0.2)',
            background: 'rgba(176,38,255,0.04)',
            fontSize: 12
          }}>
                    <div style={{
              display: 'flex',
              gap: 12,
              marginBottom: 6
            }}>
                      <span style={{
                color: '#B026FF'
              }}>
                        {t("home.TodoPanel.k10")} {enhance.result.suggest_priority === 'high' ? t("home.TodoPanel.k11") : enhance.result.suggest_priority === 'medium' ? t("home.TodoPanel.k12") : t("home.TodoPanel.k13")}
                      </span>
                      <span style={{
                color: 'rgba(255,255,255,0.5)'
              }}>{t("home.TodoPanel.k14")} {enhance.result.suggest_estimate_minutes} {t("home.TodoPanel.k15")}</span>
                      {enhance.result.suggest_category && <span style={{
                color: 'rgba(0,240,255,0.7)'
              }}>{t("home.TodoPanel.k16")} {enhance.result.suggest_category}</span>}
                    </div>
                    {enhance.result.suggestions.length > 0 && <div style={{
              color: 'rgba(255,255,255,0.6)',
              lineHeight: 1.6
            }}>
                        {enhance.result.suggestions.map((s, i) => <div key={i}>· {s}</div>)}
                      </div>}
                    <button onClick={() => dismissTodoEnhance(todo.id)} style={{
              marginTop: 6,
              padding: '2px 8px',
              fontSize: 11,
              borderRadius: 3,
              border: '1px solid rgba(255,255,255,0.15)',
              background: 'transparent',
              color: 'rgba(255,255,255,0.4)',
              cursor: 'pointer'
            }}>
                      {t("common.close")}
                    </button>
                  </div>}
              </div>;
      })}
      </div>
      <div style={{
      display: 'flex',
      gap: 10,
      marginTop: 10
    }}>
        <button className={styles.addButton} onClick={addTodo}>{t("home.TodoPanel.k17")}</button>
      </div>
      {aiTodoGhostLoading && <div className={styles.aiChecking} style={{
      marginTop: 8
    }}>{t("home.TodoPanel.k18")}</div>}
      {aiTodoGhost && <div className={styles.aiGhostText}>
          <div className={styles.aiGhostLabel}>{t("home.TodoPanel.k19")}</div>
          <div className={styles.aiGhostContent}>{aiTodoGhost}</div>
          <button className={styles.aiGhostDismiss} onClick={() => setAiTodoGhost('')}>✕</button>
        </div>}
    </div>;
}