import { t } from "i18next";
import { useState, useEffect, useRef, useCallback } from 'react';
import styles from '../YuanCode.module.css';
import { file } from '@/lib/utils';
const MAX_BUFFER_KB = 10;
interface RunPanelProps {
  visible: boolean;
  code: string;
  language: string;
  output: string;
  error: string;
  running: boolean;
  durationMs: number | null;
  truncated: boolean;
  onLanguageChange: (lang: string) => void;
  onRun: () => void;
  onCancel: () => void;
  onForceCancel: () => void;
  onSendStdin: (input: string) => void;
  onClose: () => void;
}
const SUPPORTED_LANGUAGES = [{
  value: 'python',
  label: 'Python'
}, {
  value: 'javascript',
  label: 'JavaScript'
}, {
  value: 'typescript',
  label: 'TypeScript'
}, {
  value: 'rust',
  label: 'Rust'
}, {
  value: 'go',
  label: 'Go'
}, {
  value: 'java',
  label: 'Java'
}, {
  value: 'sh',
  label: 'Shell'
}, {
  value: 'powershell',
  label: 'PowerShell'
}, {
  value: 'bat',
  label: 'Batch'
}];
export default function RunPanel({
  visible,
  code,
  language,
  output,
  error,
  running,
  durationMs,
  truncated,
  onLanguageChange,
  onRun,
  onCancel,
  onForceCancel,
  onSendStdin,
  onClose
}: RunPanelProps) {
  const [activeOutputTab, setActiveOutputTab] = useState<'output' | 'error'>('output');
  const [stdinValue, setStdinValue] = useState('');
  const [liveElapsed, setLiveElapsed] = useState(0);
  const [showForceKill, setShowForceKill] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const forceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(() => {
    if (running) {
      const start = Date.now();
      timerRef.current = setInterval(() => {
        setLiveElapsed(Date.now() - start);
      }, 100);
      forceTimerRef.current = setTimeout(() => {
        setShowForceKill(true);
      }, 5000);
    } else {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
      if (forceTimerRef.current) {
        clearTimeout(forceTimerRef.current);
        forceTimerRef.current = null;
      }
      setShowForceKill(false);
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
      if (forceTimerRef.current) clearTimeout(forceTimerRef.current);
    };
  }, [running]);
  const handleSendStdin = useCallback(() => {
    if (!stdinValue.trim()) return;
    onSendStdin(stdinValue);
    setStdinValue('');
  }, [stdinValue, onSendStdin]);
  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendStdin();
    }
  }, [handleSendStdin]);
  if (!visible) return null;
  const hasError = error && error.trim();
  const hasOutput = output && output.trim();
  const outputBytes = new TextEncoder().encode(output).length;
  const errorBytes = new TextEncoder().encode(error).length;
  const stdoutPct = Math.min(100, outputBytes / (MAX_BUFFER_KB * 1024) * 100);
  const stderrPct = Math.min(100, errorBytes / (MAX_BUFFER_KB * 1024) * 100);
  const elapsed = durationMs ?? liveElapsed;
  const elapsedStr = elapsed >= 1000 ? `${(elapsed / 1000).toFixed(1)}s` : `${elapsed}ms`;
  return <div className={styles.runPanel}>
      <div className={styles.runPanelHeader}>
        <div className={styles.runPanelTabs}>
          <div className={`${styles.runPanelTab} ${activeOutputTab === 'output' ? styles.runPanelTabActive : ''}`} onClick={() => setActiveOutputTab('output')}>
            {t("common.output")}
          </div>
          {hasError && <div className={`${styles.runPanelTab} ${activeOutputTab === 'error' ? styles.runPanelTabActive : ''}`} onClick={() => setActiveOutputTab('error')}>
              {t("common.error")}
            </div>}
        </div>
        <div className={styles.runPanelActions}>
          {running && <span className={`${styles.liveTimer} ${styles.liveTimerActive}`}>
              ⏱ {elapsedStr}
            </span>}
          <select className={styles.runPanelLang} value={language} onChange={e => onLanguageChange(e.target.value)} disabled={running}>
            {SUPPORTED_LANGUAGES.map(l => <option key={l.value} value={l.value}>{l.label}</option>)}
          </select>
          <button className={styles.runBtn} onClick={onRun} disabled={running || !code.trim()}>
            {running ? t("yuan-code.RunPanel.k1") : t("yuan-code.RunPanel.k2")}
          </button>
          {running && (showForceKill ? <div className={styles.cancelConfirm}>
                <span className={styles.cancelConfirmText}>{t("yuan-code.RunPanel.k3")}</span>
                <button className={styles.forceKillBtn} onClick={onForceCancel}>{t("yuan-code.RunPanel.k4")}</button>
              </div> : <button className={styles.cancelBtn} onClick={onCancel}>{t("yuan-code.RunPanel.k5")}</button>)}
          <button className={styles.closeRunBtn} onClick={onClose}>✕</button>
        </div>
      </div>

      {/* IO 管道可视化 */}
      {running && <div className={styles.ioPipes}>
          <span className={`${styles.ioPipeLabel} ${styles.ioPipeLabelStdin}`}>STDIN</span>
          <span className={styles.ioPipeArrow}>→</span>
          <span className={styles.ioStreamIndicator}>{t("yuan-code.RunPanel.k6")}</span>
          <span className={styles.ioPipeArrow}>→</span>
          <span className={`${styles.ioPipeLabel} ${styles.ioPipeLabelStdout}`}>STDOUT</span>
          <span className={`${styles.ioPipeLabel} ${styles.ioPipeLabelStderr}`}>STDERR</span>
        </div>}

      {/* STDIN 输入区 */}
      {running && <div className={styles.stdinArea}>
          <input className={styles.stdinInput} placeholder={t("yuan-code.RunPanel.k7")} value={stdinValue} onChange={e => setStdinValue(e.target.value)} onKeyDown={handleKeyDown} autoFocus />
          <button className={styles.stdinBtn} onClick={handleSendStdin} disabled={!stdinValue.trim()}>
            {t("components.FloatingBall.k64")}
          </button>
        </div>}

      <div className={styles.runOutput}>
        {running && <div style={{
        color: 'var(--nt-text-muted)',
        fontFamily: 'var(--nt-font-mono)',
        fontSize: 12
      }}>
            {t("yuan-code.RunPanel.k8")}
          </div>}
        {!running && activeOutputTab === 'output' && <pre style={{
        margin: 0,
        whiteSpace: 'pre-wrap'
      }}>
            {hasOutput ? output : t("yuan-code.RunPanel.k9")}
          </pre>}
        {!running && activeOutputTab === 'error' && hasError && <pre className={styles.runError} style={{
        margin: 0,
        whiteSpace: 'pre-wrap'
      }}>
            {error}
          </pre>}
        {!running && durationMs != null && <div className={styles.runMeta}>
            {t("yuan-code.RunPanel.k10")} {durationMs}ms
          </div>}
        {!running && truncated && <div className={styles.truncatedNotice}>
            {t("yuan-code.RunPanel.k11")} {MAX_BUFFER_KB}{t("yuan-code.RunPanel.k12")} {file.formatSize(MAX_BUFFER_KB * 1024)}
          </div>}
      </div>

      {/* IO 缓冲区计量 */}
      {!running && (hasOutput || hasError) && <div className={styles.ioPipes}>
          <span className={`${styles.ioPipeLabel} ${styles.ioPipeLabelStdout}`}>STDOUT</span>
          <div className={styles.ioBufferMeter}>
            <div className={styles.ioBufferBar}>
              <div className={`${styles.ioBufferFill} ${stdoutPct > 80 ? styles.ioBufferFillWarn : styles.ioBufferFillStdout}`} style={{
            width: `${stdoutPct}%`
          }} />
            </div>
            <span className={styles.ioBufferText}>
              {file.formatSize(outputBytes)} / {MAX_BUFFER_KB}KB
            </span>
          </div>

          {hasError && <>
              <span className={`${styles.ioPipeLabel} ${styles.ioPipeLabelStderr}`}>STDERR</span>
              <div className={styles.ioBufferMeter}>
                <div className={styles.ioBufferBar}>
                  <div className={`${styles.ioBufferFill} ${stderrPct > 80 ? styles.ioBufferFillWarn : styles.ioBufferFillStderr}`} style={{
              width: `${stderrPct}%`
            }} />
                </div>
                <span className={styles.ioBufferText}>
                  {file.formatSize(errorBytes)} / {MAX_BUFFER_KB}KB
                </span>
              </div>
            </>}
        </div>}
    </div>;
}