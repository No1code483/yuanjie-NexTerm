// LSP IPC 封装 — 前端调用后端 LSP 命令
import { ipc } from './ipc'

/** 诊断信息 */
export interface LspDiagnostic {
  message: string
  severity: 'error' | 'warning' | 'info' | 'hint'
  start_line: number
  start_column: number
  end_line: number
  end_column: number
  source: string
  code: string
}

/** 补全项 */
export interface LspCompletionItem {
  label: string
  kind?: string
  detail?: string
  documentation?: string
  insert_text?: string
  sort_text?: string
}

/** 位置 */
export interface LspLocation {
  uri: string
  range: {
    start: { line: number; character: number }
    end: { line: number; character: number }
  }
}

/** 悬停结果 */
export interface LspHoverResult {
  contents: string
  range?: {
    start: { line: number; character: number }
    end: { line: number; character: number }
  }
}

/**
 * 获取文件诊断信息
 */
export async function getDiagnostics(
  filePath: string,
  workspaceRoot: string,
): Promise<LspDiagnostic[]> {
  try {
    const res = await ipc.invoke<LspDiagnostic[]>('lsp_diagnostics', {
      filePath,
      workspaceRoot,
    })
    return res && res.code === 0 ? (res.data ?? []) : []
  } catch {
    return []
  }
}

/**
 * 获取代码补全
 */
export async function getCompletions(
  filePath: string,
  line: number,
  character: number,
  workspaceRoot: string,
): Promise<LspCompletionItem[]> {
  try {
    const res = await ipc.invoke<LspCompletionItem[]>('lsp_completions', {
      filePath,
      line,
      character,
      workspaceRoot,
    })
    return res.code === 0 && res.data ? res.data : []
  } catch {
    return []
  }
}

/**
 * 获取悬停提示
 */
export async function getHover(
  filePath: string,
  line: number,
  character: number,
  workspaceRoot: string,
): Promise<LspHoverResult | null> {
  try {
    const res = await ipc.invoke<LspHoverResult | null>('lsp_hover', {
      filePath,
      line,
      character,
      workspaceRoot,
    })
    return res && res.code === 0 ? (res.data ?? null) : null
  } catch {
    return null
  }
}

/**
 * 获取跳转定义
 */
export async function getDefinition(
  filePath: string,
  line: number,
  character: number,
  workspaceRoot: string,
): Promise<LspLocation[]> {
  try {
    const res = await ipc.invoke<LspLocation[]>('lsp_definition', {
      filePath,
      line,
      character,
      workspaceRoot,
    })
    return res.code === 0 && res.data ? res.data : []
  } catch {
    return []
  }
}

/**
 * 检测文件语言
 */
export async function detectLanguage(filePath: string): Promise<string | null> {
  try {
    const res = await ipc.invoke<string | null>('lsp_detect_language', {
      filePath,
    })
    return res.code === 0 && res.data !== undefined ? res.data : null
  } catch {
    return null
  }
}

/**
 * 将 LSP 诊断转换为 Monaco 标记
 */
export function toMonacoMarkers(diagnostics: LspDiagnostic[]) {
  return diagnostics.map((d) => ({
    severity: mapSeverity(d.severity),
    message: d.message,
    startLineNumber: d.start_line + 1,
    startColumn: d.start_column + 1,
    endLineNumber: d.end_line + 1,
    endColumn: d.end_column + 1,
    source: d.source,
    code: d.code,
  }))
}

function mapSeverity(s: string): 1 | 2 | 4 | 8 {
  switch (s) {
    case 'error':
      return 8 // monaco.MarkerSeverity.Error
    case 'warning':
      return 4 // monaco.MarkerSeverity.Warning
    case 'info':
      return 2 // monaco.MarkerSeverity.Info
    case 'hint':
      return 1 // monaco.MarkerSeverity.Hint
    default:
      return 2
  }
}