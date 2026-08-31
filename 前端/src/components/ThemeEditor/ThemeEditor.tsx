/**
 * ThemeEditor 主题编辑器（C1.1 主题编辑器主组件）
 *
 * 功能：
 *   - Modal 弹窗形式（点击 ThemeSelector 中的"自定义主题"按钮触发）
 *   - 左侧：变量分组手风琴（背景/文字/主色/状态/边框/阴影/ANSI/几何）
 *   - 右侧：ThemePreviewPanel 实时预览
 *   - 每个颜色变量点击展开 ColorPicker，实时编辑
 *   - 阴影变量：文本输入框
 *   - 几何变量（圆角）：数字输入 + 滑块
 *   - 底部操作：重置 / 另存为 / 导出 / 导入 / 关闭
 *
 * 实时预览机制：
 *   - 修改任何变量时，立即通过 document.documentElement.style.setProperty 应用到 DOM
 *   - ThemePreviewPanel 引用 --nt-* 变量自动反映变化
 *   - 关闭 Modal 时将 draft 持久化到 themeStore + localStorage
 *
 * 关联文档：功能展望/体验深化/01_主题自定义系统_未来展望.md §2.1
 */

import { useEffect, useMemo, useState } from 'react';
import { t } from 'i18next';
import { useThemeStore } from '../../stores/themeStore';
import { ColorPicker } from '../ColorPicker/ColorPicker';
import { ThemePreviewPanel } from './ThemePreviewPanel';
import {
  THEME_VARIABLE_GROUPS,
  type ThemeVariableMeta,
  getVariableValue,
} from './themeVariables';
import styles from './ThemeEditor.module.css';

export interface ThemeEditorProps {
  /** 是否显示 */
  isOpen: boolean;
  /** 关闭回调 */
  onClose: () => void;
}

