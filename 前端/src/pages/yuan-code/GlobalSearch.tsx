import { t } from "i18next";
import { useState, useCallback, useRef } from 'react';
import { ipc } from '@/lib/ipc';
import styles from './GlobalSearch.module.css';
interface GlobalSearchProps {
  workspacePath: string;
  onFileOpen?: (filePath: string) => void;
  onNavigateTo?: (filePath: string, line: number, column: number) => void;
}
interface SearchMatch {
  file: string;
  line: number;
  column: number;
  content: string;
  matchStart: number;
  matchEnd: number;
}
interface FileResult {
  file: string;
  matches: SearchMatch[];
  matchCount: number;
}
export default function GlobalSearch({
  workspacePath,
  onFileOpen,
  onNavigateTo
}: GlobalSearchProps) {
  const [query, setQuery] = useState('');
  const [replaceText, setReplaceText] = useState('');
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [useRegex, setUseRegex] = useState(false);
  const [includePattern, setIncludePattern] = useState('');
  const [excludePattern, setExcludePattern] = useState('');
  const [isSearching, setIsSearching] = useState(false);
  const [results, setResults] = useState<FileResult[]>([]);
  const [collapsedFiles, setCollapsedFiles] = useState<Set<string>>(new Set());
  const [expandedAll, setExpandedAll] = useState(true);
  const [showReplace, setShowReplace] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const handleSearch = useCallback(async () => {
    if (!query.trim() || !workspacePath) return;
    setIsSearching(true);
    setResults([]);
    try {
      const res = await ipc.invoke<any>('yuan_search_files', {
        request: {
          workspace_path: workspacePath,
          query,
          case_sensitive: caseSensitive,
          whole_word: wholeWord,
          use_regex: useRegex,
          include_pattern: includePattern || undefined,
          exclude_pattern: excludePattern || undefined,
          max_results: 500
        }
      });
      if (res.code === 0 && res.data) {
        // Group results by file
        const fileMap = new Map<string, SearchMatch[]>();
        for (const match of res.data.matches || []) {
          if (!fileMap.has(match.file)) {
            fileMap.set(match.file, []);
          }
          fileMap.get(match.file)!.push(match);
        }
        const fileResults: FileResult[] = Array.from(fileMap.entries()).map(([file, matches]) => ({
          file,
          matches,
          matchCount: matches.length
        }));
        setResults(fileResults);
        setExpandedAll(true);
      }
    } catch (err) {
      console.error('Search failed:', err);
    } finally {
      setIsSearching(false);
    }
  }, [query, workspacePath, caseSensitive, wholeWord, useRegex, includePattern, excludePattern]);
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleSearch();
    }
  };
  const handleReplace = useCallback(async () => {
    if (!query.trim() || !replaceText || results.length === 0) return;
    try {
      const totalMatches = results.reduce((sum, r) => sum + r.matchCount, 0);
      const confirmed = window.confirm(t("yuan-code.GlobalSearch.k1", {
        length: results.length,
        totalMatches: totalMatches
      }));
      if (!confirmed) return;
      const res = await ipc.invoke<any>('yuan_replace_files', {
        request: {
          workspace_path: workspacePath,
          query,
          replacement: replaceText,
          case_sensitive: caseSensitive,
          whole_word: wholeWord,
          use_regex: useRegex,
          include_pattern: includePattern || undefined,
          exclude_pattern: excludePattern || undefined
        }
      });
      if (res.code === 0) {
        // Re-search to show updated results
        handleSearch();
      }
    } catch (err) {
      console.error('Replace failed:', err);
    }
  }, [query, replaceText, results, workspacePath, caseSensitive, wholeWord, useRegex, includePattern, excludePattern, handleSearch]);
  const toggleFile = (file: string) => {
    setCollapsedFiles(prev => {
      const next = new Set(prev);
      if (next.has(file)) {
        next.delete(file);
      } else {
        next.add(file);
      }
      return next;
    });
  };
  const toggleAll = () => {
    if (expandedAll) {
      const allFiles = new Set(results.map(r => r.file));
      setCollapsedFiles(allFiles);
      setExpandedAll(false);
    } else {
      setCollapsedFiles(new Set());
      setExpandedAll(true);
    }
  };
  const handleMatchClick = (file: string, match: SearchMatch) => {
    onNavigateTo?.(file, match.line, match.column);
  };
  const handleFileClick = (file: string) => {
    onFileOpen?.(file);
  };

  // Highlight matching text
  const highlightMatch = (content: string, start: number, end: number) => {
    const before = content.substring(0, start);
    const match = content.substring(start, end);
    const after = content.substring(end);
    return <>
        <span className={styles.matchContext}>{before}</span>
        <span className={styles.matchHighlight}>{match}</span>
        <span className={styles.matchContext}>{after}</span>
      </>;
  };
  const totalMatches = results.reduce((sum, r) => sum + r.matchCount, 0);
  return <div className={styles.globalSearch}>
      <div className={styles.searchHeader}>
        <div className={styles.searchRow}>
          <div className={styles.inputWrapper}>
            <span className={styles.inputIcon}>🔍</span>
            <input ref={inputRef} className={styles.searchInput} type="text" value={query} onChange={e => setQuery(e.target.value)} onKeyDown={handleKeyDown} placeholder={t("components.NexTermTerminal.k1")} autoFocus />
          </div>
          <div className={styles.searchActions}>
            <button className={styles.toggleReplaceBtn} onClick={() => setShowReplace(!showReplace)} title={showReplace ? t("yuan-code.GlobalSearch.k2") : t("yuan-code.GlobalSearch.k3")}>
              {showReplace ? '▼' : '▶'}
            </button>
            <button className={styles.searchBtn} onClick={handleSearch} disabled={isSearching}>
              {isSearching ? '...' : t("common.search")}
            </button>
          </div>
        </div>

        {showReplace && <div className={styles.replaceRow}>
            <div className={styles.inputWrapper}>
              <span className={styles.inputIcon}>↻</span>
              <input className={styles.replaceInput} type="text" value={replaceText} onChange={e => setReplaceText(e.target.value)} placeholder={t("yuan-code.GlobalSearch.k4")} />
            </div>
            <button className={styles.replaceAllBtn} onClick={handleReplace}>
              {t("yuan-code.FindReplace.k11")}
            </button>
          </div>}

        <div className={styles.searchOptions}>
          <label className={styles.option}>
            <input type="checkbox" checked={caseSensitive} onChange={e => setCaseSensitive(e.target.checked)} />
            <span>Aa</span>
          </label>
          <label className={styles.option}>
            <input type="checkbox" checked={wholeWord} onChange={e => setWholeWord(e.target.checked)} />
            <span>ab</span>
          </label>
          <label className={styles.option}>
            <input type="checkbox" checked={useRegex} onChange={e => setUseRegex(e.target.checked)} />
            <span>.*</span>
          </label>
          <div className={styles.optionInput}>
            <input className={styles.patternInput} type="text" value={includePattern} onChange={e => setIncludePattern(e.target.value)} placeholder={t("yuan-code.GlobalSearch.k5")} />
          </div>
          <div className={styles.optionInput}>
            <input className={styles.patternInput} type="text" value={excludePattern} onChange={e => setExcludePattern(e.target.value)} placeholder={t("yuan-code.GlobalSearch.k6")} />
          </div>
        </div>
      </div>

      <div className={styles.resultsHeader}>
        <span className={styles.resultsCount}>
          {isSearching ? t("ai.ConversationList.k4") : results.length > 0 ? t("yuan-code.GlobalSearch.k7", {
          length: results.length,
          totalMatches: totalMatches
        }) : query.trim() ? t("yuan-code.GlobalSearch.k8") : t("yuan-code.GlobalSearch.k9")}
        </span>
        {results.length > 0 && <button className={styles.toggleAllBtn} onClick={toggleAll}>
            {expandedAll ? t("yuan-code.GlobalSearch.k10") : t("yuan-code.GlobalSearch.k11")}
          </button>}
      </div>

      <div className={styles.resultsList}>
        {results.map(fileResult => {
        const isCollapsed = collapsedFiles.has(fileResult.file);
        return <div key={fileResult.file} className={styles.fileResult}>
              <div className={styles.fileHeader}>
                <span className={styles.collapseToggle} onClick={() => toggleFile(fileResult.file)}>
                  {isCollapsed ? '▶' : '▼'}
                </span>
                <span className={styles.fileIcon}>📄</span>
                <span className={styles.fileName} onClick={() => handleFileClick(fileResult.file)}>
                  {fileResult.file}
                </span>
                <span className={styles.matchCount}>{fileResult.matchCount}</span>
              </div>
              {!isCollapsed && fileResult.matches.map((match, idx) => <div key={`${fileResult.file}-${match.line}-${idx}`} className={styles.matchLine} onClick={() => handleMatchClick(fileResult.file, match)}>
                    <span className={styles.lineNumber}>{match.line}</span>
                    <span className={styles.lineContent}>
                      {highlightMatch(match.content, match.matchStart, match.matchEnd)}
                    </span>
                    <span className={styles.lineColumn}>:{match.column}</span>
                  </div>)}
            </div>;
      })}
      </div>
    </div>;
}