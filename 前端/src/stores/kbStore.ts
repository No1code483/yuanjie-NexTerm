import { t } from "i18next";
import { create } from 'zustand';

// 对齐后端 models/knowledge.rs

export interface KnowledgeCategory {
  id: number;
  name: string;
  parentId?: number;
  children?: KnowledgeCategory[];
  itemCount: number;
  level: number;
  expanded: boolean;
}
export interface KnowledgeItem {
  id: number;
  categoryId: number;
  title: string;
  content?: string;
  source: 'file' | 'link' | 'text';
  filePath?: string;
  url?: string;
  preview?: string;
  createdAt: string;
  updatedAt: string;
}
interface KnowledgeState {
  categories: KnowledgeCategory[];
  items: KnowledgeItem[];
  selectedCategoryId: number | null;
  selectedItem: KnowledgeItem | null;
  searchQuery: string;
  isLoading: boolean;
  isEditing: boolean;

  // Category actions
  loadCategories: (categories: KnowledgeCategory[]) => void;
  addCategory: (name: string, parentId?: number) => void;
  updateCategory: (id: number, name: string) => void;
  deleteCategory: (id: number) => void;
  toggleCategoryExpanded: (id: number) => void;
  setSelectedCategory: (id: number | null) => void;

  // Item actions
  loadItems: (items: KnowledgeItem[]) => void;
  addItem: (item: Omit<KnowledgeItem, 'id' | 'createdAt' | 'updatedAt'>) => void;
  updateItem: (id: number, updates: Partial<KnowledgeItem>) => void;
  deleteItem: (id: number) => void;
  setSelectedItem: (item: KnowledgeItem | null) => void;

  // Search & filter
  setSearchQuery: (query: string) => void;
  getFilteredItems: () => KnowledgeItem[];

  // UI state
  setLoading: (loading: boolean) => void;
  setEditing: (editing: boolean) => void;
}
type KnowledgeActions = {
  loadCategories: (categories: KnowledgeCategory[]) => void;
  addCategory: (name: string, parentId?: number) => void;
  updateCategory: (id: number, name: string) => void;
  deleteCategory: (id: number) => void;
  toggleCategoryExpanded: (id: number) => void;
  setSelectedCategory: (id: number | null) => void;
  loadItems: (items: KnowledgeItem[]) => void;
  addItem: (item: Omit<KnowledgeItem, 'id' | 'createdAt' | 'updatedAt'>) => void;
  updateItem: (id: number, updates: Partial<KnowledgeItem>) => void;
  deleteItem: (id: number) => void;
  setSelectedItem: (item: KnowledgeItem | null) => void;
  setSearchQuery: (query: string) => void;
  getFilteredItems: () => KnowledgeItem[];
  setLoading: (loading: boolean) => void;
  setEditing: (editing: boolean) => void;
};
export const useKnowledgeStore = create<KnowledgeState & KnowledgeActions>()((set, get) => ({
  categories: [
  // 预设分类（对齐项目书）
  {
    id: 1,
    name: t("lib.ipcMock.k61"),
    itemCount: 0,
    level: 0,
    expanded: false
  }, {
    id: 2,
    name: t("lib.ipcMock.k62"),
    itemCount: 0,
    level: 0,
    expanded: false
  }, {
    id: 3,
    name: t("components.AddExtensionModal.k12"),
    itemCount: 0,
    level: 0,
    expanded: false
  }, {
    id: 4,
    name: t("stores.kbStore.k1"),
    itemCount: 0,
    level: 0,
    expanded: false
  },
  // 编程开发子分类
  {
    id: 21,
    name: 'C/C++',
    parentId: 2,
    itemCount: 0,
    level: 1,
    expanded: false
  }, {
    id: 22,
    name: 'Python',
    parentId: 2,
    itemCount: 0,
    level: 1,
    expanded: false
  }, {
    id: 23,
    name: 'Java',
    parentId: 2,
    itemCount: 0,
    level: 1,
    expanded: false
  }, {
    id: 24,
    name: 'C#',
    parentId: 2,
    itemCount: 0,
    level: 1,
    expanded: false
  }, {
    id: 25,
    name: 'Rust',
    parentId: 2,
    itemCount: 0,
    level: 1,
    expanded: false
  }, {
    id: 26,
    name: 'Go',
    parentId: 2,
    itemCount: 0,
    level: 1,
    expanded: false
  }],
  items: [],
  selectedCategoryId: null,
  selectedItem: null,
  searchQuery: '',
  isLoading: false,
  isEditing: false,
  // Category actions
  loadCategories: categories => set({
    categories
  }),
  addCategory: (name, parentId) => {
    const id = Math.max(...get().categories.map(c => c.id), 0) + 1;
    const level = parentId ? (get().categories.find(c => c.id === parentId)?.level ?? 0) + 1 : 0;
    const newCategory: KnowledgeCategory = {
      id,
      name,
      parentId,
      itemCount: 0,
      level,
      expanded: false
    };
    set(state => ({
      categories: [...state.categories, newCategory]
    }));
  },
  updateCategory: (id, name) => set(state => ({
    categories: state.categories.map(c => c.id === id ? {
      ...c,
      name
    } : c)
  })),
  deleteCategory: id => set(state => ({
    categories: state.categories.filter(c => c.id !== id),
    items: state.items.filter(item => item.categoryId !== id),
    selectedCategoryId: state.selectedCategoryId === id ? null : state.selectedCategoryId
  })),
  toggleCategoryExpanded: id => set(state => ({
    categories: state.categories.map(c => c.id === id ? {
      ...c,
      expanded: !c.expanded
    } : c)
  })),
  setSelectedCategory: id => set({
    selectedCategoryId: id
  }),
  // Item actions
  loadItems: items => set({
    items
  }),
  addItem: itemData => {
    const id = Date.now();
    const now = new Date().toISOString();
    const item: KnowledgeItem = {
      ...itemData,
      id,
      createdAt: now,
      updatedAt: now
    };
    set(state => ({
      items: [...state.items, item],
      categories: state.categories.map(c => c.id === itemData.categoryId ? {
        ...c,
        itemCount: c.itemCount + 1
      } : c)
    }));
  },
  updateItem: (id, updates) => set(state => ({
    items: state.items.map(item => item.id === id ? {
      ...item,
      ...updates,
      updatedAt: new Date().toISOString()
    } : item)
  })),
  deleteItem: id => {
    const item = get().items.find(i => i.id === id);
    set(state => ({
      items: state.items.filter(i => i.id !== id),
      selectedItem: state.selectedItem?.id === id ? null : state.selectedItem,
      categories: item ? state.categories.map(c => c.id === item.categoryId ? {
        ...c,
        itemCount: Math.max(0, c.itemCount - 1)
      } : c) : state.categories
    }));
  },
  setSelectedItem: item => set({
    selectedItem: item
  }),
  // Search & filter
  setSearchQuery: query => set({
    searchQuery: query
  }),
  getFilteredItems: () => {
    const {
      items,
      selectedCategoryId,
      searchQuery
    } = get();
    let filtered = items;

    // Filter by category
    if (selectedCategoryId) {
      filtered = filtered.filter(item => item.categoryId === selectedCategoryId);
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(item => item.title.toLowerCase().includes(query) || item.content?.toLowerCase().includes(query));
    }
    return filtered;
  },
  // UI state
  setLoading: loading => set({
    isLoading: loading
  }),
  setEditing: editing => set({
    isEditing: editing
  })
}));