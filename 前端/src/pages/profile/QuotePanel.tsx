import { t } from "i18next";
import { useRef, useCallback, useState, useEffect } from 'react';
import type { QuoteItem } from './types';
import styles from '../Profile.module.css';
interface QuotePanelProps {
  showDuplicateDialog: boolean;
  duplicateResults: any[];
  showAddQuote: boolean;
  quoteForm: {
    content: string;
    source: string;
  };
  quoteCheckLoading: boolean;
  quoteCheckResult: {
    field: string;
    issue: string;
    suggestion: string | null;
    severity: string;
  } | null;
  ghostLoading: boolean;
  quoteGhostText: string;
  quotesLoading: boolean;
  quotes: QuoteItem[];
  isLoading: boolean;
  aiOn: boolean;
  featureOn: (feature: string) => boolean;
  setQuoteForm: React.Dispatch<React.SetStateAction<{
    content: string;
    source: string;
  }>>;
  setShowAddQuote: React.Dispatch<React.SetStateAction<boolean>>;
  setQuoteGhostText: React.Dispatch<React.SetStateAction<string>>;
  setQuoteCheckResult: React.Dispatch<React.SetStateAction<{
    field: string;
    issue: string;
    suggestion: string | null;
    severity: string;
  } | null>>;
  handleDuplicateKeepExisting: () => void;
  handleDuplicateReplaceAll: () => Promise<void>;
  handleDuplicateAddAnyway: () => Promise<void>;
  handleQuoteSpellCheck: () => Promise<void>;
  handleQuoteSourceVerify: () => Promise<void>;
  handleQuoteSmartComplete: () => Promise<void>;
  handleAddQuote: () => Promise<void>;
  handleDeleteQuote: (id: number) => Promise<void>;
}
export default function QuotePanel({
  showDuplicateDialog,
  duplicateResults,
  showAddQuote,
  quoteForm,
  quoteCheckLoading,
  quoteCheckResult,
  ghostLoading,
  quoteGhostText,
  quotesLoading,
  quotes,
  isLoading,
  aiOn,
  featureOn,
  setQuoteForm,
  setShowAddQuote,
  setQuoteGhostText,
  setQuoteCheckResult,
  handleDuplicateKeepExisting,
  handleDuplicateReplaceAll,
  handleDuplicateAddAnyway,
  handleQuoteSpellCheck,
  handleQuoteSourceVerify,
  handleQuoteSmartComplete,
  handleAddQuote,
  handleDeleteQuote
}: QuotePanelProps) {
  const quoteContentRef = useRef<HTMLTextAreaElement>(null);
  const [tabGhostText, setTabGhostText] = useState('');
  const tabDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Tab 补全：从历史语录中获取建议
  const fetchQuoteSuggestions = useCallback((text: string) => {
    if (!text.trim() || text.length < 2) return [];
    const prefix = text.toLowerCase();
    return quotes
      .filter(q => q.content.toLowerCase().startsWith(prefix) && q.content !== text)
      .map(q => q.content)
      .slice(0, 5);
  }, [quotes]);

  // 输入变化时触发补全
  const handleQuoteContentChange = useCallback((value: string) => {
    setQuoteForm(prev => ({ ...prev, content: value }));
    setTabGhostText('');

    if (tabDebounceRef.current) clearTimeout(tabDebounceRef.current);
    tabDebounceRef.current = setTimeout(() => {
      const suggestions = fetchQuoteSuggestions(value);
      if (suggestions.length > 0 && suggestions[0].toLowerCase().startsWith(value.toLowerCase())) {
        setTabGhostText(suggestions[0].slice(value.length));
      }
    }, 150);
  }, [fetchQuoteSuggestions, setQuoteForm]);

  // Tab 键处理
  const handleQuoteKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Tab' && tabGhostText) {
      e.preventDefault();
      const fullText = quoteForm.content + tabGhostText;
      setQuoteForm(prev => ({ ...prev, content: fullText }));
      setTabGhostText('');
    } else if (e.key === 'Escape') {
      setTabGhostText('');
    }
  }, [tabGhostText, quoteForm.content, setQuoteForm]);

  // 清理定时器
  useEffect(() => {
    return () => {
      if (tabDebounceRef.current) clearTimeout(tabDebounceRef.current);
    };
  }, []);

  return <div>
      {showDuplicateDialog && <div style={{
      position: 'fixed',
      inset: 0,
      background: 'rgba(0, 0, 0, 0.7)',
      zIndex: 1000,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      backdropFilter: 'blur(4px)'
    }}>
          <div style={{
        background: 'rgba(10, 2, 20, 0.97)',
        border: '1.5px solid rgba(255, 180, 0, 0.5)',
        borderRadius: 10,
        padding: '28px 32px',
        maxWidth: 520,
        width: '90%',
        boxShadow: '0 0 60px rgba(255, 180, 0, 0.25), 0 0 120px rgba(255, 180, 0, 0.1)'
      }}>
            <div style={{
          color: '#FFB400',
          fontSize: 18,
          fontFamily: 'inherit',
          marginBottom: 18,
          display: 'flex',
          alignItems: 'center',
          gap: 10
        }}>
              <span style={{
            fontSize: 22
          }}>⚠</span>
              <span>{t("profile.QuotePanel.k1")}</span>
            </div>
            <div style={{
          color: 'rgba(200, 200, 220, 0.7)',
          fontSize: 13,
          marginBottom: 16,
          lineHeight: 1.7
        }}>
              {t("profile.QuotePanel.k2")}
            </div>
            <div style={{
          maxHeight: 200,
          overflowY: 'auto',
          marginBottom: 20,
          display: 'flex',
          flexDirection: 'column',
          gap: 10
        }}>
              {duplicateResults.map((dup, i) => <div key={i} style={{
            background: 'rgba(255, 180, 0, 0.06)',
            border: '1px solid rgba(255, 180, 0, 0.2)',
            borderRadius: 6,
            padding: '10px 14px'
          }}>
                  <div style={{
              color: 'rgba(210, 210, 230, 0.85)',
              fontSize: 13,
              marginBottom: 4
            }}>
                    "{dup.existing.content}"
                  </div>
                  <div style={{
              display: 'flex',
              gap: 16,
              fontSize: 11
            }}>
                    {dup.existing.source && <span style={{
                color: 'rgba(150, 150, 170, 0.5)'
              }}>—— {dup.existing.source}</span>}
                    <span style={{
                color: dup.match_type === 'exact' ? '#FF5555' : '#FFB400',
                fontWeight: 600
              }}>
                      {dup.match_detail}
                    </span>
                  </div>
                </div>)}
            </div>
            <div style={{
          display: 'flex',
          gap: 10,
          justifyContent: 'flex-end',
          flexWrap: 'wrap'
        }}>
              <button onClick={handleDuplicateKeepExisting} style={{
            padding: '8px 18px',
            background: 'rgba(100, 100, 120, 0.15)',
            border: '1px solid rgba(100, 100, 120, 0.3)',
            color: 'rgba(180, 180, 200, 0.7)',
            borderRadius: 6,
            cursor: 'pointer',
            fontSize: 13,
            fontFamily: 'inherit',
            transition: 'all 0.2s'
          }} onMouseEnter={e => {
            e.currentTarget.style.background = 'rgba(100, 100, 120, 0.25)';
            e.currentTarget.style.borderColor = 'rgba(180, 180, 200, 0.5)';
          }} onMouseLeave={e => {
            e.currentTarget.style.background = 'rgba(100, 100, 120, 0.15)';
            e.currentTarget.style.borderColor = 'rgba(100, 100, 120, 0.3)';
          }}>
                {t("profile.QuotePanel.k3")}
              </button>
              <button onClick={handleDuplicateReplaceAll} disabled={isLoading} style={{
            padding: '8px 18px',
            background: 'rgba(255, 180, 0, 0.12)',
            border: '1px solid rgba(255, 180, 0, 0.4)',
            color: '#FFB400',
            borderRadius: 6,
            cursor: isLoading ? 'not-allowed' : 'pointer',
            fontSize: 13,
            fontFamily: 'inherit',
            opacity: isLoading ? 0.5 : 1,
            transition: 'all 0.2s'
          }} onMouseEnter={e => {
            if (!isLoading) {
              e.currentTarget.style.background = 'rgba(255, 180, 0, 0.22)';
              e.currentTarget.style.boxShadow = '0 0 14px rgba(255, 180, 0, 0.35)';
            }
          }} onMouseLeave={e => {
            e.currentTarget.style.background = 'rgba(255, 180, 0, 0.12)';
            e.currentTarget.style.boxShadow = 'none';
          }}>
                {isLoading ? t("profile.QuotePanel.k4") : t("profile.QuotePanel.k5")}
              </button>
              <button onClick={handleDuplicateAddAnyway} disabled={isLoading} style={{
            padding: '8px 18px',
            background: 'rgba(0, 240, 255, 0.12)',
            border: '1px solid rgba(0, 240, 255, 0.4)',
            color: '#00F0FF',
            borderRadius: 6,
            cursor: isLoading ? 'not-allowed' : 'pointer',
            fontSize: 13,
            fontFamily: 'inherit',
            opacity: isLoading ? 0.5 : 1,
            transition: 'all 0.2s'
          }} onMouseEnter={e => {
            if (!isLoading) {
              e.currentTarget.style.background = 'rgba(0, 240, 255, 0.22)';
              e.currentTarget.style.boxShadow = '0 0 14px rgba(0, 240, 255, 0.35)';
            }
          }} onMouseLeave={e => {
            e.currentTarget.style.background = 'rgba(0, 240, 255, 0.12)';
            e.currentTarget.style.boxShadow = 'none';
          }}>
                {t("profile.QuotePanel.k6")}
              </button>
            </div>
          </div>
        </div>}
      {showAddQuote && <div className={styles.addSourceForm} style={{
      marginBottom: 16
    }}>
          <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
            <label className={styles.formLabel}>{t("common.content")}</label>
            <div style={{ position: 'relative' }}>
              <textarea
                ref={quoteContentRef}
                value={quoteForm.content}
                onChange={e => handleQuoteContentChange(e.target.value)}
                onKeyDown={handleQuoteKeyDown}
                placeholder={t("profile.QuotePanel.k7")}
                className={styles.formInput}
                rows={3}
                style={{ resize: 'vertical' }}
              />
              {tabGhostText && (
                <div style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  right: 0,
                  padding: '8px 12px',
                  pointerEvents: 'none',
                  color: 'rgba(0, 240, 255, 0.4)',
                  fontSize: '13px',
                  whiteSpace: 'pre-wrap',
                  wordBreak: 'break-word'
                }}>
                  <span style={{ visibility: 'hidden' }}>{quoteForm.content}</span>
                  <span>{tabGhostText}</span>
                </div>
              )}
            </div>
          </div>
          <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
            <label className={styles.formLabel}>{t("profile.QuotePanel.k8")}</label>
            <input type="text" value={quoteForm.source} onChange={e => setQuoteForm(prev => ({
          ...prev,
          source: e.target.value
        }))} placeholder={t("profile.QuotePanel.k9")} className={styles.formInput} />
          </div>
          <div style={{
        display: 'flex',
        gap: 10
      }}>
            {aiOn && featureOn('smart_complete') && <>
                <button onClick={handleQuoteSpellCheck} disabled={quoteCheckLoading || !quoteForm.content.trim()} style={{
            padding: '6px 12px',
            fontSize: 12,
            borderRadius: 4,
            border: '1px solid rgba(0,240,255,0.3)',
            background: 'rgba(0,240,255,0.08)',
            color: '#00F0FF',
            cursor: 'pointer'
          }}>
                  {quoteCheckLoading ? '⏳' : '🔍'} {t("profile.QuotePanel.k10")}
                </button>
                <button onClick={handleQuoteSourceVerify} disabled={quoteCheckLoading || !quoteForm.source.trim()} style={{
            padding: '6px 12px',
            fontSize: 12,
            borderRadius: 4,
            border: '1px solid rgba(176,38,255,0.3)',
            background: 'rgba(176,38,255,0.08)',
            color: '#B026FF',
            cursor: 'pointer'
          }}>
                  {t("profile.QuotePanel.k11")}
                </button>
                <button onClick={handleQuoteSmartComplete} disabled={quoteCheckLoading || !quoteForm.content.trim()} style={{
            padding: '6px 12px',
            fontSize: 12,
            borderRadius: 4,
            border: '1px solid rgba(0,240,255,0.3)',
            background: 'rgba(0,240,255,0.08)',
            color: '#00F0FF',
            cursor: 'pointer'
          }}>
                  {t("components.intelligence.SettingsPanel.k10")}
                </button>
              </>}
            <button className={styles.primaryButton} onClick={handleAddQuote} disabled={isLoading} style={{
          padding: '8px 20px',
          fontSize: 13
        }}>
              {isLoading ? t("components.AudioEditor.k6") : t("profile.QuotePanel.k12")}
            </button>
            <button className={styles.dangerButton} onClick={() => {
          setShowAddQuote(false);
          setQuoteForm({
            content: '',
            source: ''
          });
          setQuoteGhostText('');
        }} style={{
          padding: '8px 20px',
          fontSize: 13
        }}>
              {t("common.cancel")}
            </button>
          </div>
          {quoteCheckResult && <div className={styles.aiGhostText} style={{
        marginTop: 8,
        borderColor: quoteCheckResult.severity === 'error' ? 'rgba(255,80,80,0.3)' : 'rgba(0,240,255,0.2)'
      }}>
              <span style={{
          fontSize: 12,
          color: quoteCheckResult.severity === 'error' ? '#FF5050' : '#00F0FF',
          marginRight: 8
        }}>
                {quoteCheckResult.field}: {quoteCheckResult.issue}
              </span>
              {quoteCheckResult.suggestion && <span style={{
          fontSize: 12,
          color: 'rgba(255,255,255,0.6)'
        }}>{t("profile.QuotePanel.k13")} {quoteCheckResult.suggestion}</span>}
              <button style={{
          marginLeft: 8,
          background: 'none',
          border: 'none',
          color: 'rgba(255,255,255,0.4)',
          cursor: 'pointer',
          fontSize: 12
        }} onClick={() => setQuoteCheckResult(null)}>✕</button>
            </div>}
          {ghostLoading && <div className={styles.aiChecking} style={{
        marginTop: 8
      }}>{t("profile.QuotePanel.k14")}</div>}
          {quoteGhostText && <div className={styles.aiGhostText}>
              <span className={styles.aiGhostLabel}>{t("profile.QuotePanel.k15")}</span>
              <span className={styles.aiGhostContent}>{quoteGhostText}</span>
              <button className={styles.aiGhostApply} onClick={() => {
          setQuoteForm(prev => ({
            ...prev,
            content: prev.content + quoteGhostText
          }));
          setQuoteGhostText('');
        }}>↩</button>
            </div>}
        </div>}

      {quotesLoading && <div className={styles.infoContent}>{t("profile.QuotePanel.k16")}</div>}

      {!quotesLoading && quotes.length > 0 && <div>
          {quotes.map(q => <div key={q.id} className={styles.quoteCard}>
              <div className={styles.quoteCardHeader}>
                <div className={styles.quoteContent}>"{q.content}"</div>
                <div style={{
            display: 'flex',
            gap: 6
          }}>
                  <button className={styles.quoteDeleteBtn} onClick={() => handleDeleteQuote(q.id)} title={t("profile.QuotePanel.k17")}>
                    ✕
                  </button>
                </div>
              </div>
              {q.source && <div className={styles.quoteSource}>—— {q.source}</div>}
            </div>)}
        </div>}

      {!quotesLoading && quotes.length === 0 && !showAddQuote && <div style={{
      textAlign: 'center',
      padding: '28px 0'
    }}>
          <div className={styles.infoContent} style={{
        marginBottom: 14
      }}>{t("profile.QuotePanel.k18")}</div>
        </div>}
    </div>;
}