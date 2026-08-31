import { t } from "i18next";
import { useState, useCallback, useEffect } from 'react';
import {
  DndContext,
  PointerSensor,
  KeyboardSensor,
  useSensor,
  useSensors,
  closestCenter,
} from '@dnd-kit/core';
import {
  SortableContext,
  useSortable,
  arrayMove,
  verticalListSortingStrategy,
  sortableKeyboardCoordinates,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import type { AIModel, Conversation, Participant } from './types';
import { time } from '@/lib/utils';
import { formatRelativeTime } from '@/lib/l10n';
import { ai } from '@/lib/ipc';
import styles from '../AI.module.css';
interface Props {
  conversations: Conversation[];
  loadingConversations: boolean;
  activeChat: number | null;
  models: AIModel[];
  participants: Map<number, Participant[]>;
  onSelect: (id: number) => void;
  onDelete: (id: number) => void;
  onCreate: () => void;
  onRefresh: () => void;
}
type FilterMode = 'all' | 'starred' | 'today' | 'week' | 'month';
// spec ai-chat-enhancement Phase 2 Task 12: 排序模式
// - custom: 按 sort_order ASC, updated_at DESC（拖拽自定义顺序）
// - time:   按 updated_at DESC（忽略 sort_order）
type SortMode = 'custom' | 'time';

// spec ai-chat-enhancement Phase 2 Task 10: 会话右键菜单状态
interface ConvMenuState {
  visible: boolean;
  x: number;
  y: number;
  convId: number | null;
}

// spec ai-chat-enhancement Phase 2 Task 9/10/11/12: 可拖拽会话列表项
// 三行布局（标题/预览/元信息）+ 双击重命名 + 拖拽手柄 + 右键菜单触发
function SortableConversationItem({
  conv,
  activeChat,
  participants,
  onSelect,
  onToggleStar,
  onContextMenu,
  editing,
  editValue,
  onStartEdit,
  onEditChange,
  onEditSave,
  onEditCancel,
  sortMode
}: {
  conv: Conversation;
  activeChat: number | null;
  participants: Map<number, Participant[]>;
  onSelect: (id: number) => void;
  onToggleStar: (id: number, e: React.MouseEvent) => void;
  onContextMenu: (e: React.MouseEvent, convId: number) => void;
  editing: boolean;
  editValue: string;
  onStartEdit: () => void;
  onEditChange: (v: string) => void;
  onEditSave: () => void;
  onEditCancel: () => void;
  sortMode: SortMode;
}) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({ id: conv.id });
  const parts = participants.get(conv.id) || [];
  const modelParts = parts.filter(p => p.model_id);
  const isMulti = modelParts.length > 1;

  // Task 9.2: 未读数 + 最后消息预览 + 相对时间
  const unread = conv.unread_count ?? 0;
  const preview = conv.last_message_preview || '';
  const senderType = conv.last_message_sender_type;
  const previewPrefix = senderType === 'assistant' || senderType === 'model' ? '🤖:' : senderType === 'user' ? '👤:' : '';
  const previewText = preview ? `${previewPrefix} ${preview.slice(0, 50)}`.trim() : '';
  const lastMsgAt = conv.last_message_at;
  const timeText = lastMsgAt ? formatRelativeTime(lastMsgAt) : time.formatUtcToLocal(conv.updated_at);

  const style: React.CSSProperties = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1
  };

  return <div ref={setNodeRef} style={style} className={`${styles.sidebarItem} ${activeChat === conv.id ? styles.sidebarItemActive : ''} ${isDragging ? styles.dragging : ''}`} onClick={() => {
    if (!editing) onSelect(conv.id);
  }} onContextMenu={(e) => onContextMenu(e, conv.id)}>
      <div className={styles.sidebarItemHeader}>
        {editing ? <input className={styles.editInput} autoFocus value={editValue} onChange={(e) => onEditChange(e.target.value)} onClick={(e) => e.stopPropagation()} onFocus={(e) => e.target.select()} onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            onEditSave();
          } else if (e.key === 'Escape') {
            e.preventDefault();
            onEditCancel();
          }
        }} onBlur={() => onEditSave()} /> : <span className={styles.sidebarItemName} onDoubleClick={(e) => {
          e.stopPropagation();
          onStartEdit();
        }}>{conv.title || t("ai.ConversationList.k9", {
          id: conv.id
        })}</span>}
        <div className={styles.sidebarItemActions}>
          {sortMode === 'custom' && !editing && <button className={styles.dragHandle} onClick={(e) => e.stopPropagation()} title="拖拽排序" {...attributes} {...listeners}>⋮⋮</button>}
          <button className={styles.starBtn} onClick={(e) => onToggleStar(conv.id, e)}>
            {conv.starred ? '★' : '☆'}
          </button>
          {unread > 0 && !editing && <span className={styles.unreadBadge}>未读 {unread}</span>}
        </div>
      </div>
      {previewText && !editing && <div className={styles.messagePreview}>{previewText}</div>}
      {!editing && <div className={styles.metaRow}>
          <span className={`${styles.convTypeTag} ${isMulti ? styles.convTypeMulti : styles.convTypeSingle}`}>
            {isMulti ? t("ai.ConversationList.k10", {
              length: modelParts.length
            }) : t("ai.ConversationList.k11")}
          </span>
          <span>{timeText}</span>
        </div>}
    </div>;
}

