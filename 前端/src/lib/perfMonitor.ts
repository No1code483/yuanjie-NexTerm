/**
 * 前端性能监控核心（v1.52 性能基线）
 *
 * 职责：
 * 1. 启动耗时测量（performance.mark/measure API）
 * 2. Web Vitals 采集（LCP / FCP / CLS / TTFB / INP）
 * 3. 通过 IPC 批量上报到后端 perf_metrics 表
 *
 * 参见：01_性能优化_首屏200ms计划.md §2.1.2
 *
 * 设计原则：
 * - 静默失败：性能监控任何异常都不影响业务
 * - 批量上报：5s 定时刷新 + 满 20 条立即刷新 + beforeunload 兜底
 * - 无侵入：业务代码只需调用 markStartupBegin/markStartupEnd
 *
 * 使用方式：
 *   import { initPerfMonitor, markStartupBegin, markStartupEnd } from '@/lib/perfMonitor'
 *   initPerfMonitor()        // 应用入口调用一次
 *   markStartupBegin()       // 最早执行
 *   markStartupEnd()         // 首屏渲染完成后
 */

// ===== 类型定义 =====
interface PerfMetricRecord {
  metric_name: string
  metric_value_ms: number
  route?: string
  command_name?: string
  metadata?: string
}

// ===== 上报队列（批量刷新，减少 IPC 调用）=====
const FLUSH_INTERVAL_MS = 5000
const MAX_QUEUE_SIZE = 20

let reportQueue: PerfMetricRecord[] = []
let flushTimer: ReturnType<typeof setTimeout> | null = null
let ipcInvoke: ((command: string, args?: any) => Promise<any>) | null = null

/**
 * 懒加载 IPC invoke，避免循环依赖
 * perfMonitor 不应在模块加载时就触发 ipc.ts 的初始化
 */
async function getInvoke(): Promise<typeof ipcInvoke> {
  if (ipcInvoke) return ipcInvoke
  try {
    const { ipc } = await import('@/lib/ipc')
    ipcInvoke = ipc.invoke.bind(ipc)
    return ipcInvoke
  } catch (e) {
    console.warn('[perfMonitor] IPC 加载失败，性能数据将丢弃:', e)
    return null
  }
}

/**
 * 立即刷新队列，把积压的指标全部上报到后端
 */
async function flush(): Promise<void> {
  if (flushTimer) {
    clearTimeout(flushTimer)
    flushTimer = null
  }
  if (reportQueue.length === 0) return

  const batch = reportQueue.splice(0)
  const invoke = await getInvoke()
  if (!invoke) return

  // 逐条上报（后端命令设计为单条 INSERT，避免事务复杂度）
  // 失败静默：性能监控不能阻塞业务
  await Promise.all(
    batch.map((r) =>
      invoke('record_perf_metric', r).catch(() => {
        /* 静默丢弃 */
      })
    )
  )
}

/**
 * 入队一条性能指标
 */
