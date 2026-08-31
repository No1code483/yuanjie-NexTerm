import { useState, useCallback, useRef, useEffect } from 'react';

interface TabCompletionOptions {
  /** 获取补全建议的函数 */
  getSuggestions: (text: string) => Promise<string[]> | string[];
  /** 延迟触发时间（ms） */
  debounceMs?: number;
}

interface TabCompletionResult {
  /** ghost text 显示内容（补全部分的文本） */
  ghostText: string;
  /** 键盘事件处理器 */
  handleKeyDown: (e: React.KeyboardEvent<HTMLTextAreaElement | HTMLInputElement>) => void;
  /** 输入变化处理器 */
  handleChange: (value: string) => void;
  /** 清除 ghost text */
  clearGhost: () => void;
}

/**
 * 通用 Tab 补全 hook
 * 用于在 textarea/input 中实现 Tab 键 ghost text 补全
 */
export function useTabCompletion({
  getSuggestions,
  debounceMs = 150
}: TabCompletionOptions): TabCompletionResult {
  const [ghostText, setGhostText] = useState('');
  const [currentValue, setCurrentValue] = useState('');
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const suggestionsRef = useRef<string[]>([]);
  const suggestionIndexRef = useRef(0);

  // 清除防抖定时器
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, []);

  // 获取补全建议
  const fetchSuggestions = useCallback(async (text: string) => {
    if (!text.trim()) {
      setGhostText('');
      suggestionsRef.current = [];
      return;
    }

    try {
      const suggestions = await getSuggestions(text);
      suggestionsRef.current = suggestions;
      suggestionIndexRef.current = 0;

      if (suggestions.length > 0) {
        const firstSuggestion = suggestions[0];
        // ghost text 是补全的部分（不包含已输入的文本）
        if (firstSuggestion.toLowerCase().startsWith(text.toLowerCase())) {
          setGhostText(firstSuggestion.slice(text.length));
        } else {
          setGhostText('');
        }
      } else {
        setGhostText('');
      }
    } catch {
      setGhostText('');
    }
  }, [getSuggestions]);

  // 输入变化处理
  const handleChange = useCallback((value: string) => {
    setCurrentValue(value);
    setGhostText('');

    // 防抖触发补全
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }

    debounceTimerRef.current = setTimeout(() => {
      fetchSuggestions(value);
    }, debounceMs);
  }, [fetchSuggestions, debounceMs]);

  // 键盘事件处理
  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement | HTMLInputElement>) => {
    if (e.key === 'Tab') {
      e.preventDefault();

      if (ghostText) {
        // 接受补建议
        const fullText = currentValue + ghostText;
        setCurrentValue(fullText);
        setGhostText('');
        suggestionsRef.current = [];

        // 触发 onChange 事件更新父组件状态
        const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
          window.HTMLInputElement.prototype, 'value'
        )?.set || Object.getOwnPropertyDescriptor(
          window.HTMLTextAreaElement.prototype, 'value'
        )?.set;

        if (nativeInputValueSetter && e.target) {
          nativeInputValueSetter.call(e.target, fullText);
          e.target.dispatchEvent(new Event('input', { bubbles: true }));
        }
      } else if (suggestionsRef.current.length > 0) {
        // 循环切换建议
        suggestionIndexRef.current = (suggestionIndexRef.current + 1) % suggestionsRef.current.length;
        const nextSuggestion = suggestionsRef.current[suggestionIndexRef.current];

        if (nextSuggestion.toLowerCase().startsWith(currentValue.toLowerCase())) {
          setGhostText(nextSuggestion.slice(currentValue.length));
        }
      }
    } else if (e.key === 'Escape') {
      // 取消补全
      setGhostText('');
      suggestionsRef.current = [];
    } else if (e.key === 'ArrowRight') {
      // 逐词接受（可选）
      if (ghostText) {
        const words = ghostText.trim().split(/\s+/);
        if (words.length > 0) {
          const firstWord = words[0];
          const accepted = currentValue + firstWord + ' ';
          setCurrentValue(accepted);
          setGhostText(ghostText.slice(firstWord.length + 1));
        }
      }
    }
  }, [ghostText, currentValue]);

  const clearGhost = useCallback(() => {
    setGhostText('');
    suggestionsRef.current = [];
  }, []);

  return {
    ghostText,
    handleKeyDown,
    handleChange,
    clearGhost
  };
}
