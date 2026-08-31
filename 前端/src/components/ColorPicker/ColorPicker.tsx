/**
 * ColorPicker 自研颜色选择器（C1.1 主题编辑器组件）
 *
 * 功能：
 *   - HEX / RGB / HSL 三种输入模式切换
 *   - 色相滑块（hue: 0-360）
 *   - 饱和度+明度面板（SV panel，鼠标点击/拖拽选色）
 *   - 透明度滑块（alpha: 0-1）
 *   - 最近使用的颜色历史（最多 10 个，localStorage 持久化）
 *   - HEX 输入框支持粘贴
 *   - 不依赖外部库（自研）
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.1 ColorPicker 组件规格
 */

import { useEffect, useMemo, useRef, useState } from 'react';
import { t } from 'i18next';
import styles from './ColorPicker.module.css';

export interface ColorPickerProps {
  /** 当前颜色值（支持 #HEX / rgb() / rgba() / hsl() / hsla()） */
  value: string;
  /** 颜色变化回调（实时触发，拖拽中也会调用） */
  onChange: (value: string) => void;
  /** 是否禁用 */
  disabled?: boolean;
}

type Mode = 'hex' | 'rgb' | 'hsl';

interface RGBA { r: number; g: number; b: number; a: number; }
interface HSLA { h: number; s: number; l: number; a: number; }

// ===== 颜色格式转换工具 =====

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

/** 解析任意颜色字符串为 RGBA */
function parseColor(input: string): RGBA {
  const s = input.trim().toLowerCase();
  // #HEX / #HEXAA
  const hexMatch = s.match(/^#([0-9a-f]{3,8})$/);
  if (hexMatch) {
    let hex = hexMatch[1];
    if (hex.length === 3) hex = hex.split('').map(c => c + c).join('');
    if (hex.length === 4) hex = hex.split('').map(c => c + c).join('');
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);
    const a = hex.length === 8 ? parseInt(hex.slice(6, 8), 16) / 255 : 1;
    return { r, g, b, a };
  }
  // rgb() / rgba()
  const rgbMatch = s.match(/^rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+)\s*)?\)$/);
  if (rgbMatch) {
    return {
      r: clamp(+rgbMatch[1], 0, 255),
      g: clamp(+rgbMatch[2], 0, 255),
      b: clamp(+rgbMatch[3], 0, 255),
      a: rgbMatch[4] !== undefined ? clamp(+rgbMatch[4], 0, 1) : 1,
    };
  }
  // hsl() / hsla()
  const hslMatch = s.match(/^hsla?\(\s*([\d.]+)\s*,\s*([\d.]+)%\s*,\s*([\d.]+)%\s*(?:,\s*([\d.]+)\s*)?\)$/);
  if (hslMatch) {
    return hslToRgb({
      h: clamp(+hslMatch[1], 0, 360),
      s: clamp(+hslMatch[2], 0, 100) / 100,
      l: clamp(+hslMatch[3], 0, 100) / 100,
      a: hslMatch[4] !== undefined ? clamp(+hslMatch[4], 0, 1) : 1,
    });
  }
  // 默认返回黑色
  return { r: 0, g: 0, b: 0, a: 1 };
}

function rgbToHsl({ r, g, b, a }: RGBA): HSLA {
  r /= 255; g /= 255; b /= 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  let h = 0, s = 0;
  const l = (max + min) / 2;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case r: h = ((g - b) / d + (g < b ? 6 : 0)); break;
      case g: h = ((b - r) / d + 2); break;
      case b: h = ((r - g) / d + 4); break;
    }
    h *= 60;
  }
  return { h, s: s * 100, l: l * 100, a };
}

function hslToRgb({ h, s, l, a }: HSLA): RGBA {
  h /= 360; s /= 100; l /= 100;
  let r: number, g: number, b: number;
  if (s === 0) {
    r = g = b = l;
  } else {
    const hue2rgb = (p: number, q: number, t: number) => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;
    r = hue2rgb(p, q, h + 1 / 3);
    g = hue2rgb(p, q, h);
    b = hue2rgb(p, q, h - 1 / 3);
  }
  return { r: Math.round(r * 255), g: Math.round(g * 255), b: Math.round(b * 255), a };
}

function rgbToHex({ r, g, b, a }: RGBA): string {
  const toHex = (n: number) => n.toString(16).padStart(2, '0');
  const base = `#${toHex(r)}${toHex(g)}${toHex(b)}`;
  return a < 1 ? `${base}${toHex(Math.round(a * 255))}` : base;
}

function rgbToCss({ r, g, b, a }: RGBA): string {
  return a < 1 ? `rgba(${r}, ${g}, ${b}, ${a})` : `rgb(${r}, ${g}, ${b})`;
}

function hslToCss({ h, s, l, a }: HSLA): string {
  return a < 1 ? `hsla(${h.toFixed(1)}, ${s.toFixed(1)}%, ${l.toFixed(1)}%, ${a})` : `hsl(${h.toFixed(1)}, ${s.toFixed(1)}%, ${l.toFixed(1)}%)`;
}

