// _hello 插件前端半体（阶段1 F5 完整版；manifest 数据与后端 manifest.rs 同源）
import { lazy } from 'react';
import { definePlugin } from '@/kernel/registry/definePlugin';
import { manifest } from './manifest';

export default definePlugin({
  manifest,
  contributions: {
    routes: [{ path: '/hello', component: lazy(() => import('./routes/HelloPage')) }],
    navItems: [
      {
        target: 'kernel:main',
        labelKey: '_hello:nav.hello',
        order: 99, // 新板块默认尾部（≥90）
        routePath: '/hello',
      },
    ],
    slotComponents: [
      { slot: 'hello.panel', component: lazy(() => import('./components/HelloPanel')) },
    ],
  },
});
