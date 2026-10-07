// xin.wellness L2 —— 健康/效率面板（productivity tab：提醒 / 习惯 / 番茄钟）。
import { t } from "i18next";
import { useState, useEffect, useRef } from 'react';
import styles from '../../Xin.module.css';
import type { Reminder, Habit } from '../../xin/types';
import type { XinCore } from '../../core';

export function useWellness(_core: XinCore, active: boolean) {
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [habits, setHabits] = useState<Habit[]>([]);
  const [pomodoroRunning, setPomodoroRunning] = useState(false);
  const [pomodoroTime, setPomodoroTime] = useState(25 * 60);
  const [pomodoroTask, setPomodoroTask] = useState('');
  const pomodoroRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const loadProductivity = () => {
    setReminders([{
      id: 'r1',
      title: t("Xin.k8"),
      description: t("Xin.k9"),
      time: t("Xin.k10"),
      repeat: 'work',
      active: true
    }, {
      id: 'r2',
      title: t("Xin.k11"),
      description: t("Xin.k12"),
      time: t("Xin.k13"),
      repeat: 'hourly',
      active: true
    }, {
      id: 'r3',
      title: t("Xin.k14"),
      description: t("Xin.k15"),
      time: t("Xin.k16"),
      repeat: 'daily',
      active: false
    }]);
    setHabits([{
      id: 'h1',
      name: t("Xin.k17"),
      category: t("home.utils.k6"),
      streak: 12,
      total: 45,
      checked: true
    }, {
      id: 'h2',
      name: t("Xin.k18"),
      category: t("Xin.k19"),
      streak: 5,
      total: 30,
      checked: false
    }, {
      id: 'h3',
      name: t("Xin.k20"),
      category: t("layout.k20"),
      streak: 8,
      total: 22,
      checked: true
    }]);
  };

  useEffect(() => {
    if (active && reminders.length === 0) loadProductivity();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  useEffect(() => {
    return () => {
      if (pomodoroRef.current) clearInterval(pomodoroRef.current);
    };
  }, []);

  const togglePomodoro = () => {
    if (pomodoroRunning) {
      if (pomodoroRef.current) clearInterval(pomodoroRef.current);
      pomodoroRef.current = null;
      setPomodoroRunning(false);
    } else {
      if (!pomodoroTask.trim()) {
        setPomodoroTask(t("components.FloatingBall.k26"));
      }
      setPomodoroRunning(true);
      pomodoroRef.current = setInterval(() => {
        setPomodoroTime(prev => {
          if (prev <= 1) return 0;
          return prev - 1;
        });
      }, 1000);
    }
  };
  const resetPomodoro = () => {
    if (pomodoroRef.current) clearInterval(pomodoroRef.current);
    pomodoroRef.current = null;
    setPomodoroRunning(false);
    setPomodoroTime(25 * 60);
  };
  const formatPomodoro = (seconds: number) => {
    const m = Math.floor(seconds / 60);
    const s = seconds % 60;
    return `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  };

  const renderWellness = () => <div className={styles.tabContent}>
      {/* Pomodoro */}
      <div className={styles.card}>
        <div className={styles.panelTitle}>{t("Xin.k48")}</div>
        {pomodoroRunning ? <div className={styles.pomodoroActive}>
            <div className={styles.pomodoroDisplay}>
              <span className={styles.pomodoroTask}>{pomodoroTask}</span>
              <span className={styles.pomodoroTime}>{formatPomodoro(pomodoroTime)}</span>
            </div>
            <button className={styles.btn} onClick={togglePomodoro}>{t("common.pause")}</button>
            <button className={styles.btnSmall} onClick={resetPomodoro}>{t("common.reset")}</button>
          </div> : <div className={styles.pomodoroSetup}>
            <div className={styles.addRow}>
              <span className={styles.inputLabel}>{t("Xin.k49")}</span>
              <input className={styles.input} value={pomodoroTask} onChange={e => setPomodoroTask(e.target.value)} placeholder={t("Xin.k50")} />
              <button className={styles.btn} onClick={togglePomodoro}>{t("common.start")}</button>
            </div>
          </div>}
      </div>

      {/* Reminders */}
      <div className={styles.card}>
        <div className={styles.panelTitle}>{t("Xin.k51")}</div>
        <div className={styles.list}>
          {reminders.map(r => <div key={r.id} className={`${styles.reminderItem} ${!r.active ? styles.reminderInactive : ''}`}>
              <div className={styles.reminderInfo}>
                <span className={styles.reminderTitle}>{r.title}</span>
                <span className={styles.reminderDesc}>{r.description}</span>
                <span className={styles.reminderMeta}>{r.time}</span>
              </div>
            </div>)}
        </div>
      </div>

      {/* Habits */}
      <div className={styles.card}>
        <div className={styles.panelTitle}>{t("Xin.k52")}</div>
        <div className={styles.list}>
          {habits.map(h => <div key={h.id} className={styles.habitItem}>
              <div className={styles.habitInfo}>
                <span className={styles.habitName}>{h.name}</span>
                <span className={styles.habitCategory}>{h.category}</span>
              </div>
              <div className={styles.habitStats}>
                <span className={styles.habitStreak}>🔥 {h.streak}{t("components.FocusMode.k9")}</span>
                <span className={styles.habitTotal}>{t("components.GroupChatOrchestrationPanel.k26")}{h.total}{t("profile.StatsDashboard.k3")}</span>
                <span className={styles.checkedBadge}>{h.checked ? '✅' : '⬜'}</span>
              </div>
            </div>)}
        </div>
      </div>
    </div>;

  return { renderWellness };
}
