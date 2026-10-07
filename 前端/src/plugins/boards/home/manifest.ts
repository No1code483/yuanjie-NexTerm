import type { Manifest } from '@/kernel/types';

/** 与后端 src/plugins/boards/home/manifest.rs 逐字段对齐（L1 Board） */
export const manifest: Manifest = {
  id: 'boards.home',
  name: '首页',
  level: 'board',
  version: '0.1.0',
  kernelApi: '1',
  permissions: {
    // L1 骨架不直接持有业务表：todos / journals 归属由对应 L2 自行登记。
    db: [],
    // 本批不发布/订阅领域事件（home 域事件契约属批次 1b-2c）。
    events: { subscribe: [], publish: [] },
    ipc: [],
    fs: [],
    net: [],
  },
  // 7 个插槽承载子插件：home.todo / home.journal / home.timer / home.news
  // （todo / journal 属批次 1b-1，timer 属批次 1b-2a，news 属批次 1b-2b）+
  // home.profile（个人中心）/ home.recycle（回收站）（手稿 20260926：首页三必备子插件归位）+
  // home.focus（专注，批C4，可选）
  slots: [
    { id: 'home.profile', type: 'panel', description: '个人中心（必备）', capacity: 1 },
    { id: 'home.recycle', type: 'panel', description: '回收站（必备）', capacity: 1 },
    { id: 'home.todo', type: 'panel', description: '待办面板', capacity: 1 },
    { id: 'home.journal', type: 'panel', description: '日志面板', capacity: 1 },
    { id: 'home.news', type: 'panel', description: '新闻面板', capacity: 1 },
    { id: 'home.timer', type: 'panel', description: '计时器面板', capacity: 1 },
    { id: 'home.focus', type: 'panel', description: '专注', capacity: 1 },
  ],
  i18nNamespace: 'home',
};
