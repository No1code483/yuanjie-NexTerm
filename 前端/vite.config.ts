import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// 注：T2.16 A1.5 的强制 manualChunks 分包已移除——在 WebView2 生产环境下
// 强制分组会造成 ESM 循环依赖（React 导出对象未初始化即被访问），
// 导致打包后白屏（react-vendor 中 `Cannot set properties of undefined (setting 'Activity')`）。
// Rollup 默认按路由 lazy import 自动分割已足够；如需重新引入缓存优化，
// 必须先验证生产构建无循环依赖告警且 WebView2 中首屏可见。
export default defineConfig({
  plugins: [react()],
  server: {
    port: 3001,
    strictPort: true
  },
  build: {
    outDir: 'dist',
    sourcemap: false,
    chunkSizeWarningLimit: 500,
    rollupOptions: {
      output: {
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
