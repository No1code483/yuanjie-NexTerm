// profile.quote L2 —— 语录功能域（局部 state + handlers + JSX）。
// 物理迁入本目录：QuotePanel 组件与本 useQuote hook；核心共享项经 core（ProfileCore）注入。
// active 表示当前是否处于语录 tab（且已鉴权），用于惰性加载（等价于原壳 currentTab==='quote' 分支）。
import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { intelligence } from '@/lib/ipc';
import { profile } from '../../ipc/profile';
import type { ProfileCore } from '../../core';
import type { QuoteItem } from '../../types';
import QuotePanel from './QuotePanel';

export function useQuote(core: ProfileCore, active: boolean) {
  const { showNotify, setQuoteCount, aiOn, featureOn } = core;

  const [quotes, setQuotes] = useState<QuoteItem[]>([]);
  const [quotesLoading, setQuotesLoading] = useState(false);
  const [showAddQuote, setShowAddQuote] = useState(false);
  const [quoteForm, setQuoteForm] = useState({
    content: '',
    source: ''
  });
  const [duplicateResults, setDuplicateResults] = useState<any[]>([]);
  const [showDuplicateDialog, setShowDuplicateDialog] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  const [quoteGhostText, setQuoteGhostText] = useState('');
  const [ghostLoading, setGhostLoading] = useState(false);
  // 语录智能辅助
  const [quoteCheckResult, setQuoteCheckResult] = useState<{
    field: string;
    issue: string;
    suggestion: string | null;
    severity: string;
  } | null>(null);
  const [quoteCheckLoading, setQuoteCheckLoading] = useState(false);
  const handleQuoteSpellCheck = useCallback(async () => {
    if (!quoteForm.content.trim()) return;
    setQuoteCheckLoading(true);
    setQuoteCheckResult(null);
    try {
      const res = await intelligence.quoteSpellCheck(quoteForm.content);
      if (res?.data) setQuoteCheckResult(res.data);
    } catch {/* ignore */} finally {
      setQuoteCheckLoading(false);
    }
  }, [quoteForm.content]);
  const handleQuoteSourceVerify = useCallback(async () => {
    if (!quoteForm.source.trim()) return;
    setQuoteCheckLoading(true);
    setQuoteCheckResult(null);
    try {
      const res = await intelligence.quoteSourceVerify(quoteForm.source, undefined);
      if (res?.data) setQuoteCheckResult(res.data);
    } catch {/* ignore */} finally {
      setQuoteCheckLoading(false);
    }
  }, [quoteForm.source]);
  const handleQuoteSmartComplete = useCallback(async () => {
    if (!quoteForm.content.trim()) return;
    setQuoteCheckLoading(true);
    setQuoteCheckResult(null);
    try {
      const res = await intelligence.quoteSmartComplete(quoteForm.content, quoteForm.source || undefined, undefined);
      if (res?.data) setQuoteCheckResult(res.data);
    } catch {/* ignore */} finally {
      setQuoteCheckLoading(false);
    }
  }, [quoteForm.content, quoteForm.source]);
  const ghostTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchAllQuotes = useCallback(async () => {
    setQuotesLoading(true);
    try {
      const response = await profile.getAllQuotes();
      if (response.code === 0 && response.data) {
        setQuotes(response.data);
        setQuoteCount(response.data.length);
      }
    } catch (err) {
      console.error('获取语录列表失败:', err);
    } finally {
      setQuotesLoading(false);
    }
  }, [setQuoteCount]);

  // 语录 tab 激活时加载（等价原壳 currentTab==='quote' 分支）
  useEffect(() => {
    if (active) {
      profile.seedDefaultQuotes().then(() => fetchAllQuotes());
    }
  }, [active, fetchAllQuotes]);

  const handleDeleteQuote = async (id: number) => {
    try {
      const response = await profile.deleteQuote(id);
      if (response.code === 0) {
        await fetchAllQuotes();
        showNotify('success', t("Profile.k18"));
      }
    } catch (err) {
      console.error('删除语录失败:', err);
    }
  };
  const handleAddQuote = async () => {
    if (!quoteForm.content.trim()) {
      showNotify('error', t("Profile.k54"));
      return;
    }
    setIsLoading(true);
    try {
      const dupRes = await profile.checkQuoteDuplicate(quoteForm.content);
      if (dupRes.code === 0 && dupRes.data && dupRes.data.length > 0) {
        setDuplicateResults(dupRes.data);
        setShowDuplicateDialog(true);
        return;
      }
      const response = await profile.addQuote(quoteForm.content, quoteForm.source || undefined, 'daily');
      if (response.code === 0) {
        setQuoteForm({
          content: '',
          source: ''
        });
        setShowAddQuote(false);
        await fetchAllQuotes();
        showNotify('success', t("Profile.k55"));
      } else {
        showNotify('error', response.message || t("components.PptEditor.k4"));
      }
    } catch (err) {
      console.error('添加语录失败:', err);
      showNotify('error', t("Profile.k17"));
    } finally {
      setIsLoading(false);
    }
  };
  const handleDuplicateKeepExisting = () => {
    setShowDuplicateDialog(false);
    setDuplicateResults([]);
  };
  const handleDuplicateReplaceAll = async () => {
    setIsLoading(true);
    try {
      for (const dup of duplicateResults) {
        await profile.deleteQuote(dup.existing.id);
      }
      const response = await profile.addQuote(quoteForm.content, quoteForm.source || undefined, 'daily');
      if (response.code === 0) {
        setQuoteForm({
          content: '',
          source: ''
        });
        setShowAddQuote(false);
        await fetchAllQuotes();
        showNotify('success', t("Profile.k56"));
      } else {
        showNotify('error', response.message || t("Knowledge.k96"));
      }
    } catch (err) {
      console.error('替换语录失败:', err);
      showNotify('error', t("Profile.k57"));
    } finally {
      setShowDuplicateDialog(false);
      setDuplicateResults([]);
      setIsLoading(false);
    }
  };
  const handleDuplicateAddAnyway = async () => {
    setShowDuplicateDialog(false);
    setDuplicateResults([]);
    setIsLoading(true);
    try {
      const response = await profile.addQuote(quoteForm.content, quoteForm.source || undefined, 'daily');
      if (response.code === 0) {
        setQuoteForm({
          content: '',
          source: ''
        });
        setShowAddQuote(false);
        await fetchAllQuotes();
        showNotify('success', t("Profile.k55"));
      } else {
        showNotify('error', response.message || t("components.PptEditor.k4"));
      }
    } catch (err) {
      console.error('添加语录失败:', err);
      showNotify('error', t("Profile.k17"));
    } finally {
      setIsLoading(false);
    }
  };

  // === Passive AI: auto-complete ghost text when typing quote (debounced 800ms) ===
  useEffect(() => {
    if (!aiOn || !featureOn('smart_complete') || !showAddQuote || quoteForm.content.length < 4) {
      setQuoteGhostText('');
      return;
    }
    if (ghostTimerRef.current) clearTimeout(ghostTimerRef.current);
    setGhostLoading(true);
    ghostTimerRef.current = setTimeout(async () => {
      try {
        const res = await profile.aiQuoteComplete(quoteForm.content);
        if (res.code === 0 && res.data?.completion) {
          setQuoteGhostText(res.data.completion);
        } else {
          setQuoteGhostText('');
        }
      } catch {
        setQuoteGhostText('');
      } finally {
        setGhostLoading(false);
      }
    }, 800);
    return () => {
      if (ghostTimerRef.current) clearTimeout(ghostTimerRef.current);
    };
  }, [quoteForm.content, showAddQuote, aiOn, featureOn]);

  const renderQuote = () => <QuotePanel showDuplicateDialog={showDuplicateDialog} duplicateResults={duplicateResults} showAddQuote={showAddQuote} quoteForm={quoteForm} quoteCheckLoading={quoteCheckLoading} quoteCheckResult={quoteCheckResult} ghostLoading={ghostLoading} quoteGhostText={quoteGhostText} quotesLoading={quotesLoading} quotes={quotes} isLoading={isLoading} aiOn={aiOn} featureOn={featureOn} setQuoteForm={setQuoteForm} setShowAddQuote={setShowAddQuote} setQuoteGhostText={setQuoteGhostText} setQuoteCheckResult={setQuoteCheckResult} handleDuplicateKeepExisting={handleDuplicateKeepExisting} handleDuplicateReplaceAll={handleDuplicateReplaceAll} handleDuplicateAddAnyway={handleDuplicateAddAnyway} handleQuoteSpellCheck={handleQuoteSpellCheck} handleQuoteSourceVerify={handleQuoteSourceVerify} handleQuoteSmartComplete={handleQuoteSmartComplete} handleAddQuote={handleAddQuote} handleDeleteQuote={handleDeleteQuote} />;

  return { showAddQuote, setShowAddQuote, renderQuote };
}
