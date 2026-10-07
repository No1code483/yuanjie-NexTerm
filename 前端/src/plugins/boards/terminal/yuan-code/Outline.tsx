import { t } from "i18next";
import { useEffect, useState, useCallback } from 'react';
import styles from './Outline.module.css';
interface OutlineProps {
  editor: any;
  onSymbolClick?: (line: number) => void;
}
interface SymbolEntry {
  name: string;
  kind: string;
  line: number;
  column: number;
  children?: SymbolEntry[];
}
const KIND_ICONS: Record<string, string> = {
  class: 'C',
  interface: 'I',
  struct: 'S',
  enum: 'E',
  function: 'f',
  method: 'm',
  variable: 'v',
  constant: 'k',
  property: 'p',
  module: 'M',
  namespace: 'N',
  package: 'P',
  string: 's',
  number: 'n',
  boolean: 'b',
  array: '[]',
  object: '{}',
  key: 'K',
  file: 'F',
  reference: '→',
  constructor: 'C+',
  field: 'Fd',
  typeParameter: 'T',
  operator: 'op',
  event: '⚡'
};
function getIcon(kind: string): string {
  for (const [key, icon] of Object.entries(KIND_ICONS)) {
    if (kind.toLowerCase().includes(key.toLowerCase())) {
      return icon;
    }
  }
  return '•';
}
export default function Outline({
  editor,
  onSymbolClick
}: OutlineProps) {
  const [symbols, setSymbols] = useState<SymbolEntry[]>([]);
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const refreshSymbols = useCallback(() => {
    if (!editor) {
      setSymbols([]);
      return;
    }
    const model = editor.getModel();
    if (!model) {
      setSymbols([]);
      return;
    }
    try {
      // Try to get document symbols from the model
      const text = model.getValue();
      const lines = text.split('\n');
      const foundSymbols: SymbolEntry[] = [];

      // Simple heuristic symbol detection based on language
      const language = model.getLanguageId();
      for (let i = 0; i < lines.length; i++) {
        const line = lines[i].trim();
        const symbol = detectSymbol(line, i + 1, language);
        if (symbol) {
          foundSymbols.push(symbol);
        }
      }
      setSymbols(foundSymbols);
    } catch {
      setSymbols([]);
    }
  }, [editor]);
  useEffect(() => {
    refreshSymbols();
    if (!editor) return;
    const disposable = editor.onDidChangeModelContent(() => {
      // Debounce symbol refresh
      const handle = setTimeout(refreshSymbols, 500);
      return () => clearTimeout(handle);
    });
    const modelDisposable = editor.onDidChangeModel(() => {
      refreshSymbols();
    });
    return () => {
      disposable.dispose();
      modelDisposable.dispose();
    };
  }, [editor, refreshSymbols]);
  const toggleCollapse = (key: string) => {
    setCollapsed(prev => {
      const next = new Set(prev);
      if (next.has(key)) {
        next.delete(key);
      } else {
        next.add(key);
      }
      return next;
    });
  };
  const renderSymbol = (symbol: SymbolEntry, depth: number = 0, parentKey: string = '') => {
    const key = `${parentKey}-${symbol.name}-${symbol.line}`;
    const isCollapsed = collapsed.has(key);
    const hasChildren = symbol.children && symbol.children.length > 0;
    return <div key={key}>
        <div className={styles.symbolItem} style={{
        paddingLeft: `${12 + depth * 16}px`
      }} onClick={() => onSymbolClick?.(symbol.line)} title={`${symbol.kind}: ${symbol.name} (line ${symbol.line})`}>
          {hasChildren && <span className={styles.collapseToggle} onClick={e => {
          e.stopPropagation();
          toggleCollapse(key);
        }}>
              {isCollapsed ? '▶' : '▼'}
            </span>}
          <span className={styles.symbolIcon} title={symbol.kind}>
            {getIcon(symbol.kind)}
          </span>
          <span className={styles.symbolName}>{symbol.name}</span>
          <span className={styles.symbolLine}>{symbol.line}</span>
        </div>
        {hasChildren && !isCollapsed && <div className={styles.symbolChildren}>
            {symbol.children!.map(child => renderSymbol(child, depth + 1, key))}
          </div>}
      </div>;
  };
  return <div className={styles.outline}>
      <div className={styles.header}>
        <span className={styles.title}>{t("yuan-code.Outline.k1")}</span>
        <button className={styles.refreshBtn} onClick={refreshSymbols} title={t("common.refresh")}>
          ⟳
        </button>
      </div>
      <div className={styles.list}>
        {symbols.length === 0 ? <div className={styles.empty}>{t("yuan-code.Outline.k2")}</div> : symbols.map(sym => renderSymbol(sym))}
      </div>
    </div>;
}
function detectSymbol(line: string, lineNum: number, _language: string): SymbolEntry | null {
  line = line.trim();

  // Skip comments and empty lines
  if (!line || line.startsWith('//') || line.startsWith('#') || line.startsWith('--')) {
    return null;
  }

  // Function declarations
  const funcMatch = line.match(/^(?:export\s+)?(?:async\s+)?(?:static\s+)?(?:public\s+)?(?:private\s+)?(?:protected\s+)?(?:function\s+)?(\w+)\s*\(/);
  if (funcMatch && !line.startsWith('if') && !line.startsWith('for') && !line.startsWith('while') && !line.startsWith('switch')) {
    return {
      name: funcMatch[1],
      kind: line.includes('class') ? 'method' : 'function',
      line: lineNum,
      column: 1
    };
  }

  // Rust function
  const rustFnMatch = line.match(/^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)/);
  if (rustFnMatch) {
    return {
      name: rustFnMatch[1],
      kind: 'function',
      line: lineNum,
      column: 1
    };
  }

  // Class/Struct declarations
  const classMatch = line.match(/^(?:export\s+)?(?:abstract\s+)?(?:class|interface|struct|enum|trait|impl)\s+(\w+)/);
  if (classMatch) {
    return {
      name: classMatch[1],
      kind: classMatch[0].includes('class') ? 'class' : classMatch[0].includes('interface') ? 'interface' : classMatch[0].includes('struct') ? 'struct' : classMatch[0].includes('enum') ? 'enum' : classMatch[0].includes('trait') ? 'interface' : 'class',
      line: lineNum,
      column: 1
    };
  }

  // Variable declarations
  const varMatch = line.match(/^(?:export\s+)?(?:const|let|var)\s+(\w+)/);
  if (varMatch && !line.includes('=') === false) {
    return {
      name: varMatch[1],
      kind: line.includes('const') ? 'constant' : 'variable',
      line: lineNum,
      column: 1
    };
  }

  // TypeScript type/interface
  const typeMatch = line.match(/^(?:export\s+)?(?:type)\s+(\w+)/);
  if (typeMatch) {
    return {
      name: typeMatch[1],
      kind: 'typeParameter',
      line: lineNum,
      column: 1
    };
  }
  return null;
}