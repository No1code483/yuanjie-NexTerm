/**
 * D4.4 自然语言交互浮窗
 *
 * 玩家输入自然语言命令（如「建造图书馆」），调用云端 API 解析为结构化动作，
 * 展示解析结果（动作 / 目标 / 置信度 / 是否走 AI）。
 *
 * 数据流：
 *   - 输入命令 → game.nlParse()（后端 game_nl_parse，云端 API + 规则降级）
 *   - 展示 ParsedCommand：action / target / explanation / confidence / used_ai
 *
 * 边界：仅做 NLU 解析展示；具体执行（建造/突破）走既有 game 命令与各自 UI，
 * 此组件不重复实现建造/突破逻辑（非侵入式）。
 */
import { useCallback, useState } from 'react';
import { game, ai } from '@/lib/ipc';
import type { ParsedCommand } from '@/types/game';

interface NaturalLanguageInputProps {
  worldId: string;
  playerName: string;
  realm: string;
  onClose: () => void;
}

const QUICK_EXAMPLES = [
  '建造图书馆',
  '我要突破',
  '升级藏书阁',
  '和青鸾老人聊聊',
  '看看我的建筑',
];

const ACTION_LABEL: Record<string, string> = {
  build: '建造',
  upgrade: '升级',
  remove: '拆除',
  breakthrough: '突破考验',
  query_status: '查询状态',
  chat_npc: 'NPC 对话',
  unknown: '未识别',
};

const SEVERITY_COLOR: Record<string, string> = {
  build: '#00f0ff',
  upgrade: '#b026ff',
  remove: '#ff6b6b',
  breakthrough: '#ffd93d',
  query_status: '#7dd3fc',
  chat_npc: '#86efac',
  unknown: '#888',
};

export default function NaturalLanguageInput({
  worldId,
  playerName,
  realm,
  onClose,
}: NaturalLanguageInputProps) {
  const [input, setInput] = useState('');
  const [parsing, setParsing] = useState(false);
  const [result, setResult] = useState<ParsedCommand | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [defaultModelId, setDefaultModelId] = useState<number | undefined>(undefined);

  /** 解析玩家命令 */
  const handleParse = useCallback(async () => {
    const message = input.trim();
    if (!message) {
      setError('请输入指令');
      return;
    }
    setParsing(true);
    setError(null);
    try {
      // 懒加载默认模型（首次解析时取第一个可用模型）
      let modelId = defaultModelId;
      if (!modelId) {
        const modelsRes = await ai.getModels();
        if (!modelsRes?.data?.length) {
          setError('未找到可用 AI 模型，请先在 AI 设置中配置模型');
          setParsing(false);
          return;
        }
        modelId = modelsRes.data[0].id;
        setDefaultModelId(modelId);
      }

      const res = await game.nlParse({
        world_id: worldId,
        player_name: playerName,
        realm,
        message,
        model_id: modelId!,
      });
      if (res.code !== 0) throw new Error(res.message);
      setResult(res.data ?? null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setParsing(false);
    }
  }, [input, defaultModelId, worldId, playerName, realm]);

  const handleExample = (text: string) => {
    setInput(text);
    setResult(null);
    setError(null);
  };

  const action = result?.action ?? 'unknown';
  const color = SEVERITY_COLOR[action] ?? '#888';

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
          width: 560,
          maxWidth: '92vw',
          maxHeight: '80vh',
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
            <div style={{ fontSize: 16, fontWeight: 600, color: '#fff' }}>💬 自然语言交互</div>
            <div style={{ fontSize: 11, color: '#888', marginTop: 2 }}>
              {playerName} · {realm}
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
          {/* 输入框 */}
          <div style={{ display: 'flex', gap: 8 }}>
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !parsing) void handleParse();
              }}
              placeholder='试试输入「建造图书馆」「我要突破」'
              style={{
                flex: 1,
                background: 'rgba(255,255,255,0.04)',
                border: '1px solid rgba(0,240,255,0.2)',
                borderRadius: 8,
                padding: '10px 12px',
                color: '#fff',
                fontSize: 14,
                outline: 'none',
              }}
            />
            <button
              onClick={handleParse}
              disabled={parsing}
              style={{
                background: 'linear-gradient(135deg, rgba(0,240,255,0.3), rgba(176,38,255,0.2))',
                border: '1px solid rgba(0,240,255,0.3)',
                color: '#fff',
                borderRadius: 8,
                padding: '0 16px',
                cursor: parsing ? 'not-allowed' : 'pointer',
                opacity: parsing ? 0.5 : 1,
                fontSize: 14,
              }}
            >
              {parsing ? '解析中…' : '解析'}
            </button>
          </div>

          {/* 快捷示例 */}
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6, marginTop: 10 }}>
            {QUICK_EXAMPLES.map((ex) => (
              <button
                key={ex}
                onClick={() => handleExample(ex)}
                style={{
                  background: 'rgba(255,255,255,0.04)',
                  border: '1px solid rgba(255,255,255,0.08)',
                  color: '#aaa',
                  borderRadius: 12,
                  padding: '3px 10px',
                  fontSize: 12,
                  cursor: 'pointer',
                }}
              >
                {ex}
              </button>
            ))}
          </div>

          {error && (
            <div
              style={{
                marginTop: 12,
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

          {/* 解析结果 */}
          {result && (
            <div
              style={{
                marginTop: 14,
                padding: 14,
                background: 'rgba(255,255,255,0.03)',
                border: '1px solid rgba(255,255,255,0.08)',
                borderRadius: 10,
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                <span
                  style={{
                    color: '#fff',
                    fontWeight: 600,
                    fontSize: 13,
                    padding: '2px 10px',
                    borderRadius: 10,
                    background: `${color}22`,
                    border: `1px solid ${color}55`,
                  }}
                >
                  {ACTION_LABEL[action] ?? action}
                </span>
                {result.target && (
                  <span style={{ color: '#ccc', fontSize: 13 }}>目标：{result.target}</span>
                )}
                {result.used_ai ? (
                  <span
                    style={{
                      marginLeft: 'auto',
                      fontSize: 10,
                      color: '#00f0ff',
                      border: '1px solid rgba(0,240,255,0.3)',
                      borderRadius: 8,
                      padding: '1px 7px',
                    }}
                  >
                    ☁ 云端 AI
                  </span>
                ) : (
                  <span
                    style={{
                      marginLeft: 'auto',
                      fontSize: 10,
                      color: '#888',
                      border: '1px solid rgba(255,255,255,0.15)',
                      borderRadius: 8,
                      padding: '1px 7px',
                    }}
                  >
                    规则解析
                  </span>
                )}
              </div>
              <div style={{ color: '#ddd', fontSize: 13, lineHeight: 1.6 }}>
                {result.explanation}
              </div>
              <div style={{ marginTop: 10, color: '#888', fontSize: 11 }}>
                置信度 {(result.confidence * 100).toFixed(0)}% · 可在对应入口（建造 / 突破 / NPC）执行该动作
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