export function recordMetric(
  metricName: string,
  metricValueMs: number,
  route?: string,
  commandName?: string,
  metadata?: Record<string, unknown>
): void {
  // Tauri 环境检测（v1.51.5 修复）
  //
  // 历史 bug（v1.51.3）：原检测依赖 window.__TAURI__ + import.meta.env.TAURI_PLATFORM
  //   - Tauri 2.x 默认不暴露 window.__TAURI__（需 app.withGlobalTauri: true，本项目未启用）
  //   - import.meta.env.TAURI_PLATFORM 需要 @tauri-apps/cli 的 Vite 插件注入，本项目未引入
  //   - 结果：isTauri 始终为 false，所有 recordMetric 调用被静默丢弃
  //
  // 修复方案（v1.51.5）：改用 window.__TAURI_INTERNALS__
  //   - Tauri 2.x 的内部 invoke 机制，始终在 webview 注入（即使用户未启用 withGlobalTauri）
  //   - 同时保留 __TAURI__ 检测以兼容 Tauri 1.x 和启用 withGlobalTauri 的场景
  //
  // 参考：https://tauri.app/v2/guides/debug/
  //       https://v2.tauri.app/reference/javascript/api/namespacecore/#isTauri
  const isTauri =
    typeof window !== 'undefined' &&
    ('__TAURI_INTERNALS__' in window || '__TAURI__' in window)

  if (!isTauri) {
    // 非 Tauri 环境（纯浏览器开发）直接丢弃
    // 调试模式可解开下一行查看是否真的未在 Tauri 中
    // console.debug('[perfMonitor] 非 Tauri 环境，丢弃指标:', metricName)
    return
  }

  const record: PerfMetricRecord = {
    metric_name: metricName,
    metric_value_ms: Math.round(metricValueMs),
    route,
    command_name: commandName,
    metadata: metadata ? JSON.stringify(metadata) : undefined,
  }
  reportQueue.push(record)

  // 满额立即刷新
  if (reportQueue.length >= MAX_QUEUE_SIZE) {
    void flush()
    return
  }
  // 否则启动/重置定时器
  if (!flushTimer) {
    flushTimer = setTimeout(() => void flush(), FLUSH_INTERVAL_MS)
  }
}

// ===== 启动耗时测量 =====
const STARTUP_MARK = 'nexterm:startup-begin'

/**
 * 标记应用启动开始
 * 应在 App.tsx 模块最早执行的位置调用（或 main.tsx 入口）
 */
export function markStartupBegin(): void {
  if (performance.getEntriesByName(STARTUP_MARK).length > 0) return
  performance.mark(STARTUP_MARK)
}

/**
 * 标记应用启动结束（首屏渲染完成）
 * 自动计算 startup_total_ms 并上报
 * @param label 可选，区分多个阶段（如 'first-paint' / 'interactive'）
 */
export function markStartupEnd(label: string = 'first-paint'): void {
  const endMark = `nexterm:startup-end-${label}`
  try {
    performance.mark(endMark)
    const measure = performance.measure(
      `nexterm:startup-${label}`,
      STARTUP_MARK,
      endMark
    )
    recordMetric(
      'startup_total_ms',
      measure.duration,
      window.location?.pathname,
      undefined,
      { label }
    )
    console.log(
      `[perfMonitor] 🚀 启动耗时 (${label}): ${Math.round(measure.duration)}ms`
    )
  } catch (e) {
    console.warn('[perfMonitor] 启动耗时测量失败:', e)
  }
}

// ===== 通用区间测量 =====
/**
 * 标记区间开始
 */
export function markStart(name: string): void {
  performance.mark(`nexterm:${name}-start`)
}

/**
 * 标记区间结束并上报
 * @returns 测量值（ms），失败返回 0
 */
export function markEnd(
  name: string,
  route?: string,
  metadata?: Record<string, unknown>
): number {
  const startMark = `nexterm:${name}-start`
  const endMark = `nexterm:${name}-end`
  try {
    performance.mark(endMark)
    const measure = performance.measure(`nexterm:${name}`, startMark, endMark)
    const duration = Math.round(measure.duration)
    recordMetric(name, duration, route, undefined, metadata)
    return duration
  } catch (e) {
    console.warn(`[perfMonitor] 区间测量 ${name} 失败:`, e)
    return 0
  }
}

// ===== IPC 调用耗时包裹器 =====
/**
 * 包裹一个 IPC 调用，自动测量并上报耗时
 * @param command 命令名
 * @param fn 实际调用函数
 * @returns fn 的返回值
 */
export async function measureIPC<T>(
  command: string,
  fn: () => Promise<T>
): Promise<T> {
  const start = performance.now()
  try {
    const result = await fn()
    const duration = performance.now() - start
    recordMetric('ipc_duration_ms', duration, undefined, command)
    return result
  } catch (e) {
    const duration = performance.now() - start
    recordMetric('ipc_error_ms', duration, undefined, command)
    throw e
  }
}

