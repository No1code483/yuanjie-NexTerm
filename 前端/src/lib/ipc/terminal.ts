// ipc/terminal.ts — terminal 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';

export const terminal = {
  createSession: (type: 'terminal' | 'cmd') => ipc.invoke<string>('terminal_create_session', {
    session_type: type
  }),
  createWslSession: (distro?: string, shell?: string) => ipc.invoke<string>('terminal_create_wsl_session', {
    distro,
    shell
  }),
  detectWsl: () => ipc.invoke<any>('terminal_detect_wsl'),
  executeCommand: (sessionId: string, command: string) => ipc.invoke('terminal_write_input', {
    session_id: sessionId,
    input: command
  }),
  executeBuiltin: (command: string) => ipc.invoke<any>('terminal_execute_builtin', {
    command
  }),
  resize: (sessionId: string, rows: number, cols: number) => ipc.invoke('terminal_resize', {
    session_id: sessionId,
    rows,
    cols
  }),
  closeSession: (sessionId: string) => ipc.invoke('terminal_kill_session', {
    session_id: sessionId
  })
};

export const linux = {
  getEnvironment: () => ipc.invoke<any>('linux_get_environment'),
  listVersions: () => ipc.invoke<any>('linux_list_versions'),
  listDirectory: (version: string, path?: string) => ipc.invoke<any>('linux_list_directory', {
    version,
    path
  }),
  viewFile: (version: string, filePath: string) => ipc.invoke<any>('linux_view_file', {
    version,
    file_path: filePath
  }),
  searchSource: (version: string, query: string, maxResults: number) => ipc.invoke<any>('linux_search_source', {
    version,
    query,
    max_results: maxResults
  }),
  downloadKernel: (version: string) => ipc.invoke<any>('linux_download_kernel', {
    version
  }),
  removeKernel: (version: string) => ipc.invoke<any>('linux_remove_kernel', {
    version
  }),
  setActive: (version: string) => ipc.invoke<any>('linux_set_active', {
    version
  }),
  // === 状态面板 Tab ===
  getStatusPanel: () => ipc.invoke<any>('linux_get_status_panel'),
  getShellStatus: () => ipc.invoke<any>('linux_get_shell_status'),
  getSystemInfo: () => ipc.invoke<any>('linux_get_system_info'),
  getIsoProgress: () => ipc.invoke<any>('linux_get_iso_progress'),
  // === 构建与配置 ===
  buildKernel: (sourceDir: string, arch: string, jobs?: number) => ipc.invoke<any>('linux_build_kernel', {
    source_dir: sourceDir,
    arch,
    jobs
  }),
  analyzeConfig: (sourceDir: string) => ipc.invoke<any>('linux_analyze_config', {
    source_dir: sourceDir
  }),
  // === 模块与日志 ===
  listModules: () => ipc.invoke<any>('linux_list_modules'),
  analyzeLogs: (level?: string) => ipc.invoke<any>('linux_analyze_logs', {
    level
  }),
  // === 性能与测试 ===
  perfProfile: (durationSecs?: number) => ipc.invoke<any>('linux_perf_profile', {
    duration_secs: durationSecs
  }),
  runBenchmark: () => ipc.invoke<any>('linux_run_benchmark'),
  runStress: () => ipc.invoke<any>('linux_run_stress'),
  // === P2: Docker 管理 ===
  dockerListContainers: (all?: boolean) => ipc.invoke<any[]>('docker_list_containers', {
    all
  }),
  dockerContainerStart: (containerId: string) => ipc.invoke<any>('docker_container_start', {
    container_id: containerId
  }),
  dockerContainerStop: (containerId: string) => ipc.invoke<any>('docker_container_stop', {
    container_id: containerId
  }),
  dockerContainerLogs: (containerId: string, tail?: number) => ipc.invoke<string>('docker_container_logs', {
    container_id: containerId,
    tail
  }),
  dockerListImages: () => ipc.invoke<any[]>('docker_list_images'),
  // === P2: 网络监控 ===
  networkStats: () => ipc.invoke<any>('network_stats')
};
