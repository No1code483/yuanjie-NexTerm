import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { ipc } from '@/lib/ipc';
import { time } from '@/lib/utils';
import type { LinuxEnvironmentInfo, LinuxKernelVersion, LinuxSourceFile, LinuxSearchResponse, KernelBuildResult, KernelConfigResult, KernelModuleInfo, KernelLogAnalysis, PerfProfileResult, LinuxShellStatus, LinuxSystemInfo, LinuxIsoProgress, LinuxStatusPanel, LinuxBenchmarkResult, LinuxStressResult } from '@/types';
import styles from './Linux.module.css';
import { TabId, TABS, LOG_LEVEL_COLORS } from './linux/types';
export default function Linux() {
  const [activeTab, setActiveTab] = useState<TabId>('versions');
  const [loading, setLoading] = useState(true);
  const [envInfo, setEnvInfo] = useState<LinuxEnvironmentInfo | null>(null);
  const [versions, setVersions] = useState<LinuxKernelVersion[]>([]);
  const [activeVersion, setActiveVersion] = useState<string>('');
  const [downloadProgress, setDownloadProgress] = useState<Record<string, {
    status: string;
    percent: number;
  }>>({});
  const [_sourcePath, setSourcePath] = useState('');
  const [sourceFiles, setSourceFiles] = useState<LinuxSourceFile[]>([]);
  const [sourceBreadcrumb, setSourceBreadcrumb] = useState<string[]>([]);
  const [fileViewer, setFileViewer] = useState<{
    path: string;
    name: string;
    content: string;
    lineCount: number;
  } | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<LinuxSearchResponse | null>(null);
  const [searching, setSearching] = useState(false);
  const [buildResult, setBuildResult] = useState<KernelBuildResult | null>(null);
  const [building, setBuilding] = useState(false);
  const [buildArch, setBuildArch] = useState('x86_64');
  const [configResult, setConfigResult] = useState<KernelConfigResult | null>(null);
  const [configCategory, setConfigCategory] = useState('');
  const [modules, setModules] = useState<KernelModuleInfo[]>([]);
  const [selectedModule, setSelectedModule] = useState<KernelModuleInfo | null>(null);
  const [logAnalysis, setLogAnalysis] = useState<KernelLogAnalysis | null>(null);
  const [logLevelFilter, setLogLevelFilter] = useState('');
  const [perfResult, setPerfResult] = useState<PerfProfileResult | null>(null);
  const [perfDuration, setPerfDuration] = useState(30);
  const [benchmarkResult, setBenchmarkResult] = useState<LinuxBenchmarkResult | null>(null);
  const [benchmarkRunning, setBenchmarkRunning] = useState(false);
  const [stressResult, setStressResult] = useState<LinuxStressResult | null>(null);
  const [stressRunning, setStressRunning] = useState(false);

  // Docker 状态
  const [dockerContainers, setDockerContainers] = useState<any[]>([]);
  const [dockerImages, setDockerImages] = useState<any[]>([]);
  const [dockerLoading, setDockerLoading] = useState(false);
  const [dockerLogs, setDockerLogs] = useState<{
    containerId: string;
    logs: string;
  } | null>(null);
  const [dockerLogsLoading, setDockerLogsLoading] = useState(false);

  // 网络状态
  const [networkData, setNetworkData] = useState<any>(null);
  const [networkLoading, setNetworkLoading] = useState(false);
  const [statusPanel, setStatusPanel] = useState<LinuxStatusPanel | null>(null);
  const [shellStatus, setShellStatus] = useState<LinuxShellStatus | null>(null);
  const [systemInfo, setSystemInfo] = useState<LinuxSystemInfo | null>(null);
  const [isoProgress, setIsoProgress] = useState<LinuxIsoProgress | null>(null);
  const buildOutputRef = useRef<HTMLPreElement>(null);
  const loadEnv = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_get_environment');
      if (res?.data) {
        setEnvInfo(res.data);
        if (res.data.active_version) setActiveVersion(res.data.active_version);
      }
    } catch (e) {
      console.error('获取Linux环境信息失败:', e);
    }
  }, []);
  const loadVersions = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_list_versions');
      if (res?.data) setVersions(res.data);
    } catch (e) {
      console.error('获取内核版本列表失败:', e);
    }
  }, []);
  const loadStatusPanel = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_get_status_panel');
      if (res?.data) setStatusPanel(res.data);
    } catch (e) {
      console.error('获取状态面板失败:', e);
    }
  }, []);
  const loadShellStatus = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_get_shell_status');
      if (res?.data) setShellStatus(res.data);
    } catch (e) {
      console.error('获取Shell状态失败:', e);
    }
  }, []);
  const loadSystemInfo = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_get_system_info');
      if (res?.data) setSystemInfo(res.data);
    } catch (e) {
      console.error('获取系统信息失败:', e);
    }
  }, []);
  const loadIsoProgress = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_get_iso_progress');
      if (res?.data) setIsoProgress(res.data);
    } catch (e) {
      console.error('获取ISO进度失败:', e);
    }
  }, []);
  const loadSourceDir = useCallback(async (version: string, dirPath?: string) => {
    try {
      const res = await ipc.invoke<any>('linux_list_directory', {
        version,
        path: dirPath || null
      });
      if (res?.data) {
        setSourceFiles(res.data.entries || []);
        if (dirPath) {
          setSourceBreadcrumb(dirPath.split('/').filter(Boolean));
        } else {
          setSourceBreadcrumb([version]);
        }
        setSourcePath(dirPath || '');
      }
    } catch (e) {
      console.error('获取源码目录失败:', e);
    }
  }, []);
  const viewFile = useCallback(async (filePath: string, fileName: string) => {
    try {
      const res = await ipc.invoke<any>('linux_view_file', {
        version: activeVersion,
        filePath
      });
      if (res?.data) {
        setFileViewer({
          path: filePath,
          name: fileName,
          content: res.data.content || '',
          lineCount: res.data.line_count || 0
        });
      }
    } catch (e) {
      console.error('查看文件失败:', e);
    }
  }, [activeVersion]);
  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) return;
    setSearching(true);
    try {
      const res = await ipc.invoke<any>('linux_search_source', {
        version: activeVersion,
        query: searchQuery,
        maxResults: 50
      });
      if (res?.data) setSearchResults(res.data);
    } catch (e) {
      console.error('搜索失败:', e);
    } finally {
      setSearching(false);
    }
  }, [activeVersion, searchQuery]);
  const handleDownload = useCallback(async (version: string) => {
    setDownloadProgress(prev => ({
      ...prev,
      [version]: {
        status: t("Linux.k1"),
        percent: 0
      }
    }));
    try {
      await ipc.invoke('linux_download_kernel', {
        version
      });
      setDownloadProgress(prev => ({
        ...prev,
        [version]: {
          status: t("common.finish"),
          percent: 100
        }
      }));
      await loadEnv();
      await loadSourceDir(version);
    } catch (e) {
      setDownloadProgress(prev => ({
        ...prev,
        [version]: {
          status: t("common.failed"),
          percent: 0
        }
      }));
      console.error('下载失败:', e);
    }
  }, [loadEnv, loadSourceDir]);
  const handleSetActive = useCallback(async (version: string) => {
    try {
      await ipc.invoke('linux_set_active', {
        version
      });
      setActiveVersion(version);
      await loadEnv();
      await loadSourceDir(version);
    } catch (e) {
      console.error('设置活跃版本失败:', e);
    }
  }, [loadEnv, loadSourceDir]);
  const handleBuild = useCallback(async () => {
    setBuilding(true);
    setBuildResult(null);
    try {
      const res = await ipc.invoke<any>('linux_build_kernel', {
        sourceDir: envInfo?.source_dir || '',
        arch: buildArch,
        jobs: 4
      });
      if (res?.data) setBuildResult(res.data);
    } catch (e) {
      console.error('构建失败:', e);
    } finally {
      setBuilding(false);
    }
  }, [envInfo, buildArch]);
  const handleLoadConfig = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_analyze_config', {
        sourceDir: envInfo?.source_dir || ''
      });
      if (res?.data) setConfigResult(res.data);
    } catch (e) {
      console.error('加载配置失败:', e);
    }
  }, [envInfo]);
  const handleLoadModules = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_list_modules');
      if (res?.data) setModules(res.data);
    } catch (e) {
      console.error('加载模块失败:', e);
    }
  }, []);
  const handleLoadLogs = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_analyze_logs', {
        level: logLevelFilter || null
      });
      if (res?.data) setLogAnalysis(res.data);
    } catch (e) {
      console.error('加载日志失败:', e);
    }
  }, [logLevelFilter]);
  const handlePerf = useCallback(async () => {
    try {
      const res = await ipc.invoke<any>('linux_perf_profile', {
        durationSecs: perfDuration
      });
      if (res?.data) setPerfResult(res.data);
    } catch (e) {
      console.error('性能分析失败:', e);
    }
  }, [perfDuration]);
  const handleBenchmark = useCallback(async () => {
    setBenchmarkRunning(true);
    try {
      const res = await ipc.invoke<LinuxBenchmarkResult>('linux_run_benchmark', {});
      if (res?.data) setBenchmarkResult(res.data);
    } catch (e) {
      console.error('基准评测失败:', e);
    } finally {
      setBenchmarkRunning(false);
    }
  }, []);
  const handleStress = useCallback(async () => {
    setStressRunning(true);
    try {
      const res = await ipc.invoke<any>('linux_run_stress', {});
      if (res?.data) setStressResult(res.data);
    } catch (e) {
      console.error('压力测试失败:', e);
    } finally {
      setStressRunning(false);
    }
  }, []);

  // Docker 操作
  const loadDockerContainers = useCallback(async () => {
    setDockerLoading(true);
    try {
      const res = await ipc.invoke<any>('docker_list_containers', {
        all: true
      });
      if (res?.data) setDockerContainers(res.data);
    } catch (e) {
      console.error('获取Docker容器失败:', e);
    } finally {
      setDockerLoading(false);
    }
  }, []);
  const loadDockerImages = useCallback(async () => {
    setDockerLoading(true);
    try {
      const res = await ipc.invoke<any>('docker_list_images');
      if (res?.data) setDockerImages(res.data);
    } catch (e) {
      console.error('获取Docker镜像失败:', e);
    } finally {
      setDockerLoading(false);
    }
  }, []);
  const handleDockerStart = useCallback(async (containerId: string) => {
    try {
      await ipc.invoke('docker_container_start', {
        container_id: containerId
      });
      await loadDockerContainers();
    } catch (e) {
      console.error('启动容器失败:', e);
    }
  }, [loadDockerContainers]);
  const handleDockerStop = useCallback(async (containerId: string) => {
    try {
      await ipc.invoke('docker_container_stop', {
        container_id: containerId
      });
      await loadDockerContainers();
    } catch (e) {
      console.error('停止容器失败:', e);
    }
  }, [loadDockerContainers]);
  const handleDockerLogs = useCallback(async (containerId: string) => {
    setDockerLogsLoading(true);
    setDockerLogs(null);
    try {
      const res = await ipc.invoke<string>('docker_container_logs', {
        container_id: containerId,
        tail: 100
      });
      if (res?.data) setDockerLogs({
        containerId,
        logs: res.data
      });
    } catch (e) {
      console.error('获取容器日志失败:', e);
    } finally {
      setDockerLogsLoading(false);
    }
  }, []);

  // 网络监控
  const loadNetworkStats = useCallback(async () => {
    setNetworkLoading(true);
    try {
      const res = await ipc.invoke<any>('network_stats');
      if (res?.data) setNetworkData(res.data);
    } catch (e) {
      console.error('获取网络状态失败:', e);
    } finally {
      setNetworkLoading(false);
    }
  }, []);
  useEffect(() => {
    const init = async () => {
      setLoading(true);
      await loadEnv();
      await loadVersions();
      await loadStatusPanel();
      await loadShellStatus();
      await loadSystemInfo();
      await loadIsoProgress();
      setLoading(false);
    };
    init();
  }, [loadEnv, loadVersions, loadStatusPanel, loadShellStatus, loadSystemInfo, loadIsoProgress]);
  if (loading) {
    return <div className={styles.page}>
        <div className={styles.loadingOverlay}>
          <span className={styles.loadingDot} />
          <span>{t("Linux.k2")}</span>
        </div>
      </div>;
  }
  const formatBytes = (bytes: number) => {
    if (bytes >= 1_000_000_000) return `${(bytes / 1_000_000_000).toFixed(1)} GB`;
    if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
    if (bytes >= 1_000) return `${(bytes / 1_000).toFixed(1)} KB`;
    return `${bytes} B`;
  };
  return <div className={styles.page}>
      <div className={styles.topBar}>
        <div className={styles.envStatus}>
          <span className={`${styles.statusDot} ${envInfo?.has_kernel ? styles.statusOnline : styles.statusOffline}`} />
          <span className={styles.envLabel}>
            {envInfo?.has_kernel ? t("Linux.k3") : t("Linux.k4")}
          </span>
          {envInfo?.active_version && <span className={styles.activeVersion}>
              {t("Linux.k5")} <strong>{envInfo.active_version}</strong>
            </span>}
          {envInfo?.working_directory && <span className={styles.workDir} title={envInfo.working_directory}>
              📁 {envInfo.working_directory.split(/[/\\]/).pop()}
            </span>}
        </div>
        <div className={styles.topActions}>
          <button className={styles.btnSmall} onClick={loadEnv} title={t("Linux.k6")}>
            🔄
          </button>
        </div>
      </div>

      <div className={styles.tabBar}>
        {TABS.map(tab => <button key={tab.id} className={`${styles.tab} ${activeTab === tab.id ? styles.tabActive : ''}`} onClick={() => setActiveTab(tab.id)}>
            <span className={styles.tabIcon}>{tab.icon}</span>
            {tab.label}
          </button>)}
      </div>

      <div className={styles.content}>
        {activeTab === 'status' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k7")}</h3>
              <button className={styles.btnSmall} onClick={loadStatusPanel} title={t("Linux.k8")}>
                🔄
              </button>
            </div>

            <div className={styles.statusGrid}>
              <div className={styles.statusCard}>
                <div className={styles.statusCardHeader}>
                  <span className={styles.statusCardIcon}>🖥️</span>
                  <span className={styles.statusCardTitle}>{t("Linux.k9")}</span>
                </div>
                <div className={styles.statusCardBody}>
                  <div className={styles.statusRow}>
                    <span className={styles.statusLabel}>{t("Linux.k10")}</span>
                    <span className={`${styles.statusValue} ${shellStatus?.connected ? styles.statusOnline : styles.statusOffline}`}>
                      <span className={styles.statusDot} />
                      {shellStatus?.connected ? t("Linux.k11") : t("Linux.k12")}
                    </span>
                  </div>
                  <div className={styles.statusRow}>
                    <span className={styles.statusLabel}>{t("Linux.k13")}</span>
                    <span className={styles.statusValue}>{shellStatus?.connection_type || '-'}</span>
                  </div>
                  <div className={styles.statusRow}>
                    <span className={styles.statusLabel}>{t("Linux.k14")}</span>
                    <span className={styles.statusValue}>{shellStatus?.terminal_pid?.toString() || '-'}</span>
                  </div>
                  <div className={styles.statusRow}>
                    <span className={styles.statusLabel}>{t("Linux.k15")}</span>
                    <span className={styles.statusValue}>{shellStatus?.active_sessions ?? '-'}</span>
                  </div>
                  <div className={styles.statusRow}>
                    <span className={styles.statusLabel}>{t("Linux.k16")}</span>
                    <span className={styles.statusValue}>
                      {shellStatus?.terminal_uptime_secs ? `${Math.floor(shellStatus.terminal_uptime_secs / 3600)}h ${Math.floor(shellStatus.terminal_uptime_secs % 3600 / 60)}m` : '-'}
                    </span>
                  </div>
                  <div className={styles.statusRow}>
                    <span className={styles.statusLabel}>{t("Linux.k17")}</span>
                    <span className={styles.statusValue}>
                      {shellStatus?.last_command_time ? new Date(shellStatus.last_command_time).toLocaleTimeString() : '-'}
                    </span>
                  </div>
                </div>
              </div>

              <div className={styles.statusCard}>
                <div className={styles.statusCardHeader}>
                  <span className={styles.statusCardIcon}>💿</span>
                  <span className={styles.statusCardTitle}>{t("Linux.k18")}</span>
                </div>
                <div className={styles.statusCardBody}>
                  {isoProgress ? <>
                      <div className={styles.statusRow}>
                        <span className={styles.statusLabel}>{t("Linux.k19")}</span>
                        <span className={styles.statusValue}>{isoProgress.iso_name}</span>
                      </div>
                      <div className={styles.statusRow}>
                        <span className={styles.statusLabel}>{t("common.version")}</span>
                        <span className={styles.statusValue}>{isoProgress.iso_version}</span>
                      </div>
                      <div className={styles.statusRow}>
                        <span className={styles.statusLabel}>{t("common.status")}</span>
                        <span className={`${styles.statusValue} ${styles.isoStatus} ${styles[`iso${(isoProgress.status ?? '')[0]?.toUpperCase() ?? ''}${(isoProgress.status ?? '').slice(1)}` as keyof typeof styles] || ''}`}>
                          {isoProgress.status === 'running' && t("Linux.k20")}
                          {isoProgress.status === 'idle' && t("Linux.k21")}
                          {isoProgress.status === 'downloading' && t("Linux.k22")}
                          {isoProgress.status === 'booting' && t("Linux.k23")}
                          {isoProgress.status === 'error' && t("Linux.k24")}
                        </span>
                      </div>
                      {(isoProgress.download_percent ?? 0) > 0 && (isoProgress.download_percent ?? 0) < 100 && <div className={styles.statusRow}>
                          <span className={styles.statusLabel}>{t("Linux.k25")}</span>
                          <span className={styles.statusValue}>
                            <div className={styles.progressBar}>
                              <div className={styles.progressFill} style={{
                        width: `${isoProgress.download_percent ?? 0}%`
                      }} />
                            </div>
                            {(isoProgress.download_percent ?? 0).toFixed(1)}%
                          </span>
                        </div>}
                      {isoProgress.boot_percent < 100 && isoProgress.status === 'booting' && <div className={styles.statusRow}>
                          <span className={styles.statusLabel}>{t("Linux.k26")}</span>
                          <span className={styles.statusValue}>
                            <div className={styles.progressBar}>
                              <div className={styles.progressFill} style={{
                        width: `${isoProgress.boot_percent}%`
                      }} />
                            </div>
                            {isoProgress.boot_percent.toFixed(1)}%
                          </span>
                        </div>}
                      {isoProgress.error_message && <div className={`${styles.statusRow} ${styles.errorRow}`}>
                          <span className={styles.statusLabel}>{t("Linux.k27")}</span>
                          <span className={styles.statusValue}>{isoProgress.error_message}</span>
                        </div>}
                    </> : <div className={styles.emptyState}>{t("Linux.k28")}</div>}
                </div>
              </div>
            </div>

            {systemInfo && <div className={styles.statusCard} style={{
          marginTop: 16
        }}>
                <div className={styles.statusCardHeader}>
                  <span className={styles.statusCardIcon}>📊</span>
                  <span className={styles.statusCardTitle}>{t("CommandManual.k36")}</span>
                </div>
                <div className={styles.statusCardBody}>
                  <div className={styles.systemInfoGrid}>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>{t("linux.types.k2")}</span>
                      <span className={styles.statusValue}>{systemInfo.kernel_version}</span>
                    </div>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>{t("Linux.k29")}</span>
                      <span className={styles.statusValue} style={{
                  fontSize: 11
                }}>{systemInfo.kernel_build}</span>
                    </div>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>{t("Linux.k30")}</span>
                      <span className={styles.statusValue}>{systemInfo.hostname}</span>
                    </div>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>{t("Linux.k31")}</span>
                      <span className={styles.statusValue}>{systemInfo.uptime_hours.toFixed(1)} {t("Linux.k32")}</span>
                    </div>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>CPU</span>
                      <span className={styles.statusValue}>{systemInfo.cpu_model} ({systemInfo.cpu_cores} {t("Linux.k33")}</span>
                    </div>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>{t("Linux.k34")}</span>
                      <span className={styles.statusValue}>
                        <div className={styles.progressBar} style={{
                    marginBottom: 4
                  }}>
                          <div className={styles.progressFill} style={{
                      width: `${systemInfo.cpu_usage_percent}%`,
                      background: systemInfo.cpu_usage_percent > 80 ? '#FF4500' : '#00F0FF'
                    }} />
                        </div>
                        {systemInfo.cpu_usage_percent.toFixed(1)}%
                      </span>
                    </div>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>{t("Linux.k35")}</span>
                      <span className={styles.statusValue}>
                        <div className={styles.progressBar} style={{
                    marginBottom: 4
                  }}>
                          <div className={styles.progressFill} style={{
                      width: `${systemInfo.memory_usage_percent}%`,
                      background: systemInfo.memory_usage_percent > 80 ? '#FF4500' : '#00F0FF'
                    }} />
                        </div>
                        {systemInfo.memory_used_gb.toFixed(1)} / {systemInfo.memory_total_gb.toFixed(0)} GB ({systemInfo.memory_usage_percent.toFixed(1)}%)
                      </span>
                    </div>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>{t("Linux.k36")}</span>
                      <span className={styles.statusValue}>
                        <div className={styles.progressBar} style={{
                    marginBottom: 4
                  }}>
                          <div className={styles.progressFill} style={{
                      width: `${systemInfo.disk_usage_percent}%`,
                      background: systemInfo.disk_usage_percent > 80 ? '#FF4500' : '#00F0FF'
                    }} />
                        </div>
                        {systemInfo.disk_used_gb.toFixed(1)} / {systemInfo.disk_total_gb.toFixed(0)} GB ({systemInfo.disk_usage_percent.toFixed(1)}%)
                      </span>
                    </div>
                    <div className={styles.systemInfoItem}>
                      <span className={styles.statusLabel}>{t("Linux.k37")}</span>
                      <span className={styles.statusValue}>
                        <span className={styles.loadBadge}>1m: {systemInfo.system_load[0]?.toFixed(2) ?? '-'}</span>
                        <span className={styles.loadBadge}>5m: {systemInfo.system_load[1]?.toFixed(2) ?? '-'}</span>
                        <span className={styles.loadBadge}>15m: {systemInfo.system_load[2]?.toFixed(2) ?? '-'}</span>
                      </span>
                    </div>
                  </div>
                </div>
              </div>}

            {statusPanel && <div className={styles.statusTimestamp}>
                {t("Linux.k38")} {statusPanel.last_updated ? time.formatUtcToLocal(statusPanel.last_updated) : '—'}
              </div>}
          </div>}
        {activeTab === 'versions' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k39")}</h3>
              <span className={styles.count}>
                {statusPanel?.last_updated ? t("Linux.k40", {
              arg0: new Date(statusPanel.last_updated).toLocaleTimeString()
            }) : t("common.loading")}
              </span>
            </div>

            <div className={styles.statusGrid}>
              <div className={styles.statusCard}>
                <div className={styles.statusCardHeader}>
                  <span className={styles.statusCardIcon}>🔌</span>
                  <span>{t("Linux.k9")}</span>
                </div>
                <div className={styles.statusCardBody}>
                  <div className={styles.statusIndicator}>
                    <span className={`${styles.statusDot} ${shellStatus?.connected ? styles.statusOnline : styles.statusOffline}`} />
                    <span className={styles.statusLabel}>
                      {shellStatus?.connected ? t("Linux.k11") : t("Linux.k12")}
                    </span>
                  </div>
                  {shellStatus && <div className={styles.statusDetails}>
                      <div className={styles.statusRow}>
                        <span>{t("Linux.k13")}</span>
                        <span className={styles.statusValue}>{shellStatus.connection_type}</span>
                      </div>
                      <div className={styles.statusRow}>
                        <span>{t("Linux.k15")}</span>
                        <span className={styles.statusValue}>{shellStatus.active_sessions} {t("Linux.k41")}</span>
                      </div>
                      <div className={styles.statusRow}>
                        <span>{t("Linux.k14")}</span>
                        <span className={styles.statusValue}>{shellStatus.terminal_pid || 'N/A'}</span>
                      </div>
                      <div className={styles.statusRow}>
                        <span>{t("Linux.k31")}</span>
                        <span className={styles.statusValue}>
                          {Math.floor(shellStatus.terminal_uptime_secs / 60)} {t("home.TodoPanel.k15")}
                        </span>
                      </div>
                    </div>}
                </div>
              </div>

              <div className={styles.statusCard}>
                <div className={styles.statusCardHeader}>
                  <span className={styles.statusCardIcon}>💿</span>
                  <span>{t("Linux.k42")}</span>
                </div>
                <div className={styles.statusCardBody}>
                  {isoProgress ? <>
                      <div className={styles.statusIndicator}>
                        <span className={`${styles.statusDot} ${isoProgress.status === 'running' ? styles.statusOnline : isoProgress.status === 'error' ? styles.statusDanger : styles.statusPending}`} />
                        <span className={styles.statusLabel}>
                          {isoProgress.status === 'idle' && t("Linux.k43")}
                          {isoProgress.status === 'downloading' && t("Linux.k44")}
                          {isoProgress.status === 'booting' && t("Linux.k45")}
                          {isoProgress.status === 'running' && t("Linux.k46")}
                          {isoProgress.status === 'error' && t("common.error")}
                        </span>
                      </div>
                      <div className={styles.statusDetails}>
                        <div className={styles.statusRow}>
                          <span>{t("Linux.k19")}</span>
                          <span className={styles.statusValue}>{isoProgress.iso_name}</span>
                        </div>
                        <div className={styles.statusRow}>
                          <span>{t("common.version")}</span>
                          <span className={styles.statusValue}>{isoProgress.iso_version}</span>
                        </div>
                        {isoProgress.status === 'downloading' && <>
                            <div className={styles.statusRow}>
                              <span>{t("Linux.k25")}</span>
                              <span className={styles.statusValue}>{isoProgress.download_percent}%</span>
                            </div>
                            <div className={styles.progressBar}>
                              <div className={styles.progressFill} style={{
                        width: `${isoProgress.download_percent}%`
                      }} />
                            </div>
                            <div className={styles.statusRow}>
                              <span>{t("Linux.k47")}</span>
                              <span className={styles.statusValue}>{isoProgress.download_speed_mbps.toFixed(1)} MB/s</span>
                            </div>
                          </>}
                        {isoProgress.status === 'booting' && <>
                            <div className={styles.statusRow}>
                              <span>{t("Linux.k26")}</span>
                              <span className={styles.statusValue}>{isoProgress.boot_percent}%</span>
                            </div>
                            <div className={styles.progressBar}>
                              <div className={styles.progressFill} style={{
                        width: `${isoProgress.boot_percent}%`
                      }} />
                            </div>
                          </>}
                        {isoProgress.status === 'error' && isoProgress.error_message && <div className={styles.statusError}>
                            <span>⚠ {isoProgress.error_message}</span>
                          </div>}
                      </div>
                    </> : <div className={styles.statusPlaceholder}>{t("Linux.k48")}</div>}
                </div>
              </div>

              {systemInfo && <div className={styles.statusCard} style={{
            gridColumn: '1 / -1'
          }}>
                  <div className={styles.statusCardHeader}>
                    <span className={styles.statusCardIcon}>📊</span>
                    <span>{t("CommandManual.k36")}</span>
                  </div>
                  <div className={styles.statusCardBody}>
                    <div className={styles.sysInfoGrid}>
                      <div className={styles.sysInfoItem}>
                        <span className={styles.sysInfoLabel}>{t("linux.types.k2")}</span>
                        <span className={styles.sysInfoValue}>{systemInfo.kernel_version}</span>
                      </div>
                      <div className={styles.sysInfoItem}>
                        <span className={styles.sysInfoLabel}>{t("Linux.k49")}</span>
                        <span className={styles.sysInfoValue}>{systemInfo.kernel_build}</span>
                      </div>
                      <div className={styles.sysInfoItem}>
                        <span className={styles.sysInfoLabel}>{t("Linux.k30")}</span>
                        <span className={styles.sysInfoValue}>{systemInfo.hostname}</span>
                      </div>
                      <div className={styles.sysInfoItem}>
                        <span className={styles.sysInfoLabel}>{t("Linux.k31")}</span>
                        <span className={styles.sysInfoValue}>{systemInfo.uptime_hours.toFixed(1)} {t("Linux.k32")}</span>
                      </div>
                    </div>

                    <div className={styles.sysDivider}>{t("Linux.k50")}</div>

                    <div className={styles.resourceGrid}>
                      <div className={styles.resourceCard}>
                        <div className={styles.resourceHeader}>
                          <span>🖥 CPU</span>
                          <span className={styles.resourcePercent}>{systemInfo.cpu_usage_percent}%</span>
                        </div>
                        <div className={styles.progressBar}>
                          <div className={`${styles.progressFill} ${systemInfo.cpu_usage_percent > 80 ? styles.progressDanger : ''}`} style={{
                      width: `${systemInfo.cpu_usage_percent}%`
                    }} />
                        </div>
                        <div className={styles.resourceMeta}>
                          <span>{systemInfo.cpu_cores} {t("Linux.k51")} {systemInfo.cpu_model}</span>
                        </div>
                        <div className={styles.resourceMeta}>
                          <span>{t("Linux.k52")} {systemInfo.system_load.join(' / ')}</span>
                        </div>
                      </div>

                      <div className={styles.resourceCard}>
                        <div className={styles.resourceHeader}>
                          <span>{t("Linux.k53")}</span>
                          <span className={styles.resourcePercent}>{systemInfo.memory_usage_percent}%</span>
                        </div>
                        <div className={styles.progressBar}>
                          <div className={`${styles.progressFill} ${systemInfo.memory_usage_percent > 80 ? styles.progressDanger : ''}`} style={{
                      width: `${systemInfo.memory_usage_percent}%`
                    }} />
                        </div>
                        <div className={styles.resourceMeta}>
                          <span>{systemInfo.memory_used_gb.toFixed(1)} GB / {systemInfo.memory_total_gb} GB</span>
                        </div>
                      </div>

                      <div className={styles.resourceCard}>
                        <div className={styles.resourceHeader}>
                          <span>{t("Linux.k54")}</span>
                          <span className={styles.resourcePercent}>{systemInfo.disk_usage_percent}%</span>
                        </div>
                        <div className={styles.progressBar}>
                          <div className={`${styles.progressFill} ${systemInfo.disk_usage_percent > 80 ? styles.progressDanger : ''}`} style={{
                      width: `${systemInfo.disk_usage_percent}%`
                    }} />
                        </div>
                        <div className={styles.resourceMeta}>
                          <span>{systemInfo.disk_used_gb.toFixed(1)} GB / {systemInfo.disk_total_gb} GB</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>}
            </div>

            <div className={styles.statusActions}>
              <button className={styles.btnSmall} onClick={loadStatusPanel} title={t("Linux.k55")}>
                {t("Linux.k56")}
              </button>
              <button className={styles.btnSmall} onClick={loadShellStatus} title={t("Linux.k57")}>
                🔌 Shell
              </button>
              <button className={styles.btnSmall} onClick={loadSystemInfo} title={t("Linux.k58")}>
                {t("Linux.k59")}
              </button>
              <button className={styles.btnSmall} onClick={loadIsoProgress} title={t("Linux.k60")}>
                💿 ISO
              </button>
            </div>
          </div>}

        {activeTab === 'versions' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k61")}</h3>
              <span className={styles.count}>{versions.length} {t("components.Linux.k11")}</span>
            </div>
            <div className={styles.versionGrid}>
              {versions.map(v => <div key={v.version} className={`${styles.versionCard} ${activeVersion === v.version ? styles.versionActive : ''}`}>
                  <div className={styles.versionHeader}>
                    <span className={styles.versionTag}>{v.version}</span>
                    <div className={styles.versionBadges}>
                      {v.is_lts && <span className={styles.badgeLts}>LTS</span>}
                      {v.is_stable && <span className={styles.badgeStable}>STABLE</span>}
                    </div>
                  </div>
                  <div className={styles.versionMeta}>
                    <span>{v.release_type}</span>
                    <span>{v.size_mb.toFixed(1)} MB</span>
                    <span>{v.release_date}</span>
                  </div>
                  <div className={styles.versionActions}>
                    {envInfo?.installed_versions?.includes(v.version) ? <button className={`${styles.btnAction} ${styles.btnActive} ${activeVersion === v.version ? styles.btnCurrent : ''}`} onClick={() => handleSetActive(v.version)}>
                        {activeVersion === v.version ? t("components.Linux.k23") : t("components.Linux.k24")}
                      </button> : <button className={styles.btnAction} onClick={() => handleDownload(v.version)}>
                        {downloadProgress[v.version]?.status || t("Linux.k62")}
                      </button>}
                    {downloadProgress[v.version] && downloadProgress[v.version].status === t("Linux.k1") && <div className={styles.progressBar}>
                        <div className={styles.progressFill} style={{
                  width: `${downloadProgress[v.version].percent}%`
                }} />
                      </div>}
                  </div>
                </div>)}
              {versions.length === 0 && <div className={styles.emptyState}>{t("Linux.k63")}</div>}
            </div>
          </div>}

        {activeTab === 'source' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("components.Linux.k16")}</h3>
              <div className={styles.breadcrumb}>
                {sourceBreadcrumb.map((part, i) => <span key={i}>
                    {i > 0 && <span className={styles.breadcrumbSep}>/</span>}
                    <span className={styles.breadcrumbItem}>{part}</span>
                  </span>)}
              </div>
            </div>

            {!envInfo?.has_kernel ? <div className={styles.emptyState}>{t("Linux.k64")}</div> : <>
                <div className={styles.searchBar}>
                  <input className={styles.searchInput} type="text" placeholder={t("Linux.k65")} value={searchQuery} onChange={e => setSearchQuery(e.target.value)} onKeyDown={e => e.key === 'Enter' && handleSearch()} />
                  <button className={styles.btnSearch} onClick={handleSearch} disabled={searching}>
                    {searching ? t("ai.ConversationList.k4") : t("common.search")}
                  </button>
                </div>

                {searchResults && <div className={styles.searchPanel}>
                    <div className={styles.searchStats}>
                      {t("components.Linux.k31")} {searchResults.total_matches} {t("Linux.k66")}{searchResults.files_searched} {t("Linux.k67")}{searchResults.elapsed_ms}ms）
                      <button className={styles.btnClear} onClick={() => setSearchResults(null)}>{t("Linux.k68")}</button>
                    </div>
                    <div className={styles.searchResults}>
                      {searchResults.results.map((r, i) => <div key={i} className={styles.searchResultItem} onClick={() => viewFile(r.file_path, r.file_name)}>
                          <span className={styles.resultFile}>{r.file_name}:{r.line_number}</span>
                          <span className={styles.resultLine}>{r.line_content}</span>
                        </div>)}
                    </div>
                  </div>}

                <div className={styles.sourceList}>
                  {sourceFiles.map(f => <div key={f.path} className={styles.sourceItem} onClick={() => {
              if (f.is_directory) {
                loadSourceDir(activeVersion, f.path);
                setFileViewer(null);
                setSearchResults(null);
              } else {
                viewFile(f.path, f.name);
              }
            }}>
                      <span className={styles.sourceIcon}>
                        {f.is_directory ? '📁' : '📄'}
                      </span>
                      <span className={styles.sourceName}>{f.name}</span>
                      <span className={styles.sourceMeta}>
                        {f.language}
                        {!f.is_directory && ` · ${formatBytes(f.size_bytes)}`}
                      </span>
                    </div>)}
                  {sourceFiles.length === 0 && !searchResults && <div className={styles.emptyState}>{t("Linux.k69")}</div>}
                </div>
              </>}

            {fileViewer && <div className={styles.fileViewer}>
                <div className={styles.fileViewerHeader}>
                  <span>📄 {fileViewer.name}</span>
                  <span className={styles.fileViewerMeta}>{fileViewer.lineCount} {t("components.CodePreview.k1")}</span>
                  <button className={styles.btnClose} onClick={() => setFileViewer(null)}>✕</button>
                </div>
                <pre className={styles.fileViewerContent}>
                  <code>{fileViewer.content}</code>
                </pre>
              </div>}
          </div>}

        {activeTab === 'build' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("linux.types.k3")}</h3>
            </div>
            {!envInfo?.has_kernel ? <div className={styles.emptyState}>{t("Linux.k64")}</div> : <>
                <div className={styles.buildForm}>
                  <label className={styles.formLabel}>
                    {t("Linux.k70")}
                    <select className={styles.formSelect} value={buildArch} onChange={e => setBuildArch(e.target.value)}>
                      <option value="x86_64">x86_64</option>
                      <option value="arm64">ARM64</option>
                      <option value="riscv64">RISC-V 64</option>
                    </select>
                  </label>
                  <button className={styles.btnBuild} onClick={handleBuild} disabled={building}>
                    {building ? t("Linux.k71") : t("Linux.k72")}
                  </button>
                </div>

                {buildResult && <div className={styles.buildResult}>
                    <div className={styles.buildStats}>
                      <span className={buildResult.exit_code === 0 ? styles.buildOk : styles.buildFail}>
                        {t("Linux.k73")} {buildResult.exit_code}
                      </span>
                      <span>{t("Linux.k74")} {buildResult.duration_secs.toFixed(1)}s</span>
                      {buildResult.artifact_path && <span>{t("Linux.k75")} {buildResult.artifact_path}</span>}
                    </div>
                    {buildResult.errors.length > 0 && <div className={styles.buildErrors}>
                        <h4>{t("Linux.k76")}{buildResult.errors.length})</h4>
                        {buildResult.errors.map((e, i) => <div key={i} className={styles.buildErrorItem}>
                            <span className={styles.errorFile}>{e.file_path || 'unknown'}:{e.line_number}</span>
                            <span className={styles.errorMsg}>{e.message}</span>
                          </div>)}
                      </div>}
                    {buildResult.warnings.length > 0 && <div className={styles.buildWarnings}>
                        <h4>{t("Linux.k77")}{buildResult.warnings.length})</h4>
                        {buildResult.warnings.map((w, i) => <div key={i} className={styles.buildWarningItem}>
                            <span className={styles.warningFile}>{w.file_path || 'unknown'}:{w.line_number}</span>
                            <span className={styles.warningMsg}>{w.message}</span>
                          </div>)}
                      </div>}
                    <pre className={styles.buildOutput} ref={buildOutputRef}>
                      {buildResult.stdout}
                      {buildResult.stderr && <span className={styles.stderrText}>{buildResult.stderr}</span>}
                    </pre>
                  </div>}
              </>}
          </div>}

        {activeTab === 'config' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k78")}</h3>
              <button className={styles.btnAction} onClick={handleLoadConfig}>
                {t("Linux.k79")}
              </button>
            </div>
            {configResult && <>
                <div className={styles.configStats}>
                  <span className={styles.configStat}>{t("Linux.k80")} {configResult.total_enabled}</span>
                  <span className={styles.configStat}>{t("Linux.k81")} {configResult.total_disabled}</span>
                  <span className={styles.configStat}>{t("Linux.k82")} {configResult.total_modules}</span>
                  <span className={styles.configStat}>{t("Linux.k83")} {configResult.config_path}</span>
                </div>
                <div className={styles.categoryFilter}>
                  <span>{t("Linux.k84")}</span>
                  <select className={styles.formSelect} value={configCategory} onChange={e => setConfigCategory(e.target.value)}>
                    <option value="">{t("common.all")}</option>
                    {configResult.categories.map((c: any) => <option key={c as string} value={c as string}>{c as string}</option>)}
                  </select>
                </div>
                <div className={styles.configList}>
                  {configResult.items.filter((item: any) => !configCategory || item.category === configCategory).map((item: any, i: number) => <div key={i} className={styles.configItem}>
                        <span className={`${styles.configKey} ${item.disabled ? styles.configDisabled : ''}`}>
                          {item.key}
                        </span>
                        <span className={styles.configValue}>
                          {item.is_module ? '=m' : item.disabled ? 'is not set' : `=${item.value}`}
                        </span>
                        <span className={styles.configHelp}>{item.help_text}</span>
                      </div>)}
                </div>
              </>}
            {!configResult && <div className={styles.emptyState}>{t("Linux.k85")}</div>}
          </div>}

        {activeTab === 'modules' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k86")}</h3>
              <button className={styles.btnAction} onClick={handleLoadModules}>
                {t("Linux.k87")}
              </button>
            </div>
            {selectedModule ? <div className={styles.moduleDetail}>
                <div className={styles.moduleDetailHeader}>
                  <button className={styles.btnBack} onClick={() => setSelectedModule(null)}>
                    {t("Linux.k88")}
                  </button>
                  <h4>{selectedModule.name}</h4>
                  <span className={`${styles.moduleStatus} ${selectedModule.loaded ? styles.loaded : styles.unloaded}`}>
                    {selectedModule.loaded ? t("Linux.k89") : t("Linux.k90")}
                  </span>
                </div>
                <div className={styles.moduleInfo}>
                  <div className={styles.moduleInfoRow}>
                    <span className={styles.moduleInfoLabel}>{t("Linux.k91")}</span>
                    <span className={styles.moduleInfoValue}>{selectedModule.path}</span>
                  </div>
                  <div className={styles.moduleInfoRow}>
                    <span className={styles.moduleInfoLabel}>{t("common.size")}</span>
                    <span className={styles.moduleInfoValue}>{selectedModule.size_mb.toFixed(2)} MB</span>
                  </div>
                  <div className={styles.moduleInfoRow}>
                    <span className={styles.moduleInfoLabel}>{t("Linux.k92")}</span>
                    <span className={styles.moduleInfoValue}>{selectedModule.license}</span>
                  </div>
                  <div className={styles.moduleInfoRow}>
                    <span className={styles.moduleInfoLabel}>{t("common.description")}</span>
                    <span className={styles.moduleInfoValue}>{selectedModule.description}</span>
                  </div>
                  {selectedModule.depends_on.length > 0 && <div className={styles.moduleInfoRow}>
                      <span className={styles.moduleInfoLabel}>{t("Linux.k93")}</span>
                      <span className={styles.moduleInfoValue}>{selectedModule.depends_on.join(', ')}</span>
                    </div>}
                  {selectedModule.used_by.length > 0 && <div className={styles.moduleInfoRow}>
                      <span className={styles.moduleInfoLabel}>{t("Linux.k94")}</span>
                      <span className={styles.moduleInfoValue}>{selectedModule.used_by.join(', ')}</span>
                    </div>}
                </div>
                {selectedModule.parameters.length > 0 && <div className={styles.moduleParams}>
                    <h4>{t("Linux.k95")}</h4>
                    {selectedModule.parameters.map((p, i) => <div key={i} className={styles.moduleParamItem}>
                        <span className={styles.paramName}>{p.name}</span>
                        <span className={styles.paramValue}>={p.value}</span>
                        <span className={styles.paramDesc}>{p.description}</span>
                      </div>)}
                  </div>}
              </div> : <div className={styles.moduleList}>
                {modules.map(m => <div key={m.name} className={styles.moduleItem} onClick={() => setSelectedModule(m)}>
                    <span className={styles.moduleIcon}>🧩</span>
                    <span className={styles.moduleName}>{m.name}</span>
                    <span className={`${styles.moduleStatus} ${m.loaded ? styles.loaded : styles.unloaded}`}>
                      {m.loaded ? 'loaded' : '—'}
                    </span>
                    <span className={styles.moduleSize}>{m.size_mb.toFixed(2)} MB</span>
                    <span className={styles.moduleLicense}>{m.license}</span>
                  </div>)}
                {modules.length === 0 && <div className={styles.emptyState}>{t("Linux.k96")}</div>}
              </div>}
          </div>}

        {activeTab === 'logs' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k97")}</h3>
              <div className={styles.logControls}>
                <select className={styles.formSelect} value={logLevelFilter} onChange={e => setLogLevelFilter(e.target.value)}>
                  <option value="">{t("Linux.k98")}</option>
                  <option value="emerg">EMERG</option>
                  <option value="alert">ALERT</option>
                  <option value="crit">CRIT</option>
                  <option value="err">ERROR</option>
                  <option value="warn">WARNING</option>
                  <option value="notice">NOTICE</option>
                  <option value="info">INFO</option>
                  <option value="debug">DEBUG</option>
                </select>
                <button className={styles.btnAction} onClick={handleLoadLogs}>
                  {t("Linux.k99")}
                </button>
              </div>
            </div>
            {logAnalysis && <>
                <div className={styles.logSummary}>
                  <div className={styles.logSummaryCard}>
                    <span className={styles.logSummaryTitle}>{t("IntelligenceCharts.k4")}</span>
                    <span className={styles.logSummaryValue}>{logAnalysis.total_entries}</span>
                  </div>
                  {Object.entries(logAnalysis.by_level ?? {}).map(([level, count]: [string, any]) => <div key={level} className={styles.logSummaryCard}>
                      <span className={styles.logSummaryTitle} style={{
                color: LOG_LEVEL_COLORS[level]
              }}>
                        {level.toUpperCase()}
                      </span>
                      <span className={styles.logSummaryValue} style={{
                color: LOG_LEVEL_COLORS[level]
              }}>
                        {count}
                      </span>
                    </div>)}
                  {logAnalysis.boot_time && <div className={styles.logSummaryCard}>
                      <span className={styles.logSummaryTitle}>{t("Linux.k100")}</span>
                      <span className={styles.logSummaryValue}>{logAnalysis.boot_time}</span>
                    </div>}
                </div>
                {logAnalysis.summary && <div className={styles.logSummaryText}>
                    <span className={styles.logSummaryLabel}>{t("Linux.k101")}</span>
                    {logAnalysis.summary}
                  </div>}
                {logAnalysis.critical_events.length > 0 && <div className={styles.logCritical}>
                    <h4>{t("Linux.k102")}{logAnalysis.critical_events.length})</h4>
                    {logAnalysis.critical_events.map((e, i) => <div key={i} className={styles.logCriticalItem}>
                        <span className={styles.logTime}>{e.timestamp}</span>
                        <span className={styles.logSource}>{e.source}</span>
                        <span className={styles.logMsg}>{e.message}</span>
                      </div>)}
                  </div>}
                <div className={styles.logList}>
                  {logAnalysis.entries.filter(e => !logLevelFilter || e.level === logLevelFilter).map((e, i) => <div key={i} className={styles.logItem}>
                        <span className={styles.logTime}>{e.timestamp}</span>
                        <span className={styles.logLevel} style={{
                color: LOG_LEVEL_COLORS[e.level] || '#888'
              }}>
                          [{e.level}]
                        </span>
                        <span className={styles.logFacility}>{e.facility}</span>
                        <span className={styles.logMsg}>{e.message}</span>
                      </div>)}
                </div>
              </>}
            {!logAnalysis && <div className={styles.emptyState}>{t("Linux.k103")}</div>}
          </div>}

        {activeTab === 'perf' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k104")}</h3>
            </div>
            <div className={styles.perfForm}>
              <label className={styles.formLabel}>
                {t("Linux.k105")}
                <input className={styles.formInput} type="number" min="5" max="300" value={perfDuration} onChange={e => setPerfDuration(Number(e.target.value))} />
              </label>
              <button className={styles.btnAction} onClick={handlePerf}>
                {t("Linux.k106")}
              </button>
            </div>
            {perfResult && <div className={styles.perfResult}>
                <div className={styles.perfStats}>
                  <span>{t("Linux.k107")} {perfResult.event}</span>
                  <span>{t("Linux.k108")} {perfResult.duration_secs}s</span>
                  <span>{t("Linux.k109")} {perfResult.samples}</span>
                </div>
                {perfResult.top_functions.length > 0 && <div className={styles.perfSection}>
                    <h4>{t("Linux.k110")}</h4>
                    <div className={styles.perfTable}>
                      <div className={styles.perfTableHeader}>
                        <span>{t("Linux.k111")}</span>
                        <span>{t("Linux.k112")}</span>
                        <span>{t("Linux.k113")}</span>
                      </div>
                      {perfResult.top_functions.map((f, i) => <div key={i} className={styles.perfTableRow}>
                          <span className={styles.perfFuncName}>{f.function_name}</span>
                          <span className={styles.perfPercent}>
                            <span className={styles.perfBar} style={{
                    width: `${Math.min(f.overhead_percent, 100)}%`
                  }} />
                            {f.overhead_percent.toFixed(2)}%
                          </span>
                          <span>{f.samples}</span>
                        </div>)}
                    </div>
                  </div>}
                {perfResult.top_symbols.length > 0 && <div className={styles.perfSection}>
                    <h4>{t("Linux.k114")}</h4>
                    <div className={styles.perfTable}>
                      <div className={styles.perfTableHeader}>
                        <span>{t("Linux.k115")}</span>
                        <span>{t("Linux.k112")}</span>
                      </div>
                      {perfResult.top_symbols.map((s, i) => <div key={i} className={styles.perfTableRow}>
                          <span className={styles.perfFuncName}>{s.symbol}</span>
                          <span className={styles.perfPercent}>
                            <span className={styles.perfBar} style={{
                    width: `${Math.min(s.overhead_percent, 100)}%`
                  }} />
                            {s.overhead_percent.toFixed(2)}%
                          </span>
                        </div>)}
                    </div>
                  </div>}
                {perfResult.raw_output && <details className={styles.perfRaw}>
                    <summary>{t("Linux.k116")}</summary>
                    <pre>{perfResult.raw_output}</pre>
                  </details>}
              </div>}
            {!perfResult && <div className={styles.emptyState}>{t("Linux.k117")}</div>}
          </div>}

        {activeTab === 'benchmark' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k118")}</h3>
              <button className={styles.btnAction} onClick={handleBenchmark} disabled={benchmarkRunning}>
                {benchmarkRunning ? t("Linux.k119") : t("Linux.k120")}
              </button>
            </div>
            {benchmarkResult && <>
                <div className={styles.benchmarkScore}>
                  <div className={styles.benchmarkScoreCard}>
                    <span className={styles.benchmarkScoreLabel}>{t("Linux.k121")}</span>
                    <span className={styles.benchmarkScoreValue}>
                      {benchmarkResult.total_score?.toFixed(1) ?? '—'}
                    </span>
                    <span className={styles.benchmarkScoreUnit}>
                      / {benchmarkResult.total_baseline?.toFixed(0) ?? '100'}
                    </span>
                  </div>
                  <div className={styles.benchmarkScoreBar}>
                    <div className={styles.benchmarkScoreFill} style={{
                width: `${Math.min((benchmarkResult.total_score ?? 0) / (benchmarkResult.total_baseline ?? 100) * 100, 100)}%`
              }} />
                  </div>
                </div>
                {benchmarkResult.categories.map(cat => <div key={cat.name} className={styles.benchmarkCategory}>
                    <div className={styles.benchmarkCategoryHeader}>
                      <span className={styles.benchmarkCategoryIcon}>{cat.icon}</span>
                      <span className={styles.benchmarkCategoryLabel}>{cat.label}</span>
                    </div>
                    <div className={styles.benchmarkTestGrid}>
                      {cat.tests.map((test: any) => {
                const ratio = test.baseline_value && test.current_value ? test.lower_is_better ? test.baseline_value / test.current_value : test.current_value / test.baseline_value : 0;
                const pct = Math.min(ratio * 100, 100);
                const isGood = test.lower_is_better ? ratio >= 0.9 : ratio >= 0.9;
                return <div key={test.name} className={styles.benchmarkTestItem}>
                            <div className={styles.benchmarkTestHeader}>
                              <span className={styles.benchmarkTestLabel}>{test.label}</span>
                              <span className={`${styles.benchmarkTestStatus} ${isGood ? styles.benchmarkGood : styles.benchmarkWarn}`}>
                                {isGood ? '✓' : '⚠'}
                              </span>
                            </div>
                            <div className={styles.benchmarkTestValues}>
                              <span className={styles.benchmarkTestCurrent}>
                                {test.current_value?.toLocaleString() ?? '—'}
                              </span>
                              <span className={styles.benchmarkTestUnit}>{test.unit}</span>
                              <span className={styles.benchmarkTestBaseline}>
                                {t("Linux.k122")} {test.baseline_value?.toLocaleString() ?? '—'}
                              </span>
                            </div>
                            <div className={styles.benchmarkTestBar}>
                              <div className={`${styles.benchmarkTestFill} ${isGood ? styles.benchmarkFillGood : styles.benchmarkFillWarn}`} style={{
                      width: `${pct}%`
                    }} />
                            </div>
                          </div>;
              })}
                    </div>
                  </div>)}
              </>}
            {!benchmarkResult && !benchmarkRunning && <div className={styles.emptyState}>
                {t("Linux.k123")}
              </div>}
            {benchmarkRunning && !benchmarkResult && <div className={styles.emptyState}>{t("Linux.k124")}</div>}
          </div>}

        {activeTab === 'stress' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k125")}</h3>
              <button className={styles.btnAction} onClick={handleStress} disabled={stressRunning} style={stressRunning ? {} : {
            background: '#E26D6D',
            borderColor: '#E26D6D'
          }}>
                {stressRunning ? t("Linux.k126") : t("Linux.k127")}
              </button>
            </div>
            {stressResult && <>
                <div className={styles.stressSummary}>
                  <div className={styles.stressSummaryCard}>
                    <span className={styles.stressSummaryLabel}>{t("common.status")}</span>
                    <span className={styles.stressSummaryValue} style={{
                color: stressResult.overall_status === 'done' ? '#7FD962' : '#E26D6D'
              }}>
                      {stressResult.overall_status === 'done' ? t("Linux.k128") : stressResult.overall_status === 'running' ? t("Linux.k129") : t("Linux.k130")}
                    </span>
                  </div>
                  <div className={styles.stressSummaryCard}>
                    <span className={styles.stressSummaryLabel}>{t("Linux.k131")}</span>
                    <span className={styles.stressSummaryValue}>{stressResult.total_duration_secs}s</span>
                  </div>
                  <div className={styles.stressSummaryCard}>
                    <span className={styles.stressSummaryLabel}>{t("Linux.k132")}</span>
                    <span className={styles.stressSummaryValue}>
                      <span style={{
                  color: '#7FD962'
                }}>{stressResult.passed_scenarios}</span>
                      {' / '}
                      <span style={{
                  color: stressResult.failed_scenarios > 0 ? '#E26D6D' : '#7A828E'
                }}>{stressResult.failed_scenarios}</span>
                    </span>
                  </div>
                </div>
                {stressResult.scenarios.map(scenario => <div key={scenario.id} className={styles.stressScenario}>
                    <div className={styles.stressScenarioHeader}>
                      <div className={styles.stressScenarioLeft}>
                        <span className={styles.stressScenarioIcon}>{scenario.icon}</span>
                        <div>
                          <div className={styles.stressScenarioLabel}>{scenario.label}</div>
                          <div className={styles.stressScenarioDesc}>{scenario.description}</div>
                        </div>
                      </div>
                      <div className={styles.stressScenarioRight}>
                        <span className={`${styles.stressScenarioStatus} ${scenario.status === 'done' ? styles.stressStatusDone : scenario.status === 'error' ? styles.stressStatusError : ''}`}>
                          {scenario.status === 'done' ? '✓' : scenario.status === 'running' ? '⏳' : '✗'}
                        </span>
                        <span className={styles.stressScenarioDuration}>{scenario.duration_secs}s</span>
                      </div>
                    </div>
                    {scenario.error_message && <div className={styles.stressScenarioError}>
                        ⚠ {scenario.error_message}
                      </div>}
                    <div className={styles.stressProgress}>
                      <div className={styles.stressProgressFill} style={{
                width: `${scenario.progress_percent}%`
              }} />
                    </div>
                    <div className={styles.stressMetrics}>
                      {scenario.metrics.map((m: any, i: number) => <div key={i} className={styles.stressMetricItem}>
                          <div className={styles.stressMetricHeader}>
                            <span className={styles.stressMetricLabel}>{m.label}</span>
                            <span className={`${styles.stressMetricBadge} ${m.status === 'ok' ? styles.stressMetricOk : m.status === 'warn' ? styles.stressMetricWarn : styles.stressMetricCritical}`}>
                              {m.status === 'ok' ? t("Linux.k133") : m.status === 'warn' ? t("common.warning") : t("Linux.k134")}
                            </span>
                          </div>
                          <div className={styles.stressMetricValues}>
                            <span className={styles.stressMetricValue}>{m.value.toLocaleString()}</span>
                            <span className={styles.stressMetricUnit}>{m.unit}</span>
                            <span className={styles.stressMetricThreshold}>{t("Linux.k135")} {m.threshold.toLocaleString()}</span>
                          </div>
                        </div>)}
                    </div>
                  </div>)}
              </>}
            {!stressResult && !stressRunning && <div className={styles.emptyState}>
                {t("Linux.k136")}
              </div>}
            {stressRunning && !stressResult && <div className={styles.emptyState}>{t("Linux.k137")}</div>}
          </div>}

        {activeTab === 'docker' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k138")}</h3>
              <div style={{
            display: 'flex',
            gap: 8
          }}>
                <button className={styles.btnSmall} onClick={loadDockerContainers} title={t("Linux.k139")}>
                  {t("Linux.k140")}
                </button>
                <button className={styles.btnSmall} onClick={loadDockerImages} title={t("Linux.k141")}>
                  {t("Linux.k142")}
                </button>
              </div>
            </div>

            {dockerLogsLoading && <div style={{
          marginBottom: 16,
          padding: 8,
          color: '#888',
          fontSize: 12
        }}>
                {t("Linux.k143")}
              </div>}

            {dockerLogs && <div style={{
          marginBottom: 16,
          background: '#000',
          border: '1px solid #00FF0040',
          borderRadius: 6,
          padding: 12
        }}>
                <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            marginBottom: 8
          }}>
                  <span style={{
              color: '#00FF00',
              fontSize: 12
            }}>{t("Linux.k144")} {dockerLogs.containerId.substring(0, 12)}</span>
                  <button className={styles.btnSmall} onClick={() => setDockerLogs(null)}>✕</button>
                </div>
                <pre style={{
            color: '#00FF00',
            fontSize: 11,
            maxHeight: 300,
            overflow: 'auto',
            margin: 0,
            fontFamily: 'var(--nt-font-mono)',
            whiteSpace: 'pre-wrap'
          }}>
                  {dockerLogs.logs || t("Linux.k145")}
                </pre>
              </div>}

            <div style={{
          marginBottom: 20
        }}>
              <div className={styles.panelHeader}>
                <h4>{t("Linux.k146")}</h4>
                <span className={styles.count}>{dockerContainers.length} {t("Linux.k147")}</span>
              </div>
              {dockerLoading && <div className={styles.emptyState}>{t("common.loading")}</div>}
              {!dockerLoading && dockerContainers.length === 0 && <div className={styles.emptyState}>{t("Linux.k148")}</div>}
              <div style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 8
          }}>
                {dockerContainers.map((c: any) => <div key={c.id || c.container_id} style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '10px 14px',
              background: '#0a0a10',
              border: '1px solid #00FF0020',
              borderRadius: 6
            }}>
                    <div style={{
                flex: 1
              }}>
                      <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8
                }}>
                        <span style={{
                    color: c.status?.includes('Up') ? '#00FF00' : '#FF0000',
                    fontSize: 12
                  }}>
                          {c.status?.includes('Up') ? '●' : '○'}
                        </span>
                        <span style={{
                    color: '#00FF00',
                    fontSize: 13,
                    fontWeight: 600
                  }}>
                          {c.names || c.name || c.id?.substring(0, 12)}
                        </span>
                        <span style={{
                    color: '#888',
                    fontSize: 11
                  }}>{c.image || c.image_name}</span>
                      </div>
                      <div style={{
                  color: '#666',
                  fontSize: 11,
                  marginTop: 2
                }}>
                        {c.status || c.state} · {c.ports || c.port_mappings || '-'}
                      </div>
                    </div>
                    <div style={{
                display: 'flex',
                gap: 6
              }}>
                      {c.status?.includes('Up') || c.state === 'running' ? <button className={styles.btnSmall} onClick={() => handleDockerStop(c.id || c.container_id)} style={{
                  color: '#FF0000',
                  borderColor: '#FF000040'
                }}>{t("common.stop")}</button> : <button className={styles.btnSmall} onClick={() => handleDockerStart(c.id || c.container_id)} style={{
                  color: '#00FF00',
                  borderColor: '#00FF0040'
                }}>{t("Linux.k149")}</button>}
                      <button className={styles.btnSmall} onClick={() => handleDockerLogs(c.id || c.container_id)}>{t("components.intelligence.ActivityPanel.k3")}</button>
                    </div>
                  </div>)}
              </div>
            </div>

            <div>
              <div className={styles.panelHeader}>
                <h4>{t("Linux.k150")}</h4>
                <span className={styles.count}>{dockerImages.length} {t("Linux.k151")}</span>
              </div>
              {!dockerLoading && dockerImages.length === 0 && <div className={styles.emptyState}>{t("Linux.k152")}</div>}
              <div style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 6
          }}>
                {dockerImages.map((img: any, i: number) => <div key={i} style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '8px 14px',
              background: '#0a0a10',
              border: '1px solid #00F0FF20',
              borderRadius: 6
            }}>
                    <span style={{
                color: '#00F0FF',
                fontSize: 13
              }}>{img.repository || img.name}</span>
                    <div style={{
                display: 'flex',
                gap: 16
              }}>
                      <span style={{
                  color: '#888',
                  fontSize: 11
                }}>{img.tag || 'latest'}</span>
                      <span style={{
                  color: '#666',
                  fontSize: 11
                }}>{img.size || img.size_mb}</span>
                      <span style={{
                  color: '#555',
                  fontSize: 10
                }}>{img.created || img.created_at}</span>
                    </div>
                  </div>)}
              </div>
            </div>
          </div>}

        {activeTab === 'network' && <div className={styles.panel}>
            <div className={styles.panelHeader}>
              <h3>{t("Linux.k153")}</h3>
              <button className={styles.btnSmall} onClick={loadNetworkStats} title={t("Linux.k154")}>
                {t("components.Linux.k21")}
              </button>
            </div>

            {networkLoading && <div className={styles.emptyState}>{t("common.loading")}</div>}

            {networkData && <div>
                <div className={styles.statusGrid}>
                  <div className={styles.statusCard}>
                    <div className={styles.statusCardHeader}>
                      <span className={styles.statusCardIcon}>📥</span>
                      <span>{t("Linux.k155")}</span>
                    </div>
                    <div className={styles.statusCardBody}>
                      <div style={{
                  fontSize: 24,
                  color: '#00FF00',
                  fontWeight: 700
                }}>
                        {formatBytes(networkData.total_received || 0)}
                      </div>
                    </div>
                  </div>
                  <div className={styles.statusCard}>
                    <div className={styles.statusCardHeader}>
                      <span className={styles.statusCardIcon}>📤</span>
                      <span>{t("Linux.k156")}</span>
                    </div>
                    <div className={styles.statusCardBody}>
                      <div style={{
                  fontSize: 24,
                  color: '#00F0FF',
                  fontWeight: 700
                }}>
                        {formatBytes(networkData.total_sent || 0)}
                      </div>
                    </div>
                  </div>
                  <div className={styles.statusCard}>
                    <div className={styles.statusCardHeader}>
                      <span className={styles.statusCardIcon}>🔗</span>
                      <span>{t("Linux.k157")}</span>
                    </div>
                    <div className={styles.statusCardBody}>
                      <div style={{
                  fontSize: 24,
                  color: '#FFD700',
                  fontWeight: 700
                }}>
                        {networkData.active_connections ?? networkData.connection_count ?? '-'}
                      </div>
                    </div>
                  </div>
                  <div className={styles.statusCard}>
                    <div className={styles.statusCardHeader}>
                      <span className={styles.statusCardIcon}>📊</span>
                      <span>{t("Linux.k158")}</span>
                    </div>
                    <div className={styles.statusCardBody}>
                      <div style={{
                  fontSize: 14,
                  color: '#888'
                }}>
                        {t("Linux.k159")} {networkData.packets_received?.toLocaleString() ?? '-'}<br />
                        {t("Linux.k160")} {networkData.packets_sent?.toLocaleString() ?? '-'}
                      </div>
                    </div>
                  </div>
                </div>

                {networkData.interfaces && networkData.interfaces.length > 0 && <div style={{
            marginTop: 16
          }}>
                    <div className={styles.panelHeader}>
                      <h4>{t("Linux.k161")}</h4>
                    </div>
                    <div style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 6
            }}>
                      {networkData.interfaces.map((iface: any, i: number) => <div key={i} style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '8px 14px',
                background: '#0a0a10',
                border: '1px solid #00F0FF20',
                borderRadius: 6
              }}>
                          <span style={{
                  color: '#00F0FF',
                  fontSize: 13
                }}>{iface.name}</span>
                          <div style={{
                  display: 'flex',
                  gap: 16
                }}>
                            <span style={{
                    color: '#888',
                    fontSize: 11
                  }}>IP: {iface.ip || '-'}</span>
                            <span style={{
                    color: '#00FF00',
                    fontSize: 11
                  }}>↓ {formatBytes(iface.rx_bytes || 0)}</span>
                            <span style={{
                    color: '#00F0FF',
                    fontSize: 11
                  }}>↑ {formatBytes(iface.tx_bytes || 0)}</span>
                          </div>
                        </div>)}
                    </div>
                  </div>}
              </div>}

            {!networkData && !networkLoading && <div className={styles.emptyState}>{t("Linux.k162")}</div>}
          </div>}
      </div>
    </div>;
}