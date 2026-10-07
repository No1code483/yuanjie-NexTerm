// knowledge.ai L2 功能域：AI 辅助（分类 / 摘要 / 标签建议）。
// 局部 state + handlers + JSX 物理落在本目录；核心共享数据经 KnowledgeCore 注入。
import { t } from "i18next";
import { useState } from 'react';
import { intelligence } from '@/lib/ipc';
import { useIntelligence } from '@/hooks/useIntelligence';
import { useFloatingOrbStore } from '@/kernel/state/floatingOrbStore';
import styles from '../../Knowledge.module.css';
import type { KbEntry, KnowledgeCore } from '../../knowledge/types';

export function useAi(core: KnowledgeCore) {
  const { aiOn, featureOn, llmConfigured, llmConfig } = useIntelligence();
  const [aiClassifyResults, setAiClassifyResults] = useState<Array<{
    category_name: string;
    confidence: number;
    reason: string;
    description: string;
  }>>([]);
  const [aiClassifyLoading, setAiClassifyLoading] = useState(false);
  const [aiSummaryLoading, setAiSummaryLoading] = useState(false);
  const [aiTagsLoading, setAiTagsLoading] = useState(false);
  const [aiTags, setAiTags] = useState<string[]>([]);

  const handleAiClassify = async (fileNames: string[]) => {
    setAiClassifyLoading(true);
    setAiClassifyResults([]);
    try {
      const names = fileNames.join(' ');
      const existingCats = core.categories.map(c => c.name);
      const res = await intelligence.kbClassify(names.slice(0, 200), '', existingCats);
      if (res?.data) {
        setAiClassifyResults(res.data.slice(0, 5));
      } else {
        core.showStatus('error', res?.message || t("Knowledge.k132"));
      }
    } catch (e: any) {
      core.showStatus('error', t("Knowledge.k133", { e: e }));
    }
    setAiClassifyLoading(false);
  };

  // Task 5.5: AI 智能分类推荐（右键菜单）
  const handleAiClassifyEntry = async (entryId: number, entryName: string) => {
    setAiClassifyLoading(true);
    try {
      const entry = core.allEntries.find(e => e.id === entryId);
      const content = entry?.content || entryName;
      const res = await intelligence.classifyKbEntryById(entryId, content);
      if (res?.data) {
        const { addOrb } = useFloatingOrbStore.getState();
        addOrb({
          type: 'summary',
          title: t("Knowledge.k134", { entryName: entryName }),
          content: t("Knowledge.k135", {
            category_name: res.data.category_name,
            arg0: (res.data.confidence * 100).toFixed(0),
            reason: res.data.reason
          }),
          source: 'knowledge',
          sourceId: entryId,
          icon: '🧠',
          color: '#00FF00'
        });
        core.showStatus('success', t("Knowledge.k136", {
          category_name: res.data.category_name,
          arg0: (res.data.confidence * 100).toFixed(0)
        }));
      } else {
        core.showStatus('error', res?.message || t("Knowledge.k137"));
      }
    } catch (e: any) {
      core.showStatus('error', t("Knowledge.k138", { e: e }));
    }
    setAiClassifyLoading(false);
  };

  const handleAiSummarizeEntry = async (entry: KbEntry) => {
    if (!entry) return;
    if (!llmConfigured) {
      core.showStatus('error', t("Knowledge.k139"));
      return;
    }
    setAiSummaryLoading(true);
    try {
      const res = await intelligence.kbSummarize(entry.name, entry.content || '', {
        provider: llmConfig.provider,
        endpoint: llmConfig.endpoint,
        api_key: llmConfig.apiKey || undefined,
        model: llmConfig.model || undefined
      }, String(entry.id));
      if (res?.data) {
        // dispatch 到全局悬浮球
        const { addOrb } = useFloatingOrbStore.getState();
        addOrb({
          type: 'summary',
          title: t("Knowledge.k140", { name: entry.name }),
          content: res.data.summary,
          keyPoints: res.data.key_points,
          source: 'knowledge',
          sourceId: entry.id,
          icon: '🧠',
          color: '#00F0FF'
        });
        core.showStatus('success', t("Knowledge.k141", {
          original_length: res.data.original_length,
          summary_length: res.data.summary_length
        }));
      } else {
        core.showStatus('error', res?.message || t("Knowledge.k142"));
      }
    } catch (e: any) {
      core.showStatus('error', t("Knowledge.k143", { e: e }));
    }
    setAiSummaryLoading(false);
  };

  const handleAiGenerateTags = async (entry: KbEntry) => {
    if (!entry) return;
    if (!llmConfigured) {
      core.showStatus('error', t("Knowledge.k139"));
      return;
    }
    setAiTagsLoading(true);
    setAiTags([]);
    try {
      const existing = (core.entryTags.get(entry.id) || []).map(tg => tg.name);
      const res = await intelligence.kbTags(entry.content || '', existing, {
        provider: llmConfig.provider,
        endpoint: llmConfig.endpoint,
        api_key: llmConfig.apiKey || undefined,
        model: llmConfig.model || undefined
      });
      if (res?.data) {
        setAiTags(res.data.suggested_tags);
        core.showStatus('success', t("Knowledge.k144", { length: res.data.suggested_tags.length }));
      } else {
        core.showStatus('error', res?.message || t("Knowledge.k145"));
      }
    } catch (e: any) {
      core.showStatus('error', t("Knowledge.k146", { e: e }));
    }
    setAiTagsLoading(false);
  };

  /** AI 智能体工具栏（条目预览区顶部） */
  const renderAiBar = (entry: KbEntry | null) => {
    if (!entry || !aiOn) return null;
    return (
      <div className={styles.aiAgentBar}>
        <button className={styles.aiAgentBtn} onClick={() => handleAiSummarizeEntry(entry)} disabled={aiSummaryLoading || !llmConfigured} title={llmConfigured ? t("Knowledge.k214") : t("Knowledge.k215")}>
          {aiSummaryLoading ? '⏳' : '🧠'} {t("components.intelligence.SuggestionsPanel.k5")}
        </button>
        <button className={styles.aiAgentBtn} onClick={() => handleAiGenerateTags(entry)} disabled={aiTagsLoading || !llmConfigured} title={llmConfigured ? t("Knowledge.k216") : t("Knowledge.k215")}>
          {aiTagsLoading ? '⏳' : '🏷️'} {t("Knowledge.k217")}
        </button>
      </div>
    );
  };

  /** AI 摘要加载占位 */
  const renderSummaryLoading = () => {
    if (!aiSummaryLoading) return null;
    return (
      <div className={styles.aiAgentResult} style={{ opacity: 0.6, pointerEvents: 'none' }}>
        <div className={styles.aiAgentResultHeader}>
          <span>{t("Knowledge.k218")}</span>
        </div>
        <p className={styles.aiAgentSummary} style={{ color: '#6B7280' }}>{t("Knowledge.k219")}</p>
      </div>
    );
  };

  /** AI 标签建议面板 */
  const renderTagsPanel = () => {
    if (aiTags.length === 0) return null;
    return (
      <div className={styles.aiAgentResult}>
        <div className={styles.aiAgentResultHeader}>
          <span>{t("Knowledge.k220")}</span>
          <button onClick={() => setAiTags([])} style={{ background: 'none', border: 'none', color: '#FF5050', cursor: 'pointer' }}>✕</button>
        </div>
        <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {aiTags.map((tagName, i) => <span key={i} className={styles.aiTagBadge}>{tagName}</span>)}
        </div>
      </div>
    );
  };

  /** 目录扫描 Modal 内的 AI 分类工具条 */
  const renderAiClassifyToolbar = (fileNames: string[]) => (
    <div className={styles.aiClassifyToolbar}>
      <button className={styles.btnSm} onClick={() => handleAiClassify(fileNames)} disabled={aiClassifyLoading || fileNames.length === 0}>
        {aiClassifyLoading ? t("home.TimerPanel.k4") : t("Knowledge.k285")}
      </button>
      {aiClassifyResults.length > 0 && <span className={styles.aiClassifyHint}>
          {t("Knowledge.k286")}
          {aiClassifyResults.map((r, i) => <span key={i} className={styles.aiClassifyTag}>
              {r.category_name} ({Math.round(r.confidence)}%)
            </span>)}
        </span>}
    </div>
  );

  /** 侧边栏 AI 状态徽标 */
  const renderSidebarBadge = () => {
    if (aiOn && featureOn('auto_classify')) return null;
    if (aiOn && !llmConfigured) {
      return <span className={styles.aiPulsingBadge} style={{ opacity: 0.5, background: '#1A1A1F' }}>{t("Knowledge.k158")}</span>;
    }
    return null;
  };

  /** 右键菜单「AI 分类」入口（仅启用时提供） */
  const aiClassifyMenuItem = aiOn && featureOn('auto_classify') ? handleAiClassifyEntry : undefined;

  return {
    renderAiBar,
    renderSummaryLoading,
    renderTagsPanel,
    renderAiClassifyToolbar,
    renderSidebarBadge,
    aiClassifyMenuItem,
    handleAiClassifyEntry
  };
}
