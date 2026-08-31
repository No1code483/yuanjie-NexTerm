/// <reference types="vitest" />
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import path from 'node:path'

// vitest 配置（v1.52.0.0 T2.3 前端测试基础设施）
// 与 vite.config.ts 共享 react 插件与 @ 别名；新增 test 选项
// 参见：03_测试体系_单元与集成测试.md §2.2.2（前端测试栈：vitest + jsdom + RTL + msw）
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  },
  test: {
    // jsdom 提供 DOM 环境（React 组件测试必需）
    environment: 'jsdom',
    // 启用全局 API（describe/it/expect 无需显式 import）
    globals: true,
    // setup 文件：导入 jest-dom 断言扩展 + afterEach cleanup + 全局 Tauri mock
    setupFiles: ['./src/test/setup.ts'],
    // 测试文件匹配规则
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    // 排除 node_modules / dist / e2e
    exclude: ['node_modules', 'dist', 'e2e', '**/.next/**'],
    // 覆盖率配置（@vitest/coverage-v8 需要时再装）
    coverage: {
      provider: 'v8',
      reporter: ['text', 'html', 'lcov'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/**/*.d.ts',
        'src/**/*.test.{ts,tsx}',
        'src/**/*.spec.{ts,tsx}',
        'src/test/**',
        'src/main.tsx',
        'src/vite-env.d.ts'
      ],
      // T2.5.5 覆盖率阈值：防止覆盖率回退
      // 当前整体覆盖率约 6%（v1.52.0.5），阈值设为 5% 作为回归守护
      // 目标：Phase 5 提升至 60%（03_测试体系_单元与集成测试.md §5.1）
      // 达标后逐步上调阈值：10% → 20% → 40% → 60%
      thresholds: {
        lines: 5,
        functions: 5,
        branches: 5,
        statements: 5,
      },
    }
  }
})
