// knowledge.search L2 功能域：基础搜索 + 语义搜索。
// 局部 state + handlers + JSX 物理落在本目录；核心共享数据经 KnowledgeCore 注入。
import { t } from "i18next";
import { useEffect, useRef, useState } from 'react';
import { kb } from '../../ipc';
import styles from '../../Knowledge.module.css';
import type { KbEntry, KnowledgeCore } from '../../knowledge/types';

export function useSearch(
  core: KnowledgeCore,
  handleOpenEntry: (entry: KbEntry) => void,
  setImporting: (value: boolean) => void
) {
  const [searchQuery, setSearchQuery] = useState('');
  const [isSearching, setIsSearching] = useState(false);
  const [searchResults, setSearchResults] = useState<KbEntry[]>([]);
  const [semanticSearch, setSemanticSearch] = useState(false);
  const [semanticSearchResults, setSemanticSearchResults] = useState<Array<{
    entry_id: number;
    title: string;
    content_preview: string;
    score: number;
    category_name: string;
    entry_type: string;
  }>>([]);
  const searchRef = useRef<HTMLInputElement>(null);

  const handleSearch = async () => {
    if (!searchQuery.trim()) return;
    setIsSearching(true);
    setImporting(true);
    try {
      if (semanticSearch) {
        const res = await kb.kbSemanticSearch({
          request: { query: searchQuery.trim(), limit: 50 }
        });
        if (res.code === 0 && res.data) {
          setSemanticSearchResults(res.data);
          setSearchResults([]);
        } else {
          setSemanticSearchResults([]);
        }
      } else {
        const res = await kb.searchKbEntries({ query: searchQuery.trim() });
        if (res.code === 0 && res.data) {
          setSearchResults(res.data);
          setSemanticSearchResults([]);
        } else setSearchResults([]);
      }
    } catch {
      core.showStatus('error', t("Knowledge.k58"));
    } finally {
      setImporting(false);
    }
  };

  const handleClearSearch = () => {
    setSearchQuery('');
    setIsSearching(false);
    setSearchResults([]);
    setSemanticSearchResults([]);
  };

  /** 供 browse handleSelect 等场景重置搜索态 */
  const resetSearch = () => {
    setIsSearching(false);
    setSearchQuery('');
  };

  // Ctrl+K 聚焦搜索框
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
        e.preventDefault();
        searchRef.current?.focus();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  // 300ms 防抖搜索
  useEffect(() => {
    if (!searchQuery.trim()) {
      if (isSearching) {
        setIsSearching(false);
        setSearchResults([]);
      }
      return;
    }
    const timer = setTimeout(() => {
      handleSearch();
    }, 300);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchQuery]);

  const typeFilterOptions = (
    <>
      <option value="all">{t("Knowledge.k192")}</option>
      <option value="text">{t("Knowledge.k193")}</option>
      <option value="file">{t("Knowledge.k194")}</option>
      <option value="document">{t("Knowledge.k195")}</option>
      <option value="image">{t("Knowledge.k196")}</option>
      <option value="video">{t("Knowledge.k197")}</option>
      <option value="audio">{t("Knowledge.k198")}</option>
      <option value="link">{t("Knowledge.k199")}</option>
    </>
  );

  /** 主内容区工具栏：左侧搜索控件 */
  const renderToolbarLeft = () => {
    if (isSearching) {
      return <>
        <button onClick={handleClearSearch} className={styles.btnWarn}>{t("Knowledge.k200")}</button>
        <select value={core.typeFilter} onChange={e => core.setTypeFilter(e.target.value)} className={styles.typeFilterSelect}>
          {typeFilterOptions}
        </select>
      </>;
    }
    return <>
      <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)} ref={searchRef} placeholder={t("Knowledge.k189")} className={styles.searchInput} />
      <button onClick={handleSearch} className={styles.btnSm}>{t("common.search")}</button>
      <label className={styles.semanticToggle} title={t("Knowledge.k190")}>
        <span className={styles.semanticToggleLabel} style={{ color: semanticSearch ? '#0F0' : '#555' }}>{t("Knowledge.k191")}</span>
        <div className={`${styles.semanticSwitch} ${semanticSearch ? styles.semanticSwitchOn : ''}`} onClick={() => setSemanticSearch(!semanticSearch)}>
          <div className={styles.semanticSwitchKnob} />
        </div>
      </label>
      <select value={core.typeFilter} onChange={e => core.setTypeFilter(e.target.value)} className={styles.typeFilterSelect}>
        {typeFilterOptions}
      </select>
    </>;
  };

  /** 主内容区工具栏：右侧命中数 */
  const renderToolbarCount = () => {
    if (!isSearching) return null;
    return (
      <span style={{ color: '#FFD700', fontSize: 12 }}>
        「{searchQuery}」({semanticSearch ? semanticSearchResults.length : searchResults.filter(e => core.typeFilter === 'all' || e.entry_type === core.typeFilter).length})
      </span>
    );
  };

  /** 搜索结果列表 */
  const renderResults = () => {
    if (semanticSearch) {
      if (semanticSearchResults.length === 0) return <div className={styles.emptyTip}>{t("Knowledge.k222")}</div>;
      return <div className={styles.resultList}>
        {semanticSearchResults.map(r => <div key={r.entry_id} className={styles.resultCard} onClick={() => {
          const entry = core.allEntries.find(e => e.id === r.entry_id);
          if (entry) handleOpenEntry(entry);
        }}>
            <span>{core.getTypeIcon(r.entry_type)}</span>
            <span className={styles.resultName}>{r.title}</span>
            <span className={styles.semanticScore}>
              {r.score >= 0.7 ? '🟢' : r.score >= 0.4 ? '🟡' : '🔴'} {(r.score * 100).toFixed(0)}%
            </span>
            <span className={styles.resultType}>{r.category_name}</span>
            <span className={styles.resultPath} style={{ fontSize: 10, color: '#666' }}>{r.content_preview.slice(0, 80)}</span>
          </div>)}
      </div>;
    }
    const filtered = searchResults.filter(e => core.typeFilter === 'all' || e.entry_type === core.typeFilter);
    if (filtered.length === 0) return <div className={styles.emptyTip}>{t("Knowledge.k222")}</div>;
    return <div className={styles.resultList}>
      {filtered.map(entry => <div key={entry.id} className={`${styles.resultCard}${core.missingFiles.has(entry.id) ? ` ${styles.entryMissing}` : ''}`} onClick={() => handleOpenEntry(entry)}>
          <span>{core.getTypeIcon(entry.entry_type)}</span>
          <span className={styles.resultName}>{entry.name}</span>
          <span className={styles.resultPath}>{entry.path_url}</span>
          {entry.source_path && <span className={styles.resultPath} style={{ color: '#FFC107', fontSize: 10 }}>📌 {entry.source_path}</span>}
          <span className={styles.resultType}>{core.getTypeLabel(entry.entry_type)}</span>
          <span className={styles.resultTime}>{core.formatTime(entry.updated_at)}</span>
        </div>)}
    </div>;
  };

  return {
    isSearching,
    renderToolbarLeft,
    renderToolbarCount,
    renderResults,
    resetSearch,
    focusSearch: () => searchRef.current?.focus()
  };
}
