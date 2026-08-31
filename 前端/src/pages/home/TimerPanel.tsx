import { t } from "i18next";
import { useState, useEffect, useRef } from 'react';
import { useTimer, LongCountdownItem } from '@/contexts/TimerContext';
import { useIntelligence } from '@/hooks/useIntelligence';
import { intelligence } from '@/lib/ipc';
import { formatTimerTime, formatDateChinese } from './utils';
import styles from '../Home.module.css';
export default function TimerPanel() {
  const {
    addLongCountdown,
    longCountdowns,
    removeLongCountdown,
    updateLongCountdown
  } = useTimer();
  const {
    aiOn,
    featureOn
  } = useIntelligence();
  const [timerSeconds, setTimerSeconds] = useState(0);
  const [isTimerRunning, setIsTimerRunning] = useState(false);
  const [hasStarted, setHasStarted] = useState(false);
  const [timerMode, setTimerMode] = useState<'stopwatch' | 'countdown'>('stopwatch');
  const [countdownHours, setCountdownHours] = useState(0);
  const [countdownMinutes, setCountdownMinutes] = useState(0);
  const [countdownSeconds, setCountdownSeconds] = useState(0);
  const [showLongCountdown, setShowLongCountdown] = useState(false);
  const [newCountdown, setNewCountdown] = useState({
    name: '',
    shortName: '',
    targetDate: ''
  });
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editData, setEditData] = useState({
    name: '',
    shortName: '',
    targetDate: ''
  });
  const countdownDateRef = useRef<HTMLInputElement>(null);
  const editDateRef = useRef<HTMLInputElement>(null);
  const [aiTimerReminder, setAiTimerReminder] = useState('');
  const aiTimerCheckRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Task 5.4: AI 智能提醒开关
  const [aiReminderEnabled, setAiReminderEnabled] = useState(false);
  useEffect(() => {
    let timerInterval: number;
    if (isTimerRunning) {
      timerInterval = window.setInterval(() => {
        if (timerMode === 'stopwatch') {
          setTimerSeconds(prev => prev + 1);
        } else {
          setTimerSeconds(prev => {
            if (prev <= 1) {
              setIsTimerRunning(false);
              return 0;
            }
            return prev - 1;
          });
        }
      }, 1000);
    }
    return () => clearInterval(timerInterval);
  }, [isTimerRunning, timerMode]);
  useEffect(() => {
    if (!aiOn || !featureOn('smart_complete') || !aiReminderEnabled || !isTimerRunning || timerSeconds < 60) {
      setAiTimerReminder('');
      return;
    }
    const checkInterval = 5 * 60;
    if (timerSeconds % checkInterval === 0) {
      if (aiTimerCheckRef.current) clearTimeout(aiTimerCheckRef.current);
      aiTimerCheckRef.current = setTimeout(async () => {
        try {
          const res = await intelligence.timerRemind(timerSeconds, timerMode);
          if (res?.data) {
            const parts: string[] = [];
            if (res.data.suggest_break) parts.push(t("home.TimerPanel.k1"));
            if (res.data.suggest_stop) parts.push(t("home.TimerPanel.k2"));
            if (res.data.suggestions?.length) parts.push(res.data.suggestions.join('；'));
            if (parts.length) {
              setAiTimerReminder(t("home.TimerPanel.k3", {
                arg0: Math.floor(timerSeconds / 60),
                arg1: parts.join(' | '),
                optimal_session_minutes: res.data.optimal_session_minutes
              }));
            }
          }
        } catch {/* ignore */}
      }, 500);
    }
    return () => {
      if (aiTimerCheckRef.current) clearTimeout(aiTimerCheckRef.current);
    };
  }, [timerSeconds, isTimerRunning, aiOn, featureOn]);
  const handleTimerRemind = async () => {
    if (!isTimerRunning) return;
    setAiTimerReminder(t("home.TimerPanel.k4"));
    try {
      const res = await intelligence.timerRemind(timerSeconds, timerMode);
      if (res?.data) {
        const parts: string[] = [];
        if (res.data.suggest_break) parts.push(t("home.TimerPanel.k1"));
        if (res.data.suggest_stop) parts.push(t("home.TimerPanel.k2"));
        if (res.data.suggestions?.length) parts.push(res.data.suggestions.join('；'));
        setAiTimerReminder(t("home.TimerPanel.k5", {
          arg0: parts.join(' | '),
          optimal_session_minutes: res.data.optimal_session_minutes
        }));
      }
    } catch {
      setAiTimerReminder('');
    }
  };
  const startTimer = () => {
    if (timerMode === 'countdown' && timerSeconds === 0) {
      setTimerSeconds(countdownHours * 3600 + countdownMinutes * 60 + countdownSeconds);
    }
    setIsTimerRunning(true);
    setHasStarted(true); // BUG-005: 标记已启动
  };
  const pauseTimer = () => setIsTimerRunning(false);
  const resetTimer = () => {
    setIsTimerRunning(false);
    setTimerSeconds(0);
    setHasStarted(false); // BUG-005: 重置启动状态
  };
  const calculateRemainingDays = (targetDate: string) => {
    const target = new Date(targetDate);
    const diff = target.getTime() - Date.now();
    return Math.ceil(diff / (1000 * 60 * 60 * 24));
  };
  const handleAddLongCountdown = () => {
    if (newCountdown.name && newCountdown.shortName && newCountdown.targetDate) {
      addLongCountdown({
        ...newCountdown,
        isActive: true
      });
      setNewCountdown({
        name: '',
        shortName: '',
        targetDate: ''
      });
    }
  };
  const handleEditLongCountdown = (item: LongCountdownItem) => {
    setEditingId(item.id);
    setEditData({
      name: item.name,
      shortName: item.shortName,
      targetDate: item.targetDate
    });
  };
  const handleSaveEdit = () => {
    if (editingId && editData.name && editData.shortName && editData.targetDate) {
      updateLongCountdown(editingId, editData);
      setEditingId(null);
      setEditData({ name: '', shortName: '', targetDate: '' });
    }
  };
  const handleCancelEdit = () => {
    setEditingId(null);
    setEditData({ name: '', shortName: '', targetDate: '' });
  };
  if (showLongCountdown) {
    return <div className={styles.subpage}>
        <div className={styles.longCountdownSection}>
          <h3 className={styles.sectionTitle}>{t("home.TimerPanel.k6")}</h3>
          <div className={styles.addCountdownForm}>
            <input type="text" placeholder={t("home.TimerPanel.k7")} value={newCountdown.name} onChange={e => setNewCountdown(prev => ({
            ...prev,
            name: e.target.value
          }))} className={styles.countdownInput} />
            <input type="text" placeholder={t("home.TimerPanel.k8")} value={newCountdown.shortName} onChange={e => setNewCountdown(prev => ({
            ...prev,
            shortName: e.target.value
          }))} className={styles.countdownInput} />
            <input ref={countdownDateRef} type="date" value={newCountdown.targetDate} onChange={e => setNewCountdown(prev => ({
            ...prev,
            targetDate: e.target.value
          }))} className={styles.hiddenNativeInput} />
            <button className={styles.countdownInput} onClick={() => countdownDateRef.current?.showPicker()}>
              {newCountdown.targetDate ? formatDateChinese(newCountdown.targetDate) : t("home.TimerPanel.k9")}
            </button>
            <button className={styles.timerButton} onClick={handleAddLongCountdown}>{t("common.add")}</button>
          </div>
          {longCountdowns.length > 0 && <div className={styles.countdownList}>
              {longCountdowns.map(item => <div key={item.id} className={`${styles.countdownItem} ${item.isActive ? styles.active : ''}`}>
                  {editingId === item.id ? <div className={styles.editCountdownForm}>
                      <input type="text" placeholder={t("home.TimerPanel.k7")} value={editData.name} onChange={e => setEditData(prev => ({ ...prev, name: e.target.value }))} className={styles.countdownInput} />
                      <input type="text" placeholder={t("home.TimerPanel.k8")} value={editData.shortName} onChange={e => setEditData(prev => ({ ...prev, shortName: e.target.value }))} className={styles.countdownInput} />
                      <input ref={editDateRef} type="date" value={editData.targetDate} onChange={e => setEditData(prev => ({ ...prev, targetDate: e.target.value }))} className={styles.hiddenNativeInput} />
                      <button className={styles.countdownInput} onClick={() => editDateRef.current?.showPicker()}>
                        {editData.targetDate ? formatDateChinese(editData.targetDate) : t("home.TimerPanel.k9")}
                      </button>
                      <div className={styles.countdownActions}>
                        <button className={styles.smallButton} onClick={handleSaveEdit}>{t("common.save")}</button>
                        <button className={styles.smallButton} onClick={handleCancelEdit}>{t("common.cancel")}</button>
                      </div>
                    </div> : <>
                    <div className={styles.countdownInfo}>
                      <span className={styles.countdownName}>{item.name}</span>
                      <span className={styles.countdownShort}>{item.shortName}</span>
                      <span className={styles.countdownDate}>{formatDateChinese(item.targetDate)}</span>
                      <span className={styles.countdownDays}>{calculateRemainingDays(item.targetDate)}{t("components.FocusMode.k9")}</span>
                    </div>
                    <div className={styles.countdownActions}>
                      <button className={styles.smallButton} onClick={() => handleEditLongCountdown(item)}>{t("common.edit")}</button>
                      <button className={styles.smallButton} onClick={() => removeLongCountdown(item.id)}>{t("common.delete")}</button>
                    </div>
                  </>}
                </div>)}
            </div>}
          <button className={styles.backArrow} onClick={() => setShowLongCountdown(false)} title={t("home.TimerPanel.k10")}>▲</button>
        </div>
      </div>;
  }
  return <div className={styles.subpage}>
      <div className={styles.timerSection}>
        <div className={styles.timerModeSwitch}>
          <button className={`${styles.modeButton} ${timerMode === 'stopwatch' ? styles.activeMode : ''}`} onClick={() => {
          setTimerMode('stopwatch');
          resetTimer();
        }}>{t("home.TimerPanel.k11")}</button>
          <button className={`${styles.modeButton} ${timerMode === 'countdown' ? styles.activeMode : ''}`} onClick={() => {
          setTimerMode('countdown');
          resetTimer();
        }}>{t("home.TimerPanel.k12")}</button>
        </div>
        {aiTimerReminder && <div style={{
        textAlign: 'center',
        fontSize: 11,
        color: '#00F0FF',
        marginBottom: 8,
        padding: '6px 12px',
        background: 'rgba(0,240,255,0.06)',
        borderRadius: 4,
        border: '1px solid rgba(0,240,255,0.15)'
      }}>
            {aiTimerReminder}
            <button onClick={() => setAiTimerReminder('')} style={{
          marginLeft: 10,
          background: 'none',
          border: 'none',
          color: '#FF5050',
          cursor: 'pointer',
          fontSize: 11
        }}>✕</button>
          </div>}
        {timerMode === 'countdown' && !isTimerRunning && timerSeconds === 0 && <div className={styles.countdownPicker}>
            <div className={styles.pickerGroup}>
              <label>{t("home.TimerPanel.k15")}</label>
              <input type="number" min="0" max="23" value={countdownHours} onChange={e => setCountdownHours(Math.max(0, Math.min(23, parseInt(e.target.value) || 0)))} className={styles.pickerInput} />
            </div>
            <span className={styles.pickerSeparator}>:</span>
            <div className={styles.pickerGroup}>
              <label>{t("components.FocusMode.k3")}</label>
              <input type="number" min="0" max="59" value={countdownMinutes} onChange={e => setCountdownMinutes(Math.max(0, Math.min(59, parseInt(e.target.value) || 0)))} className={styles.pickerInput} />
            </div>
            <span className={styles.pickerSeparator}>:</span>
            <div className={styles.pickerGroup}>
              <label>{t("home.TimerPanel.k19")}</label>
              <input type="number" min="0" max="59" value={countdownSeconds} onChange={e => setCountdownSeconds(Math.max(0, Math.min(59, parseInt(e.target.value) || 0)))} className={styles.pickerInput} />
            </div>
          </div>}
        <div className={styles.timerDisplay}>{formatTimerTime(timerSeconds)}</div>
        <div className={styles.timerControls}>
          <button className={styles.timerButton} onClick={isTimerRunning ? pauseTimer : startTimer} disabled={timerMode === 'stopwatch' && timerSeconds >= 86400}>
            {isTimerRunning ? t("common.pause") : (hasStarted ? t("common.resume") : t("common.start"))}
          </button>
          <button className={styles.timerButton} onClick={resetTimer} disabled={timerSeconds === 0}>{t("common.reset")}</button>
          {aiOn && featureOn('smart_complete') && isTimerRunning && <button onClick={() => {
          const next = !aiReminderEnabled;
          setAiReminderEnabled(next);
          if (next) handleTimerRemind();else setAiTimerReminder('');
        }} style={{
          padding: '4px 10px',
          fontSize: 11,
          borderRadius: 4,
          fontFamily: 'monospace',
          border: aiReminderEnabled ? '1px solid #00FF00' : '1px solid rgba(255,0,0,0.3)',
          background: aiReminderEnabled ? '#000000' : 'transparent',
          color: aiReminderEnabled ? '#00FF00' : '#FF0000',
          cursor: 'pointer'
        }} title={aiReminderEnabled ? t("home.TimerPanel.k16") : t("home.TimerPanel.k17")}>
              {aiReminderEnabled ? '🧠 ON' : '🧠 OFF'}
            </button>}
        </div>
        <button className={styles.downArrow} onClick={() => setShowLongCountdown(true)} title={t("home.TimerPanel.k18")}>▼</button>
      </div>
    </div>;
}