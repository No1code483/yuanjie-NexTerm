import { t } from "i18next";
/**
 * AiErrorBanner - AI 错误/降级状态提示横幅
 *
 * 使用场景：
 * - LLM 未配置时显示"请先配置 LLM"
 * - AI 调用失败时显示错误信息
 * - 功能关闭时显示灰态提示
 */

import styles from './Intelligence.module.css';
interface Props {
  type: 'disabled' | 'no_llm' | 'error';
  message?: string;
  onRetry?: () => void;
  inline?: boolean;
}
const DEFAULT_MESSAGES: Record<Props['type'], string> = {
  disabled: t("components.intelligence.AiErrorBanner.k1"),
  no_llm: t("components.intelligence.AiErrorBanner.k2"),
  error: t("components.intelligence.AiErrorBanner.k3")
};
export default function AiErrorBanner({
  type,
  message,
  onRetry,
  inline
}: Props) {
  const text = message || DEFAULT_MESSAGES[type];
  const Tag = inline ? 'span' : 'div';
  return <Tag className={inline ? styles.aiHintGray : styles.aiErrorBanner} title={text}>
      <span className={styles.aiHintIcon}>
        {type === 'error' ? '⚠' : type === 'no_llm' ? '⚙' : '🔒'}
      </span>
      <span className={styles.aiHintText}>{text}</span>
      {type === 'error' && onRetry && <button className={styles.aiRetryBtn} onClick={onRetry}>
          {t("components.ErrorBoundary.k5")}
        </button>}
    </Tag>;
}