import { t } from "i18next";
/**
 * GroupChatOrchestrationPanel - 群聊编排引擎面板
 * 
 * v1.02 新增：AI 编排事件实时监听与展示
 *        - 监听后端 'ai-orchestrator' Tauri Event
 *        - 实时显示轮次进度、参与者状态、Token 使用
 *        - 支持事件日志回放
 */

import React, { useEffect, useState, useRef } from 'react';
import { useChatStore } from '@/stores/chatStore';
import { ai } from '@/lib/ipc';
import type { GroupChatOrchestration } from '@/types';
interface GroupChatOrchestrationPanelProps {
  sessionId: number;
}
interface OrchestrationEvent {
  id: number;
  timestamp: string;
  type: 'round_start' | 'message_complete' | 'token_warning' | 'token_exhausted' | 'converged' | 'round_limit' | 'error';
  data: {
    conversation_id?: number;
    event?: string;
    round?: number;
    participant?: string;
    tokens?: number;
    [key: string]: any;
  };
}
export const GroupChatOrchestrationPanel: React.FC<GroupChatOrchestrationPanelProps> = ({
  sessionId
}) => {
  const {
    updateOrchestration,
    endOrchestration,
    getOrchestration
  } = useChatStore();
  const currentOrchestration = getOrchestration(sessionId);
  const [eventLog, setEventLog] = useState<OrchestrationEvent[]>([]);
  const [isListening, setIsListening] = useState(false);
  const eventIdRef = useRef(0);
  const unlistenRef = useRef<(() => void) | null>(null);
  useEffect(() => {
    const setupEventListener = async () => {
      if (typeof window === 'undefined' || !window.__TAURI__) {
        console.warn('[GroupChat] ⚠️ 当前不在 Tauri 环境中，无法监听 AI 编排事件');
        return;
      }
      try {
        const {
          listen
        } = await import('@tauri-apps/api/event');
        const unlisten = await listen<{
          conversation_id: number;
          event: string;
          round?: number;
          participant?: string;
          tokens?: number;
          [key: string]: any;
        }>('ai-orchestrator', event => {
          const payload = event.payload;
          if (payload.conversation_id !== sessionId) return;
          const newEvent: OrchestrationEvent = {
            id: ++eventIdRef.current,
            timestamp: new Date().toLocaleTimeString('zh-CN'),
            type: payload.event as OrchestrationEvent['type'],
            data: payload
          };
          setEventLog(prev => [...prev.slice(-50), newEvent]);
          switch (payload.event) {
            case 'round_start':
              updateOrchestration(sessionId, {
                current_round: payload.round || 1,
                status: 'discussing'
              });
              break;
            case 'message_complete':
              if (currentOrchestration) {
                updateOrchestration(sessionId, {
                  used_tokens: currentOrchestration.used_tokens + (payload.tokens || 0)
                });
              }
              break;
            case 'token_warning':
              console.warn(`[GroupChat] ⚠️ Token 使用量达到 80% 预警`);
              break;
            case 'token_exhausted':
              updateOrchestration(sessionId, {
                status: 'timeout'
              });
              endOrchestration(sessionId, t("components.GroupChatOrchestrationPanel.k1"), t("components.GroupChatOrchestrationPanel.k2"));
              break;
            case 'converged':
              updateOrchestration(sessionId, {
                status: 'converged',
                current_round: payload.round || currentOrchestration?.current_round || 0
              });
              setTimeout(() => {
                updateOrchestration(sessionId, {
                  status: 'summarizing'
                });
              }, 1000);
              break;
            case 'round_limit':
              updateOrchestration(sessionId, {
                status: 'summarizing',
                current_round: currentOrchestration?.max_rounds || 10
              });
              break;
            default:
              console.log('[GroupChat] 📨 收到未知事件:', payload.event);
          }
        });
        unlistenRef.current = unlisten;
        setIsListening(true);
        console.log('[GroupChat] ✅ 已开始监听 ai-orchestrator 事件');
      } catch (error) {
        console.error('[GroupChat] ❌ 设置事件监听失败:', error);
      }
    };
    if (currentOrchestration && (currentOrchestration.status === 'discussing' || currentOrchestration.status === 'converged')) {
      setupEventListener();
    }
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
        setIsListening(false);
      }
    };
  }, [sessionId, currentOrchestration?.status]);
  if (!currentOrchestration) {
    return null;
  }
  const tokenPercentage = currentOrchestration.used_tokens / currentOrchestration.total_token_budget * 100;
  const getTokenColor = (): string => {
    if (tokenPercentage >= 95) return '#FF0000';
    if (tokenPercentage >= 80) return '#FFFF00';
    return '#00FF00';
  };
  const getStatusStyle = (): React.CSSProperties => {
    switch (currentOrchestration.status) {
      case 'discussing':
        return {
          backgroundColor: '#00E0E0',
          color: '#000000'
        };
      case 'converged':
        return {
          backgroundColor: '#00FF00',
          color: '#000000'
        };
      case 'summarizing':
        return {
          backgroundColor: '#9D00FF',
          color: '#FFFFFF'
        };
      case 'completed':
        return {
          backgroundColor: '#00FF00',
          color: '#000000'
        };
      case 'timeout':
        return {
          backgroundColor: '#FF4444',
          color: '#FFFFFF'
        };
      default:
        return {
          backgroundColor: '#888888',
          color: '#000000'
        };
    }
  };
  const getStatusText = (): string => {
    switch (currentOrchestration.status) {
      case 'discussing':
        return t("components.GroupChatOrchestrationPanel.k3");
      case 'converged':
        return t("components.GroupChatOrchestrationPanel.k4");
      case 'summarizing':
        return t("components.GroupChatOrchestrationPanel.k5");
      case 'completed':
        return t("components.GroupChatOrchestrationPanel.k6");
      case 'timeout':
        return t("components.GroupChatOrchestrationPanel.k7");
      default:
        return t("components.GroupChatOrchestrationPanel.k8");
    }
  };
  const handleEndDiscussion = async () => {
    if (window.confirm(t("components.GroupChatOrchestrationPanel.k9"))) {
      try {
        updateOrchestration(sessionId, {
          status: 'summarizing'
        });
        const response = await ai.endGroupChat(sessionId);
        if (response.code === 0 && response.data) {
          endOrchestration(sessionId, response.data.summary, response.data.summarizer);
        } else {
          endOrchestration(sessionId, t("components.GroupChatOrchestrationPanel.k10"), t("components.GroupChatOrchestrationPanel.k2"));
        }
      } catch (error) {
        console.error('结束讨论失败:', error);
        alert(t("components.GroupChatOrchestrationPanel.k11"));
        updateOrchestration(sessionId, {
          status: 'discussing'
        });
      }
    }
  };
  return <div style={{
    border: '1px solid #00E0E0',
    backgroundColor: '#000000',
    padding: '16px',
    marginBottom: '16px',
    fontFamily: 'Consolas, monospace'
  }}>
      <div style={{
      display: 'flex',
      justifyContent: 'space-between',
      alignItems: 'center',
      marginBottom: '12px'
    }}>
        <h3 style={{
        color: '#00E0E0',
        margin: 0,
        fontSize: '14px'
      }}>
          {t("components.GroupChatOrchestrationPanel.k12")}
        </h3>
        <span style={{
        ...getStatusStyle(),
        padding: '4px 12px',
        borderRadius: '0',
        fontSize: '12px',
        fontWeight: 'bold'
      }}>
          {getStatusText()}
        </span>
      </div>

      <div style={{
      display: 'flex',
      gap: '24px',
      marginBottom: '12px',
      alignItems: 'center'
    }}>
        <div style={{
        flex: 1
      }}>
          <div style={{
          color: '#888888',
          fontSize: '12px',
          marginBottom: '4px'
        }}>
            {t("components.GroupChatOrchestrationPanel.k13")}
          </div>
          <div style={{
          color: '#00FF00',
          fontSize: '24px',
          fontWeight: 'bold'
        }}>
            {currentOrchestration.current_round} / {currentOrchestration.max_rounds}
          </div>
        </div>

        <div style={{
        flex: 2
      }}>
          <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          color: '#888888',
          fontSize: '12px',
          marginBottom: '4px'
        }}>
            <span>{t("components.GroupChatOrchestrationPanel.k14")}</span>
            <span style={{
            color: getTokenColor()
          }}>
              {currentOrchestration.used_tokens.toLocaleString()} / {currentOrchestration.total_token_budget.toLocaleString()}
              ({tokenPercentage.toFixed(1)}%)
            </span>
          </div>
          
          <div style={{
          width: '100%',
          height: '8px',
          backgroundColor: '#1a1a1a',
          border: '1px solid #333333'
        }}>
            <div style={{
            width: `${Math.min(tokenPercentage, 100)}%`,
            height: '100%',
            backgroundColor: getTokenColor(),
            transition: 'width 0.3s ease, background-color 0.3s ease'
          }} />
          </div>

          {tokenPercentage >= 80 && <div style={{
          color: tokenPercentage >= 95 ? '#FF0000' : '#FFFF00',
          fontSize: '11px',
          marginTop: '4px'
        }}>
              ⚠️ {tokenPercentage >= 95 ? t("components.GroupChatOrchestrationPanel.k15") : t("components.GroupChatOrchestrationPanel.k16")}
            </div>}
        </div>
      </div>

      {currentOrchestration.timeout_models.length > 0 && <div style={{
      backgroundColor: '#1a0000',
      border: '1px solid #FF4444',
      padding: '8px 12px',
      marginBottom: '12px'
    }}>
          <div style={{
        color: '#FF4444',
        fontSize: '12px',
        marginBottom: '4px'
      }}>
            {t("components.GroupChatOrchestrationPanel.k17")}
          </div>
          <div style={{
        color: '#FF0000',
        fontSize: '13px'
      }}>
            {currentOrchestration.timeout_models.join(', ')}
          </div>
        </div>}

      {currentOrchestration.status !== 'completed' && currentOrchestration.status !== 'timeout' && <div style={{
      display: 'flex',
      gap: '12px',
      justifyContent: 'flex-end'
    }}>
          {(currentOrchestration.status === 'discussing' || currentOrchestration.status === 'converged') && <button onClick={handleEndDiscussion} style={{
        padding: '8px 20px',
        backgroundColor: '#000000',
        border: '2px solid #FF0000',
        color: '#FF0000',
        fontFamily: 'Consolas, monospace',
        fontSize: '13px',
        cursor: 'pointer',
        fontWeight: 'bold',
        transition: 'all 0.2s'
      }} onMouseEnter={(e: React.MouseEvent<HTMLButtonElement>) => {
        e.currentTarget.style.backgroundColor = '#FF0000';
        e.currentTarget.style.color = '#000000';
      }} onMouseLeave={(e: React.MouseEvent<HTMLButtonElement>) => {
        e.currentTarget.style.backgroundColor = '#000000';
        e.currentTarget.style.color = '#FF0000';
      }}>
              {t("components.GroupChatOrchestrationPanel.k18")}
            </button>}
        </div>}

      {(currentOrchestration.status === 'completed' || currentOrchestration.summary) && <div style={{
      marginTop: '16px',
      borderTop: '1px solid #00E0E0',
      paddingTop: '12px'
    }}>
          <div style={{
        color: '#00FF00',
        fontSize: '13px',
        fontWeight: 'bold',
        marginBottom: '8px'
      }}>
            {t("components.GroupChatOrchestrationPanel.k19")}
          </div>
          
          {currentOrchestration.summarizer && <div style={{
        color: '#00E0E0',
        fontSize: '11px',
        marginBottom: '6px'
      }}>
              {t("components.GroupChatOrchestrationPanel.k20")} {currentOrchestration.summarizer}
            </div>}

          <div style={{
        color: '#CCCCCC',
        fontSize: '13px',
        lineHeight: '1.6',
        whiteSpace: 'pre-wrap'
      }}>
            {currentOrchestration.summary}
          </div>

          <div style={{
        display: 'flex',
        gap: '24px',
        marginTop: '10px',
        color: '#666666',
        fontSize: '11px'
      }}>
            <span>{t("components.GroupChatOrchestrationPanel.k21")} {new Date(currentOrchestration.started_at).toLocaleString('zh-CN')}</span>
            {currentOrchestration.ended_at && <span>{t("components.GroupChatOrchestrationPanel.k22")} {new Date(currentOrchestration.ended_at).toLocaleString('zh-CN')}</span>}
          </div>
        </div>}

      <details style={{
      marginTop: '12px'
    }}>
        <summary style={{
        color: '#666666',
        cursor: 'pointer',
        fontSize: '11px'
      }}>
          {t("components.GroupChatOrchestrationPanel.k23")}
        </summary>
        <pre style={{
        color: '#888888',
        fontSize: '11px',
        marginTop: '8px',
        padding: '8px',
        backgroundColor: '#0a0a0a',
        border: '1px solid #333333',
        overflow: 'auto',
        maxHeight: '200px'
      }}>
          {JSON.stringify(currentOrchestration, null, 2)}
        </pre>
      </details>

      {eventLog.length > 0 && <div style={{
      marginTop: '12px',
      borderTop: '1px solid #333333',
      paddingTop: '12px'
    }}>
          <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: '8px'
      }}>
            <span style={{
          color: isListening ? '#00FF00' : '#888888',
          fontSize: '12px',
          fontWeight: 'bold'
        }}>
              {t("components.GroupChatOrchestrationPanel.k24")} {isListening && t("components.GroupChatOrchestrationPanel.k25")}
            </span>
            <span style={{
          color: '#666666',
          fontSize: '11px'
        }}>
              {t("components.GroupChatOrchestrationPanel.k26")} {eventLog.length} {t("components.GroupChatOrchestrationPanel.k27")}
            </span>
          </div>
          
          <div style={{
        backgroundColor: '#0a0a0a',
        border: '1px solid #333333',
        padding: '8px',
        maxHeight: '250px',
        overflowY: 'auto',
        fontFamily: 'Consolas, monospace',
        fontSize: '11px'
      }}>
            {eventLog.slice(-20).map(event => <div key={event.id} style={{
          display: 'flex',
          gap: '12px',
          padding: '4px 0',
          borderBottom: '1px solid #1a1a1a',
          alignItems: 'flex-start'
        }}>
                <span style={{
            color: '#666666',
            minWidth: '70px',
            flexShrink: 0
          }}>
                  {event.timestamp}
                </span>
                
                <span style={{
            color: getEventColor(event.type),
            fontWeight: 'bold',
            minWidth: '120px',
            flexShrink: 0
          }}>
                  [{getEventLabel(event.type)}]
                </span>
                
                <span style={{
            color: '#CCCCCC',
            wordBreak: 'break-word'
          }}>
                  {getEventDescription(event)}
                </span>
              </div>)}
            
            {eventLog.length === 0 && <div style={{
          color: '#666666',
          textAlign: 'center',
          padding: '20px'
        }}>
                {t("components.GroupChatOrchestrationPanel.k28")}
              </div>}
          </div>
        </div>}
    </div>;
};
const getEventColor = (type: OrchestrationEvent['type']): string => {
  switch (type) {
    case 'round_start':
      return '#00E0E0';
    case 'message_complete':
      return '#00FF00';
    case 'token_warning':
      return '#FFFF00';
    case 'token_exhausted':
      return '#FF0000';
    case 'converged':
      return '#9D00FF';
    case 'round_limit':
      return '#FFA500';
    case 'error':
      return '#FF4444';
    default:
      return '#888888';
  }
};
const getEventLabel = (type: OrchestrationEvent['type']): string => {
  switch (type) {
    case 'round_start':
      return t("components.GroupChatOrchestrationPanel.k29");
    case 'message_complete':
      return t("components.GroupChatOrchestrationPanel.k30");
    case 'token_warning':
      return t("components.GroupChatOrchestrationPanel.k31");
    case 'token_exhausted':
      return t("components.GroupChatOrchestrationPanel.k32");
    case 'converged':
      return t("components.GroupChatOrchestrationPanel.k33");
    case 'round_limit':
      return t("components.GroupChatOrchestrationPanel.k34");
    case 'error':
      return t("common.error");
    default:
      return t("components.GroupChatOrchestrationPanel.k35");
  }
};
const getEventDescription = (event: OrchestrationEvent): string => {
  const {
    data
  } = event;
  switch (event.type) {
    case 'round_start':
      return t("components.GroupChatOrchestrationPanel.k36", {
        arg0: data.round || '?'
      });
    case 'message_complete':
      return t("components.GroupChatOrchestrationPanel.k37", {
        arg0: data.participant || t("components.GroupChatOrchestrationPanel.k38"),
        arg1: data.tokens ? ` (+${data.tokens} tokens)` : ''
      });
    case 'token_warning':
      return t("components.GroupChatOrchestrationPanel.k39");
    case 'token_exhausted':
      return t("components.GroupChatOrchestrationPanel.k40");
    case 'converged':
      return t("components.GroupChatOrchestrationPanel.k41", {
        arg0: data.round || '?'
      });
    case 'round_limit':
      return t("components.GroupChatOrchestrationPanel.k42", {
        arg0: data.round || '?'
      });
    case 'error':
      return data.message || t("components.GroupChatOrchestrationPanel.k43");
    default:
      return JSON.stringify(data);
  }
};
export const RoundIndicator: React.FC<{
  current: number;
  max: number;
}> = ({
  current,
  max
}) => <div style={{
  textAlign: 'center'
}}>
    <div style={{
    color: '#888888',
    fontSize: '11px'
  }}>{t("components.GroupChatOrchestrationPanel.k44")}</div>
    <div style={{
    color: '#00FF00',
    fontSize: '20px',
    fontWeight: 'bold'
  }}>
      {current} / {max}
    </div>
  </div>;
