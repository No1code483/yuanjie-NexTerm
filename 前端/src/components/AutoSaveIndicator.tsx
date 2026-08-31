import { t } from "i18next";
import React, { useState, useEffect } from 'react';
import { time } from '@/lib/utils';

/**
 * 自动保存状态指示器
 */
interface AutoSaveIndicatorProps {
  isSaving?: boolean;
  lastSaved?: Date;
  className?: string;
  style?: React.CSSProperties;
}
export const AutoSaveIndicator: React.FC<AutoSaveIndicatorProps> = ({
  isSaving = false,
  lastSaved,
  className = '',
  style
}) => {
  const [showSaved, setShowSaved] = useState(false);

  // 显示"已保存"提示的效果
  useEffect(() => {
    if (lastSaved && !isSaving) {
      setShowSaved(true);
      const timer = setTimeout(() => setShowSaved(false), 2000);
      return () => clearTimeout(timer);
    }
  }, [lastSaved, isSaving]);
  const getStatusText = () => {
    if (isSaving) return t("components.AudioEditor.k6");
    if (showSaved) return t("components.AutoSaveIndicator.k1");
    return '';
  };
  const getStatusColor = () => {
    if (isSaving) return '#00F0FF';
    if (showSaved) return '#FF006E';
    return 'rgba(106, 106, 138, 0.65)';
  };
  if (!isSaving && !showSaved) {
    return null;
  }
  return <div className={`nt-auto-save-indicator ${className}`} style={{
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    fontSize: '12px',
    color: getStatusColor(),
    transition: 'all 0.3s ease',
    ...style
  }}>
      <div className="nt-auto-save-dot" style={{
      width: '6px',
      height: '6px',
      borderRadius: '50%',
      backgroundColor: getStatusColor(),
      animation: isSaving ? 'pulse 1.5s infinite' : 'none'
    }} />
      <span>{getStatusText()}</span>
      {lastSaved && !isSaving && <span style={{
      fontSize: '10px',
      opacity: 0.7
    }}>
          {time.timeAgo(new Date(lastSaved).toISOString())}
        </span>}
    </div>;
};

/**
 * 带自动保存指示器的输入框组件
 */
