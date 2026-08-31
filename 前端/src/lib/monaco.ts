/**
 * T2.1.7 Monaco Editor 按需加载配置
 *
 * 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §Phase 2 Task 2.1.7
 *
 * 设计说明：
 * - @monaco-editor/react 内部使用 loader 机制，Monaco 编辑器（~5MB）仅在
 *   <Editor> 组件首次挂载时加载，不会阻塞应用启动
 * - 本文件配置 loader 的加载路径，确保行为可预测、可配置
 * - 默认从 jsdelivr CDN 加载（与 @monaco-editor/react 默认行为一致）
 *
 * 离线方案（如需 Tauri 离线使用）：
 * 1. npm install monaco-editor
 * 2. 将下方 loader.config 改为：
 *    import * as monaco from 'monaco-editor';
 *    loader.config({ monaco });
 * 3. Vite manualChunks 已将 @monaco-editor/ 拆分到 editor-vendor chunk
 *
 * 验证：Monaco 仅在使用 <Editor> 组件时加载 ✅（loader.init 懒加载机制）
 */

import { loader } from '@monaco-editor/react';

// 显式配置 CDN 路径（版本与 @monaco-editor/react 内置默认一致）
// 仅配置一次，在应用启动时通过 side-effect import 触发
loader.config({
  paths: {
    vs: 'https://cdn.jsdelivr.net/npm/monaco-editor@0.52.2/min/vs',
  },
});

export { loader as monacoLoader };