export const TokenBudgetBar: React.FC<{
  used: number;
  total: number;
}> = ({
  used,
  total
}) => {
  const percentage = used / total * 100;
  const getColor = (): string => {
    if (percentage >= 95) return '#FF0000';
    if (percentage >= 80) return '#FFFF00';
    return '#00FF00';
  };
  return <div>
      <div style={{
      display: 'flex',
      justifyContent: 'space-between',
      fontSize: '11px',
      color: '#888888'
    }}>
        <span>Token</span>
        <span style={{
        color: getColor()
      }}>
          {used.toLocaleString()}/{total.toLocaleString()}
        </span>
      </div>
      <div style={{
      width: '100%',
      height: '6px',
      backgroundColor: '#1a1a1a',
      border: '1px solid #333'
    }}>
        <div style={{
        width: `${Math.min(percentage, 100)}%`,
        height: '100%',
        backgroundColor: getColor()
      }} />
      </div>
    </div>;
};
export const ConvergenceStatus: React.FC<{
  status: GroupChatOrchestration['status'];
}> = ({
  status
}) => {
  const config: Record<string, {
    text: string;
    bgColor: string;
    textColor: string;
  }> = {
    discussing: {
      text: t("components.GroupChatOrchestrationPanel.k3"),
      bgColor: '#00E0E0',
      textColor: '#000'
    },
    converged: {
      text: t("components.GroupChatOrchestrationPanel.k4"),
      bgColor: '#00FF00',
      textColor: '#000'
    },
    summarizing: {
      text: t("components.GroupChatOrchestrationPanel.k5"),
      bgColor: '#9D00FF',
      textColor: '#FFF'
    },
    completed: {
      text: t("components.GroupChatOrchestrationPanel.k6"),
      bgColor: '#00FF00',
      textColor: '#000'
    },
    timeout: {
      text: t("components.GroupChatOrchestrationPanel.k7"),
      bgColor: '#FF4444',
      textColor: '#FFF'
    }
  };
  const {
    text,
    bgColor,
    textColor
  } = config[status] || config.discussing;
  return <span style={{
    backgroundColor: bgColor,
    color: textColor,
    padding: '3px 10px',
    fontSize: '11px',
    fontWeight: 'bold'
  }}>
      {text}
    </span>;
};
export default GroupChatOrchestrationPanel;