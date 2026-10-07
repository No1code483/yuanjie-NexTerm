import { useEffect, useState } from 'react';

// search.bookmarks 域：收藏站点逻辑（localStorage `nexterm_bookmarks` 的读写 + 事件派发）。
// 契约：事件名 `bookmark-add` / `bookmark-remove` 与迁移前逐字一致（Layout 侧边栏收藏列表依赖）。
export interface Bookmark {
  id: string;
  label: string;
  icon: string;
  iconUrl: string;
  url: string;
}

const BOOKMARKS_KEY = 'nexterm_bookmarks';

/** 读取收藏列表（iconUrl 归一为空串，与迁移前一致） */
export function loadBookmarks(): Bookmark[] {
  try {
    const raw = localStorage.getItem(BOOKMARKS_KEY);
    if (raw) {
      return (JSON.parse(raw) as Bookmark[]).map(bookmark => ({
        ...bookmark,
        iconUrl: ''
      }));
    }
  } catch {/* ignore */}
  return [];
}

/** 新增收藏：派发 `bookmark-add` 事件，由 Layout 负责持久化（与迁移前契约一致） */
export function addBookmark(bookmark: Bookmark): void {
  window.dispatchEvent(new CustomEvent('bookmark-add', {
    detail: bookmark
  }));
}

/** 删除收藏：写入 localStorage 并派发 `bookmark-remove` 事件 */
export function removeBookmark(url: string): void {
  const next = loadBookmarks().filter(b => b.url !== url);
  localStorage.setItem(BOOKMARKS_KEY, JSON.stringify(next));
  window.dispatchEvent(new CustomEvent('bookmark-remove'));
}

/** 订阅收藏变更（bookmark-add / bookmark-remove），返回退订函数 */
export function subscribeBookmarks(onChange: () => void): () => void {
  window.addEventListener('bookmark-add', onChange);
  window.addEventListener('bookmark-remove', onChange);
  return () => {
    window.removeEventListener('bookmark-add', onChange);
    window.removeEventListener('bookmark-remove', onChange);
  };
}

/** 收藏列表状态：初始读取 + 订阅事件自动刷新 */
export function useBookmarks(): Bookmark[] {
  const [bookmarks, setBookmarks] = useState<Bookmark[]>(loadBookmarks);
  useEffect(() => subscribeBookmarks(() => setBookmarks(loadBookmarks())), []);
  return bookmarks;
}
