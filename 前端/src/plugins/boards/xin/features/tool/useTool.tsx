// xin.tool L2 —— 工具调用 / 融合检索功能域（局部 state + handlers + JSX）。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xinOrchestration } from '../../ipc';
import styles from '../../Xin.module.css';
import type { ToolDef, ToolCall, ToolExecResult, FusionResult } from '../../xin/types';
import type { XinCore } from '../../core';

export function useTool(_core: XinCore, active: boolean) {
  const [tools, setTools] = useState<ToolDef[]>([]);
  const [toolInputText, setToolInputText] = useState('');
  const [parsedCalls, setParsedCalls] = useState<ToolCall[]>([]);
  const [toolResults, setToolResults] = useState<ToolExecResult[]>([]);
  const [toolExecLoading, setToolExecLoading] = useState(false);
  const [fusionQuery, setFusionQuery] = useState('');
  const [fusionResults, setFusionResults] = useState<FusionResult[]>([]);
  const [fusionKeywords, setFusionKeywords] = useState<string[]>([]);
  const [fusionLoading, setFusionLoading] = useState(false);

  const loadTools = async () => {
    try {
      const res = await xinOrchestration.listTools();
      if (res?.data) setTools(Array.isArray(res.data) ? res.data : []);
    } catch {/* silent */}
  };
  const parseToolCalls = async () => {
    if (!toolInputText.trim()) return;
    try {
      const res = await xinOrchestration.parseToolCalls(toolInputText);
      if (res?.data) setParsedCalls(Array.isArray(res.data) ? res.data : []);
    } catch {/* silent */}
  };
  const executeToolCalls = async () => {
    if (parsedCalls.length === 0) return;
    setToolExecLoading(true);
    try {
      const res = await xinOrchestration.executeTool(JSON.stringify(parsedCalls));
      if (res?.data) setToolResults(Array.isArray(res.data) ? res.data : []);
    } catch {/* silent */} finally {
      setToolExecLoading(false);
    }
  };
  const doFusionQuery = async () => {
    if (!fusionQuery.trim()) return;
    setFusionLoading(true);
    try {
      const [memoryRes, keywordRes] = await Promise.all([xinOrchestration.fusionMemoryQuery(fusionQuery, 10), xinOrchestration.fusionExtractKeywords(fusionQuery)]);
      if (memoryRes?.data) setFusionResults(Array.isArray(memoryRes.data) ? memoryRes.data : []);
      if (keywordRes?.data) setFusionKeywords(keywordRes.data);
    } catch {/* silent */} finally {
      setFusionLoading(false);
    }
  };

  useEffect(() => {
    if (active && tools.length === 0) loadTools();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const renderTool = () => <div className={styles.tabContent}>
      {tools.length === 0 ? <div className={styles.loading}>{t("Xin.k165")}</div> : <>
          <div className={styles.card}>
            <div className={styles.panelTitle}>{t("Xin.k166")}</div>
            <div className={styles.toolGrid}>
              {tools.map((t, i) => <div key={i} className={styles.toolCard}>
                  <div style={{
              display: 'flex',
              justifyContent: 'space-between'
            }}>
                    <span style={{
                fontSize: '12px',
                color: '#00F0FF'
              }}>{t.name}</span>
                    <span className={styles.skillCategory}>{t.category}</span>
                  </div>
                  <p style={{
              fontSize: '10px',
              color: '#8a8aaa',
              margin: '4px 0'
            }}>{t.description}</p>
                  {t.parameters.length > 0 && <div style={{
              display: 'flex',
              gap: '4px',
              flexWrap: 'wrap'
            }}>
                      {t.parameters.map((p, j) => <span key={j} style={{
                fontSize: '8px',
                padding: '1px 5px',
                border: `1px solid ${p.required ? '#FFD700' : 'rgba(176,38,255,0.2)'}`,
                borderRadius: '2px',
                color: p.required ? '#FFD700' : '#6a6a8a'
              }}>
                          {p.name}:{p.param_type}{p.required ? '*' : ''}
                        </span>)}
                    </div>}
                </div>)}
            </div>
          </div>

          <div className={styles.card}>
            <div className={styles.panelTitle}>{t("Xin.k167")}</div>
            <div style={{
        display: 'flex',
        gap: '8px',
        marginBottom: '8px'
      }}>
              <textarea className={styles.input} placeholder={t("Xin.k168")} value={toolInputText} onChange={e => setToolInputText(e.target.value)} rows={3} style={{
          flex: '1',
          resize: 'vertical',
          fontFamily: 'inherit'
        }} />
            </div>
            <div style={{
        display: 'flex',
        gap: '8px'
      }}>
              <button className={styles.btn} onClick={parseToolCalls}>{t("Xin.k169")}</button>
              <button className={styles.btn} onClick={executeToolCalls} disabled={parsedCalls.length === 0 || toolExecLoading}>
                {toolExecLoading ? t("Xin.k159") : t("Xin.k170")}
              </button>
            </div>
            {parsedCalls.length > 0 && <div style={{
        marginTop: '8px'
      }}>
                <span style={{
          fontSize: '10px',
          color: '#6a6a8a'
        }}>{t("Xin.k171")} {parsedCalls.length} {t("Xin.k172")}</span>
                {parsedCalls.map((call, i) => <div key={i} style={{
          fontSize: '10px',
          color: '#c8c8dd',
          padding: '4px 8px',
          background: 'rgba(0,240,255,0.02)',
          border: '1px solid rgba(0,240,255,0.15)',
          borderRadius: '3px',
          marginTop: '4px'
        }}>
                    <span style={{
            color: '#00F0FF'
          }}>{call.tool_name}</span>
                    {Object.entries(call.arguments).map(([k, v]) => <span key={k} style={{
            marginLeft: '8px',
            fontSize: '9px',
            color: '#8a8aaa'
          }}>{k}={v}</span>)}
                  </div>)}
              </div>}
          </div>

          {toolResults.length > 0 && <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k161")}</div>
              {toolResults.map((r, i) => <div key={i} style={{
          marginBottom: '8px',
          padding: '8px',
          background: 'rgba(0,240,255,0.02)',
          border: `1px solid ${r.success ? 'rgba(0,240,255,0.15)' : 'rgba(255,0,110,0.15)'}`,
          borderRadius: '4px'
        }}>
                  <div style={{
            fontSize: '11px',
            marginBottom: '4px'
          }}>
                    <span style={{
              color: '#00F0FF'
            }}>{r.tool_name}</span>
                    <span style={{
              marginLeft: '12px',
              color: r.success ? '#00F0FF' : '#FF006E'
            }}>
                      {r.success ? '✓' : '✗'}
                    </span>
                    <span style={{
              marginLeft: '8px',
              fontSize: '9px',
              color: '#6a6a8a'
            }}>{r.duration_ms}ms</span>
                  </div>
                  {r.output && <pre className={styles.resultPre} style={{
            maxHeight: '120px'
          }}>{r.output.slice(0, 500)}</pre>}
                  {r.error && <div style={{
            fontSize: '10px',
            color: '#FF006E'
          }}>{r.error}</div>}
                </div>)}
            </div>}

          <div className={styles.card}>
            <div className={styles.panelTitle}>{t("Xin.k173")}</div>
            <div className={styles.searchRow}>
              <input className={styles.input} placeholder={t("Xin.k174")} value={fusionQuery} onChange={e => setFusionQuery(e.target.value)} onKeyDown={e => {
            if (e.key === 'Enter') doFusionQuery();
          }} style={{
            flex: '1'
          }} />
              <button className={styles.btn} onClick={doFusionQuery} disabled={fusionLoading}>
                {fusionLoading ? t("Xin.k175") : t("Xin.k176")}
              </button>
            </div>
            {fusionKeywords.length > 0 && <div style={{
          display: 'flex',
          gap: '4px',
          flexWrap: 'wrap',
          marginBottom: '8px'
        }}>
                <span style={{
            fontSize: '9px',
            color: '#6a6a8a'
          }}>{t("Xin.k177")} </span>
                {fusionKeywords.map((kw, i) => <span key={i} style={{
            fontSize: '9px',
            padding: '1px 6px',
            background: 'rgba(0,240,255,0.08)',
            borderRadius: '2px',
            color: '#00F0FF',
            border: '1px solid rgba(0,240,255,0.2)'
          }}>{kw}</span>)}
              </div>}
            {fusionResults.length > 0 && <div>
                {fusionResults.map((r, i) => <div key={i} style={{
            padding: '8px',
            marginBottom: '6px',
            background: 'rgba(0,240,255,0.02)',
            border: '1px solid rgba(0,240,255,0.15)',
            borderRadius: '4px'
          }}>
                    <div style={{
              display: 'flex',
              justifyContent: 'space-between',
              marginBottom: '4px'
            }}>
                      <span style={{
                fontSize: '10px',
                color: '#00F0FF'
              }}>{r.source}</span>
                      <span style={{
                fontSize: '9px',
                color: '#FFD700'
              }}>
                        score: {typeof r.score === 'number' ? r.score.toFixed(2) : r.score}
                      </span>
                    </div>
                    <p style={{
              fontSize: '10px',
              color: '#c8c8dd',
              lineHeight: '1.4'
            }}>{r.content.slice(0, 200)}</p>
                  </div>)}
              </div>}
          </div>
        </>}
    </div>;

  return { renderTool };
}
