import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// T2.16 A1.5 关键路径性能优化 - Vite manualChunks 分包
//
// 设计依据：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §2.3.2
//
// 分包策略（按依赖稳定性 + 业务领域）：
// - react-vendor:    React 核心（react/react-dom/react-router-dom），稳定，长期缓存
// - state-vendor:    状态管理（zustand），稳定
// - terminal-vendor: 终端依赖（@xterm/*），仅终端路由加载
// - editor-vendor:   Monaco + TipTap 编辑器，仅元编程/知识库路由加载
// - three-vendor:    Three.js 3D 渲染，仅游戏 3D 路由加载
// - doc-vendor:      文档处理（docx/mammoth/pdfjs/pptxgenjs/handsontable），按需加载
// - i18n-vendor:     i18next 国际化，启动即加载但稳定
function manualChunks(id: string): string | undefined {
  // node_modules 中的依赖按上述分组分包
  if (id.includes('node_modules')) {
    if (id.includes('react-router') || id.includes('react-dom') || id.includes('/react/')) {
      return 'react-vendor'
    }
    if (id.includes('zustand') || id.includes('immer')) {
      return 'state-vendor'
    }
    if (id.includes('@xterm/')) {
      return 'terminal-vendor'
    }
    if (id.includes('@monaco-editor/') || id.includes('monaco-editor/') || id.includes('@tiptap/')) {
      return 'editor-vendor'
    }
    if (id.includes('three') || id.includes('@react-three/')) {
      return 'three-vendor'
    }
    if (id.includes('i18next') || id.includes('react-i18next')) {
      return 'i18n-vendor'
    }
    if (
      id.includes('docx') ||
      id.includes('mammoth') ||
      id.includes('pdfjs-dist') ||
      id.includes('pdf-lib') ||
      id.includes('pptxgenjs') ||
      id.includes('@aiden0z/pptx-renderer') ||
      id.includes('handsontable') ||
      id.includes('@handsontable/')
    ) {
      return 'doc-vendor'
    }
    // 其余 node_modules 统一打到 vendor 默认 chunk
    return 'vendor'
  }
  // 业务代码不显式分组，由 Vite 默认按路由 lazy import 自动分割
  return undefined
}

export default defineConfig({
  plugins: [react()],
  server: {
    port: 3001,
    strictPort: true
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    // T2.16 A1.5: chunk 大小警告阈值（gzip 后），超 500KB 警告
    chunkSizeWarningLimit: 500,
    rollupOptions: {
      output: {
        manualChunks,
        // 输出文件名带 hash（缓存友好）
        chunkFileNames: 'assets/[name]-[hash].js',
        entryFileNames: 'assets/[name]-[hash].js',
        assetFileNames: 'assets/[name]-[hash].[ext]'
      }
    }
  },
  resolve: {
    alias: {
      '@': '/src'
    }
  }
})
