import { t } from "i18next";
/**
 * CardError - 卡片局部错误态（Task 7.3 补全：局部错误态）
 *
 * 单卡片请求失败时由卡片内部渲染此组件，不影响其他卡片。
 * 提供重试按钮，点击后重新发起该卡片的 IPC 请求。
 *
 * 风格：与卡片本体一致（--nt-* 变量），错误色用 --nt-error。
 *
 * change-id: game-3d-rebuild-refactor
 */
import type { CSSProperties } from 'react';

interface CardErrorProps {
  /** 错误消息 */
  message: string;
  /** 重试回调；不传入时不显示重试按钮 */
  onRetry?: () => void;
}

export default function CardError({ message, onRetry }: CardErrorProps) {
  return (
    <div style={cardStyle}>
      <div style={iconStyle}>⚠</div>
      <div style={bodyStyle}>
        <div style={titleStyle}>{t("game.components.CardError.k1")}</div>
        <div style={messageStyle}>{message}</div>
      </div>
      {onRetry && (
        <button style={retryBtnStyle} onClick={onRetry} type="button">
          {t("components.ErrorBoundary.k5")}
        </button>
      )}
    </div>
  );
}

// ===== 内联样式 =====

const cardStyle: CSSProperties = {
  padding: '20px',
  background: 'var(--nt-bg-secondary)',
  border: '1px solid var(--nt-error)',
  borderRadius: 'var(--nt-radius-lg)',
  backdropFilter: 'blur(10px)',
  display: 'flex',
  flexDirection: 'column',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '12px',
  minHeight: '280px',
  textAlign: 'center'
};
const iconStyle: CSSProperties = {
  fontSize: '32px',
  color: 'var(--nt-error)',
  textShadow: '0 0 12px rgba(255, 0, 110, 0.5)'
};
const bodyStyle: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px'
};
const titleStyle: CSSProperties = {
  fontSize: '14px',
  color: 'var(--nt-error)',
  fontFamily: 'var(--nt-font-chinese)',
  fontWeight: 600
};
const messageStyle: CSSProperties = {
  fontSize: '12px',
  color: 'var(--nt-text-secondary)',
  fontFamily: 'var(--nt-font-mono)',
  lineHeight: 1.5,
  wordBreak: 'break-word',
  maxWidth: '280px'
};
const retryBtnStyle: CSSProperties = {
  padding: '6px 16px',
  background: 'transparent',
  border: '1px solid var(--nt-primary)',
  borderRadius: 'var(--nt-radius-md)',
  color: 'var(--nt-primary)',
  fontFamily: 'var(--nt-font-mono)',
  fontSize: '12px',
  cursor: 'pointer',
  transition: 'all var(--nt-transition-normal)'
};
