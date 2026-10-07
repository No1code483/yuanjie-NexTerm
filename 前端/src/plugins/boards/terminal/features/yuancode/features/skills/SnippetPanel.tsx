import { t } from "i18next";
/**
 * 代码片段管理面板 — 对标 VSCode workbench/snippets/
 *
 * 功能:
 * - 按语言筛选片段
 * - 搜索片段（按名称/前缀）
 * - 新建/编辑/删除用户自定义片段
 * - 导入/导出片段 JSON
 * - 点击插入到编辑器光标位置
 */

import { useState, useCallback, useMemo } from 'react';
import { Snippet, SnippetStore } from './SnippetStore';
import styles from './SnippetPanel.module.css';
interface SnippetPanelProps {
  editor: any;
  workspacePath: string;
}
const LANGUAGES = [{
  value: '*',
  label: t("yuan-code.SnippetPanel.k1")
}, {
  value: 'python',
  label: 'Python'
}, {
  value: 'javascript',
  label: 'JavaScript'
}, {
  value: 'typescript',
  label: 'TypeScript'
}, {
  value: 'rust',
  label: 'Rust'
}, {
  value: 'go',
  label: 'Go'
}, {
  value: 'html',
  label: 'HTML'
}, {
  value: 'css',
  label: 'CSS'
}, {
  value: 'json',
  label: 'JSON'
}, {
  value: 'markdown',
  label: 'Markdown'
}];
export default function SnippetPanel({
  editor,
  workspacePath: _workspacePath
}: SnippetPanelProps) {
  const [snippets, setSnippets] = useState<Snippet[]>(() => SnippetStore.getAll());
  const [filterLang, setFilterLang] = useState('*');
  const [searchQuery, setSearchQuery] = useState('');
  const [editing, setEditing] = useState<Snippet | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [importError, setImportError] = useState('');

  // 表单状态
  const [form, setForm] = useState({
    name: '',
    prefix: '',
    description: '',
    body: '',
    scope: 'python'
  });

  // 筛选后的片段
  const filtered = useMemo(() => {
    let result = snippets;
    if (filterLang !== '*') {
      result = result.filter(s => s.scope === filterLang || s.scope === '*');
    }
    if (searchQuery) {
      const q = searchQuery.toLowerCase();
      result = result.filter(s => s.name.toLowerCase().includes(q) || s.prefix.toLowerCase().includes(q) || s.description.toLowerCase().includes(q));
    }
    return result;
  }, [snippets, filterLang, searchQuery]);
  const refresh = useCallback(() => {
    setSnippets(SnippetStore.getAll());
  }, []);

  // 插入片段到编辑器
  const insertSnippet = useCallback((snippet: Snippet) => {
    if (!editor) return;
    const selection = editor.getSelection();
    if (selection) {
      editor.executeEdits('snippet-insert', [{
        range: selection,
        text: snippet.body
      }]);
    }
  }, []);

  // 打开新建表单
  const openNewForm = useCallback(() => {
    setForm({
      name: '',
      prefix: '',
      description: '',
      body: '',
      scope: 'python'
    });
    setEditing(null);
    setShowForm(true);
  }, []);

  // 打开编辑表单
  const openEditForm = useCallback((snippet: Snippet) => {
    if (snippet.isBuiltIn) return;
    setForm({
      name: snippet.name,
      prefix: snippet.prefix,
      description: snippet.description,
      body: snippet.body,
      scope: snippet.scope
    });
    setEditing(snippet);
    setShowForm(true);
  }, []);

  // 保存片段
  const saveSnippet = useCallback(() => {
    if (!form.name || !form.prefix || !form.body) return;
    if (editing) {
      SnippetStore.update(editing.id, form);
    } else {
      SnippetStore.add(form);
    }
    setShowForm(false);
    setEditing(null);
    refresh();
  }, [form, editing, refresh]);

  // 删除片段
  const deleteSnippet = useCallback((snippet: Snippet) => {
    if (snippet.isBuiltIn) return;
    if (!confirm(t("yuan-code.SnippetPanel.k2", {
      name: snippet.name
    }))) return;
    SnippetStore.remove(snippet.id);
    refresh();
  }, [refresh]);

  // 导出片段
  const handleExport = useCallback(() => {
    const json = SnippetStore.exportAll();
    const blob = new Blob([json], {
      type: 'application/json'
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'nexterm-snippets.json';
    a.click();
    URL.revokeObjectURL(url);
  }, []);

  // 导入片段
  const handleImport = useCallback(() => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = () => {
      setImportError('');
      const file = input.files?.[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = () => {
        try {
          const count = SnippetStore.importSnippets(reader.result as string);
          refresh();
          alert(t("yuan-code.SnippetPanel.k3", {
            count: count
          }));
        } catch (e) {
          setImportError(e instanceof Error ? e.message : t("Knowledge.k99"));
        }
      };
      reader.readAsText(file);
    };
    input.click();
  }, [refresh]);
  return <div className={styles.container}>
      {/* 顶部工具栏 */}
      <div className={styles.toolbar}>
        <div className={styles.searchWrap}>
          <input className={styles.searchInput} type="text" value={searchQuery} onChange={e => setSearchQuery(e.target.value)} placeholder={t("yuan-code.SnippetPanel.k4")} />
        </div>
        <select className={styles.langSelect} value={filterLang} onChange={e => setFilterLang(e.target.value)}>
          {LANGUAGES.map(l => <option key={l.value} value={l.value}>
              {l.label}
            </option>)}
        </select>
        <button className={styles.btn} onClick={openNewForm}>
          {t("yuan-code.SkillsPanel.k24")}
        </button>
        <button className={styles.btn} onClick={handleImport}>
          {t("common.import")}
        </button>
        <button className={styles.btn} onClick={handleExport}>
          {t("common.export")}
        </button>
        {importError && <span className={styles.error}>{importError}</span>}
      </div>

      {/* 片段表单 */}
      {showForm && <div className={styles.form}>
          <div className={styles.formRow}>
            <label className={styles.formLabel}>{t("game3d.components.UI.BuildingDetail.k13")}</label>
            <input className={styles.formInput} value={form.name} onChange={e => setForm({
          ...form,
          name: e.target.value
        })} placeholder={t("yuan-code.SnippetPanel.k5")} />
          </div>
          <div className={styles.formRow}>
            <label className={styles.formLabel}>{t("yuan-code.SnippetPanel.k6")}</label>
            <input className={styles.formInput} value={form.prefix} onChange={e => setForm({
          ...form,
          prefix: e.target.value
        })} placeholder={t("yuan-code.SnippetPanel.k7")} />
          </div>
          <div className={styles.formRow}>
            <label className={styles.formLabel}>{t("common.description")}</label>
            <input className={styles.formInput} value={form.description} onChange={e => setForm({
          ...form,
          description: e.target.value
        })} placeholder={t("yuan-code.SnippetPanel.k8")} />
          </div>
          <div className={styles.formRow}>
            <label className={styles.formLabel}>{t("game3d.components.UI.BreakthroughQuiz.k19")}</label>
            <select className={styles.formInput} value={form.scope} onChange={e => setForm({
          ...form,
          scope: e.target.value
        })}>
              {LANGUAGES.filter(l => l.value !== '*').map(l => <option key={l.value} value={l.value}>
                  {l.label}
                </option>)}
            </select>
          </div>
          <div className={styles.formRow}>
            <label className={styles.formLabel}>{t("Search.k1")}</label>
            <textarea className={styles.formTextarea} value={form.body} onChange={e => setForm({
          ...form,
          body: e.target.value
        })} placeholder={t("yuan-code.SnippetPanel.k9")} rows={6} />
          </div>
          <div className={styles.formActions}>
            <button className={styles.btnPrimary} onClick={saveSnippet}>
              {t("common.save")}
            </button>
            <button className={styles.btn} onClick={() => {
          setShowForm(false);
          setEditing(null);
        }}>
              {t("common.cancel")}
            </button>
          </div>
        </div>}

      {/* 片段列表 */}
      <div className={styles.list}>
        {filtered.length === 0 ? <div className={styles.empty}>{t("yuan-code.SnippetPanel.k10")}</div> : filtered.map(snippet => <div key={snippet.id} className={styles.item}>
              <div className={styles.itemHeader}>
                <span className={styles.itemName}>{snippet.name}</span>
                <span className={styles.itemPrefix}>{snippet.prefix}</span>
                <span className={styles.itemScope}>{snippet.scope}</span>
                {snippet.isBuiltIn && <span className={styles.badge}>{t("yuan-code.SnippetPanel.k11")}</span>}
              </div>
              <div className={styles.itemDesc}>{snippet.description}</div>
              <div className={styles.itemBody}>
                <code>{snippet.body.slice(0, 80)}{snippet.body.length > 80 ? '...' : ''}</code>
              </div>
              <div className={styles.itemActions}>
                <button className={styles.btnSmall} onClick={() => insertSnippet(snippet)}>
                  {t("yuan-code.SnippetPanel.k12")}
                </button>
                {!snippet.isBuiltIn && <>
                    <button className={styles.btnSmall} onClick={() => openEditForm(snippet)}>
                      {t("common.edit")}
                    </button>
                    <button className={styles.btnSmallDanger} onClick={() => deleteSnippet(snippet)}>
                      {t("common.delete")}
                    </button>
                  </>}
              </div>
            </div>)}
      </div>
    </div>;
}