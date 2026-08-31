import { t } from "i18next";
import React, { createContext, useContext, useState, ReactNode } from 'react';
export interface LongCountdownItem {
  id: string;
  name: string;
  shortName: string;
  targetDate: string;
  isActive: boolean;
}
interface TimerContextType {
  isTiming: boolean;
  setIsTiming: (timing: boolean) => void;
  timerData: {
    time: string;
    days: string;
  };
  setTimerData: (data: {
    time: string;
    days: string;
  }) => void;
  longCountdowns: LongCountdownItem[];
  addLongCountdown: (item: Omit<LongCountdownItem, 'id'>) => void;
  removeLongCountdown: (id: string) => void;
  toggleLongCountdown: (id: string) => void;
  updateLongCountdown: (id: string, data: Pick<LongCountdownItem, 'name' | 'shortName' | 'targetDate'>) => void;
  activeCountdown: LongCountdownItem | null;
}
const TimerContext = createContext<TimerContextType | undefined>(undefined);
export const TimerProvider: React.FC<{
  children: ReactNode;
}> = ({
  children
}) => {
  const [isTiming, setIsTiming] = useState(false);
  const [timerData, setTimerData] = useState({
    time: '00:00:00',
    days: t("components.NexTermTimer.k3")
  });
  const [longCountdowns, setLongCountdowns] = useState<LongCountdownItem[]>(() => {
    try {
      const saved = localStorage.getItem('nt_long_countdowns');
      return saved ? JSON.parse(saved) : [];
    } catch {
      return [];
    }
  });
  const addLongCountdown = (item: Omit<LongCountdownItem, 'id'>) => {
    const newItem = {
      ...item,
      id: Date.now().toString()
    };
    setLongCountdowns(prev => {
      const updated = [...prev, newItem];
      localStorage.setItem('nt_long_countdowns', JSON.stringify(updated));
      return updated;
    });
  };
  const removeLongCountdown = (id: string) => {
    setLongCountdowns(prev => {
      const updated = prev.filter(item => item.id !== id);
      localStorage.setItem('nt_long_countdowns', JSON.stringify(updated));
      return updated;
    });
  };
  const toggleLongCountdown = (id: string) => {
    setLongCountdowns(prev => {
      const updated = prev.map(item => item.id === id ? {
        ...item,
        isActive: !item.isActive
      } : item);
      localStorage.setItem('nt_long_countdowns', JSON.stringify(updated));
      return updated;
    });
  };
  const updateLongCountdown = (id: string, data: Pick<LongCountdownItem, 'name' | 'shortName' | 'targetDate'>) => {
    setLongCountdowns(prev => {
      const updated = prev.map(item => item.id === id ? { ...item, ...data } : item);
      localStorage.setItem('nt_long_countdowns', JSON.stringify(updated));
      return updated;
    });
  };
  const activeCountdown = longCountdowns.find(c => c.isActive) || null;
  return <TimerContext.Provider value={{
    isTiming,
    setIsTiming,
    timerData,
    setTimerData,
    longCountdowns,
    addLongCountdown,
    removeLongCountdown,
    toggleLongCountdown,
    updateLongCountdown,
    activeCountdown
  }}>
      {children}
    </TimerContext.Provider>;
};
export const useTimer = () => {
  const context = useContext(TimerContext);
  if (context === undefined) {
    throw new Error('useTimer must be used within a TimerProvider');
  }
  return context;
};