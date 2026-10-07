import type { Manifest } from '@/kernel/types';

/** 与后端 plugins/customs/search/manifest.rs 逐字段对齐（搜索板块；手稿 20260926：搜索为八大板块之一，
 *  level custom → board 归位）。
 *  search 不直接拥有表（复用 kb_entries / editor_documents / messages / todos / journals），
 *  仅声明 cross_module_key_prefixed_tables 为只读访问。
 */
export const manifest: Manifest = {
  id: 'customs.search',
  name: '搜索',
  level: 'board',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    db: [],
    events: { subscribe: [], publish: [] },
    ipc: ['se_*'],
    fs: [],
    net: [],
  },
  // 搜索「一切皆插件」：3 个 L2 子插件插槽（browser 必备；global/bookmarks 可选）
  slots: [
    { id: 'search.browser', type: 'panel', description: '浏览器搜索', capacity: 1 },
    { id: 'search.global', type: 'panel', description: '全站搜索', capacity: 1 },
    { id: 'search.bookmarks', type: 'panel', description: '收藏站点', capacity: 1 },
  ],
  i18nNamespace: 'search',
};