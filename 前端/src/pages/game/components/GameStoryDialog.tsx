/**
 * D4.4b 动态剧情浮窗
 *
 * 使用方式：在 GamePreview 页面通过按钮触发显示，传入当前 worldId + 玩家信息
 *
 * 三种视图：
 * 1. 主视图（main）：剧情节点时间线 + 当前节点选择交互
 * 2. 历史视图（history）：玩家历史剧情列表，点击可恢复
 * 3. 空视图（empty）：无剧情时显示"开启新剧情"CTA
 *
 * 数据流：
 *   - 进入 → 调 game.storyList() 取最近一条剧情，若存在则 game.storyGet() 加载
 *   - 开启新剧情 → game.storyGenerate() → 写入 DB → 展示首节点
 *   - 玩家点选择 → game.storyAdvance() → 追加新节点 → 自动滚动到底部
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { game, ai } from '@/lib/ipc';
import type { Story, StorySummary } from '@/types/game';
import styles from './GameStoryDialog.module.css';

interface GameStoryDialogProps {
  worldId: string;
  playerName: string;
  realm: string;
  onClose: () => void;
}

type ViewMode = 'main' | 'history';

export default function GameStoryDialog({
  worldId,
  playerName,
  realm,
  onClose,
}: GameStoryDialogProps) {
  const [story, setStory] = useState<Story | null>(null);
  const [history, setHistory] = useState<StorySummary[]>([]);
  const [view, setView] = useState<ViewMode>('main');
  const [loading, setLoading] = useState(true);
  const [advancing, setAdvancing] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [defaultModelId, setDefaultModelId] = useState<number | undefined>(undefined);

  const timelineRef = useRef<HTMLDivElement | null>(null);

  /** 初始化：加载默认模型 + 最近剧情 */
  useEffect(() => {
    (async () => {
      setLoading(true);
      setError(null);
      try {
        // 加载默认 AI 模型（用第一个可用模型）
        const modelsRes = await ai.getModels();
        if (modelsRes?.data?.length) {
          setDefaultModelId(modelsRes.data[0].id);
        }

        // 加载历史剧情列表
        const listRes = await game.storyList(20);
        const list = listRes?.data ?? [];
        setHistory(list);

        // 自动加载最近一条剧情（若有）
        if (list.length > 0) {
          const latestRes = await game.storyGet(list[0].id);
          if (latestRes?.data) {
            setStory(latestRes.data);
          }
        }
      } catch (e: any) {
        setError(e?.message || '加载剧情失败');
      } finally {
        setLoading(false);
      }
    })();
  }, []);

  /** 自动滚动到时间线底部（新节点生成时） */
  useEffect(() => {
    if (story && timelineRef.current) {
      timelineRef.current.scrollTop = timelineRef.current.scrollHeight;
    }
  }, [story]);

  /** 开启新剧情 */
  const handleGenerate = useCallback(async () => {
    if (!defaultModelId) {
      setError('未找到可用 AI 模型，请先在 AI 设置中配置模型');
      return;
    }
    setGenerating(true);
    setError(null);
    try {
      const res = await game.storyGenerate({
        world_id: worldId,
        player_name: playerName,
        realm,
        model_id: defaultModelId,
      });
      if (res.code !== 0) throw new Error(res.message);
      if (res.data) {
        setStory(res.data);
        setView('main');
        // 刷新历史列表
        const listRes = await game.storyList(20);
        if (listRes?.data) setHistory(listRes.data);
      }
    } catch (e: any) {
      setError(e?.message || '生成剧情失败');
    } finally {
      setGenerating(false);
    }
  }, [defaultModelId, worldId, playerName, realm]);

  /** 推进剧情：玩家选择某选项 */
  const handleAdvance = useCallback(
    async (choiceId: string) => {
      if (!story || !defaultModelId || advancing) return;
      setAdvancing(true);
      setError(null);
      try {
        const res = await game.storyAdvance({
          story_id: story.id,
          choice_id: choiceId,
          model_id: defaultModelId,
        });
        if (res.code !== 0) throw new Error(res.message);
        if (res.data) {
          setStory(res.data);
          // 刷新历史列表（updated_at 变化）
          const listRes = await game.storyList(20);
          if (listRes?.data) setHistory(listRes.data);
        }
      } catch (e: any) {
        setError(e?.message || '推进剧情失败');
      } finally {
        setAdvancing(false);
      }
    },
    [story, defaultModelId, advancing]
  );

  /** 从历史列表恢复某条剧情 */
  const handleResume = useCallback(async (storyId: string) => {
    setLoading(true);
    setError(null);
    try {
      const res = await game.storyGet(storyId);
      if (res.code !== 0) throw new Error(res.message);
      if (res.data) {
        setStory(res.data);
        setView('main');
      }
    } catch (e: any) {
      setError(e?.message || '恢复剧情失败');
    } finally {
      setLoading(false);
    }
  }, []);

  /** 渲染单个节点卡片 */
  const renderNode = (node: Story['nodes'][number], index: number, isCurrent: boolean) => {
    const isEnding = node.is_ending;
    const cardClass = [
      styles.nodeCard,
      isCurrent ? styles.current : '',
      isEnding ? styles.ending : '',
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <div key={`${node.id}-${index}`} className={cardClass}>
        <div className={styles.nodeHeader}>
          <span className={styles.nodeIndex}>#{index + 1}</span>
          <span className={styles.nodeTitle}>{node.title}</span>
          {isEnding && <span className={styles.badgeEnding}>结局</span>}
        </div>
        {node.description && (
          <div className={styles.nodeDescription}>{node.description}</div>
        )}
        <div className={styles.nodeNarration}>{node.narration}</div>

        {node.choices.length > 0 && (
          <div className={styles.choices}>
            {node.choices.map(choice => {
              // 已经过的节点（非当前节点）：标记玩家当时的选择
              // 当前节点：可点击推进
              const isChosen =
                !isCurrent &&
                story?.nodes[index + 1] !== undefined;
              return (
                <button
                  key={choice.id}
                  className={`${styles.choiceBtn} ${isChosen ? styles.chosen : ''}`}
                  onClick={() => isCurrent && !isEnding && handleAdvance(choice.id)}
                  disabled={!isCurrent || isEnding || advancing}
                >
                  <span className={styles.choiceText}>
                    <span>▸ {choice.text}</span>
                    {isChosen && <span className={styles.choiceMark}>✓ 已选</span>}
                  </span>
                  {choice.hint && <span className={styles.choiceHint}>{choice.hint}</span>}
                </button>
              );
            })}
          </div>
        )}

        {isEnding && (
          <div className={styles.nodeNarration} style={{ opacity: 0.7, fontStyle: 'italic' }}>
            —— 剧情完结 ——
          </div>
        )}
      </div>
    );
  };

  // ===== 渲染分支 =====

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={e => e.stopPropagation()}>
        {/* Header */}
        <div className={styles.header}>
          <div className={styles.titleArea}>
            <div className={styles.title}>
              <span className={styles.titleEmoji}>📖</span>
              <span>动态剧情</span>
            </div>
            <div className={styles.subtitle}>
              {story ? `主题：${story.theme}` : '玩家行为影响剧情走向'}
            </div>
          </div>
          <div className={styles.headerActions}>
            <button
              className={styles.actionBtn}
              onClick={() => setView(view === 'main' ? 'history' : 'main')}
              disabled={history.length === 0}
              title="切换历史/当前"
            >
              {view === 'main' ? `📜 历史(${history.length})` : '◀ 返回当前'}
            </button>
            <button
              className={styles.actionBtn}
              onClick={handleGenerate}
              disabled={generating || !defaultModelId}
              title="开启新剧情"
            >
              {generating ? '生成中…' : '✨ 新剧情'}
            </button>
            <button className={styles.closeBtn} onClick={onClose}>✕</button>
          </div>
        </div>

        {/* 状态条（主视图 + 有剧情时） */}
        {view === 'main' && story && (
          <div className={styles.statusBar}>
            <span>节点 {story.nodes.length}</span>
            <span className={story.used_ai ? styles.badgeAi : styles.badgeTemplate}>
              {story.used_ai ? 'AI 生成' : '降级模板'}
            </span>
            {story.nodes[story.nodes.length - 1]?.is_ending && (
              <span className={styles.badgeEnding}>已完结</span>
            )}
            {advancing && <span className={styles.dots}>推进中···</span>}
          </div>
        )}

        {/* 错误条 */}
        {error && <div className={styles.error}>{error}</div>}

        {/* 主视图：剧情时间线 */}
        {view === 'main' && (
          <>
            {loading && (
              <div className={styles.loading}>
                <span className={styles.dots}>···</span> 加载剧情中
              </div>
            )}
            {!loading && !story && (
              <div className={styles.empty}>
                <div className={styles.emptyEmoji}>📖</div>
                <div>
                  尚无剧情记录
                  <br />
                  点击下方按钮开启你的修仙历程
                </div>
                <button
                  className={styles.startBtn}
                  onClick={handleGenerate}
                  disabled={generating || !defaultModelId}
                >
                  {generating ? '生成中…' : '✨ 开启新剧情'}
                </button>
              </div>
            )}
            {!loading && story && (
              <div className={styles.timeline} ref={timelineRef}>
                {story.nodes.map((node, i) =>
                  renderNode(node, i, node.id === story.current_node_id)
                )}
                {advancing && (
                  <div className={styles.loading}>
                    <span className={styles.dots}>···</span> AI 正在演绎下一段…
                  </div>
                )}
              </div>
            )}
          </>
        )}

        {/* 历史视图 */}
        {view === 'history' && (
          <div className={styles.historyList}>
            {history.length === 0 && (
              <div className={styles.empty}>
                <div className={styles.emptyEmoji}>📜</div>
                <div>暂无历史剧情</div>
              </div>
            )}
            {history.map(item => (
              <div
                key={item.id}
                className={styles.historyItem}
                onClick={() => handleResume(item.id)}
              >
                <div className={styles.historyRow}>
                  <span className={styles.historyTheme}>{item.theme}</span>
                  <span className={item.is_finished ? styles.badgeEnding : styles.badgeAi}>
                    {item.is_finished ? '已完结' : `第${item.node_count}节`}
                  </span>
                </div>
                <div className={styles.historyMeta}>
                  <span>{item.used_ai ? 'AI' : '模板'}</span>
                  <span>节点 {item.node_count}</span>
                  <span className={styles.historyTime}>
                    {new Date(item.updated_at).toLocaleString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