// ===== Web Vitals 采集 =====
/**
 * 初始化 Web Vitals 观察者
 * 采集 LCP / FCP / CLS / TTFB / INP 五大核心指标
 * 仅在支持 PerformanceObserver 的环境生效
 */
export function initWebVitals(): void {
  if (typeof PerformanceObserver === 'undefined') {
    console.warn('[perfMonitor] PerformanceObserver 不可用，跳过 Web Vitals')
    return
  }

  observeLCP()
  observeFCP()
  observeCLS()
  recordTTFB()
  observeINP()
}

function observeLCP(): void {
  try {
    const po = new PerformanceObserver((list) => {
      const entries = list.getEntries()
      const last = entries[entries.length - 1]
      if (last) {
        recordMetric('lcp_ms', last.startTime, window.location?.pathname)
      }
    })
    po.observe({ type: 'largest-contentful-paint', buffered: true })
  } catch {
    /* 浏览器不支持该类型，静默 */
  }
}

function observeFCP(): void {
  try {
    const po = new PerformanceObserver((list) => {
      const entry = list.getEntries()[0]
      if (entry) {
        recordMetric('fcp_ms', entry.startTime, window.location?.pathname)
      }
    })
    po.observe({ type: 'paint', buffered: true })
  } catch {
    /* 静默 */
  }
}

function observeCLS(): void {
  try {
    let clsValue = 0
    const po = new PerformanceObserver((list) => {
      for (const entry of list.getEntries() as any[]) {
        if (!entry.hadRecentInput) {
          clsValue += entry.value
        }
      }
      // CLS 是累计值，每次更新都记录最新累计
      recordMetric('cls', Math.round(clsValue * 1000), window.location?.pathname)
    })
    po.observe({ type: 'layout-shift', buffered: true })
  } catch {
    /* 静默 */
  }
}

function recordTTFB(): void {
  try {
    const navEntries = performance.getEntriesByType('navigation')
    const nav = navEntries[0] as PerformanceNavigationTiming | undefined
    if (nav && nav.responseStart > 0) {
      recordMetric(
        'ttfb_ms',
        nav.responseStart - nav.requestStart,
        window.location?.pathname
      )
    }
  } catch {
    /* 静默 */
  }
}

function observeINP(): void {
  try {
    const po = new PerformanceObserver((list) => {
      for (const entry of list.getEntries() as any[]) {
        const duration = entry.duration || 0
        if (duration > 0) {
          recordMetric('inp_ms', duration, window.location?.pathname)
        }
      }
    })
    po.observe({ type: 'event', buffered: true })
  } catch {
    /* 静默 */
  }
}

// ===== 一站式初始化入口 =====
/**
 * 初始化性能监控
 * 应在 App.tsx 最早执行的 useEffect 中调用一次
 * - 标记启动开始
 * - 注册 Web Vitals 观察者
 * - 注册 beforeunload 兜底刷新
 */
export function initPerfMonitor(): void {
  if (typeof window === 'undefined') return

  // 诊断日志（v1.51.5）：明确打印 Tauri 环境检测结果
  const isTauriEnv =
    '__TAURI_INTERNALS__' in window || '__TAURI__' in window
  console.log(
    `[perfMonitor] 📊 性能监控初始化中... Tauri 环境检测: ${isTauriEnv ? '✅ 已检测到' : '❌ 未检测到（性能数据将被丢弃）'}`
  )
  if (!isTauriEnv) {
    console.warn(
      '[perfMonitor] ⚠️ 未检测到 Tauri 环境。可能原因：1) 在纯浏览器中运行 2) Tauri webview 注入失败'
    )
  }

  markStartupBegin()
  initWebVitals()

  // 页面卸载前同步刷新剩余指标（避免数据丢失）
  window.addEventListener('beforeunload', () => {
    void flush()
  })

  // 页面可见性变化时也刷新（切后台是常见的上报时机）
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'hidden') {
      void flush()
    }
  })

  console.log('[perfMonitor] 📊 性能监控已初始化')
}
