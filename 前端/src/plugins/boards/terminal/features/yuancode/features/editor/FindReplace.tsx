import { t } from "i18next";
/**
 * 查找/替换组件 — 对标 VSCode contrib/find/
 *
 * 功能:
 * - Ctrl+F 打开查找栏，Ctrl+H 打开替换栏
 * - 支持正则表达式、大小写匹配、全词匹配
 * - 匹配计数显示
 * - 上一个/下一个匹配导航
 * - 替换/全部替换
 * - 查找历史记录
 */

import { useState, useCallback, useEffect, useRef } from 'react';
import styles from './FindReplace.module.css';
interface FindReplaceProps {
  /** 编辑器实例 */
  editor: any;
  /** 关闭回调 */
  onClose: () => void;
}
interface SearchOptions {
  regex: boolean;
  caseSensitive: boolean;
  wholeWord: boolean;
}
export default function FindReplace({
  editor,
  onClose
}: FindReplaceProps) {
  const [findText, setFindText] = useState('');
  const [replaceText, setReplaceText] = useState('');
  const [showReplace, setShowReplace] = useState(false);
  const [options, setOptions] = useState<SearchOptions>({
    regex: false,
    caseSensitive: false,
    wholeWord: false
  });
  const [matchInfo, setMatchInfo] = useState({
    current: 0,
    total: 0
  });
  const [searchError, setSearchError] = useState('');
  const findInputRef = useRef<HTMLInputElement>(null);
  const replaceInputRef = useRef<HTMLInputElement>(null);
  const lastQueryRef = useRef('');

  // 执行搜索
  const doSearch = useCallback((query: string, opts: SearchOptions) => {
    if (!editor) return;
    const model = editor.getModel();
    if (!model || !query) {
      setMatchInfo({
        current: 0,
        total: 0
      });
      setSearchError('');
      return;
    }
    try {
      // 构建正则或纯文本搜索
      let searchQuery: RegExp | string = query;
      if (opts.regex) {
        try {
          searchQuery = new RegExp(query, opts.caseSensitive ? 'g' : 'gi');
        } catch {
          setSearchError(t("yuan-code.FindReplace.k1"));
          setMatchInfo({
            current: 0,
            total: 0
          });
          return;
        }
      }
      setSearchError('');
      const matches = model.findMatches(searchQuery, true,
      // searchOnlyEditableRange
      opts.regex, opts.caseSensitive, opts.wholeWord ? query : null, false // searchInSelection
      );
      const currentPos = editor.getPosition();
      let currentIndex = 0;
      if (currentPos && matches.length > 0) {
        // 找到当前光标之后的第一个匹配
        for (let i = 0; i < matches.length; i++) {
          const m = matches[i];
          if (m.range.startLineNumber > currentPos.lineNumber || m.range.startLineNumber === currentPos.lineNumber && m.range.startColumn > currentPos.column) {
            currentIndex = i;
            break;
          }
        }
      }
      setMatchInfo({
        current: matches.length > 0 ? currentIndex + 1 : 0,
        total: matches.length
      });
    } catch {
      setSearchError(t("yuan-code.FindReplace.k2"));
    }
  }, [editor]);

  // 查找文本变化时执行搜索
  useEffect(() => {
    if (findText !== lastQueryRef.current) {
      lastQueryRef.current = findText;
      doSearch(findText, options);
    }
  }, [findText, options, doSearch]);
  const goToNext = useCallback(() => {
    if (!editor || !findText) return;
    const model = editor.getModel();
    if (!model) return;
    try {
      let searchQuery: RegExp | string = findText;
      if (options.regex) {
        searchQuery = new RegExp(findText, options.caseSensitive ? 'g' : 'gi');
      }
      const matches = model.findMatches(searchQuery, true, options.regex, options.caseSensitive, options.wholeWord ? findText : null, false);
      if (matches.length === 0) return;
      const currentPos = editor.getPosition();
      let nextIdx = 0;
      if (currentPos) {
        for (let i = 0; i < matches.length; i++) {
          const m = matches[i];
          if (m.range.startLineNumber > currentPos.lineNumber || m.range.startLineNumber === currentPos.lineNumber && m.range.startColumn > currentPos.column) {
            nextIdx = i;
            break;
          }
        }
      }
      const match = matches[nextIdx];
      editor.setSelection(match.range);
      editor.revealRangeInCenter(match.range);
      setMatchInfo({
        current: nextIdx + 1,
        total: matches.length
      });
    } catch {
      // ignore
    }
  }, [editor, findText, options]);
  const goToPrev = useCallback(() => {
    if (!editor || !findText) return;
    const model = editor.getModel();
    if (!model) return;
    try {
      let searchQuery: RegExp | string = findText;
      if (options.regex) {
        searchQuery = new RegExp(findText, options.caseSensitive ? 'g' : 'gi');
      }
      const matches = model.findMatches(searchQuery, true, options.regex, options.caseSensitive, options.wholeWord ? findText : null, false);
      if (matches.length === 0) return;
      const currentPos = editor.getPosition();
      let prevIdx = matches.length - 1;
      if (currentPos) {
        for (let i = matches.length - 1; i >= 0; i--) {
          const m = matches[i];
          if (m.range.startLineNumber < currentPos.lineNumber || m.range.startLineNumber === currentPos.lineNumber && m.range.startColumn < currentPos.column) {
            prevIdx = i;
            break;
          }
        }
      }
      const match = matches[prevIdx];
      editor.setSelection(match.range);
      editor.revealRangeInCenter(match.range);
      setMatchInfo({
        current: prevIdx + 1,
        total: matches.length
      });
    } catch {
      // ignore
    }
  }, [editor, findText, options]);

  // 替换单个
  const handleReplace = useCallback(() => {
    if (!editor || !findText) return;
    const selection = editor.getSelection();
    if (!selection || selection.isEmpty()) {
      goToNext();
      return;
    }
    const model = editor.getModel();
    if (!model) return;

    // 检查当前选中的是否匹配
    const selectedText = model.getValueInRange(selection);
    let isMatch = false;
    if (options.regex) {
      try {
        const re = new RegExp(findText, options.caseSensitive ? '' : 'i');
        isMatch = re.test(selectedText);
      } catch {
        isMatch = selectedText === findText;
      }
    } else if (options.caseSensitive) {
      isMatch = selectedText === findText;
    } else {
      isMatch = selectedText.toLowerCase() === findText.toLowerCase();
    }
    if (isMatch) {
      editor.executeEdits('find-replace', [{
        range: selection,
        text: replaceText
      }]);
    }
    goToNext();
  }, [editor, findText, replaceText, options, goToNext]);

  // 全部替换
  const handleReplaceAll = useCallback(() => {
    if (!editor || !findText) return;
    const model = editor.getModel();
    if (!model) return;
    try {
      let searchQuery: RegExp | string = findText;
      if (options.regex) {
        searchQuery = new RegExp(findText, options.caseSensitive ? 'g' : 'gi');
      }
      const matches = model.findMatches(searchQuery, true, options.regex, options.caseSensitive, options.wholeWord ? findText : null, false);
      if (matches.length === 0) return;

      // 反向替换，避免偏移问题
      const edits = matches.slice().reverse().map((m: {
        range: {
          startLineNumber: number;
          startColumn: number;
          endLineNumber: number;
          endColumn: number;
        };
      }) => {
        let text = replaceText;
        if (options.regex) {
          try {
            const re = new RegExp(findText, options.caseSensitive ? 'g' : 'gi');
            const matched = model.getValueInRange(m.range);
            text = matched.replace(re, replaceText);
          } catch {
            // fallback
          }
        }
        return {
          range: m.range,
          text
        };
      });
      editor.executeEdits('find-replace-all', edits);
      setMatchInfo({
        current: 0,
        total: 0
      });
    } catch {
      // ignore
    }
  }, [editor, findText, replaceText, options]);

  // 焦点管理
  useEffect(() => {
    findInputRef.current?.focus();
    findInputRef.current?.select();
  }, []);

  // 键盘快捷键
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (e.shiftKey) {
        goToPrev();
      } else {
        goToNext();
      }
    } else if (e.ctrlKey && e.key === 'h') {
      e.preventDefault();
      setShowReplace(v => !v);
      setTimeout(() => replaceInputRef.current?.focus(), 0);
    }
  }, [onClose, goToNext, goToPrev]);
  const toggleOption = useCallback((key: keyof SearchOptions) => {
    setOptions(prev => {
      const next = {
        ...prev,
        [key]: !prev[key]
      };
      doSearch(findText, next);
      return next;
    });
  }, [findText, doSearch]);
  return <div className={styles.findBar} onKeyDown={handleKeyDown}>
      {/* 查找行 */}
      <div className={styles.findRow}>
        <div className={styles.inputWrap}>
          <input ref={findInputRef} className={styles.input} type="text" value={findText} onChange={e => setFindText(e.target.value)} placeholder={t("yuan-code.FindReplace.k3")} spellCheck={false} />
        </div>

        {/* 匹配计数 */}
        {(matchInfo.total > 0 || searchError) && <span className={searchError ? styles.matchCountError : styles.matchCount}>
            {searchError || `${matchInfo.current} / ${matchInfo.total}`}
          </span>}

        {/* 导航按钮 */}
        <button className={styles.navBtn} onClick={goToPrev} title={t("yuan-code.FindReplace.k4")}>
          ▲
        </button>
        <button className={styles.navBtn} onClick={goToNext} title={t("yuan-code.FindReplace.k5")}>
          ▼
        </button>

        {/* 选项按钮 */}
        <button className={`${styles.optionBtn} ${options.caseSensitive ? styles.optionActive : ''}`} onClick={() => toggleOption('caseSensitive')} title={t("yuan-code.FindReplace.k6")}>
          Aa
        </button>
        <button className={`${styles.optionBtn} ${options.wholeWord ? styles.optionActive : ''}`} onClick={() => toggleOption('wholeWord')} title={t("yuan-code.FindReplace.k7")}>
          ab
        </button>
        <button className={`${styles.optionBtn} ${options.regex ? styles.optionActive : ''}`} onClick={() => toggleOption('regex')} title={t("yuan-code.FindReplace.k8")}>
          .*
        </button>

        {/* 替换切换 */}
        <button className={`${styles.optionBtn} ${showReplace ? styles.optionActive : ''}`} onClick={() => {
        setShowReplace(v => !v);
        setTimeout(() => replaceInputRef.current?.focus(), 0);
      }} title={t("yuan-code.FindReplace.k9")}>
          ⇄
        </button>

        {/* 关闭 */}
        <button className={styles.closeBtn} onClick={onClose} title={t("components.NexTermTerminal.k5")}>
          ✕
        </button>
      </div>

      {/* 替换行 */}
      {showReplace && <div className={styles.replaceRow}>
          <div className={styles.inputWrap}>
            <input ref={replaceInputRef} className={styles.input} type="text" value={replaceText} onChange={e => setReplaceText(e.target.value)} placeholder={t("yuan-code.FindReplace.k10")} spellCheck={false} />
          </div>
          <button className={styles.replaceBtn} onClick={handleReplace}>
            {t("yuan-code.FindReplace.k10")}
          </button>
          <button className={styles.replaceAllBtn} onClick={handleReplaceAll}>
            {t("yuan-code.FindReplace.k11")}
          </button>
        </div>}
    </div>;
}