interface AutoSaveInputWithIndicatorProps {
  storageKey: string;
  defaultValue: string;
  placeholder?: string;
  type?: string;
  label?: string;
  className?: string;
  style?: React.CSSProperties;
}
export const AutoSaveInputWithIndicator: React.FC<AutoSaveInputWithIndicatorProps> = ({
  storageKey,
  defaultValue,
  placeholder,
  type = 'text',
  label,
  className = '',
  style
}) => {
  const [value, setValue] = React.useState(defaultValue);
  const [isSaving, setIsSaving] = React.useState(false);
  const [lastSaved, setLastSaved] = React.useState<Date | null>(null);

  // 防抖保存
  React.useEffect(() => {
    if (value === defaultValue) return;
    setIsSaving(true);
    const timer = setTimeout(() => {
      try {
        localStorage.setItem(`nt_auto_save_${storageKey}`, value);
        setLastSaved(new Date());
      } catch (error) {
        console.error('自动保存失败:', error);
      } finally {
        setIsSaving(false);
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [value, storageKey, defaultValue]);

  // 加载保存的数据
  React.useEffect(() => {
    try {
      const saved = localStorage.getItem(`nt_auto_save_${storageKey}`);
      if (saved !== null) {
        setValue(saved);
      }
    } catch (error) {
      console.error('加载保存数据失败:', error);
    }
  }, [storageKey]);
  return <div className={`nt-auto-save-input-wrapper ${className}`} style={style}>
      {label && <label style={{
      display: 'block',
      marginBottom: '8px',
      color: '#00F0FF',
      fontSize: '14px',
      fontFamily: 'var(--nt-font-chinese)'
    }}>
          {label}
        </label>}

      <input type={type} value={value} onChange={e => setValue(e.target.value)} placeholder={placeholder} className="nt-input" style={{
      width: '100%',
      color: '#00F0FF',
      backgroundColor: 'rgba(10, 0, 20, 0.7)',
      border: '1px solid rgba(0, 240, 255, 0.35)',
      padding: '8px 12px',
      borderRadius: '4px',
      fontSize: '14px',
      fontFamily: 'var(--nt-font-mono)',
      outline: 'none'
    }} onFocus={e => {
      e.target.style.borderColor = '#00F0FF';
      e.target.style.boxShadow = '0 0 12px rgba(0, 240, 255, 0.7), 0 0 24px rgba(0, 240, 255, 0.35)';
    }} onBlur={e => {
      e.target.style.borderColor = 'rgba(0, 240, 255, 0.35)';
      e.target.style.boxShadow = 'none';
    }} />
      
      <AutoSaveIndicator isSaving={isSaving} lastSaved={lastSaved || undefined} style={{
      marginTop: '5px'
    }} />
    </div>;
};

/**
 * 带自动保存指示器的文本区域组件
 */
interface AutoSaveTextareaWithIndicatorProps {
  storageKey: string;
  defaultValue: string;
  placeholder?: string;
  rows?: number;
  label?: string;
  className?: string;
  style?: React.CSSProperties;
}
export const AutoSaveTextareaWithIndicator: React.FC<AutoSaveTextareaWithIndicatorProps> = ({
  storageKey,
  defaultValue,
  placeholder,
  rows = 4,
  label,
  className = '',
  style
}) => {
  const [value, setValue] = React.useState(defaultValue);
  const [isSaving, setIsSaving] = React.useState(false);
  const [lastSaved, setLastSaved] = React.useState<Date | null>(null);

  // 防抖保存
  React.useEffect(() => {
    if (value === defaultValue) return;
    setIsSaving(true);
    const timer = setTimeout(() => {
      try {
        localStorage.setItem(`nt_auto_save_${storageKey}`, value);
        setLastSaved(new Date());
      } catch (error) {
        console.error('自动保存失败:', error);
      } finally {
        setIsSaving(false);
      }
    }, 500);
    return () => clearTimeout(timer);
  }, [value, storageKey, defaultValue]);

  // 加载保存的数据
  React.useEffect(() => {
    try {
      const saved = localStorage.getItem(`nt_auto_save_${storageKey}`);
      if (saved !== null) {
        setValue(saved);
      }
    } catch (error) {
      console.error('加载保存数据失败:', error);
    }
  }, [storageKey]);
  return <div className={`nt-auto-save-textarea-wrapper ${className}`} style={style}>
      {label && <label style={{
      display: 'block',
      marginBottom: '8px',
      color: '#00F0FF',
      fontSize: '14px',
      fontFamily: 'var(--nt-font-chinese)'
    }}>
          {label}
        </label>}

      <textarea value={value} onChange={e => setValue(e.target.value)} placeholder={placeholder} rows={rows} className="nt-input" style={{
      width: '100%',
      resize: 'vertical',
      minHeight: '80px',
      fontFamily: 'var(--nt-font-mono)',
      color: '#00F0FF',
      backgroundColor: 'rgba(10, 0, 20, 0.7)',
      border: '1px solid rgba(0, 240, 255, 0.35)',
      padding: '8px 12px',
      borderRadius: '4px',
      fontSize: '14px',
      outline: 'none'
    }} onFocus={e => {
      e.target.style.borderColor = '#00F0FF';
      e.target.style.boxShadow = '0 0 12px rgba(0, 240, 255, 0.7), 0 0 24px rgba(0, 240, 255, 0.35)';
    }} onBlur={e => {
      e.target.style.borderColor = 'rgba(0, 240, 255, 0.35)';
      e.target.style.boxShadow = 'none';
    }} />
      
      <AutoSaveIndicator isSaving={isSaving} lastSaved={lastSaved || undefined} style={{
      marginTop: '5px'
    }} />
    </div>;
};

/**
 * 自动保存演示组件
 */
export const AutoSaveDemo: React.FC = () => {
  return <div style={{
    padding: '20px',
    maxWidth: '600px'
  }}>
      <h3 style={{
      color: '#00F0FF',
      marginBottom: '20px'
    }}>
        {t("components.AutoSaveIndicator.k2")}
      </h3>

      <AutoSaveInputWithIndicator storageKey="demo_input_1" defaultValue="" placeholder={t("components.AutoSaveIndicator.k3")} label={t("components.AutoSaveIndicator.k4")} style={{
      marginBottom: '20px'
    }} />

      <AutoSaveTextareaWithIndicator storageKey="demo_textarea_1" defaultValue="" placeholder={t("components.AutoSaveIndicator.k5")} rows={6} label={t("components.AutoSaveIndicator.k6")} style={{
      marginBottom: '20px'
    }} />

      <div style={{
      padding: '15px',
      border: '1.5px solid rgba(0, 240, 255, 0.35)',
      fontSize: '12px',
      color: '#00F0FF'
    }}>
        <strong>{t("components.AutoSaveIndicator.k7")}</strong>
        <ul style={{
        margin: '10px 0 0 20px'
      }}>
          <li>{t("components.AutoSaveIndicator.k8")}</li>
          <li>{t("components.AutoSaveIndicator.k9")}</li>
          <li>{t("components.AutoSaveIndicator.k10")}</li>
          <li>{t("components.AutoSaveIndicator.k11")}</li>
        </ul>
      </div>
    </div>;
};

/**
 * 格式化时间显示
 */
// 添加脉冲动画到全局样式
const pulseAnimation = `
@keyframes pulse {
  0% { opacity: 1; }
  50% { opacity: 0.5; }
  100% { opacity: 1; }
}
`;

// 动态添加动画样式
if (typeof document !== 'undefined') {
  const style = document.createElement('style');
  style.textContent = pulseAnimation;
  document.head.appendChild(style);
}