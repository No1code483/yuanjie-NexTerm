// plugins/boards/terminal/features/linux/ipc.ts — terminal.linux L2 IPC 客户端（短码 lx，批次3b）。
// 契约：统一经内核 dispatcher，逻辑名 `lx:plugin:<旧命令名>`（06_Rust代码契约 §8.1）。
// **裁定 T9：前端零消费**——唯一消费者 components/Linux.tsx 为死文件随批判删，
// 25 条方法仅登记 IpcMethodSpec（保持 manifest ipc 白名单与 dispatcher 键一致），
// 不提供便捷封装（43-A「有 alias 无前端方法」口径）；后续消费时在此补录。
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const LINUX_IPC_METHODS = {
  getEnvironment: { cmd: 'linux_get_environment' },
  listVersions: { cmd: 'linux_list_versions' },
  getStatusPanel: { cmd: 'linux_get_status_panel' },
  getShellStatus: { cmd: 'linux_get_shell_status' },
  getSystemInfo: { cmd: 'linux_get_system_info' },
  getIsoProgress: { cmd: 'linux_get_iso_progress' },
  listDirectory: { cmd: 'linux_list_directory' },
  viewFile: { cmd: 'linux_view_file' },
  searchSource: { cmd: 'linux_search_source' },
  downloadKernel: { cmd: 'linux_download_kernel' },
  setActive: { cmd: 'linux_set_active' },
  removeKernel: { cmd: 'linux_remove_kernel' },
  buildKernel: { cmd: 'linux_build_kernel' },
  analyzeConfig: { cmd: 'linux_analyze_config' },
  listModules: { cmd: 'linux_list_modules' },
  analyzeLogs: { cmd: 'linux_analyze_logs' },
  perfProfile: { cmd: 'linux_perf_profile' },
  runBenchmark: { cmd: 'linux_run_benchmark' },
  runStress: { cmd: 'linux_run_stress' },
  listContainers: { cmd: 'docker_list_containers' },
  containerStart: { cmd: 'docker_container_start' },
  containerStop: { cmd: 'docker_container_stop' },
  containerLogs: { cmd: 'docker_container_logs' },
  listImages: { cmd: 'docker_list_images' },
  networkStats: { cmd: 'network_stats' },
} satisfies Record<string, IpcMethodSpec>;

// 命名空间注册（供插件 registry 应用；当前无消费方，调用入口待后续补录）
void defineIpcNamespace('lx', LINUX_IPC_METHODS);
