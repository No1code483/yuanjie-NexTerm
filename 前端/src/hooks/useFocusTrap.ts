import { useEffect } from 'react';

/**
 * C4 §2.3.2 Modal 焦点陷阱
 * 设计依据：功能展望/体验深化/04_无障碍_A11y合规.md §2.3.2
 *
 * 作用：当 Modal/弹窗打开时，将键盘 Tab 焦点限制在容器内部循环，
 * 防止焦点跑到 Modal 背后的页面（键盘用户无法到达 Modal 外的元素）。
 *
 * 行为：
 * - Modal 打开时自动聚焦第一个可聚焦元素
 * - Tab 在最后一个可聚焦元素上时，循环到第一个
 * - Shift+Tab 在第一个可聚焦元素上时，循环到最后一个
 *
 * 用法：
 * ```tsx
 * const ref = useRef<HTMLDivElement>(null);
 * useFocusTrap(ref, isOpen);
 * return <div ref={ref} role="dialog" aria-modal="true">...</div>;
 * ```
 */
const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

export function useFocusTrap(
  ref: React.RefObject<HTMLElement>,
  isOpen: boolean
): void {
  useEffect(() => {
    if (!isOpen || !ref.current) return;

    const container = ref.current;
    const focusable = container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
    if (focusable.length === 0) return;

    const firstFocusable = focusable[0];
    const lastFocusable = focusable[focusable.length - 1];

    // 焦点初始定位到第一个可聚焦元素
    firstFocusable.focus();

    const handleTab = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;
      if (e.shiftKey) {
        // Shift+Tab：从第一个跳到最后一个
        if (document.activeElement === firstFocusable) {
          lastFocusable.focus();
          e.preventDefault();
        }
      } else {
        // Tab：从最后一个跳到第一个
        if (document.activeElement === lastFocusable) {
          firstFocusable.focus();
          e.preventDefault();
        }
      }
    };

    container.addEventListener('keydown', handleTab);
    return () => container.removeEventListener('keydown', handleTab);
  }, [isOpen, ref]);
}
