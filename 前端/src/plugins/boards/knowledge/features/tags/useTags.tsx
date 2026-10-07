// knowledge.tags L2 功能域：标签（TagBar / 条目标签行 / 过滤 chip + 全部标签 handler）。
// 局部 state + handlers + JSX 物理落在本目录；核心共享数据经 KnowledgeCore 注入。
import { t } from "i18next";
import { useState } from 'react';
import { kb } from '../../ipc';
import styles from '../../Knowledge.module.css';
import TagBar from '../../knowledge/TagBar';
import type { KbEntry, KbTag, KnowledgeCore } from '../../knowledge/types';

export function useTags(core: KnowledgeCore) {
  const [editingTagId, setEditingTagId] = useState<number | null>(null);
  const [editingTagName, setEditingTagName] = useState('');
  const [editingTagColor, setEditingTagColor] = useState('#00F0FF');

  const handleTagFilter = (tagId: number) => {
    core.setViewMode('tag');
    core.setViewFilterId(tagId);
  };

  const handleAddTagToEntry = async (entryId: number, tagId: number) => {
    try {
      const existing = core.entryTags.get(entryId)?.map(tg => tg.id) || [];
      const ids = [...new Set([...existing, tagId])];
      const res = await kb.setKbEntryTags({
        entryId: entryId,
        tagIds: ids
      });
      if (res.code === 0) {
        core.loadEntryTags(entryId);
        core.showStatus('success', t("Knowledge.k31"));
      } else core.showStatus('error', res.message || t("Knowledge.k32"));
    } catch {
      core.showStatus('error', t("Knowledge.k32"));
    }
  };

  const handleRemoveTagFromEntry = async (entryId: number, tagId: number) => {
    try {
      const existing = core.entryTags.get(entryId)?.map(tg => tg.id) || [];
      const ids = existing.filter(id => id !== tagId);
      const res = await kb.setKbEntryTags({
        entryId: entryId,
        tagIds: ids
      });
      if (res.code === 0) {
        core.loadEntryTags(entryId);
        core.showStatus('success', t("Knowledge.k33"));
      } else core.showStatus('error', res.message || t("Knowledge.k34"));
    } catch {
      core.showStatus('error', t("Knowledge.k34"));
    }
  };

  const handleAddGlobalTag = async (name: string) => {
    if (!name.trim()) return;
    try {
      const hues = [280, 200, 340, 160, 40, 100, 10, 50, 190, 320];
      const h = hues[Math.floor(Math.random() * hues.length)];
      const color = `hsl(${h}, 65%, 55%)`;
      const res = await kb.addKbTag({
        name: name.trim(),
        color
      });
      if (res.code === 0) {
        await core.loadTags();
        await core.loadTagStats();
        core.showStatus('success', t("Knowledge.k35", { arg0: name.trim() }));
      } else core.showStatus('error', res.message || t("Knowledge.k10"));
    } catch {
      core.showStatus('error', t("Knowledge.k36"));
    }
  };

  const handleDeleteGlobalTag = async (tagId: number) => {
    try {
      const res = await kb.deleteKbTag({ id: tagId });
      if (res.code === 0) {
        await core.loadTags();
        await core.loadTagStats();
        core.showStatus('success', t("Knowledge.k37"));
      } else core.showStatus('error', res.message || t("errors.deleteFailed"));
    } catch {
      core.showStatus('error', t("Knowledge.k38"));
    }
  };

  const handleUpdateGlobalTag = async () => {
    if (!editingTagId || !editingTagName.trim()) return;
    try {
      const res = await kb.updateKbTag({
        id: editingTagId,
        name: editingTagName.trim(),
        color: editingTagColor
      });
      if (res.code === 0) {
        await core.loadTags();
        await core.loadTagStats();
        await core.loadAllEntryTags();
        core.showStatus('success', t("Knowledge.k39"));
        setEditingTagId(null);
      } else core.showStatus('error', res.message || t("Knowledge.k40"));
    } catch {
      core.showStatus('error', t("Knowledge.k41"));
    }
  };

  const startEditTag = (tag: KbTag) => {
    setEditingTagId(tag.id);
    setEditingTagName(tag.name);
    setEditingTagColor(tag.color);
  };

  /** 侧边栏标签栏（TagBar） */
  const renderTagBar = () => (
    <TagBar
      tags={core.tags}
      tagStats={core.tagStats}
      activeTagId={core.viewMode === 'tag' ? core.viewFilterId : null}
      editingTagId={editingTagId}
      editingTagName={editingTagName}
      editingTagColor={editingTagColor}
      onTagFilter={handleTagFilter}
      onStartEdit={startEditTag}
      onDeleteTag={handleDeleteGlobalTag}
      onUpdateTag={handleUpdateGlobalTag}
      onCancelEdit={() => setEditingTagId(null)}
      onEditingNameChange={setEditingTagName}
      onEditingColorChange={setEditingTagColor}
      onAddTag={handleAddGlobalTag}
    />
  );

  /** 预览区条目标签行 */
  const renderEntryTagRow = (entry: KbEntry) => (
    <div className={styles.tagRow}>
      <span className={styles.tagRowLabel}>{t("Knowledge.k248")}</span>
      {core.entryTags.get(entry.id)?.map(tg => (
        <span key={tg.id} className={styles.tagChipSm} style={{
          borderColor: tg.color + '55',
          background: tg.color + '15'
        }}>
          <span className={styles.tagColorDot} style={{ background: tg.color }} />
          {tg.name}
          <button className={styles.tagDelBtn} onClick={() => handleRemoveTagFromEntry(entry.id, tg.id)}>×</button>
        </span>
      ))}
      <select className={styles.tagAddSelect} value="" onChange={e => {
        if (e.target.value) {
          handleAddTagToEntry(entry.id, Number(e.target.value));
          e.target.value = '';
        }
      }}>
        <option value="">{t("Knowledge.k249")}</option>
        {core.tags.filter(tg => !core.entryTags.get(entry.id)?.some(et => et.id === tg.id)).map(tg => <option key={tg.id} value={tg.id}>{tg.name}</option>)}
      </select>
    </div>
  );

  /** 标签/类型过滤 chip */
  const renderFilterChips = () => {
    if (core.viewMode !== 'tag' && core.typeFilter === 'all') return null;
    return (
      <div className={styles.filterChips}>
        {core.viewMode === 'tag' && core.viewFilterId && <span className={styles.filterChip}>
            🏷️ {core.tags.find(tg => tg.id === core.viewFilterId)?.name || t("Knowledge.k221")}
            <button className={styles.filterChipDel} onClick={() => {
              core.setViewMode('all');
              core.setViewFilterId(null);
            }}>×</button>
          </span>}
        {core.typeFilter !== 'all' && <span className={styles.filterChip}>
            {core.getTypeIcon(core.typeFilter)} {core.getTypeLabel(core.typeFilter)}
            <button className={styles.filterChipDel} onClick={() => core.setTypeFilter('all')}>×</button>
          </span>}
      </div>
    );
  };

  return {
    renderTagBar,
    renderEntryTagRow,
    renderFilterChips,
    handleTagFilter,
    handleAddTagToEntry,
    handleRemoveTagFromEntry
  };
}
