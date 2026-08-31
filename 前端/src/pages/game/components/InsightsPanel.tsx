/**
 * D4.6 数据分析 AI 洞察浮窗
 *
 * 聚合玩家行为 → 调用云端 API 生成个性化洞察（学习风格 / 薄弱领域 / 突破策略 / 节奏建议）。
 * 同时展示底层智能监测钩子状态（非侵入式、可关闭）。
 *
 * 数据流：
 *   - 打开 → game.intelligenceStatus() 取钩子状态 + game.analyzeBehavior() 取洞察
 *   - AI 失败时后端降级到规则分析，used_ai=false
 *
 * 边界：纯只读分析；底层智能关闭时，监测钩子状态显示"已关闭"，但洞察分析（云端 API）仍可用。
 */
import { useCallback, useEffect, useState } from 'react';
import { game, ai } from '@/lib/ipc';
import type { BehaviorAnalysis, GameInsight, IntelligenceHookStatus } from '@/types/game';

interface InsightsPanelProps {
  worldId: string;
  playerName: string;
  realm: string;
  onClose: () => void;
}

const INSIGHT_ICON: Record<string, string> = {
  learning_style: '🧭',
  weak_area: '⚠️',
  breakthrough_strategy: '⚔️',
  pace_advice: '⏱️',
  overall: '📊',
};

const SEVERITY_COLOR: Record<string, string> = {
  info: '#7dd3fc',
  warning: '#ffd93d',
  critical: '#ff6b6b',
};