export default function ConversationList({
  conversations,
  loadingConversations,
  activeChat,
  participants,
  onSelect,
  onDelete,
  onCreate,
  onRefresh
}: Props) {
  const singleConvs = conversations.filter(c => c.type !== 'group');
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<{
    conversation_id: number;
    title: string | null;
    matched_message_preview: string;
    updated_at: number;
  }[] | null>(null);
  const [loadingSearch, setLoadingSearch] = useState(false);
  const [filterMode, setFilterMode] = useState<FilterMode>('all');
  // Task 12: 排序模式
  const [sortMode, setSortMode] = useState<SortMode>('time');
  // Task 10: 右键菜单状态
  const [convMenu, setConvMenu] = useState<ConvMenuState>({
    visible: false,
    x: 0,
    y: 0,
    convId: null
  });
  // Task 11: 重命名编辑状态
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editValue, setEditValue] = useState('');
  // Task 9.3: 未读数乐观更新（隐藏当前激活会话的未读红点，避免等待 onRefresh）
  const [hiddenUnread, setHiddenUnread] = useState<Set<number>>(new Set());

  // Task 12: 拖拽传感器（Pointer + Keyboard）
  const sensors = useSensors(useSensor(PointerSensor, {
    activationConstraint: {
      distance: 5
    }
  }), useSensor(KeyboardSensor, {
    coordinateGetter: sortableKeyboardCoordinates
  }));

  const handleSearch = useCallback(async (q: string) => {
    setSearchQuery(q);
    if (!q.trim()) {
      setSearchResults(null);
      return;
    }
    setLoadingSearch(true);
    try {
      const res = await ai.searchConversations(q.trim());
      setSearchResults(res.data || []);
    } catch {
      setSearchResults([]);
    } finally {
      setLoadingSearch(false);
    }
  }, []);

  const handleToggleStarById = async (id: number) => {
    try {
      await ai.toggleStar(id);
      onRefresh();
    } catch {/* ignore */}
  };
  const handleToggleStar = async (id: number, e: React.MouseEvent) => {
    e.stopPropagation();
    handleToggleStarById(id);
  };

  // Task 9.3: 打开会话时调用 markConversationRead 清零未读数（乐观更新）
  const handleSelect = (id: number) => {
    onSelect(id);
    // 乐观更新：立即隐藏当前会话的未读红点
    setHiddenUnread(new Set([id]));
    ai.markConversationRead(id).catch(() => {
      // 失败则恢复（下次 onRefresh 会拉取真实 unread_count）
      setHiddenUnread(new Set());
    });
  };

  // Task 11: 双击重命名
  const handleStartEdit = (conv: Conversation) => {
    setEditValue(conv.title || '');
    setEditingId(conv.id);
  };
  const handleEditSave = async () => {
    const id = editingId;
    if (id === null) return;
    const newTitle = editValue.trim();
    setEditingId(null);
    if (!newTitle) return;
    const conv = conversations.find(c => c.id === id);
    if (!conv || conv.title === newTitle) return;
    try {
      await ai.updateConversation({
        id,
        title: newTitle
      });
      onRefresh();
    } catch {/* ignore */}
  };
  const handleEditCancel = () => {
    setEditingId(null);
  };

  // Task 10: 右键菜单
  const handleContextMenu = (e: React.MouseEvent, convId: number) => {
    e.preventDefault();
    e.stopPropagation();
    setConvMenu({
      visible: true,
      x: e.clientX,
      y: e.clientY,
      convId
    });
  };
  // Task 10.4: 点击空白处关闭菜单
  useEffect(() => {
    if (!convMenu.visible) return;
    const closeMenu = () => setConvMenu(prev => ({
      ...prev,
      visible: false
    }));
    window.addEventListener('click', closeMenu);
    window.addEventListener('contextmenu', closeMenu);
    return () => {
      window.removeEventListener('click', closeMenu);
      window.removeEventListener('contextmenu', closeMenu);
    };
  }, [convMenu.visible]);

  // Task 10: 菜单操作 — 置顶/取消置顶（通过 reorder_conversations 设置 sort_order=-1 或 0）
  const handleTogglePin = async (convId: number) => {
    const conv = conversations.find(c => c.id === convId);
    if (!conv) return;
    const newSortOrder = (conv.sort_order ?? 0) === -1 ? 0 : -1;
    try {
      await ai.reorderConversations([{
        id: convId,
        sort_order: newSortOrder
      }]);
      onRefresh();
    } catch {/* ignore */}
  };
  const handleCopyTitle = (convId: number) => {
    const conv = conversations.find(c => c.id === convId);
    navigator.clipboard.writeText(conv?.title || '').catch(() => {/* ignore */});
  };
  const handleExportMarkdown = async (convId: number) => {
    try {
      const res = await ai.exportConversation(convId, 'markdown');
      if (res.data) {
        const blob = new Blob([res.data], {
          type: 'text/markdown;charset=utf-8'
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `conversation-${convId}.md`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
      }
    } catch {/* ignore */}
  };
  const handleDelete = (convId: number) => {
    if (!window.confirm('确认删除此会话？')) return;
    onDelete(convId);
  };

  const todayStart = new Date();
  todayStart.setHours(0, 0, 0, 0);
  const weekStart = new Date();
  weekStart.setDate(weekStart.getDate() - weekStart.getDay());
  weekStart.setHours(0, 0, 0, 0);
  const monthStart = new Date();
  monthStart.setDate(1);
  monthStart.setHours(0, 0, 0, 0);

  // Task 9/12: 过滤 + 排序（保留原有星标置顶行为）
  const getFilteredConvs = () => {
    let list = singleConvs;
    if (filterMode === 'starred') list = list.filter(c => c.starred);
    else if (filterMode === 'today') list = list.filter(c => c.updated_at >= todayStart.getTime());
    else if (filterMode === 'week') list = list.filter(c => c.updated_at >= weekStart.getTime());
    else if (filterMode === 'month') list = list.filter(c => c.updated_at >= monthStart.getTime());
    return [...list].sort((a, b) => {
      // 星标置顶（保留原有行为）
      if (a.starred && !b.starred) return -1;
      if (!a.starred && b.starred) return 1;
      // Task 12: 自定义模式下按 sort_order 排序
      if (sortMode === 'custom') {
        const sa = a.sort_order ?? 0;
        const sb = b.sort_order ?? 0;
        if (sa !== sb) return sa - sb;
      }
      return b.updated_at - a.updated_at;
    });
  };

  // Task 12: 拖拽完成 — 计算新顺序并持久化
  const handleDragEnd = async (event: any) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;
    const currentList = getFilteredConvs();
    const oldIndex = currentList.findIndex(c => c.id === active.id);
    const newIndex = currentList.findIndex(c => c.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;
    const newOrder = arrayMove(currentList, oldIndex, newIndex);
    const items = newOrder.map((c, i) => ({
      id: c.id,
      sort_order: i
    }));
    try {
      await ai.reorderConversations(items);
      onRefresh();
    } catch {/* ignore */}
  };

  const isSearching = searchQuery.trim() !== '';
  const filteredConvs = getFilteredConvs();

  return <div className={styles.sidebarList}>
      <button className={styles.addBtn} onClick={onCreate}>{t("ai.ConversationList.k1")}</button>

      {/* 搜索框 */}
      <input type="text" className={styles.convSearchInput} placeholder={t("ai.ConversationList.k2")} value={searchQuery} onChange={e => handleSearch(e.target.value)} />

      {/* 过滤器 */}
      <div className={styles.filterRow}>
        {(['all', 'starred', 'today', 'week', 'month'] as FilterMode[]).map(mode => <button key={mode} className={`${styles.filterBtn} ${filterMode === mode ? styles.filterBtnActive : ''}`} onClick={() => setFilterMode(mode)}>
            {mode === 'all' ? t("common.all") : mode === 'starred' ? t("ai.ConversationList.k3") : mode === 'today' ? t("common.today") : mode === 'week' ? t("components.intelligence.DashboardPanel.k43") : t("components.intelligence.DashboardPanel.k44")}
          </button>)}
      </div>

      {/* Task 12: 排序模式切换（自定义顺序 / 时间倒序） */}
      <div className={styles.sortModeToggle}>
        <button className={`${styles.filterBtn} ${sortMode === 'time' ? styles.filterBtnActive : ''}`} onClick={() => setSortMode('time')}>时间倒序</button>
        <button className={`${styles.filterBtn} ${sortMode === 'custom' ? styles.filterBtnActive : ''}`} onClick={() => setSortMode('custom')}>自定义顺序</button>
      </div>

      {/* 搜索结果计数 */}
      {isSearching && <div className={styles.searchCount}>
          {loadingSearch ? t("ai.ConversationList.k4") : t("ai.ConversationList.k5", {
        arg0: searchResults?.length ?? 0
      })}
        </div>}

      {loadingConversations ? <div className={styles.loadingHint}>{t("common.loading")}</div> : isSearching ? searchResults && searchResults.length > 0 ? searchResults.map(result => {
      const conv = singleConvs.find(c => c.id === result.conversation_id);
      return <div key={result.conversation_id} className={`${styles.sidebarItem} ${activeChat === result.conversation_id ? styles.sidebarItemActive : ''}`} onClick={() => {
        if (conv) handleSelect(result.conversation_id);
      }}>
                <div className={styles.sidebarItemHeader}>
                  <span className={styles.sidebarItemName}>{result.title || t("ai.ConversationList.k6", {
              conversation_id: result.conversation_id
            })}</span>
                  {conv && <button className={styles.starBtn} onClick={e => handleToggleStar(result.conversation_id, e)}>
                      {conv.starred ? '★' : '☆'}
                    </button>}
                </div>
                <div className={styles.searchPreview}>{result.matched_message_preview}</div>
              </div>;
    }) : <div className={styles.emptyHint}>{t("ai.ConversationList.k7")}</div> : filteredConvs.length === 0 ? <div className={styles.emptyHint}>{t("ai.ConversationList.k8")}</div> : <DndContext sensors={sensors} collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
            <SortableContext items={filteredConvs.map(c => c.id)} strategy={verticalListSortingStrategy}>
              {filteredConvs.map(conv => {
          // Task 9.3: 乐观更新 — 隐藏当前激活会话的未读红点
          const effectiveConv = hiddenUnread.has(conv.id) ? {
            ...conv,
            unread_count: 0
          } : conv;
          return <SortableConversationItem key={conv.id} conv={effectiveConv} activeChat={activeChat} participants={participants} onSelect={handleSelect} onToggleStar={handleToggleStar} onContextMenu={handleContextMenu} editing={editingId === conv.id} editValue={editValue} onStartEdit={() => handleStartEdit(conv)} onEditChange={setEditValue} onEditSave={handleEditSave} onEditCancel={handleEditCancel} sortMode={sortMode} />;
        })}
            </SortableContext>
          </DndContext>}
      
      {/* Task 10: 会话右键菜单（终端黑客风格，复用现有 .contextMenu 样式） */}
      {convMenu.visible && convMenu.convId !== null && <div className={styles.contextMenu} style={{
      left: convMenu.x,
      top: convMenu.y
    }} onClick={(e) => e.stopPropagation()} onContextMenu={(e) => {
      e.preventDefault();
      e.stopPropagation();
    }}>
          <button className={styles.contextMenuItem} onClick={() => {
        handleTogglePin(convMenu.convId!);
        setConvMenu(prev => ({
          ...prev,
          visible: false
        }));
      }}>{(conversations.find(c => c.id === convMenu.convId)?.sort_order ?? 0) === -1 ? '取消置顶' : '置顶'}</button>
          <button className={styles.contextMenuItem} onClick={() => {
        handleToggleStarById(convMenu.convId!);
        setConvMenu(prev => ({
          ...prev,
          visible: false
        }));
      }}>{conversations.find(c => c.id === convMenu.convId)?.starred ? '取消星标' : '星标'}</button>
          <button className={styles.contextMenuItem} onClick={() => {
        const c = conversations.find(c => c.id === convMenu.convId);
        if (c) handleStartEdit(c);
        setConvMenu(prev => ({
          ...prev,
          visible: false
        }));
      }}>重命名</button>
          <button className={styles.contextMenuItem} onClick={() => {
        handleCopyTitle(convMenu.convId!);
        setConvMenu(prev => ({
          ...prev,
          visible: false
        }));
      }}>复制标题</button>
          <button className={styles.contextMenuItem} onClick={() => {
        handleExportMarkdown(convMenu.convId!);
        setConvMenu(prev => ({
          ...prev,
          visible: false
        }));
      }}>导出为 Markdown</button>
          <div className={styles.contextMenuDivider} />
          <button className={`${styles.contextMenuItem} ${styles.contextMenuItemDanger}`} onClick={() => {
        handleDelete(convMenu.convId!);
        setConvMenu(prev => ({
          ...prev,
          visible: false
        }));
      }}>删除</button>
        </div>}
    </div>;
}
