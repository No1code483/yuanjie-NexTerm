// xin.search L2 —— 对话搜索功能域（局部 state + handlers + JSX）。点击结果跳 chat 走 core.goToTab。
import { t } from "i18next";
import { useState } from 'react';
import { xinOrchestration } from '../../ipc';
import { time } from '@/lib/utils';
import styles from '../../Xin.module.css';
import type { SearchResult } from '../../xin/types';
import type { XinCore } from '../../core';

export function useSearch(core: XinCore, _active: boolean) {
  const { switchConversation, goToTab } = core;
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [searchTotal, setSearchTotal] = useState(0);
  const [searchLoading, setSearchLoading] = useState(false);

  const doSearch = async () => {
    if (!searchQuery.trim()) return;
    setSearchLoading(true);
    try {
      const res = await xinOrchestration.dialogueSearch(searchQuery, 20);
      if (res?.data) {
        setSearchResults(res.data);
        setSearchTotal(res.data.length);
      }
    } catch {/* silent */} finally {
      setSearchLoading(false);
    }
  };

  const renderSearch = () => <div className={styles.tabContent}>
      <div className={styles.searchRow}>
        <input className={styles.input} placeholder={t("Xin.k121")} value={searchQuery} onChange={e => setSearchQuery(e.target.value)} onKeyDown={e => {
        if (e.key === 'Enter') doSearch();
      }} style={{
        flex: '1'
      }} />
        <button className={styles.btn} onClick={doSearch} disabled={searchLoading}>
          {searchLoading ? t("ai.ConversationList.k4") : t("Xin.k122")}
        </button>
      </div>

      {searchLoading && <div className={styles.loading}>{t("ai.ConversationList.k4")}</div>}

      {!searchLoading && searchResults.length === 0 && searchQuery && <div style={{
      textAlign: 'center',
      padding: '20px',
      color: '#6a6a8a'
    }}>{t("components.Linux.k34")}</div>}

      {searchTotal > 0 && <div style={{
      fontSize: '10px',
      color: '#6a6a8a',
      marginBottom: '8px'
    }}>
          {t("components.GroupChatOrchestrationPanel.k26")} {searchTotal} {t("Xin.k123")}
        </div>}

      {searchResults.map(r => <div key={r.conversation_id} className={styles.card} style={{
      marginBottom: '8px'
    }}>
          <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        marginBottom: '4px'
      }}>
            <span style={{
          fontSize: '12px',
          color: '#00F0FF'
        }}>{r.title}</span>
            <span style={{
          fontSize: '10px',
          color: '#FFD700'
        }}>
              {t("Xin.k124")} {(r.relevance_score * 100).toFixed(0)}%
            </span>
          </div>
          <div style={{
        fontSize: '10px',
        color: '#6a6a8a',
        marginBottom: '6px'
      }}>
            {r.message_count}{t("Xin.k125")} {r.total_tokens} tokens | {time.formatCompact(r.created_at)}
          </div>
          {r.matched_snippets.slice(0, 3).map((snippet, i) => <div key={i} className={styles.searchSnippet}>
              <span>...</span><span>{snippet}</span><span>...</span>
            </div>)}
          <button className={styles.btnSmall} style={{
        marginTop: '6px'
      }} onClick={() => {
        switchConversation(r.conversation_id);
        goToTab('chat');
      }}>
            {t("Xin.k126")}
          </button>
        </div>)}

      {!searchQuery && <div style={{
      textAlign: 'center',
      padding: '40px',
      color: '#6a6a8a'
    }}>
          <p style={{
        fontSize: '13px'
      }}>{t("Xin.k127")}</p>
          <p style={{
        fontSize: '11px'
      }}>{t("Xin.k128")}</p>
        </div>}
    </div>;

  return { renderSearch };
}