export function ThemeEditor({ isOpen, onClose }: ThemeEditorProps) {
  const { customOverrides, setCustomVariable, resetCustomVariables, exportTheme, importTheme } = useThemeStore();
  /** draft：当前编辑中的变量覆盖（key → value） */
  const [draft, setDraft] = useState<Record<string, string>>(customOverrides || {});
  /** 当前展开的分组 ID */
  const [expandedGroup, setExpandedGroup] = useState<string | null>('background');
  /** 当前展开 ColorPicker 的变量 key */
  const [activeColorKey, setActiveColorKey] = useState<string | null>(null);
  /** 主题名称输入（另存为时使用） */
  const [themeName, setThemeName] = useState('');
  /** 是否显示另存为对话框 */
  const [showSaveAs, setShowSaveAs] = useState(false);
  /** 导入/导出提示信息 */
  const [toast, setToast] = useState<{ type: 'success' | 'error'; msg: string } | null>(null);

  // 打开 Modal 时从 store 同步 draft
  useEffect(() => {
    if (isOpen) {
      setDraft(customOverrides || {});
      setExpandedGroup('background');
      setActiveColorKey(null);
    }
  }, [isOpen, customOverrides]);

  // ESC 关闭
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') handleCancel();
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [isOpen, draft]);

  // toast 自动消失
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(null), 2500);
    return () => clearTimeout(timer);
  }, [toast]);

  /** 应用单个变量到 DOM（实时预览） */
  function applyToDOM(key: string, value: string) {
    document.documentElement.style.setProperty(key, value);
  }

  /** 移除单个变量的 DOM 覆盖（恢复到预设主题值） */
  function removeFromDOM(key: string) {
    document.documentElement.style.removeProperty(key);
  }

  /** 修改变量值（实时应用 + 更新 draft） */
  function handleChange(key: string, value: string) {
    const next = { ...draft, [key]: value };
    setDraft(next);
    applyToDOM(key, value);
  }

  /** 重置单个变量到默认值 */
  function handleResetOne(meta: ThemeVariableMeta) {
    const next = { ...draft };
    delete next[meta.key];
    setDraft(next);
    removeFromDOM(meta.key);
  }

  /** 重置全部 */
  function handleResetAll() {
    Object.keys(draft).forEach(key => removeFromDOM(key));
    setDraft({});
    resetCustomVariables();
    setToast({ type: 'success', msg: t('components.ThemeEditor.ThemeEditor.k1') });
  }

  /** 保存到 store + localStorage */
  function handleSave() {
    // 先清除所有现有覆盖，再重新应用 draft
    resetCustomVariables();
    Object.entries(draft).forEach(([key, value]) => {
      setCustomVariable(key, value);
    });
    setToast({ type: 'success', msg: t('components.ThemeEditor.ThemeEditor.k2') });
    onClose();
  }

  /** 取消（恢复到打开前的状态） */
  function handleCancel() {
    // 移除所有 draft 应用的 DOM 覆盖
    Object.keys(draft).forEach(key => removeFromDOM(key));
    // 重新应用 store 中的 customOverrides
    if (customOverrides) {
      Object.entries(customOverrides).forEach(([key, value]) => applyToDOM(key, value));
    }
    onClose();
  }

  /** 另存为新主题（C1.5 起持久化到后端 custom_themes 表） */
  async function handleSaveAs() {
    const name = themeName.trim();
    if (!name) {
      setToast({ type: 'error', msg: t('components.ThemeEditor.ThemeEditor.k3') });
      return;
    }
    // 先保存当前 draft 到 store（保证当前编辑中的覆盖生效）
    handleSave();
    // 持久化到后端 custom_themes 表（UPSERT 语义：同名覆盖）
    try {
      const { customThemeUpsert } = await import('../../lib/customThemeIpc');
      await customThemeUpsert({
        name,
        base_theme: useThemeStore.getState().themeName,
        variables: JSON.stringify(draft),
      });
      setToast({ type: 'success', msg: t('components.ThemeEditor.ThemeEditor.k4', { name }) });
      setShowSaveAs(false);
      setThemeName('');
    } catch (e) {
      setToast({ type: 'error', msg: (e as Error).message || t('components.ThemeEditor.ThemeEditor.k5') });
    }
  }

  /** 导出 JSON */
  function handleExport() {
    const json = exportTheme();
    const blob = new Blob([json], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `nexterm-theme-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
    setToast({ type: 'success', msg: t('components.ThemeEditor.ThemeEditor.k6') });
  }

  /** 导入 JSON */
  function handleImport(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      const result = importTheme(String(reader.result));
      if (result.success) {
        // 同步 draft 到 store 状态
        const storeOverrides = useThemeStore.getState().customOverrides;
        if (storeOverrides) {
          Object.entries(storeOverrides).forEach(([k, v]) => applyToDOM(k, v));
          setDraft(storeOverrides);
        }
        setToast({ type: 'success', msg: t('components.ThemeEditor.ThemeEditor.k7') });
      } else {
        setToast({ type: 'error', msg: result.error || t('components.ThemeEditor.ThemeEditor.k8') });
      }
    };
    reader.readAsText(file);
    // 清空 input，允许重复选择同一文件
    e.target.value = '';
  }

  /** 获取变量的当前显示值（draft 优先，否则读取 DOM 计算值） */
  function getDisplayValue(meta: ThemeVariableMeta): string {
    if (draft[meta.key] !== undefined) return draft[meta.key];
    return getVariableValue(meta.key) || meta.defaultValue;
  }

  /** 判断变量是否已被修改（与默认值不同） */
  function isModified(meta: ThemeVariableMeta): boolean {
    return draft[meta.key] !== undefined;
  }

  const modifiedCount = useMemo(() => Object.keys(draft).length, [draft]);

  if (!isOpen) return null;

  return (
    <div className={styles.overlay} onClick={handleCancel}>
      <div className={styles.dialog} onClick={e => e.stopPropagation()}>
        {/* 标题栏 */}
        <div className={styles.header}>
          <div className={styles.headerText}>
            <span className={styles.title}>{t('components.ThemeEditor.ThemeEditor.k9')}</span>
            <span className={styles.subtitle}>
              {t('components.ThemeEditor.ThemeEditor.k10', { count: modifiedCount })}
            </span>
          </div>
          <button className={styles.closeBtn} onClick={handleCancel} title={t('common.cancel')}>
            ×
          </button>
        </div>

        {/* 主体：左右分栏 */}
        <div className={styles.body}>
          {/* 左侧：变量分组（M4.④ Step4：quickPresets 迁至右侧，左侧专注变量编辑） */}
          <div className={styles.leftPanel}>
            {THEME_VARIABLE_GROUPS.map(group => {
              const isExpanded = expandedGroup === group.id;
              return (
                <div key={group.id} className={styles.group}>
                  <button
                    type="button"
                    className={`${styles.groupHeader} ${isExpanded ? styles.groupExpanded : ''}`}
                    onClick={() => setExpandedGroup(isExpanded ? null : group.id)}
                  >
                    <span className={styles.groupArrow}>{isExpanded ? '▼' : '▶'}</span>
                    <span className={styles.groupLabel}>
                      {t(`components.ThemeEditor.ThemeEditor.group.${group.labelKey}`)}
                    </span>
                    <span className={styles.groupCount}>
                      {group.variables.filter(v => isModified(v)).length}/{group.variables.length}
                    </span>
                  </button>
                  {isExpanded && (
                    <div className={styles.groupBody}>
                      {group.variables.map(meta => {
                        const value = getDisplayValue(meta);
                        const modified = isModified(meta);
                        const isColorActive = activeColorKey === meta.key;
                        return (
                          <div key={meta.key} className={styles.varRow}>
                            <div className={styles.varHeader}>
                              <span className={styles.varLabel}>
                                {t(`components.ThemeEditor.ThemeEditor.var.${meta.labelKey}`)}
                              </span>
                              <div className={styles.varActions}>
                                {modified && (
                                  <button
                                    type="button"
                                    className={styles.resetBtn}
                                    onClick={() => handleResetOne(meta)}
                                    title={t('components.ThemeEditor.ThemeEditor.k11')}
                                  >
                                    ↺
                                  </button>
                                )}
                              </div>
                            </div>
                            {meta.type === 'color' && (
                              <>
                                <button
                                  type="button"
                                  className={styles.colorSwatchBtn}
                                  onClick={() => setActiveColorKey(isColorActive ? null : meta.key)}
                                >
                                  <span
                                    className={styles.colorSwatch}
                                    style={{ background: value }}
                                  />
                                  <span className={styles.colorValue}>{value}</span>
                                </button>
                                {isColorActive && (
                                  <ColorPicker
                                    value={value}
                                    onChange={(v) => handleChange(meta.key, v)}
                                  />
                                )}
                              </>
                            )}
                            {meta.type === 'shadow' && (
                              <input
                                type="text"
                                className={styles.shadowInput}
                                value={value}
                                onChange={(e) => handleChange(meta.key, e.target.value)}
                              />
                            )}
                            {meta.type === 'number' && (
                              <div className={styles.numberRow}>
                                <input
                                  type="range"
                                  className={styles.rangeInput}
                                  min={meta.min}
                                  max={meta.max}
                                  step={meta.step}
                                  value={parseInt(value, 10) || 0}
                                  onChange={(e) => handleChange(meta.key, `${e.target.value}${meta.unit || ''}`)}
                                />
                                <input
                                  type="number"
                                  className={styles.numberInput}
                                  min={meta.min}
                                  max={meta.max}
                                  step={meta.step}
                                  value={parseInt(value, 10) || 0}
                                  onChange={(e) => handleChange(meta.key, `${e.target.value}${meta.unit || ''}`)}
                                />
                                <span className={styles.unit}>{meta.unit}</span>
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })}
          </div>

          {/* 右侧：快速预设 + 预览面板（M4.④ Step4：quickPresets 迁入右侧顶部，形成「配色→预览」工作流） */}
          <div className={styles.rightPanel}>
            <div className={styles.quickPresets}>
              <div className={styles.quickPresetsLabel}>
                {t('components.ThemeEditor.ThemeEditor.k18')}
              </div>
              <div className={styles.quickPresetsGrid}>
                {([
                  { name: t('Profile.k144'), bg: '#000000', color: '#00FF00', accent: '#FF0000' },
                  { name: t('Profile.k145'), bg: '#000011', color: '#00F0FF', accent: '#FF006E' },
                  { name: t('Profile.k146'), bg: '#0a0a00', color: '#FFB000', accent: '#FF4400' },
                  { name: t('Profile.k147'), bg: '#0a0a0f', color: '#B026FF', accent: '#FF006E' },
                ]).map(preset => (
                  <button
                    key={preset.name}
                    type="button"
                    className={styles.quickPresetBtn}
                    onClick={() => {
                      handleChange('--nt-bg-primary', preset.bg);
                      handleChange('--nt-primary', preset.color);
                      handleChange('--nt-danger', preset.accent);
                    }}
                    style={{
                      background: preset.bg,
                      color: preset.color,
                      borderColor: `${preset.color}40`,
                    }}
                  >
                    {preset.name}
                  </button>
                ))}
              </div>
            </div>
            <div className={styles.previewWrap}>
              <ThemePreviewPanel />
            </div>
          </div>
        </div>

        {/* 另存为对话框 */}
        {showSaveAs && (
          <div className={styles.saveAsOverlay} onClick={(e) => e.stopPropagation()}>
            <div className={styles.saveAsDialog}>
              <div className={styles.saveAsTitle}>{t('components.ThemeEditor.ThemeEditor.k12')}</div>
              <input
                type="text"
                className={styles.saveAsInput}
                value={themeName}
                onChange={(e) => setThemeName(e.target.value)}
                placeholder={t('components.ThemeEditor.ThemeEditor.k13')}
                autoFocus
                maxLength={32}
              />
              <div className={styles.saveAsActions}>
                <button className={styles.btnSecondary} onClick={() => setShowSaveAs(false)}>
                  {t('common.cancel')}
                </button>
                <button className={styles.btnPrimary} onClick={handleSaveAs}>
                  {t('common.confirm')}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* toast */}
        {toast && (
          <div className={`${styles.toast} ${toast.type === 'success' ? styles.toastSuccess : styles.toastError}`}>
            {toast.msg}
          </div>
        )}

        {/* 底部操作栏 */}
        <div className={styles.footer}>
          <div className={styles.footerLeft}>
            <button className={styles.btnSecondary} onClick={handleResetAll}>
              {t('components.ThemeEditor.ThemeEditor.k14')}
            </button>
            <button className={styles.btnSecondary} onClick={() => setShowSaveAs(true)}>
              {t('components.ThemeEditor.ThemeEditor.k15')}
            </button>
          </div>
          <div className={styles.footerRight}>
            <button className={styles.btnSecondary} onClick={handleExport}>
              {t('components.ThemeEditor.ThemeEditor.k16')}
            </button>
            <label className={styles.btnSecondary}>
              {t('components.ThemeEditor.ThemeEditor.k17')}
              <input type="file" accept=".json" onChange={handleImport} style={{ display: 'none' }} />
            </label>
            <button className={styles.btnDanger} onClick={handleCancel}>
              {t('common.cancel')}
            </button>
            <button className={styles.btnPrimary} onClick={handleSave}>
              {t('common.save')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

export default ThemeEditor;
