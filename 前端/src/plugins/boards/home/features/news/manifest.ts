import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/home/features/news/manifest.rs 逐字段对齐（L2 Feature） */
export const manifest: Manifest = {
  id: 'home.news',
  name: '新闻',
  level: 'feature',
  parent: 'boards.home',
  slot: 'home.news',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // 4 张 news 表均为既有旧表，归属登记见后端 migrations/0001_baseline.sql。
    db: ['news_cache', 'news_pending_delete', 'news_sources', 'news_offline_cache'],
    // 本批不发布/订阅领域事件（home 域事件契约属批次 1b-2c）。
    events: { subscribe: [], publish: [] },
    // ACL/handler 键由内核按本插件短码构造（<短码>_<命令名>）。
    ipc: ['nw_*'],
    fs: [],
    net: [],
  },
  slots: [],
  i18nNamespace: 'home.news',
};
