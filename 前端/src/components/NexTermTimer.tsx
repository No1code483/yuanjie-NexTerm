import { t } from "i18next";
import { useState, useEffect } from 'react';
import { useTimer } from '@/contexts/TimerContext';
import styles from './NexTermTimer.module.css';
interface NexTermTimerProps {
  type: 'countdown' | 'stopwatch';
  duration?: number;
  onComplete?: () => void;
  format?: 'HH:MM:SS' | 'DD:HH:MM:SS';
  autoStart?: boolean;
}
export default function NexTermTimer({
  type,
  duration = 0,
  onComplete,
  format = 'HH:MM:SS',
  autoStart = false
}: NexTermTimerProps) {
  const [time, setTime] = useState(duration);
  const [isRunning, setIsRunning] = useState(autoStart);
  const {
    setIsTiming,
    setTimerData
  } = useTimer();
  useEffect(() => {
    let interval: ReturnType<typeof setInterval> | undefined;
    if (isRunning) {
      setIsTiming(true);
      interval = setInterval(() => {
        setTime(prev => {
          const newTime = type === 'countdown' ? prev - 1000 : prev + 1000;
          if (type === 'countdown' && newTime <= 0) {
            setIsRunning(false);
            setIsTiming(false);
            if (onComplete) onComplete();
            return 0;
          }
          return newTime;
        });
      }, 1000);
    } else {
      setIsTiming(false);
    }
    return () => clearInterval(interval);
  }, [isRunning, type, onComplete, setIsTiming]);
  const formatTime = (ms: number) => {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    const remainingSeconds = String(seconds % 60).padStart(2, '0');
    const remainingMinutes = String(minutes % 60).padStart(2, '0');
    const remainingHours = String(hours % 24).padStart(2, '0');
    if (format === 'DD:HH:MM:SS' && days > 0) {
      return t("components.NexTermTimer.k1", {
        days: days,
        remainingHours: remainingHours,
        remainingMinutes: remainingMinutes,
        remainingSeconds: remainingSeconds
      });
    }
    return `${remainingHours}:${remainingMinutes}:${remainingSeconds}`;
  };

  // 更新计时器数据到全局状态
  useEffect(() => {
    if (isRunning) {
      const formattedTime = formatTime(time);
      const days = Math.floor(Math.floor(time / 1000) / 86400);
      setTimerData({
        time: formattedTime,
        days: days > 0 ? t("components.NexTermTimer.k2", {
          days: days
        }) : t("components.NexTermTimer.k3")
      });
    }
  }, [time, isRunning, setTimerData]);
  const start = () => setIsRunning(true);
  const pause = () => setIsRunning(false);
  const reset = () => {
    setIsRunning(false);
    setTime(duration);
  };
  const restart = () => {
    setTime(duration);
    setIsRunning(true);
  };
  return <div className={styles.timerContainer}>
      <div className={styles.timeDisplay}>
        {formatTime(time)}
      </div>
      <div className={styles.buttonContainer}>
        {!isRunning ? <button className={styles.timerButton} onClick={start}>
            {t("common.start")}
          </button> : <button className={styles.timerButton} onClick={pause}>
            {t("common.pause")}
          </button>}
        <button className={styles.timerButton} onClick={reset}>
          {t("common.reset")}
        </button>
        <button className={styles.timerButton} onClick={restart}>
          {t("components.NexTermTimer.k4")}
        </button>
      </div>
    </div>;
}