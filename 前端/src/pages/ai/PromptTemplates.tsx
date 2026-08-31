import { t } from "i18next";
import { useState, useEffect } from 'react';
import { ai } from '@/lib/ipc';
import styles from '../AI.module.css';
interface PromptTemplate {
  id: number;
  title: string;
  category: string;
  content: string;
  created_at: number;
  updated_at: number;
}
interface Props {
  onApply: (content: string) => void;
  onClose: () => void;
}
const CATEGORIES = [t("ai.PromptTemplates.k1"), t("ai.PromptTemplates.k2"), t("ai.PromptTemplates.k3"), t("ai.PromptTemplates.k4"), t("components.SelectionToolbar.k2"), t("lib.ipcMock.k42")];
const BUILTIN_TEMPLATES = [{
  title: t("ai.PromptTemplates.k2"),
  category: t("ai.PromptTemplates.k2"),
  content: t("ai.PromptTemplates.k5")
}, {
  title: t("ai.PromptTemplates.k6"),
  category: t("ai.PromptTemplates.k1"),
  content: t("ai.PromptTemplates.k7")
}, {
  title: t("ai.PromptTemplates.k8"),
  category: t("ai.PromptTemplates.k3"),
  content: t("ai.PromptTemplates.k9")
}, {
  title: t("ai.PromptTemplates.k10"),
  category: t("ai.PromptTemplates.k4"),
  content: t("ai.PromptTemplates.k11")
}, {
  title: t("ai.PromptTemplates.k12"),
  category: t("components.SelectionToolbar.k2"),
  content: t("ai.PromptTemplates.k13")
}, {
  title: t("ai.PromptTemplates.k14"),
  category: t("ai.PromptTemplates.k2"),
  content: t("ai.PromptTemplates.k15")
}];
export default function PromptTemplates({
  onApply,
  onClose
}: Props) {
  const [templates, setTemplates] = useState<PromptTemplate[]>([]);
  const [activeCategory, setActiveCategory] = useState(t("common.all"));
  const [showNewForm, setShowNewForm] = useState(false);
  const [newTitle, setNewTitle] = useState('');
  const [newCategory, setNewCategory] = useState(t("lib.ipcMock.k42"));
  const [newContent, setNewContent] = useState('');
  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);
  const fetchTemplates = async () => {
    try {
      const res = await ai.getPromptTemplates();
      setTemplates(res.data || []);
    } catch {/* ignore */}
  };
  useEffect(() => {
    fetchTemplates();
  }, []);
  const allTemplates = activeCategory === t("common.all") ? templates : templates.filter(t => t.category === activeCategory);
  const handleCreate = async () => {
    if (!newTitle.trim() || !newContent.trim()) return;
    try {
      await ai.createPromptTemplate(newTitle.trim(), newCategory, newContent.trim());
      setShowNewForm(false);
      setNewTitle('');
      setNewContent('');
      setNewCategory(t("lib.ipcMock.k42"));
      fetchTemplates();
    } catch {/* ignore */}
  };
  const handleDelete = async (id: number) => {
    try {
      await ai.deletePromptTemplate(id);
      setDeleteConfirm(null);
      fetchTemplates();
    } catch {/* ignore */}
  };
  const handleApplyBuiltin = (content: string) => {
    onApply(content);
  };
  return <div className={styles.templatePanel}>
      <div className={styles.templateHeader}>
        <span className={styles.templateTitle}>{t("ai.PromptTemplates.k16")}</span>
        <button className={styles.modalClose} onClick={onClose}>✕</button>
      </div>

      <div className={styles.templateCategories}>
        <button className={`${styles.templateCatBtn} ${activeCategory === t("common.all") ? styles.templateCatBtnActive : ''}`} onClick={() => setActiveCategory(t("common.all"))}>{t("common.all")}</button>
        {CATEGORIES.map(cat => <button key={cat} className={`${styles.templateCatBtn} ${activeCategory === cat ? styles.templateCatBtnActive : ''}`} onClick={() => setActiveCategory(cat)}>{cat}</button>)}
      </div>

      <div className={styles.templateList}>
        {/* 内置模板 */}
        {activeCategory === t("common.all") || activeCategory === t("ai.PromptTemplates.k2") ? BUILTIN_TEMPLATES.filter(tpl => activeCategory === t("common.all") || tpl.category === activeCategory).map((tpl, i) => <div key={`builtin-${i}`} className={styles.templateItem}>
              <div className={styles.templateItemHeader}>
                <span className={styles.templateItemTitle}>{tpl.title}</span>
                <span className={styles.templateItemCat}>{tpl.category}</span>
              </div>
              <div className={styles.templateItemPreview}>{tpl.content.slice(0, 80)}...</div>
              <button className={styles.templateApplyBtn} onClick={() => handleApplyBuiltin(tpl.content)}>{t("common.apply")}</button>
            </div>) : null}

        {/* 自定义模板 */}
        {allTemplates.map(tpl => <div key={tpl.id} className={styles.templateItem}>
            <div className={styles.templateItemHeader}>
              <span className={styles.templateItemTitle}>{tpl.title}</span>
              <span className={styles.templateItemCat}>{tpl.category}</span>
            </div>
            <div className={styles.templateItemPreview}>{tpl.content.slice(0, 80)}...</div>
            <div className={styles.templateItemActions}>
              <button className={styles.templateApplyBtn} onClick={() => onApply(tpl.content)}>{t("common.apply")}</button>
              <button className={styles.templateDeleteBtn} onClick={() => setDeleteConfirm(tpl.id)}>{t("common.delete")}</button>
            </div>
            {deleteConfirm === tpl.id && <div className={styles.templateDeleteConfirm}>
                <span>{t("ai.PromptTemplates.k17")}</span>
                <button className={styles.templateApplyBtn} onClick={() => handleDelete(tpl.id)}>{t("common.confirm")}</button>
                <button className={styles.templateDeleteBtn} onClick={() => setDeleteConfirm(null)}>{t("common.cancel")}</button>
              </div>}
          </div>)}
      </div>

      {showNewForm ? <div className={styles.templateNewForm}>
          <input className={styles.formInput} placeholder={t("ai.PromptTemplates.k18")} value={newTitle} onChange={e => setNewTitle(e.target.value)} />
          <select className={styles.formInput} value={newCategory} onChange={e => setNewCategory(e.target.value)}>
            {CATEGORIES.map(c => <option key={c} value={c}>{c}</option>)}
          </select>
          <textarea className={styles.formInput} style={{
        minHeight: 80,
        resize: 'vertical'
      }} placeholder={t("ai.PromptTemplates.k19")} value={newContent} onChange={e => setNewContent(e.target.value)} />
          <div style={{
        display: 'flex',
        gap: 8
      }}>
            <button className={styles.primaryBtn} onClick={handleCreate}>{t("common.save")}</button>
            <button className={styles.cancelBtn} onClick={() => setShowNewForm(false)}>{t("common.cancel")}</button>
          </div>
        </div> : <button className={styles.addBtn} onClick={() => setShowNewForm(true)}>{t("ai.PromptTemplates.k20")}</button>}
    </div>;
}