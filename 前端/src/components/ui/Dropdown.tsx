import { t } from "i18next";
import { useTranslation } from 'react-i18next';
import { useState, useRef, useEffect, memo, useId } from 'react';
import styles from './Dropdown.module.css';
interface DropdownOption {
  value: string;
  label: string;
}
interface DropdownProps {
  options: DropdownOption[];
  value?: string;
  onChange?: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  size?: 'small' | 'medium' | 'large';
  className?: string;
  ariaLabel?: string;
}
const Dropdown = memo(function Dropdown({
  options,
  value,
  onChange,
  placeholder = t("components.ui.Dropdown.k1"),
  disabled = false,
  size = 'medium',
  className = '',
  ariaLabel
}: DropdownProps) {
  // C2.4：订阅语言变化触发 memo 组件重渲染（t 在默认参数中使用模块级 t）
  useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const listboxId = useId();
  const selected = options.find(o => o.value === value);
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [isOpen]);
  return <div ref={containerRef} className={`${styles.container} ${className}`}>
      <button type="button" className={`${styles.trigger} ${styles[size]} ${isOpen ? styles.open : ''} ${disabled ? styles.disabled : ''}`} onClick={() => !disabled && setIsOpen(!isOpen)} disabled={disabled} aria-label={ariaLabel} aria-expanded={isOpen} aria-haspopup="listbox" aria-controls={isOpen ? listboxId : undefined}>
        <span className={selected ? styles.selectedText : styles.placeholder}>
          {selected ? selected.label : placeholder}
        </span>
        <span className={`${styles.arrow} ${isOpen ? styles.arrowUp : ''}`}>&#9662;</span>
      </button>

      {isOpen && <div id={listboxId} className={styles.menu} role="listbox">
          {options.map(opt => <div key={opt.value} role="option" aria-selected={opt.value === value} className={`${styles.option} ${opt.value === value ? styles.active : ''}`} onClick={() => {
        onChange?.(opt.value);
        setIsOpen(false);
      }}>
              {opt.label}
            </div>)}
        </div>}
    </div>;
});
export default Dropdown;