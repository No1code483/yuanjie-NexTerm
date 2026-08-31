import { t } from "i18next";
import { useState, useCallback, useMemo, useEffect } from 'react';
import { ipc } from '@/lib/ipc';
import styles from '../YuanCode.module.css';

/**
 * D1.8 Skill 市场 — 浏览 / 搜索 / 安装 / 卸载 预置技能
 *
 * 后端来源：src-tauri/src/services/skill_market_service.rs
 * IPC 命令：yuan_skill_market_list / _search / _install / _uninstall
 *
 * 设计原则（终端黑客风格）：
 *   - 复用 YuanCode.module.css 的 panelContainer / skillCard / cardGrid 等类
 *   - 通过 CSS 变量 --nt-primary / --nt-secondary / --nt-warning / --nt-accent 控制配色
 *   - 等宽字体 var(--nt-font-mono)
 */

// ===== 后端 SkillMarketEntry 镜像（与 skill_market_service.rs 字段保持一致） =====
interface SkillMarketEntry {
  name: string;
  description: string;
  short_description: string;
  category: string; // 编程/测试/安全/文档/性能/架构/调试/国际化
  author: string;
  version: string;
  source: string; // "builtin"
  trigger_patterns: string[];
  file_patterns: string[];
  priority: number;
  installed: boolean;
}

// 分类配色：与终端 5 色主题对齐
const CATEGORY_COLORS: Record<string, string> = {
  编程: 'var(--nt-primary)', // 青
  测试: 'var(--nt-secondary)', // 紫
  安全: 'var(--nt-accent)', // 粉红
  文档: 'var(--nt-warning)', // 金
  性能: '#00cc66', // 绿
  架构: 'var(--nt-secondary)',
  调试: 'var(--nt-accent)',
  国际化: 'var(--nt-primary)',
};

interface SkillMarketProps {
  /** 返回"我的技能"视图回调 */
  onBack: () => void;
  /** 安装/卸载后通知父组件刷新已加载列表 */
  onSkillListChanged?: () => void;
}

