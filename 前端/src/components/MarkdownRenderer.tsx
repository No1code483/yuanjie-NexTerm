import { t } from "i18next";
import { useEffect, useCallback } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { copy } from '@/lib/utils';
import styles from './MarkdownRenderer.module.css';
interface Props {
  content: string;
  /** 是否在末尾显示闪烁光标 */
  showCursor?: boolean;
}
export default function MarkdownRenderer({
  content,
  showCursor = false
}: Props) {
  // 代码块复制按钮 - 事件委托
  useEffect(() => {
    const handler = async (e: Event) => {
      const target = e.target as HTMLElement;
      if (target.classList.contains('md-copy-btn')) {
        const code = decodeURIComponent(target.getAttribute('data-code') || '');
        if (await copy(code)) {
          target.textContent = t("components.MarkdownRenderer.k1");
          setTimeout(() => {
            target.textContent = t("common.copy");
          }, 1500);
        } else {
          target.textContent = t("common.failed");
          setTimeout(() => {
            target.textContent = t("common.copy");
          }, 1500);
        }
      }
    };
    document.addEventListener('click', handler);
    return () => document.removeEventListener('click', handler);
  }, []);

  // 自定义代码块渲染
  const renderCode = useCallback(({
    node,
    className,
    children,
    ...props
  }: any) => {
    const match = /language-(\w+)/.exec(className || '');
    const code = String(children).replace(/\n$/, '');
    const encoded = encodeURIComponent(code);
    return <div className="code-block-wrapper">
        <div className="code-block-header">
          {match && <span className="code-block-lang">{match[1]}</span>}
          <button className="md-copy-btn" data-code={encoded}>{t("common.copy")}</button>
        </div>
        <pre className={className}>
          <code className={className} {...props}>{children}</code>
        </pre>
      </div>;
  }, []);
  return <div className={styles.markdown}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={{
      code: renderCode as any
    }}>
        {content}
      </ReactMarkdown>
      {showCursor && <span className="md-cursor">▋</span>}
    </div>;
}