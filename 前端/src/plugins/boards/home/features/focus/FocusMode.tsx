import { t } from "i18next";
import { useState, useEffect, useRef, useCallback } from 'react';
import styles from './FocusMode.module.css';
interface FocusSession {
  id: string;
  date: string;
  durationSeconds: number;
  completed: boolean;
  taskName: string;
}
interface FocusHistory {
  sessions: FocusSession[];
  lastDate: string;
}
const HISTORY_KEY = 'nexterm_focus_history';
const DEFAULT_DURATION = 25 * 60; // 25 minutes in seconds

function loadHistory(): FocusHistory {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (raw) return JSON.parse(raw);
  } catch {/* ignore */}
  return {
    sessions: [],
    lastDate: ''
  };
}
function saveHistory(history: FocusHistory) {
  localStorage.setItem(HISTORY_KEY, JSON.stringify(history));
}
function getToday(): string {
  return new Date().toISOString().slice(0, 10);
}
interface FocusModeProps {
  onExit: () => void;
}
export default function FocusMode({
  onExit
}: FocusModeProps) {
  const [duration, setDuration] = useState(DEFAULT_DURATION);
  const [timeLeft, setTimeLeft] = useState(DEFAULT_DURATION);
  const [isRunning, setIsRunning] = useState(false);
  const [isPaused, setIsPaused] = useState(false);
  const [sessionCount, setSessionCount] = useState(0);
  const [breakCount, setBreakCount] = useState(0);
  const [taskName, setTaskName] = useState('');
  const [showSummary, setShowSummary] = useState(false);
  const [whiteNoiseOn, setWhiteNoiseOn] = useState(false);
  const [showDurationPicker, setShowDurationPicker] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const audioCtxRef = useRef<AudioContext | null>(null);
  const noiseNodeRef = useRef<AudioBufferSourceNode | null>(null);
  const sessionStartRef = useRef<number>(Date.now());
  const totalTimeRef = useRef<number>(0);

  // Keyboard shortcut: Esc to exit
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (showSummary) {
          onExit();
        } else {
          if (isRunning || isPaused) {
            stopSession();
          }
          setShowSummary(true);
        }
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [isRunning, isPaused, showSummary]);

  // Timer logic
  useEffect(() => {
    if (isRunning && !isPaused) {
      timerRef.current = setInterval(() => {
        setTimeLeft(prev => {
          if (prev <= 1) {
            completeSession();
            return 0;
          }
          return prev - 1;
        });
      }, 1000);
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [isRunning, isPaused]);

  // White noise
  const startWhiteNoise = useCallback(() => {
    try {
      const ctx = new AudioContext();
      audioCtxRef.current = ctx;
      const bufferSize = 2 * ctx.sampleRate;
      const buffer = ctx.createBuffer(1, bufferSize, ctx.sampleRate);
      const data = buffer.getChannelData(0);

      // Brown noise
      let lastOut = 0;
      for (let i = 0; i < bufferSize; i++) {
        const white = Math.random() * 2 - 1;
        lastOut = (lastOut + 0.02 * white) / 1.02;
        data[i] = lastOut * 3.5;
      }
      const source = ctx.createBufferSource();
      source.buffer = buffer;
      source.loop = true;
      source.connect(ctx.destination);
      source.start();
      noiseNodeRef.current = source;
      setWhiteNoiseOn(true);
    } catch {
      console.warn('Web Audio API not available');
    }
  }, []);
  const stopWhiteNoise = useCallback(() => {
    if (noiseNodeRef.current) {
      try {
        noiseNodeRef.current.stop();
      } catch {/* ignore */}
      noiseNodeRef.current = null;
    }
    if (audioCtxRef.current) {
      audioCtxRef.current.close().catch(() => {});
      audioCtxRef.current = null;
    }
    setWhiteNoiseOn(false);
  }, []);
  useEffect(() => {
    return () => {
      stopWhiteNoise();
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, []);
  const startSession = () => {
    setTimeLeft(duration);
    setIsRunning(true);
    setIsPaused(false);
    sessionStartRef.current = Date.now();
  };
  const togglePause = () => {
    if (isPaused) {
      setIsPaused(false);
    } else {
      setIsPaused(true);
      totalTimeRef.current += (Date.now() - sessionStartRef.current) / 1000;
    }
  };
  const stopSession = () => {
    setIsRunning(false);
    setIsPaused(false);
    if (timerRef.current) clearInterval(timerRef.current);
    const elapsed = totalTimeRef.current + (Date.now() - sessionStartRef.current) / 1000;
    if (elapsed > 30) {
      // Only count sessions longer than 30 seconds
      const history = loadHistory();
      history.sessions.push({
        id: Date.now().toString(),
        date: getToday(),
        durationSeconds: Math.floor(elapsed),
        completed: timeLeft <= 0,
        taskName: taskName || t("components.FocusMode.k1")
      });
      history.lastDate = getToday();
      saveHistory(history);
      setSessionCount(prev => prev + 1);
    }
    totalTimeRef.current = 0;
  };
  const completeSession = () => {
    setIsRunning(false);
    setIsPaused(false);
    if (timerRef.current) clearInterval(timerRef.current);
    const elapsed = duration;
    const history = loadHistory();
    history.sessions.push({
      id: Date.now().toString(),
      date: getToday(),
      durationSeconds: elapsed,
      completed: true,
      taskName: taskName || t("components.FocusMode.k1")
    });
    history.lastDate = getToday();
    saveHistory(history);
    setSessionCount(prev => prev + 1);
    totalTimeRef.current = 0;
    stopWhiteNoise();
  };
  const handleExit = () => {
    if (isRunning) stopSession();
    stopWhiteNoise();
    setShowSummary(true);
  };
  const formatTime = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  };

  // Compute summary stats
  const history = loadHistory();
  const todaySessions = history.sessions.filter(s => s.date === getToday());
  const totalTodaySeconds = todaySessions.reduce((sum, s) => sum + s.durationSeconds, 0);
  const todayCompleted = todaySessions.filter(s => s.completed).length;

  // Streak: consecutive days with focus sessions
  const getStreak = () => {
    const dates = new Set(history.sessions.map(s => s.date));
    let streak = 0;
    const today = new Date();
    for (let i = 0; i < 365; i++) {
      const d = new Date(today);
      d.setDate(d.getDate() - i);
      const ds = d.toISOString().slice(0, 10);
      if (dates.has(ds)) {
        streak++;
      } else {
        break;
      }
    }
    return streak;
  };

  // ASCII bar chart for today's sessions
  const renderBarChart = () => {
    if (todaySessions.length === 0) return null;
    const maxMin = Math.max(...todaySessions.map(s => s.durationSeconds / 60), 1);
    return <div className={styles.chartContainer}>
        <div className={styles.chartTitle}>{t("components.FocusMode.k2")}</div>
        {todaySessions.map(s => {
        const mins = s.durationSeconds / 60;
        const barW = Math.max(mins / maxMin * 100, 5);
        return <div key={s.id} className={styles.chartBar}>
              <span className={styles.chartLabel}>{s.taskName}</span>
              <div className={styles.chartBarTrack}>
                <div className={styles.chartBarFill} style={{
              width: `${barW}%`
            }} />
              </div>
              <span className={styles.chartValue}>{Math.floor(mins)}{t("components.FocusMode.k3")}</span>
            </div>;
      })}
      </div>;
  };
  if (showSummary) {
    return <div className={styles.overlay}>
        <div className={styles.summaryContainer}>
          <div className={styles.summaryTitle}>{t("components.FocusMode.k4")}</div>

          <div className={styles.summaryGrid}>
            <div className={styles.summaryItem}>
              <span className={styles.summaryValue}>{formatTime(totalTodaySeconds)}</span>
              <span className={styles.summaryLabel}>{t("components.FocusMode.k5")}</span>
            </div>
            <div className={styles.summaryItem}>
              <span className={styles.summaryValue}>{todayCompleted}</span>
              <span className={styles.summaryLabel}>{t("components.FocusMode.k6")}</span>
            </div>
            <div className={styles.summaryItem}>
              <span className={styles.summaryValue}>{todaySessions.length}</span>
              <span className={styles.summaryLabel}>{t("components.FocusMode.k7")}</span>
            </div>
            <div className={styles.summaryItem}>
              <span className={styles.summaryValue}>{breakCount}</span>
              <span className={styles.summaryLabel}>{t("components.FocusMode.k8")}</span>
            </div>
            <div className={styles.summaryItem}>
              <span className={styles.summaryValue}>{getStreak()} {t("components.FocusMode.k9")}</span>
              <span className={styles.summaryLabel}>{t("components.FocusMode.k10")}</span>
            </div>
            <div className={styles.summaryItem}>
              <span className={styles.summaryValue}>{sessionCount + todaySessions.filter(s => s.completed).length}</span>
              <span className={styles.summaryLabel}>{t("components.FocusMode.k11")}</span>
            </div>
          </div>

          {renderBarChart()}

          <div className={styles.summaryActions}>
            <button className={styles.summaryBtn} onClick={onExit}>
              {t("components.FocusMode.k12")}
            </button>
            <button className={styles.summaryBtnSecondary} onClick={() => {
            setShowSummary(false);
            setTimeLeft(duration);
            setSessionCount(0);
            setBreakCount(0);
          }}>
              {t("components.FocusMode.k13")}
            </button>
          </div>

          <div className={styles.summaryHint}>{t("components.FocusMode.k14")}</div>
        </div>
      </div>;
  }
  return <div className={styles.overlay}>
      <div className={styles.focusContainer}>
        {/* Header */}
        <div className={styles.focusHeader}>
          <span className={styles.focusHeaderTitle}>{t("components.FocusMode.k15")}</span>
          <button className={styles.focusExitBtn} onClick={handleExit}>
            {t("components.FocusMode.k16")}
          </button>
        </div>

        {/* Task input */}
        <div className={styles.taskInput}>
          <input type="text" className={styles.taskInputField} placeholder={t("components.FocusMode.k17")} value={taskName} onChange={e => setTaskName(e.target.value)} disabled={isRunning} />
        </div>

        {/* Timer display */}
        <div className={styles.timerSection}>
          <div className={styles.timerDisplay}>
            {formatTime(timeLeft)}
          </div>
          {!isRunning && !isPaused && <div className={styles.timerLabel}>
              {duration >= 3600 ? t("components.FocusMode.k18", {
            arg0: Math.floor(duration / 3600),
            arg1: Math.floor(duration % 3600 / 60)
          }) : t("components.FocusMode.k19", {
            arg0: Math.floor(duration / 60)
          })}
            </div>}
          {isPaused && <div className={styles.timerPaused}>{t("components.FocusMode.k20")}</div>}
        </div>

        {/* Controls */}
        <div className={styles.controls}>
          {!isRunning ? <button className={styles.controlBtn} onClick={startSession}>
              {t("components.FocusMode.k21")}
            </button> : <>
              <button className={styles.controlBtn} onClick={togglePause}>
                {isPaused ? t("components.FocusMode.k22") : t("components.FocusMode.k23")}
              </button>
              <button className={styles.controlBtnSecondary} onClick={stopSession}>
                {t("components.FocusMode.k24")}
              </button>
            </>}
        </div>

        {/* Duration picker */}
        <div className={styles.durationPicker}>
          <button className={styles.durationBtn} onClick={() => setShowDurationPicker(!showDurationPicker)}>
            {showDurationPicker ? t("common.collapse") : t("components.FocusMode.k25")}
          </button>
          {showDurationPicker && <div className={styles.durationOptions}>
              {[15, 25, 30, 45, 60, 90, 120].map(m => <button key={m} className={`${styles.durationOption} ${duration === m * 60 ? styles.durationActive : ''}`} onClick={() => {
            setDuration(m * 60);
            setTimeLeft(m * 60);
            setShowDurationPicker(false);
          }} disabled={isRunning}>
                  {m >= 60 ? `${m / 60}h` : t("components.FocusMode.k26", {
              m: m
            })}
                </button>)}
            </div>}
        </div>

        {/* White noise toggle */}
        <div className={styles.noiseToggle}>
          <button className={`${styles.noiseBtn} ${whiteNoiseOn ? styles.noiseBtnActive : ''}`} onClick={() => whiteNoiseOn ? stopWhiteNoise() : startWhiteNoise()}>
            {whiteNoiseOn ? t("components.FocusMode.k27") : t("components.FocusMode.k28")}
          </button>
        </div>

        {/* Session info */}
        {isRunning && <div className={styles.sessionInfo}>
            {taskName && <span>{t("components.FocusMode.k29")} {taskName}</span>}
          </div>}

        {/* Keyboard hints */}
        <div className={styles.keyboardHints}>
          <span>{t("components.FocusMode.k30")}</span>
          <span>{t("components.FocusMode.k31")}</span>
        </div>
      </div>
    </div>;
}