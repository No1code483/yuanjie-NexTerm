import { t } from "i18next";
import { useState, useMemo, useCallback } from 'react';
import { copy } from '@/lib/utils';
interface SyntaxToken {
  text: string;
  scope: string;
}
interface CodePreviewProps {
  tokens: SyntaxToken[];
  language: string;
  className?: string;
}
const SCOPE_COLORS_DARK: Record<string, string> = {
  'keyword': '#c678dd',
  'keyword.control': '#c678dd',
  'keyword.operator': '#56b6c2',
  'string': '#98c379',
  'string.quoted': '#98c379',
  'constant': '#d19a66',
  'constant.numeric': '#d19a66',
  'constant.character': '#d19a66',
  'variable': '#e06c75',
  'variable.parameter': '#e06c75',
  'function': '#61afef',
  'entity.name.function': '#61afef',
  'entity.name.type': '#e5c07b',
  'type': '#e5c07b',
  'comment': '#5c6370',
  'comment.line': '#5c6370',
  'comment.block': '#5c6370',
  'support': '#56b6c2',
  'support.function': '#56b6c2',
  'support.type': '#56b6c2',
  'meta': '#abb2bf',
  'punctuation': '#abb2bf',
  'text': '#abb2bf'
};
const SCOPE_COLORS_LIGHT: Record<string, string> = {
  'keyword': '#a626a4',
  'keyword.control': '#a626a4',
  'keyword.operator': '#0184bc',
  'string': '#50a14f',
  'string.quoted': '#50a14f',
  'constant': '#986801',
  'constant.numeric': '#986801',
  'constant.character': '#986801',
  'variable': '#e45649',
  'variable.parameter': '#e45649',
  'function': '#4078f2',
  'entity.name.function': '#4078f2',
  'entity.name.type': '#c18401',
  'type': '#c18401',
  'comment': '#a0a1a7',
  'comment.line': '#a0a1a7',
  'comment.block': '#a0a1a7',
  'support': '#0184bc',
  'support.function': '#0184bc',
  'support.type': '#0184bc',
  'meta': '#383a42',
  'punctuation': '#383a42',
  'text': '#383a42'
};
function resolveScopeColor(scope: string, dark: boolean): string {
  const colors = dark ? SCOPE_COLORS_DARK : SCOPE_COLORS_LIGHT;
  const parts = scope.split(' ');
  for (const part of parts) {
    if (colors[part]) return colors[part];
    const dotIdx = part.lastIndexOf('.');
    if (dotIdx > 0 && colors[part.substring(0, dotIdx)]) {
      return colors[part.substring(0, dotIdx)];
    }
  }
  return colors['text'];
}
export default function CodePreview({
  tokens,
  language,
  className
}: CodePreviewProps) {
  const [dark, setDark] = useState(true);
  const [copied, setCopied] = useState(false);
  const lines = useMemo(() => {
    const result: Array<Array<{
      text: string;
      color: string;
    }>> = [[]];
    for (const token of tokens) {
      const color = resolveScopeColor(token.scope, dark);
      const parts = token.text.split('\n');
      for (let i = 0; i < parts.length; i++) {
        if (i > 0) {
          result.push([]);
        }
        if (parts[i]) {
          result[result.length - 1].push({
            text: parts[i],
            color
          });
        }
      }
    }
    return result;
  }, [tokens, dark]);
  const fullText = useMemo(() => {
    return tokens.map(t => t.text).join('');
  }, [tokens]);
  const handleCopy = useCallback(async () => {
    const ok = await copy(fullText);
    if (ok) {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }, [fullText]);
  const lineCount = lines.length;
  const maxLineNumWidth = Math.max(2, String(lineCount).length);
  return <div className={className} style={{
    display: 'flex',
    flexDirection: 'column',
    height: '100%'
  }}>
      <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '6px 12px',
      background: dark ? 'rgba(30, 30, 40, 0.8)' : 'rgba(240, 240, 245, 0.9)',
      borderBottom: `1px solid ${dark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.08)'}`
    }}>
        <span style={{
        color: dark ? '#fff' : '#333',
        fontSize: 12,
        fontWeight: 500,
        fontFamily: 'var(--nt-font-mono)'
      }}>
          💻 {language.toUpperCase()} · {lineCount} {t("components.CodePreview.k1")}
        </span>
        <div style={{
        flex: 1
      }} />
        <button onClick={() => setDark(d => !d)} title={dark ? t("components.CodePreview.k2") : t("components.CodePreview.k3")} style={{
        background: dark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.06)',
        border: `1px solid ${dark ? 'rgba(255,255,255,0.15)' : 'rgba(0,0,0,0.12)'}`,
        color: dark ? '#aaa' : '#666',
        cursor: 'pointer',
        padding: '3px 8px',
        borderRadius: 4,
        fontSize: 11
      }}>
          {dark ? '☀️' : '🌙'}
        </button>
        <button onClick={handleCopy} style={{
        background: copied ? 'rgba(0,240,255,0.15)' : dark ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.06)',
        border: `1px solid ${copied ? 'rgba(0,240,255,0.3)' : dark ? 'rgba(255,255,255,0.15)' : 'rgba(0,0,0,0.12)'}`,
        color: copied ? '#00F0FF' : dark ? '#aaa' : '#666',
        cursor: 'pointer',
        padding: '3px 8px',
        borderRadius: 4,
        fontSize: 11
      }}>
          {copied ? t("components.CodePreview.k4") : t("components.CodePreview.k5")}
        </button>
      </div>

      <div style={{
      flex: 1,
      overflow: 'auto',
      background: dark ? '#1e1e2e' : '#fafafa'
    }}>
        <pre style={{
        margin: 0,
        padding: '12px 0',
        fontFamily: '"JetBrains Mono", "Fira Code", "Cascadia Code", "Consolas", monospace',
        fontSize: 13,
        lineHeight: 1.6,
        color: dark ? '#abb2bf' : '#383a42',
        counterReset: 'code-line'
      }}>
          <code>
            {lines.map((lineTokens, lineIdx) => <div key={lineIdx} style={{
            display: 'flex',
            minHeight: `${1.6 * 13}px`
          }}>
                <span style={{
              display: 'inline-block',
              width: `${maxLineNumWidth * 10 + 24}px`,
              minWidth: '40px',
              textAlign: 'right',
              paddingRight: 12,
              userSelect: 'none',
              color: dark ? '#4a4a5a' : '#ccc',
              fontSize: 12,
              flexShrink: 0,
              borderRight: `1px solid ${dark ? 'rgba(255,255,255,0.05)' : 'rgba(0,0,0,0.05)'}`
            }}>
                  {lineIdx + 1}
                </span>
                <span style={{
              paddingLeft: 16,
              whiteSpace: 'pre'
            }}>
                  {lineTokens.map((lt, ti) => <span key={ti} style={{
                color: lt.color
              }}>{lt.text}</span>)}
                </span>
              </div>)}
          </code>
        </pre>
      </div>
    </div>;
}