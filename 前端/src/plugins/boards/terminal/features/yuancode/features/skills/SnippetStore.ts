import { t } from "i18next";
/**
 * 代码片段管理器 — 对标 VSCode workbench/snippets/
 *
 * 数据模型:
 * - Snippet: { id, name, prefix, description, body, scope, isBuiltIn }
 * - body 使用 Monaco 片段语法: $1 $2 ${1:placeholder} ${1|opt1,opt2|}
 */

import { random } from '@/lib/utils';
export interface Snippet {
  id: string;
  /** 片段名称（显示用） */
  name: string;
  /** 触发前缀（输入时触发补全） */
  prefix: string;
  /** 片段描述 */
  description: string;
  /** 代码片段内容（支持 Monaco 片段语法） */
  body: string;
  /** 作用语言（如 'python', 'javascript', 'typescript'，'*' 表示全局） */
  scope: string;
  /** 是否为内置片段（内置不可删除） */
  isBuiltIn: boolean;
}
const STORAGE_KEY = 'nexterm_yuan_snippets';

/** 从 localStorage 加载用户自定义片段 */
function loadUserSnippets(): Snippet[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return [];
}

/** 保存用户自定义片段到 localStorage */
function saveUserSnippets(snippets: Snippet[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(snippets));
}
let snippets: Snippet[] = [];
let initialized = false;
function init(): void {
  if (initialized) return;
  snippets = loadUserSnippets();
  initialized = true;
}

/** 获取所有片段 */
function getAll(): Snippet[] {
  init();
  return [...snippets];
}

/** 按语言获取片段 */
function getByScope(scope: string): Snippet[] {
  init();
  return snippets.filter(s => s.scope === scope || s.scope === '*');
}

/** 添加片段 */
function add(snippet: Omit<Snippet, 'id' | 'isBuiltIn'>): Snippet {
  init();
  const newSnippet: Snippet = {
    ...snippet,
    id: `user_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
    isBuiltIn: false
  };
  snippets.push(newSnippet);
  saveUserSnippets(snippets.filter(s => !s.isBuiltIn));
  return newSnippet;
}

/** 更新片段 */
function update(id: string, updates: Partial<Omit<Snippet, 'id' | 'isBuiltIn'>>): boolean {
  init();
  const idx = snippets.findIndex(s => s.id === id);
  if (idx === -1) return false;
  snippets[idx] = {
    ...snippets[idx],
    ...updates
  };
  saveUserSnippets(snippets.filter(s => !s.isBuiltIn));
  return true;
}

/** 删除片段 */
function remove(id: string): boolean {
  init();
  const snippet = snippets.find(s => s.id === id);
  if (!snippet || snippet.isBuiltIn) return false;
  snippets = snippets.filter(s => s.id !== id);
  saveUserSnippets(snippets.filter(s => !s.isBuiltIn));
  return true;
}

/** 注册内置片段 */
function registerBuiltIn(builtIn: Snippet[]): void {
  init();
  // 移除旧的内置片段
  snippets = snippets.filter(s => !s.isBuiltIn);
  // 添加新的内置片段
  snippets = [...builtIn, ...snippets];
}

/** 导出所有片段为 JSON */
function exportAll(): string {
  init();
  return JSON.stringify(snippets, null, 2);
}

/** 导入片段（合并模式） */
function importSnippets(json: string): number {
  init();
  try {
    const imported: Snippet[] = JSON.parse(json);
    const userSnippets = imported.filter(s => !s.isBuiltIn);
    let count = 0;
    for (const s of userSnippets) {
      // 避免重复
      if (!snippets.find(e => e.prefix === s.prefix && e.scope === s.scope)) {
        snippets.push({
          ...s,
          id: random.uid('imported_'),
          isBuiltIn: false
        });
        count++;
      }
    }
    saveUserSnippets(snippets.filter(s => !s.isBuiltIn));
    return count;
  } catch {
    throw new Error(t("yuan-code.SnippetStore.k1"));
  }
}
export const SnippetStore = {
  getAll,
  getByScope,
  add,
  update,
  remove,
  registerBuiltIn,
  exportAll,
  importSnippets
};