export default function InsightsPanel({ worldId, playerName, realm, onClose }: InsightsPanelProps) {
  const [analysis, setAnalysis] = useState<BehaviorAnalysis | null>(null);
  const [hookStatus, setHookStatus] = useState<IntelligenceHookStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadAll = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // 并行：钩子状态 + 默认模型
      const [statusRes, modelsRes] = await Promise.all([game.intelligenceStatus(), ai.getModels()]);
      if (statusRes?.data) setHookStatus(statusRes.data);

      const modelId = modelsRes?.data?.[0]?.id;
      if (!modelId) {
        setError('未找到可用 AI 模型，请先在 AI 设置中配置模型');
        setLoading(false);
        return;
      }

      const res = await game.analyzeBehavior({
        world_id: worldId,
        player_name: playerName,
        realm,
        model_id: modelId,
      });
      if (res.code !== 0) throw new Error(res.message);
      setAnalysis(res.data ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [worldId, playerName, realm]);

  useEffect(() => {
    void loadAll();
  }, [loadAll]);

  const snap = analysis?.snapshot;

  return (
    <div
      onClick={onClose}
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        background: 'rgba(0,0,0,0.6)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
        backdropFilter: 'blur(4px)',
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 600,
          maxWidth: '92vw',
          maxHeight: '82vh',
          background: 'var(--nt-bg-secondary, #1a1a1a)',
          border: '1px solid var(--nt-border, rgba(0,240,255,0.2))',
          borderRadius: 12,
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          boxShadow: '0 8px 32px rgba(0,0,0,0.4)',
        }}
      >
        {/* Header */}
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            padding: '16px',
            borderBottom: '1px solid rgba(255,255,255,0.05)',
            background: 'linear-gradient(135deg, rgba(0,240,255,0.04), rgba(176,38,255,0.03))',
          }}
        >
          <div>
            <div style={{ fontSize: 16, fontWeight: 600, color: '#fff' }}>📊 AI 行为洞察</div>
            <div style={{ fontSize: 11, color: '#888', marginTop: 2 }}>
              {playerName} · {realm}
              {analysis && (
                <span style={{ marginLeft: 8, color: analysis.used_ai ? '#00f0ff' : '#888' }}>
                  {analysis.used_ai ? '☁ 云端 AI' : '规则分析'}
                </span>
              )}
            </div>
          </div>
          <button
            onClick={onClose}
            style={{
              background: 'transparent',
              border: '1px solid rgba(255,255,255,0.1)',
              color: '#aaa',
              borderRadius: 6,
              padding: '2px 10px',
              cursor: 'pointer',
            }}
          >
            ✕
          </button>
        </div>

        {/* Body */}
        <div style={{ padding: 16, overflowY: 'auto', flex: 1 }}>
          {loading && <div style={{ color: '#888', fontSize: 13, textAlign: 'center', padding: 24 }}>分析中…</div>}

          {error && (
            <div
              style={{
                padding: '8px 12px',
                background: 'rgba(255,107,107,0.08)',
                border: '1px solid rgba(255,107,107,0.3)',
                borderRadius: 8,
                color: '#ff8b8b',
                fontSize: 13,
              }}
            >
              {error}
            </div>
          )}

          {/* 行为快照摘要 */}
          {snap && (
            <div
              style={{
                display: 'grid',
                gridTemplateColumns: 'repeat(3, 1fr)',
                gap: 8,
                marginBottom: 14,
              }}
            >
              {[
                { label: '建造次数', value: snap.build_total },
                { label: '突破次数', value: snap.breakthrough_total },
                { label: '突破成功', value: snap.breakthrough_success },
                { label: 'NPC 对话', value: snap.npc_chat_count },
                { label: '能力评分', value: snap.skill_score.toFixed(1) },
                { label: '连胜/连败', value: snap.streak },
              ].map((m) => (
                <div
                  key={m.label}
                  style={{
                    background: 'rgba(255,255,255,0.03)',
                    border: '1px solid rgba(255,255,255,0.08)',
                    borderRadius: 8,
                    padding: '8px 10px',
                  }}
                >
                  <div style={{ fontSize: 11, color: '#888' }}>{m.label}</div>
                  <div style={{ fontSize: 16, fontWeight: 600, color: '#fff', marginTop: 2 }}>{m.value}</div>
                </div>
              ))}
            </div>
          )}

          {/* 洞察列表 */}
          {analysis && analysis.insights.length > 0 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
              {analysis.insights.map((ins: GameInsight, i: number) => {
                const color = SEVERITY_COLOR[ins.severity] ?? '#7dd3fc';
                return (
                  <div
                    key={i}
                    style={{
                      background: 'rgba(255,255,255,0.03)',
                      border: '1px solid rgba(255,255,255,0.08)',
                      borderLeft: `3px solid ${color}`,
                      borderRadius: 8,
                      padding: '10px 12px',
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
                      <span style={{ fontSize: 16 }}>{INSIGHT_ICON[ins.insight_type] ?? '📌'}</span>
                      <span style={{ color: '#fff', fontWeight: 600, fontSize: 13 }}>{ins.title}</span>
                    </div>
                    <div style={{ color: '#ccc', fontSize: 13, lineHeight: 1.6 }}>{ins.content}</div>
                  </div>
                );
              })}
            </div>
          )}

          {!loading && analysis && analysis.insights.length === 0 && (
            <div style={{ color: '#888', fontSize: 13, textAlign: 'center', padding: 24 }}>
              暂无洞察数据，多进行游戏行为后将获得分析。
            </div>
          )}

          {/* 底层智能监测状态（非侵入式、可关闭） */}
          <div
            style={{
              marginTop: 16,
              padding: '10px 12px',
              background: 'rgba(0,240,255,0.03)',
              border: '1px solid rgba(0,240,255,0.12)',
              borderRadius: 8,
              fontSize: 12,
              color: '#aaa',
            }}
          >
            <div style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 4 }}>
              <span style={{ color: hookStatus?.enabled ? '#00f0ff' : '#888' }}>
                {hookStatus?.enabled ? '●' : '○'}
              </span>
              <span style={{ color: '#ccc', fontWeight: 600 }}>底层智能监测</span>
              <span style={{ marginLeft: 'auto', color: '#888' }}>
                近 7 天事件 {hookStatus?.recent_event_count ?? 0} 条
              </span>
            </div>
            <div>
              {hookStatus?.enabled
                ? '已开启：游戏事件正非侵入式地上报供行为监测。关闭底层智能后，游戏核心功能仍正常。'
                : '已关闭：监测钩子 no-op；游戏核心功能（AI 洞察 / 剧情 / 自然语言）仍正常。'}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