export default function SkillMarket({ onBack, onSkillListChanged }: SkillMarketProps) {
  const [entries, setEntries] = useState<SkillMarketEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [activeCategory, setActiveCategory] = useState<string>('all');
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState<string | null>(null); // 正在安装/卸载的技能名
  const [error, setError] = useState<string | null>(null);

  // 加载市场列表
  const loadMarket = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // ipc.invoke<T> 已返回 ApiResponse<T>，无需再嵌套
      const cmd = searchQuery.trim() ? 'yuan_skill_market_search' : 'yuan_skill_market_list';
      const args = searchQuery.trim() ? { query: searchQuery.trim() } : undefined;
      const res = await ipc.invoke<SkillMarketEntry[]>(cmd, args);
      if (res.code === 0 && res.data) {
        setEntries(res.data);
      } else {
        setError(res.message || 'load failed');
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [searchQuery]);

  // 初次加载
  useEffect(() => {
    loadMarket();
  }, [loadMarket]);

  // 分类列表（从市场数据动态聚合，保持顺序）
  const categories = useMemo(() => {
    const set = new Set<string>();
    for (const e of entries) set.add(e.category);
    return Array.from(set);
  }, [entries]);

  // 按分类过滤
  const filteredEntries = useMemo(() => {
    if (activeCategory === 'all') return entries;
    return entries.filter(e => e.category === activeCategory);
  }, [entries, activeCategory]);

  // 已安装计数
  const installedCount = useMemo(() => entries.filter(e => e.installed).length, [entries]);

  // 安装技能
  const handleInstall = useCallback(async (name: string) => {
    setInstalling(name);
    try {
      const res = await ipc.invoke<unknown>('yuan_skill_market_install', { name });
      if (res.code === 0) {
        // 本地立即标记为已安装，避免重新加载抖动
        setEntries(prev => prev.map(e => e.name === name ? { ...e, installed: true } : e));
        onSkillListChanged?.();
      } else {
        setError(res.message || `install ${name} failed`);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setInstalling(null);
    }
  }, [onSkillListChanged]);

  // 卸载技能
  const handleUninstall = useCallback(async (name: string) => {
    setInstalling(name);
    try {
      const res = await ipc.invoke<unknown>('yuan_skill_market_uninstall', { name });
      if (res.code === 0) {
        setEntries(prev => prev.map(e => e.name === name ? { ...e, installed: false } : e));
        onSkillListChanged?.();
      } else {
        setError(res.message || `uninstall ${name} failed`);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setInstalling(null);
    }
  }, [onSkillListChanged]);

  return (
    <div className={styles.panelContainer}>
      {/* ===== Header ===== */}
      <div className={styles.panelHeader}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <button
            className={styles.btnPrimary}
            onClick={onBack}
            style={{ padding: '4px 12px', fontSize: 11 }}
            title={t("yuan-code.SkillsPanel.k46")}
          >
            ← {t("yuan-code.SkillsPanel.k46")}
          </button>
          <span className={styles.panelTitle}>{t("yuan-code.SkillsPanel.k47")}</span>
        </div>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <span className={styles.panelBadge}>
            {installedCount}/{entries.length} {t("yuan-code.SkillsPanel.k49")}
          </span>
        </div>
      </div>

      {/* ===== Body ===== */}
      <div className={styles.panelBody}>
        {/* 搜索行 */}
        <div style={{ display: 'flex', gap: 10, marginBottom: 14 }}>
          <input
            className={styles.formInput}
            placeholder={t("yuan-code.SkillsPanel.k48")}
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            style={{ flex: 1 }}
          />
          <button
            className={styles.btnPrimary}
            onClick={loadMarket}
            disabled={loading}
            style={{ padding: '4px 14px', fontSize: 11 }}
          >
            {loading ? '...' : '⟳'}
          </button>
        </div>

        {/* 错误提示 */}
        {error && (
          <div style={{
            padding: '6px 10px',
            marginBottom: 12,
            background: 'rgba(255, 0, 110, 0.06)',
            border: '1px solid rgba(255, 0, 110, 0.25)',
            color: 'var(--nt-accent)',
            fontFamily: 'var(--nt-font-mono)',
            fontSize: 11,
            borderRadius: 2,
          }}>
            ⚠ {error}
          </div>
        )}

        {/* 分类标签 */}
        <div className={styles.skillScopeTabs}>
          <div
            className={`${styles.skillScopeTab} ${activeCategory === 'all' ? styles.skillScopeTabActive : ''}`}
            onClick={() => setActiveCategory('all')}
            style={activeCategory === 'all' ? { borderColor: 'var(--nt-primary)' } : undefined}
          >
            <span style={{
              fontWeight: 600,
              fontSize: 13,
              color: activeCategory === 'all' ? 'var(--nt-primary)' : 'var(--nt-text-secondary)',
              marginBottom: 4,
              display: 'block',
            }}>
              {t("yuan-code.SkillsPanel.k62")}
            </span>
            <span style={{ fontSize: 11, color: 'var(--nt-text-muted)' }}>
              {entries.length}{t("yuan-code.SkillsPanel.k54")}
            </span>
          </div>
          {categories.map(cat => {
            const count = entries.filter(e => e.category === cat).length;
            const color = CATEGORY_COLORS[cat] || 'var(--nt-primary)';
            const isActive = activeCategory === cat;
            return (
              <div
                key={cat}
                className={`${styles.skillScopeTab} ${isActive ? styles.skillScopeTabActive : ''}`}
                onClick={() => setActiveCategory(cat)}
                style={isActive ? { borderColor: color } : undefined}
              >
                <span style={{
                  fontWeight: 600,
                  fontSize: 13,
                  color: isActive ? color : 'var(--nt-text-secondary)',
                  marginBottom: 4,
                  display: 'block',
                }}>
                  {cat}
                </span>
                <span style={{ fontSize: 11, color: 'var(--nt-text-muted)' }}>
                  {count}{t("yuan-code.SkillsPanel.k54")}
                </span>
              </div>
            );
          })}
        </div>

        {/* 技能卡片网格 */}
        {filteredEntries.length === 0 ? (
          <div style={{
            textAlign: 'center',
            padding: '40px 0',
            color: 'var(--nt-text-muted)',
            fontFamily: 'var(--nt-font-mono)',
            fontSize: 13,
          }}>
            {loading ? '...' : t("yuan-code.SkillsPanel.k55")}
          </div>
        ) : (
          <div className={styles.cardGrid} style={{ marginTop: 20 }}>
            {filteredEntries.map(entry => {
              const color = CATEGORY_COLORS[entry.category] || 'var(--nt-primary)';
              const isInstalling = installing === entry.name;
              return (
                <div
                  key={entry.name}
                  className={styles.skillCard}
                  style={{
                    opacity: entry.installed ? 1 : 0.85,
                    borderColor: entry.installed ? `${color}55` : undefined,
                  }}
                >
                  {/* 卡片头：名称 + 安装状态 */}
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
                    <span className={styles.skillName} style={{ color }}>{entry.name}</span>
                    <span
                      className={`${styles.skillTag} ${entry.installed ? styles.skillTagExplicit : styles.skillTagImplicit}`}
                    >
                      {entry.installed ? t("yuan-code.SkillsPanel.k49") : t("yuan-code.SkillsPanel.k50")}
                    </span>
                  </div>

                  {/* 描述 */}
                  <p className={styles.skillDesc}>{entry.description}</p>

                  {/* 元信息：分类 / 版本 / 作者 / 优先级 */}
                  <div className={styles.skillMeta}>
                    <span className={styles.skillScope} style={{ color }}>{entry.category}</span>
                    <span style={{ color: 'var(--nt-text-muted)', fontSize: 10 }}>v{entry.version}</span>
                    <span style={{ color: 'var(--nt-text-muted)', fontSize: 10 }}>{entry.author}</span>
                    <span style={{ color: 'var(--nt-text-muted)', fontSize: 10 }}>
                      {t("yuan-code.SkillsPanel.k58")}: {entry.priority}
                    </span>
                  </div>

                  {/* 触发词 */}
                  {entry.trigger_patterns.length > 0 && (
                    <div style={{ marginTop: 8 }}>
                      <span style={{ fontFamily: 'var(--nt-font-mono)', fontSize: 10, color: 'var(--nt-text-muted)' }}>
                        {t("yuan-code.SkillsPanel.k56")}:
                      </span>
                      <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
                        {entry.trigger_patterns.map(p => (
                          <span
                            key={p}
                            style={{
                              fontFamily: 'var(--nt-font-mono)',
                              fontSize: 10,
                              color: 'var(--nt-secondary)',
                              padding: '1px 6px',
                              border: '1px solid rgba(176, 38, 255, 0.2)',
                              borderRadius: 2,
                            }}
                          >
                            {p}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* 文件模式 */}
                  {entry.file_patterns.length > 0 && (
                    <div style={{ marginTop: 6 }}>
                      <span style={{ fontFamily: 'var(--nt-font-mono)', fontSize: 10, color: 'var(--nt-text-muted)' }}>
                        {t("yuan-code.SkillsPanel.k57")}:
                      </span>
                      <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
                        {entry.file_patterns.map(p => (
                          <span
                            key={p}
                            style={{
                              fontFamily: 'var(--nt-font-mono)',
                              fontSize: 10,
                              color: 'var(--nt-primary)',
                              padding: '1px 6px',
                              border: '1px solid rgba(0, 240, 255, 0.2)',
                              borderRadius: 2,
                            }}
                          >
                            {p}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}

                  {/* 操作按钮 */}
                  <div style={{ marginTop: 10, display: 'flex', gap: 6 }}>
                    {entry.installed ? (
                      <button
                        className={styles.btnDanger}
                        onClick={() => handleUninstall(entry.name)}
                        disabled={isInstalling}
                        style={{ padding: '4px 12px', fontSize: 11, flex: 1 }}
                      >
                        {isInstalling ? t("yuan-code.SkillsPanel.k53") : `🗑 ${t("yuan-code.SkillsPanel.k52")}`}
                      </button>
                    ) : (
                      <button
                        className={styles.btnPrimary}
                        onClick={() => handleInstall(entry.name)}
                        disabled={isInstalling}
                        style={{ padding: '4px 12px', fontSize: 11, flex: 1 }}
                      >
                        {isInstalling ? t("yuan-code.SkillsPanel.k53") : `⬇ ${t("yuan-code.SkillsPanel.k51")}`}
                      </button>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
