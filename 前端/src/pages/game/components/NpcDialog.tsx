/**
 * D4.2 智能 NPC 对话浮窗
 *
 * 使用方式：在游戏页面中通过按钮触发显示，传入选中的 NPC ID
 *
 * 简化版 UI：左侧 NPC 头像 + 名字 + 角色定位；右侧消息列表 + 输入框
 */

import { useEffect, useRef, useState } from 'react';
import { game, ai } from '@/lib/ipc';
import type { GameNpc, GameNpcConversation, GameNpcMemory, GameNpcRelationship, GameNpcRumor } from '@/types/game';
import styles from './NpcDialog.module.css';

interface NpcDialogProps {
  npcId: string;
  worldId: string;
  onClose: () => void;
}

/** D4.6 关系等级中文标签映射 */
const RELATIONSHIP_LABEL_TEXT: Record<string, string> = {
  hostile: '仇恨',
  cold: '冷淡',
  neutral: '中立',
  warm: '友好',
  close: '亲密',
  sworn: '挚友',
};

/** D4.6 关系条填充色（按 label 区分） */
const RELATIONSHIP_COLOR: Record<string, string> = {
  hostile: '#ff4444',
  cold: '#aa6644',
  neutral: '#888888',
  warm: '#66ccff',
  close: '#66ff99',
  sworn: '#ffd700',
};

/** D4.6 记忆类型中文标签 */
const MEMORY_TYPE_TEXT: Record<string, string> = {
  fact: '事实',
  preference: '偏好',
  commitment: '承诺',
  event: '事件',
};

