import { useEffect } from 'react';

/**
 * C4 §2.3.1 键盘焦点管理
 * 设计依据：功能展望/体验深化/04_无障碍_A11y合规.md §2.3.1
 *
 * 作用：监听 Tab 键 / mousedown，给 body 添加/移除 `using-keyboard` class。
 *
 * 注意：现代浏览器原生 `:focus-visible` 已自动判断键盘/鼠标焦点并显示轮廓
 * （见 global.css L356-L369），此 hook 主要作为补充：
 * - 提供给需要判断用户当前输入模式的业务逻辑使用
 * - 为不支持 :focus-visible 的老浏览器提供兜底（通过 CSS 显式定义 .using-keyboard :focus）
 *
 * 用法：在应用根组件调用一次即可（App.tsx 已集成）。
 */
export function useFocusManagement(): void {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Tab') {
        document.body.classList.add('using-keyboard');
      }
    };
    const handleMouseDown = () => {
      document.body.classList.remove('using-keyboard');
    };

    document.addEventListener('keydown', handleKeyDown);
    document.addEventListener('mousedown', handleMouseDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
      document.removeEventListener('mousedown', handleMouseDown);
    };
  }, []);
}
