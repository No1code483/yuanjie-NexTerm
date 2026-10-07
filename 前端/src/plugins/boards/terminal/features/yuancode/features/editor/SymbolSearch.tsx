import { t } from "i18next";
import { useState, useCallback, useEffect, useRef } from 'react';
import styles from './SymbolSearch.module.css';
interface SymbolSearchProps {
  editor: any;
  onNavigate?: (file: string, line: number) => void;
}
interface SearchResult {
  name: string;
  kind: string;
  line: number;
  column: number;
  file: string;
  container?: string;
}
export default function SymbolSearch({
  editor,
  onNavigate
}: SymbolSearchProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [scope, setScope] = useState<'file' | 'workspace'>('file');
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Keyboard shortcut to open: Ctrl+Shift+O (file) / Ctrl+T (workspace)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && e.key === 'O') {
        e.preventDefault();
        setScope('file');
        setIsOpen(true);
        setQuery('');
      } else if (e.ctrlKey && e.key === 't') {
        e.preventDefault();
        setScope('workspace');
        setIsOpen(true);
        setQuery('');
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);
  const searchSymbols = useCallback((searchQuery: string) => {
    if (!searchQuery.trim() || !editor) {
      setResults([]);
      return;
    }
    if (scope === 'file') {
      const model = editor.getModel();
      if (!model) {
        setResults([]);
        return;
      }
      const text = model.getValue();
      const lines = text.split('\n');
      const found: SearchResult[] = [];
      const lowerQuery = searchQuery.toLowerCase();
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();
        const lowerLine = line.toLowerCase();
        if (lowerLine.includes(lowerQuery)) {
          // Try to detect symbol type
          const kind = detectSymbolKind(line);
          const name = extractSymbolName(line);
          if (name) {
            found.push({
              name,
              kind,
              line: i + 1,
              column: line.indexOf(name) + 1,
              file: model.uri.toString()
            });
          }
        }
      }
      setResults(found.slice(0, 50));
    } else {
      // Workspace search would require backend support
      // For now, show file symbols
      const model = editor?.getModel();
      if (!model) {
        setResults([]);
        return;
      }
      const text = model.getValue();
      const lines = text.split('\n');
      const found: SearchResult[] = [];
      const lowerQuery = searchQuery.toLowerCase();
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();
        const lowerLine = line.toLowerCase();
        if (lowerLine.includes(lowerQuery)) {
          const kind = detectSymbolKind(line);
          const name = extractSymbolName(line);
          if (name) {
            found.push({
              name,
              kind,
              line: i + 1,
              column: line.indexOf(name) + 1,
              file: model.uri.toString()
            });
          }
        }
      }
      setResults(found.slice(0, 50));
    }
  }, [editor, scope]);
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setQuery(value);
    setSelectedIndex(0);
    searchSymbols(value);
  };
  const handleKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'Escape':
        setIsOpen(false);
        break;
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, results.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (results[selectedIndex]) {
          handleSelect(results[selectedIndex]);
        }
        break;
    }
  };
  const handleSelect = (result: SearchResult) => {
    setIsOpen(false);
    if (editor) {
      editor.setPosition({
        lineNumber: result.line,
        column: result.column
      });
      editor.revealLineInCenter(result.line);
      editor.focus();
    }
    onNavigate?.(result.file, result.line);
  };

  // Scroll selected item into view
  useEffect(() => {
    if (listRef.current) {
      const selected = listRef.current.children[selectedIndex] as HTMLElement;
      if (selected) {
        selected.scrollIntoView({
          block: 'nearest'
        });
      }
    }
  }, [selectedIndex]);
  if (!isOpen) return null;
  return <div className={styles.overlay} onClick={() => setIsOpen(false)}>
      <div className={styles.dialog} onClick={e => e.stopPropagation()}>
        <div className={styles.inputArea}>
          <span className={styles.scopeIndicator}>
            {scope === 'file' ? '📄 @' : '🔍 #'}
          </span>
          <input ref={inputRef} className={styles.input} type="text" value={query} onChange={handleInputChange} onKeyDown={handleKeyDown} placeholder={scope === 'file' ? t("yuan-code.SymbolSearch.k1") : t("yuan-code.SymbolSearch.k2")} autoFocus />
          <div className={styles.scopeButtons}>
            <button className={`${styles.scopeBtn} ${scope === 'file' ? styles.active : ''}`} onClick={() => setScope('file')} title={t("yuan-code.SymbolSearch.k3")}>
              @
            </button>
            <button className={`${styles.scopeBtn} ${scope === 'workspace' ? styles.active : ''}`} onClick={() => setScope('workspace')} title={t("yuan-code.SymbolSearch.k4")}>
              #
            </button>
          </div>
        </div>
        <div className={styles.results} ref={listRef}>
          {results.length === 0 && query.trim() ? <div className={styles.noResults}>{t("yuan-code.SymbolSearch.k5")}</div> : results.map((result, index) => <div key={`${result.file}-${result.line}-${result.name}`} className={`${styles.resultItem} ${index === selectedIndex ? styles.selected : ''}`} onClick={() => handleSelect(result)} onMouseEnter={() => setSelectedIndex(index)}>
                <span className={styles.resultKind}>{getKindIcon(result.kind)}</span>
                <span className={styles.resultName}>{result.name}</span>
                {result.container && <span className={styles.resultContainer}>{result.container}</span>}
                <span className={styles.resultLine}>:{result.line}</span>
              </div>)}
        </div>
        <div className={styles.footer}>
          <span className={styles.hint}>{t("yuan-code.SymbolSearch.k6")}</span>
          <span className={styles.hint}>{t("yuan-code.SymbolSearch.k7")}</span>
          <span className={styles.hint}>{t("Knowledge.k294")}</span>
        </div>
      </div>
    </div>;
}
function detectSymbolKind(line: string): string {
  const trimmed = line.trim();
  if (trimmed.startsWith('fn ') || trimmed.startsWith('pub fn ')) return 'function';
  if (trimmed.match(/^(?:export\s+)?(?:async\s+)?function\s+\w+/)) return 'function';
  if (trimmed.match(/^(?:export\s+)?(?:class|struct|enum|trait|impl)\s+\w+/)) {
    if (trimmed.includes('class')) return 'class';
    if (trimmed.includes('struct')) return 'struct';
    if (trimmed.includes('enum')) return 'enum';
    if (trimmed.includes('trait')) return 'interface';
    return 'class';
  }
  if (trimmed.match(/^(?:export\s+)?(?:interface|type)\s+\w+/)) return 'interface';
  if (trimmed.match(/^(?:export\s+)?(?:const|let|var)\s+\w+/)) {
    return trimmed.includes('const') ? 'constant' : 'variable';
  }
  return 'symbol';
}
function extractSymbolName(line: string): string | null {
  const trimmed = line.trim();

  // Rust: fn name, pub fn name, struct Name, enum Name, trait Name, impl Name
  const rustMatch = trimmed.match(/^(?:pub(?:\s*\(\s*(?:crate|super|self)\s*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:fn|struct|enum|trait|impl|mod|type|const|static)\s+(\w+)/);
  if (rustMatch) return rustMatch[1];

  // JS/TS: function name, class Name, const name, let name, var name, interface Name, type Name
  const jsMatch = trimmed.match(/^(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:function|class|const|let|var|interface|type)\s+(\w+)/);
  if (jsMatch) return jsMatch[1];

  // Python: def name, class Name
  const pyMatch = trimmed.match(/^(?:async\s+)?(?:def|class)\s+(\w+)/);
  if (pyMatch) return pyMatch[1];
  return null;
}
function getKindIcon(kind: string): string {
  switch (kind.toLowerCase()) {
    case 'function':
      return '𝑓';
    case 'method':
      return '𝑚';
    case 'class':
      return '𝐶';
    case 'interface':
      return '𝐼';
    case 'struct':
      return '𝑆';
    case 'enum':
      return '𝐸';
    case 'variable':
      return '𝑣';
    case 'constant':
      return '𝑐';
    case 'symbol':
      return '𝑠';
    default:
      return '◦';
  }
}