export default function NpcDialog({ npcId, worldId, onClose }: NpcDialogProps) {
  const [npc, setNpc] = useState<GameNpc | null>(null);
  const [history, setHistory] = useState<GameNpcConversation[]>([]);
  const [input, setInput] = useState('');
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  // 默认模型 ID（用第一个可用模型）
  const [defaultModelId, setDefaultModelId] = useState<number | undefined>(undefined);

  // D4.6 关系 + 记忆状态
  const [relationship, setRelationship] = useState<GameNpcRelationship | null>(null);
  const [memories, setMemories] = useState<GameNpcMemory[]>([]);
  const [memoryPanelOpen, setMemoryPanelOpen] = useState(false);
  // 最近一次互动的关系增量与新增记忆条数（用于即时提示气泡）
  const [lastDelta, setLastDelta] = useState<number | null>(null);
  const [lastMemoryAdded, setLastMemoryAdded] = useState<number>(0);

  // D4.7 传闻状态：从其他 NPC 听说的关于玩家的事 + NPC 名字映射（显示传闻来源）
  const [rumors, setRumors] = useState<GameNpcRumor[]>([]);
  const [npcNameMap, setNpcNameMap] = useState<Record<string, string>>({});
  // 记忆面板子标签：亲历记忆 / 道听途说
  const [memoryTab, setMemoryTab] = useState<'own' | 'rumor'>('own');

  /** D4.6+D4.7 刷新关系 / 亲历记忆 / 传闻 + NPC 名字映射（对话后调用） */
  const refreshRelationshipAndMemories = async () => {
    try {
      const [relRes, memRes, rumorRes, npcListRes] = await Promise.all([
        game.npcRelationship(worldId, npcId),
        game.npcMemories(worldId, npcId, 50),
        game.npcRumors(worldId, npcId, 20).catch(() => null),
        game.npcList().catch(() => null),
      ]);
      if (relRes?.data) setRelationship(relRes.data);
      if (memRes?.data) setMemories(memRes.data);
      // D4.7 传闻加载失败不阻塞对话（catch 降级为 null → 空数组）
      if (rumorRes?.data) setRumors(rumorRes.data);
      if (npcListRes?.data) {
        const map: Record<string, string> = {};
        npcListRes.data.forEach(n => { map[n.id] = n.name; });
        setNpcNameMap(map);
      }
    } catch {
      // 静默失败：关系/记忆/传闻面板为辅助信息，不应阻塞对话
    }
  };

  useEffect(() => {
    (async () => {
      try {
        setError(null);
        // 加载 NPC 详情
        const npcRes = await game.npcGet(npcId);
        if (npcRes?.data) setNpc(npcRes.data);

        // 加载对话历史
        const historyRes = await game.npcHistory(worldId, npcId, 50);
        if (historyRes?.data) setHistory(historyRes.data);

        // D4.6 加载关系 + 长期记忆
        await refreshRelationshipAndMemories();

        // 加载默认模型
        const modelsRes = await ai.getModels();
        if (modelsRes?.data?.length) {
          setDefaultModelId(modelsRes.data[0].id);
        }
      } catch (e: any) {
        setError(e?.message || '加载 NPC 失败');
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [npcId, worldId]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [history]);

  const sendMessage = async () => {
    const text = input.trim();
    if (!text || sending) return;
    setSending(true);
    setError(null);

    // 乐观插入用户消息
    const tempId = `temp_${Date.now()}`;
    setHistory(prev => [...prev, {
      id: tempId,
      world_id: worldId,
      npc_id: npcId,
      role: 'user',
      content: text,
      turn_index: prev.length,
      created_at: Date.now()
    }]);
    setInput('');

    try {
      const res = await game.npcChat(worldId, npcId, text, defaultModelId);
      if (res?.data) {
        const data = res.data;
        setHistory(prev => [...prev, {
          id: `reply_${Date.now()}`,
          world_id: worldId,
          npc_id: npcId,
          role: 'assistant',
          content: data.reply,
          turn_index: data.turn_index,
          created_at: Date.now()
        }]);

        // D4.6 即时显示本次关系增量 + 新增记忆条数
        setLastDelta(data.relationship_delta);
        setLastMemoryAdded(data.memory_added);
        // 3 秒后淡出提示
        window.setTimeout(() => {
          setLastDelta(null);
          setLastMemoryAdded(0);
        }, 3500);

        // D4.6 后台刷新关系条 + 记忆列表（不阻塞 UI）
        refreshRelationshipAndMemories();
      }
    } catch (e: any) {
      setError(e?.message || '对话失败');
      // 移除乐观消息
      setHistory(prev => prev.filter(h => h.id !== tempId));
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

  const clearHistory = async () => {
    if (!confirm('确定清空与此 NPC 的对话历史？')) return;
    try {
      await game.npcClearHistory(worldId, npcId);
      setHistory([]);
    } catch (e: any) {
      setError(e?.message || '清空失败');
    }
  };

  if (!npc) {
    return <div className={styles.overlay}>
      <div className={styles.dialog}>
        <div className={styles.loading}>{error || '加载中...'}</div>
        <button className={styles.closeBtn} onClick={onClose}>✕</button>
      </div>
    </div>;
  }

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={e => e.stopPropagation()}>
        <div className={styles.header}>
          <div className={styles.npcInfo}>
            <span className={styles.avatar}>{npc.avatar_emoji}</span>
            <div>
              <div className={styles.name}>{npc.name}</div>
              <div className={styles.role}>{npc.location || '神秘地点'}</div>
            </div>
          </div>
          <div className={styles.headerActions}>
            <button className={styles.clearBtn} onClick={clearHistory} title="清空历史">🗑</button>
            <button className={styles.closeBtn} onClick={onClose}>✕</button>
          </div>
        </div>

        <div className={styles.personality}>{npc.personality}</div>

        {/* D4.6 关系条 + 记忆入口 */}
        {relationship && (
          <div className={styles.relationBar}>
            <div className={styles.relationHeader}>
              <span className={styles.relationLabel}>
                {RELATIONSHIP_LABEL_TEXT[relationship.relationship_label] || '中立'}
              </span>
              <span className={styles.relationValue}>
                {relationship.relationship_value > 0
                  ? `+${relationship.relationship_value}`
                  : relationship.relationship_value}
                /100
              </span>
              <button
                className={styles.memoryToggle}
                onClick={() => setMemoryPanelOpen(!memoryPanelOpen)}
                title="查看 NPC 对你的记忆与道听途说"
              >
                🧠 记忆 {memories.length} · 传闻 {rumors.length}
              </button>
            </div>
            <div className={styles.relationTrack}>
              <div
                className={styles.relationFill}
                style={{
                  // -100~100 映射到 0~100% 宽度
                  width: `${(relationship.relationship_value + 100) / 2}%`,
                  background:
                    RELATIONSHIP_COLOR[relationship.relationship_label] || '#888',
                }}
              />
            </div>
            {/* 本次互动关系增量提示 */}
            {lastDelta !== null && (
              <div
                className={`${styles.deltaBadge} ${
                  lastDelta > 0
                    ? styles.deltaPos
                    : lastDelta < 0
                      ? styles.deltaNeg
                      : styles.deltaNeutral
                }`}
              >
                关系 {lastDelta > 0 ? `+${lastDelta}` : lastDelta}
                {lastMemoryAdded > 0 ? ` · 记忆 +${lastMemoryAdded}` : ''}
              </div>
            )}
          </div>
        )}

        {/* D4.6+D4.7 记忆面板（可折叠）：亲历记忆 / 道听途说 两个子标签 */}
        {memoryPanelOpen && (
          <div className={styles.memoryPanel}>
            <div className={styles.memoryPanelHeader}>
              <button
                className={`${styles.memoryTab} ${memoryTab === 'own' ? styles.memoryTabActive : ''}`}
                onClick={() => setMemoryTab('own')}
              >
                🧠 亲历记忆 ({memories.length})
              </button>
              <button
                className={`${styles.memoryTab} ${memoryTab === 'rumor' ? styles.memoryTabActive : ''}`}
                onClick={() => setMemoryTab('rumor')}
              >
                📜 道听途说 ({rumors.length})
              </button>
            </div>

            {memoryTab === 'own' ? (
              memories.length === 0 ? (
                <div className={styles.memoryEmpty}>暂无长期记忆（多互动以累积）</div>
              ) : (
                <div className={styles.memoryList}>
                  {memories.map(m => (
                    <div key={m.id} className={styles.memoryItem}>
                      <span
                        className={`${styles.memoryType} ${
                          styles[`memType_${m.memory_type}`] || ''
                        }`}
                      >
                        {MEMORY_TYPE_TEXT[m.memory_type] || m.memory_type}
                      </span>
                      <span className={styles.memoryContent}>{m.content}</span>
                      <span className={styles.memoryMeta}>
                        {m.source === 'ai' ? 'AI' : '规则'} · ★{m.importance.toFixed(1)}
                      </span>
                    </div>
                  ))}
                </div>
              )
            ) : (
              // D4.7 道听途说：从其他 NPC 传播来的传闻
              rumors.length === 0 ? (
                <div className={styles.memoryEmpty}>
                  尚未听闻关于这位道友的事迹（与其他 NPC 互动可产生传闻传播）
                </div>
              ) : (
                <div className={styles.memoryList}>
                  {rumors.map(r => (
                    <div key={r.id} className={`${styles.memoryItem} ${styles.rumorItem}`}>
                      <span className={styles.rumorSource}>
                        传闻自 {npcNameMap[r.source_npc_id] || '某位道友'}
                      </span>
                      <span
                        className={`${styles.memoryType} ${
                          styles[`memType_${r.memory_type}`] || ''
                        }`}
                      >
                        {MEMORY_TYPE_TEXT[r.memory_type] || r.memory_type}
                      </span>
                      <span className={styles.memoryContent}>{r.content}</span>
                      <span className={styles.memoryMeta}>★{r.importance.toFixed(1)}</span>
                    </div>
                  ))}
                </div>
              )
            )}
          </div>
        )}

        <div className={styles.messages}>
          {history.length === 0 && (
            <div className={styles.empty}>
              <div className={styles.greetingBubble}>
                <span className={styles.avatarSmall}>{npc.avatar_emoji}</span>
                <span>{npc.greeting}</span>
              </div>
            </div>
          )}
          {history.map(m => (
            <div key={m.id} className={`${styles.msg} ${m.role === 'user' ? styles.msgUser : styles.msgNpc}`}>
              {m.role === 'assistant' && <span className={styles.avatarSmall}>{npc.avatar_emoji}</span>}
              <div className={styles.bubble}>{m.content}</div>
            </div>
          ))}
          {sending && (
            <div className={`${styles.msg} ${styles.msgNpc}`}>
              <span className={styles.avatarSmall}>{npc.avatar_emoji}</span>
              <div className={styles.bubble}><span className={styles.dots}>···</span></div>
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>

        {error && <div className={styles.error}>{error}</div>}

        <div className={styles.inputArea}>
          <input
            className={styles.input}
            value={input}
            onChange={e => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={`向 ${npc.name} 提问...`}
            disabled={sending}
          />
          <button
            className={styles.sendBtn}
            onClick={sendMessage}
            disabled={sending || !input.trim()}
          >
            {sending ? '···' : '发送'}
          </button>
        </div>
      </div>
    </div>
  );
}