// ===== 历史色持久化 =====

const HISTORY_KEY = 'nexterm-color-history';
const MAX_HISTORY = 10;

function loadHistory(): string[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch { return []; }
}

function saveHistory(history: string[]): void {
  try { localStorage.setItem(HISTORY_KEY, JSON.stringify(history)); } catch { /* ignore */ }
}

// ===== 组件 =====

export function ColorPicker({ value, onChange, disabled }: ColorPickerProps) {
  const [mode, setMode] = useState<Mode>('hex');
  const [history, setHistory] = useState<string[]>(loadHistory);
  const [draft, setDraft] = useState<RGBA>(() => parseColor(value));
  const svPanelRef = useRef<HTMLDivElement>(null);
  const hueSliderRef = useRef<HTMLDivElement>(null);
  const alphaSliderRef = useRef<HTMLDivElement>(null);
  const draggingRef = useRef<null | 'sv' | 'hue' | 'alpha'>(null);

  // 外部 value 变化时同步 draft（如用户重置主题）
  useEffect(() => {
    setDraft(parseColor(value));
  }, [value]);

  // 拖拽全局事件
  useEffect(() => {
    if (!draggingRef.current) return;
    const onMove = (e: MouseEvent) => handleDrag(e);
    const onUp = () => {
      // 拖拽结束时记录到历史
      if (draggingRef.current) {
        commitHistory(rgbToHex(draft));
      }
      draggingRef.current = null;
    };
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('mousemove', onMove);
      window.removeEventListener('mouseup', onUp);
    };
  }, [draft]);

  const hsl = useMemo(() => rgbToHsl(draft), [draft]);

  function emitChange(next: RGBA) {
    setDraft(next);
    if (mode === 'hex') onChange(rgbToHex(next));
    else if (mode === 'rgb') onChange(rgbToCss(next));
    else onChange(hslToCss(rgbToHsl(next)));
  }

  function commitHistory(hex: string) {
    const next = [hex, ...history.filter(h => h !== hex)].slice(0, MAX_HISTORY);
    setHistory(next);
    saveHistory(next);
  }

  function handleDrag(e: MouseEvent) {
    if (!draggingRef.current) return;
    e.preventDefault();
    if (draggingRef.current === 'sv' && svPanelRef.current) {
      const rect = svPanelRef.current.getBoundingClientRect();
      const x = clamp((e.clientX - rect.left) / rect.width, 0, 1);
      const y = clamp((e.clientY - rect.top) / rect.height, 0, 1);
      // SV panel: x = saturation (0-100%), y = lightness inverted (100% at top, 0% at bottom)
      // 但实际 SV panel 用 HSV 更直观。这里用 HSL：s=x*100, l=(1-y)*100 简化处理
      const newHsl: HSLA = { h: hsl.h, s: x * 100, l: (1 - y) * 100, a: draft.a };
      emitChange(hslToRgb(newHsl));
    } else if (draggingRef.current === 'hue' && hueSliderRef.current) {
      const rect = hueSliderRef.current.getBoundingClientRect();
      const x = clamp((e.clientX - rect.left) / rect.width, 0, 1);
      const newHsl: HSLA = { h: x * 360, s: hsl.s, l: hsl.l, a: draft.a };
      emitChange(hslToRgb(newHsl));
    } else if (draggingRef.current === 'alpha' && alphaSliderRef.current) {
      const rect = alphaSliderRef.current.getBoundingClientRect();
      const x = clamp((e.clientX - rect.left) / rect.width, 0, 1);
      emitChange({ ...draft, a: x });
    }
  }

  function startDrag(type: 'sv' | 'hue' | 'alpha', e: React.MouseEvent) {
    if (disabled) return;
    e.preventDefault();
    draggingRef.current = type;
    // 立即处理一次点击位置
    const fakeEvent = { clientX: e.clientX, clientY: e.clientY, preventDefault: () => {} } as MouseEvent;
    handleDrag(fakeEvent);
  }

  function handleHexInput(v: string) {
    try {
      const parsed = parseColor(v);
      setDraft(parsed);
      onChange(v);
    } catch { /* 无效输入忽略 */ }
  }

  function handleRgbInput(field: 'r' | 'g' | 'b' | 'a', v: number) {
    const next = { ...draft, [field]: field === 'a' ? clamp(v, 0, 1) : clamp(v, 0, 255) };
    emitChange(next);
  }

  function handleHslInput(field: 'h' | 's' | 'l' | 'a', v: number) {
    const next: HSLA = {
      h: field === 'h' ? clamp(v, 0, 360) : hsl.h,
      s: field === 's' ? clamp(v, 0, 100) : hsl.s,
      l: field === 'l' ? clamp(v, 0, 100) : hsl.l,
      a: field === 'a' ? clamp(v, 0, 1) : draft.a,
    };
    emitChange(hslToRgb(next));
  }

  const hueColor = useMemo(() => {
    // SV panel 的背景色随 hue 变化（纯色相）
    return `hsl(${hsl.h}, 100%, 50%)`;
  }, [hsl.h]);

  const svPointer = { left: `${(hsl.s / 100) * 100}%`, top: `${(1 - hsl.l / 100) * 100}%` };
  const huePointer = { left: `${(hsl.h / 360) * 100}%` };
  const alphaPointer = { left: `${draft.a * 100}%` };

  const alphaBackground = useMemo(() => {
    // 透明度滑块背景：从透明到当前色
    const hex = rgbToHex({ ...draft, a: 1 });
    return `linear-gradient(to right, transparent, ${hex})`;
  }, [draft]);

  return (
    <div className={`${styles.container} ${disabled ? styles.disabled : ''}`}>
      {/* SV 面板 */}
      <div
        ref={svPanelRef}
        className={styles.svPanel}
        style={{ backgroundColor: hueColor }}
        onMouseDown={(e) => startDrag('sv', e)}
      >
        <div className={styles.svOverlay} />
        <div className={styles.svPointer} style={svPointer} />
      </div>

      {/* 色相滑块 */}
      <div className={styles.sliderRow}>
        <span className={styles.sliderLabel}>H</span>
        <div
          ref={hueSliderRef}
          className={styles.hueSlider}
          onMouseDown={(e) => startDrag('hue', e)}
        >
          <div className={styles.huePointer} style={huePointer} />
        </div>
      </div>

      {/* 透明度滑块 */}
      <div className={styles.sliderRow}>
        <span className={styles.sliderLabel}>A</span>
        <div
          ref={alphaSliderRef}
          className={styles.alphaSlider}
          style={{ background: alphaBackground }}
          onMouseDown={(e) => startDrag('alpha', e)}
        >
          <div className={styles.alphaPointer} style={alphaPointer} />
        </div>
      </div>

      {/* 模式切换 + 输入框 */}
      <div className={styles.inputRow}>
        <div className={styles.modeTabs}>
          {(['hex', 'rgb', 'hsl'] as Mode[]).map(m => (
            <button
              key={m}
              type="button"
              className={`${styles.modeTab} ${mode === m ? styles.modeTabActive : ''}`}
              onClick={() => setMode(m)}
            >
              {m.toUpperCase()}
            </button>
          ))}
        </div>
        {mode === 'hex' && (
          <input
            type="text"
            className={styles.textInput}
            value={rgbToHex(draft)}
            onChange={(e) => handleHexInput(e.target.value)}
            disabled={disabled}
            maxLength={9}
          />
        )}
        {mode === 'rgb' && (
          <div className={styles.numberGroup}>
            {(['r', 'g', 'b'] as const).map(f => (
              <label key={f} className={styles.numberField}>
                <span>{f.toUpperCase()}</span>
                <input
                  type="number"
                  min={0}
                  max={255}
                  value={draft[f]}
                  onChange={(e) => handleRgbInput(f, +e.target.value)}
                  disabled={disabled}
                />
              </label>
            ))}
            <label className={styles.numberField}>
              <span>A</span>
              <input
                type="number"
                min={0}
                max={1}
                step={0.01}
                value={draft.a.toFixed(2)}
                onChange={(e) => handleRgbInput('a', +e.target.value)}
                disabled={disabled}
              />
            </label>
          </div>
        )}
        {mode === 'hsl' && (
          <div className={styles.numberGroup}>
            <label className={styles.numberField}>
              <span>H</span>
              <input
                type="number"
                min={0}
                max={360}
                value={Math.round(hsl.h)}
                onChange={(e) => handleHslInput('h', +e.target.value)}
                disabled={disabled}
              />
            </label>
            {(['s', 'l'] as const).map(f => (
              <label key={f} className={styles.numberField}>
                <span>{f.toUpperCase()}</span>
                <input
                  type="number"
                  min={0}
                  max={100}
                  value={Math.round(hsl[f])}
                  onChange={(e) => handleHslInput(f, +e.target.value)}
                  disabled={disabled}
                />
              </label>
            ))}
            <label className={styles.numberField}>
              <span>A</span>
              <input
                type="number"
                min={0}
                max={1}
                step={0.01}
                value={draft.a.toFixed(2)}
                onChange={(e) => handleHslInput('a', +e.target.value)}
                disabled={disabled}
              />
            </label>
          </div>
        )}
      </div>

      {/* 历史色 */}
      {history.length > 0 && (
        <div className={styles.historyRow}>
          <span className={styles.historyLabel}>{t('components.ColorPicker.ColorPicker.k1')}</span>
          <div className={styles.historySwatches}>
            {history.map((h, i) => (
              <button
                key={i}
                type="button"
                className={styles.historySwatch}
                style={{ backgroundColor: h }}
                onClick={() => {
                  const parsed = parseColor(h);
                  emitChange(parsed);
                }}
                title={h}
                disabled={disabled}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

export default ColorPicker;
