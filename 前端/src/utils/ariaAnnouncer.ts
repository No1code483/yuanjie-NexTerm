/**
 * C4 §2.4.5 ARIA Live 通知器
 *
 * 提供应用内全局屏幕阅读器播报能力：
 *   - polite：非紧急通知（默认），屏幕阅读器空闲时播报
 *   - assertive：紧急通知（错误提示），立即打断当前朗读
 *
 * 设计依据：功能展望/体验深化/04_无障碍_A11y合规.md §2.4.5
 *
 * 用法：
 *   initAriaAnnouncer();                      // 应用启动时调用一次（App.tsx）
 *   announce('已保存');                       // polite 播报
 *   announce('网络断开', 'assertive');        // 紧急播报
 */

const POLITELIVE_ID = 'nt-aria-live-polite';
const ASSERTIVE_LIVE_ID = 'nt-aria-live-assertive';

let initialized = false;

function ensureLiveRegion(id: string, ariaLive: 'polite' | 'assertive'): HTMLElement {
  let el = document.getElementById(id);
  if (!el) {
    el = document.createElement('div');
    el.id = id;
    el.setAttribute('role', 'status');
    el.setAttribute('aria-live', ariaLive);
    el.setAttribute('aria-atomic', 'true');
    el.classList.add('sr-only');
    document.body.appendChild(el);
  }
  return el;
}

/** 初始化 ARIA Live 通知器（应用启动时调用一次即可） */
export function initAriaAnnouncer(): void {
  if (initialized || typeof document === 'undefined') return;
  ensureLiveRegion(POLITELIVE_ID, 'polite');
  ensureLiveRegion(ASSERTIVE_LIVE_ID, 'assertive');
  initialized = true;
}

/** 播报一条消息到屏幕阅读器 */
export function announce(message: string, mode: 'polite' | 'assertive' = 'polite'): void {
  if (typeof document === 'undefined') return;
  if (!initialized) initAriaAnnouncer();

  const id = mode === 'assertive' ? ASSERTIVE_LIVE_ID : POLITELIVE_ID;
  const region = document.getElementById(id);
  if (!region) return;

  // 先清空再延迟写入，确保屏幕阅读器能感知变化
  region.textContent = '';
  // 50ms 延迟保证 SR 重新检测 DOM 变更
  window.setTimeout(() => {
    region.textContent = message;
  }, 50);
}

/** 清空通知区（测试用） */
export function clearAnnouncement(mode: 'polite' | 'assertive' = 'polite'): void {
  if (typeof document === 'undefined') return;
  const id = mode === 'assertive' ? ASSERTIVE_LIVE_ID : POLITELIVE_ID;
  const region = document.getElementById(id);
  if (region) region.textContent = '';
}
