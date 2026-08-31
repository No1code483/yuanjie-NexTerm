import { t } from "i18next";
import { useState, useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';
import { linux, type ApiResponse } from '@/lib/ipc';
import ConfirmDialog from '@/components/ConfirmDialog';
import styles from './Linux.module.css';
interface KernelVersion {
  version: string;
  major: number;
  minor: number;
  patch: number;
  release_type: string;
  release_date: string;
  download_url: string;
  size_mb: number;
  is_lts: boolean;
  is_stable: boolean;
  changelog: string;
}
interface KernelSource {
  version: string;
  download_url: string;
  tarball_path: string;
  extracted_dir: string;
  size_bytes: number;
  download_date: string;
}
interface DirectoryEntry {
  name: string;
  is_dir: boolean;
  size: number;
  modified: string;
  path: string;
}
interface SourceDirectory {
  current_path: string;
  entries: DirectoryEntry[];
  total_count: number;
}
interface FileContent {
  file_path: string;
  file_name: string;
  content: string;
  total_lines: number;
  display_lines: string;
  language: string;
  size_bytes: number;
}
interface SearchResult {
  file_path: string;
  file_name: string;
  line_number: number;
  line_content: string;
  context_before: string[];
  context_after: string[];
}
interface SearchResponse {
  query: string;
  version: string;
  total_matches: number;
  files_searched: number;
  search_time_ms: number;
  results: SearchResult[];
}
interface EnvironmentInfo {
  data_dir: string;
  downloaded_versions: string[];
  active_version: string | null;
  total_disk_usage_mb: number;
  available_versions: number;
}
type TabKey = 'browser' | 'versions' | 'search' | 'info';
export default function LinuxEnvironment() {
  const [searchParams] = useSearchParams();
  const initialTab = searchParams.get('tab') as TabKey || 'browser';
  const initialDownload = searchParams.get('download') || '';
  const initialSearch = searchParams.get('search') || '';
  const initialPath = searchParams.get('path') || '';
  const initialRemove = searchParams.get('remove') || '';
  const [activeTab, setActiveTab] = useState<TabKey>(initialTab);
  const [envInfo, setEnvInfo] = useState<EnvironmentInfo | null>(null);
  const [versions, setVersions] = useState<KernelVersion[]>([]);
  const [activeVersion, setActiveVersion] = useState<string>('');
  const [sourceDir, setSourceDir] = useState<SourceDirectory | null>(null);
  const [currentPath, setCurrentPath] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [searchResults, setSearchResults] = useState<SearchResponse | null>(null);
  const [fileContent, setFileContent] = useState<FileContent | null>(null);
  const [downloadVersion, setDownloadVersion] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');

  // ===== 确认弹窗（删除内核） =====
  const [confirmDialog, setConfirmDialog] = useState<{
    isOpen: boolean;
    targetName: string;
    onConfirm: () => void | Promise<void>;
  }>({
    isOpen: false,
    targetName: '',
    onConfirm: () => {}
  });
  const [downloadProgress, setDownloadProgress] = useState('');
  useEffect(() => {
    loadEnvironment();
  }, []);
  useEffect(() => {
    if (initialDownload) {
      setDownloadVersion(initialDownload);
      setActiveTab('versions');
    }
    if (initialSearch) {
      setSearchQuery(initialSearch);
      setActiveTab('search');
    }
    if (initialPath) {
      setCurrentPath(initialPath);
      setActiveTab('browser');
    }
    if (initialRemove) {
      handleRemoveKernel(initialRemove);
    }
  }, [initialDownload, initialSearch, initialPath, initialRemove]);
  useEffect(() => {
    if (activeVersion && activeTab === 'browser') {
      browseDirectory(currentPath);
    }
  }, [activeVersion, activeTab, currentPath]);
  const apiCall = async <T,>(fn: () => Promise<ApiResponse<T>>): Promise<T | null> => {
    try {
      const res = await fn();
      if (res.code === 0 && res.data !== undefined) return res.data;
      setError(res.message || t("common.failed"));
      return null;
    } catch (e: any) {
      setError(e?.message || String(e));
      return null;
    }
  };
  const loadEnvironment = async () => {
    setIsLoading(true);
    setError('');
    const info = await apiCall<EnvironmentInfo>(() => linux.getEnvironment());
    if (info) {
      setEnvInfo(info);
      if (info.active_version) {
        setActiveVersion(info.active_version);
      } else if ((info.downloaded_versions?.length ?? 0) > 0) {
        setActiveVersion(info.downloaded_versions![0]);
      }
    }
    setIsLoading(false);
  };
  const loadVersions = async () => {
    setIsLoading(true);
    setError('');
    const data = await apiCall<KernelVersion[]>(() => linux.listVersions());
    if (data) setVersions(data);
    setIsLoading(false);
  };
  const browseDirectory = async (path?: string) => {
    if (!activeVersion) return;
    setIsLoading(true);
    setError('');
    setFileContent(null);
    const data = await apiCall<SourceDirectory>(() => linux.listDirectory(activeVersion, path || undefined));
    if (data) {
      setSourceDir(data);
      setCurrentPath(data.current_path);
    }
    setIsLoading(false);
  };
  const viewFile = async (filePath: string) => {
    if (!activeVersion) return;
    setIsLoading(true);
    setError('');
    const data = await apiCall<FileContent>(() => linux.viewFile(activeVersion, filePath));
    if (data) setFileContent(data);
    setIsLoading(false);
  };
  const executeSearch = async () => {
    if (!activeVersion || !searchQuery.trim()) return;
    setIsLoading(true);
    setError('');
    setFileContent(null);
    const data = await apiCall<SearchResponse>(() => linux.searchSource(activeVersion, searchQuery.trim(), 50));
    if (data) setSearchResults(data);
    setIsLoading(false);
  };
  const handleDownload = async (version: string) => {
    setDownloadVersion(version);
    setIsLoading(true);
    setError('');
    setDownloadProgress(t("components.Linux.k1", {
      version: version
    }));
    const data = await apiCall<KernelSource>(() => linux.downloadKernel(version));
    if (data) {
      setActiveVersion(version);
      setDownloadProgress('');
      loadEnvironment();
    } else {
      setDownloadProgress('');
    }
    setIsLoading(false);
  };
  const handleRemoveKernel = async (version: string) => {
    setConfirmDialog({
      isOpen: true,
      targetName: t("components.Linux.k2", {
        version: version
      }),
      onConfirm: async () => {
        setIsLoading(true);
        setError('');
        await apiCall(() => linux.removeKernel(version));
        loadEnvironment();
        setIsLoading(false);
      }
    });
  };
  const handleSetActive = async (version: string) => {
    await apiCall(() => linux.setActive(version));
    setActiveVersion(version);
    setSourceDir(null);
    setFileContent(null);
    setSearchResults(null);
    browseDirectory('');
  };
  const sortedEntries = () => {
    if (!sourceDir?.entries) return [];
    return [...sourceDir.entries].sort((a, b) => {
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
      return a.name.localeCompare(b.name);
    });
  };
  const getReleaseBadge = (v: KernelVersion) => {
    if (v.is_lts) return <span className={styles.badgeLts}>LTS</span>;
    if (v.is_stable) return <span className={styles.badgeStable}>Stable</span>;
    if (v.release_type === 'mainline') return <span className={styles.badgeMainline}>Mainline</span>;
    return null;
  };
  const formatBytes = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };
  const getFileIcon = (name: string, isDir: boolean) => {
    if (isDir) return '📁';
    if (name.endsWith('.c')) return '⚙️';
    if (name.endsWith('.h')) return '📋';
    if (name.endsWith('.rs')) return '🦀';
    if (name.endsWith('.py')) return '🐍';
    if (name.endsWith('.sh')) return '💻';
    if (name.endsWith('.md') || name.endsWith('.txt')) return '📄';
    if (name.endsWith('.S') || name.endsWith('.asm')) return '🔧';
    if (name === 'Makefile' || name === 'Kconfig' || name === 'Kbuild') return '🔨';
    return '📄';
  };
  return <div className={styles.linuxContainer}>
      <div className={styles.linuxSidebar}>
        <div className={styles.sidebarHeader}>
          <span className={styles.sidebarIcon}>🐧</span>
          <h2 className={styles.sidebarTitle}>{t("components.Linux.k3")}</h2>
        </div>
        <nav className={styles.sidebarNav}>
          <button className={`${styles.navItem} ${activeTab === 'browser' ? styles.navItemActive : ''}`} onClick={() => {
          setActiveTab('browser');
          setSearchResults(null);
        }} disabled={!activeVersion}>
            {t("components.Linux.k4")}
          </button>
          <button className={`${styles.navItem} ${activeTab === 'versions' ? styles.navItemActive : ''}`} onClick={() => {
          setActiveTab('versions');
          loadVersions();
        }}>
            {t("components.Linux.k5")}
          </button>
          <button className={`${styles.navItem} ${activeTab === 'search' ? styles.navItemActive : ''}`} onClick={() => setActiveTab('search')} disabled={!activeVersion}>
            {t("components.Linux.k6")}
          </button>
          <button className={`${styles.navItem} ${activeTab === 'info' ? styles.navItemActive : ''}`} onClick={() => {
          setActiveTab('info');
          loadEnvironment();
        }}>
            {t("components.Linux.k7")}
          </button>
        </nav>

        {envInfo && <div className={styles.sidebarInfo}>
            <div className={styles.sidebarInfoItem}>
              <span className={styles.sidebarInfoLabel}>{t("components.Linux.k8")}</span>
              <span className={styles.sidebarInfoValue}>
                {envInfo.active_version || t("components.Linux.k9")}
              </span>
            </div>
            <div className={styles.sidebarInfoItem}>
              <span className={styles.sidebarInfoLabel}>{t("components.Linux.k10")}</span>
              <span className={styles.sidebarInfoValue}>
                {envInfo.downloaded_versions?.length ?? 0} {t("components.Linux.k11")}
              </span>
            </div>
            <div className={styles.sidebarInfoItem}>
              <span className={styles.sidebarInfoLabel}>{t("components.Linux.k12")}</span>
              <span className={styles.sidebarInfoValue}>
                {envInfo.total_disk_usage_mb?.toFixed(1) ?? '0.0'} MB
              </span>
            </div>
          </div>}
      </div>

      <div className={styles.linuxMain}>
        {error && <div className={styles.errorBar}>
            <span>{error}</span>
            <button onClick={() => setError('')} className={styles.errorClose}>×</button>
          </div>}

        {isLoading && <div className={styles.loadingBar}>
            <span className={styles.spinner} />
            {downloadProgress || t("common.loading")}
          </div>}

        {!activeVersion && activeTab !== 'versions' && activeTab !== 'info' && <div className={styles.emptyState}>
            <div className={styles.emptyIcon}>📦</div>
            <h3>{t("components.Linux.k13")}</h3>
            <p>{t("components.Linux.k14")}</p>
            <button className={styles.btnPrimary} onClick={() => {
          setActiveTab('versions');
          loadVersions();
        }}>
              {t("components.Linux.k15")}
            </button>
          </div>}

        {activeTab === 'browser' && activeVersion && <div className={styles.browserPanel}>
            <div className={styles.panelHeader}>
              <h3>{t("components.Linux.k16")}</h3>
              <span className={styles.versionTag}>v{activeVersion}</span>
            </div>
            <div className={styles.breadcrumbs}>
              <button onClick={() => browseDirectory('')} className={styles.breadcrumbLink}>
                /
              </button>
              {currentPath.split('/').filter(Boolean).map((segment, i, arr) => <span key={i}>
                  <span className={styles.breadcrumbSep}>/</span>
                  <button onClick={() => browseDirectory(arr.slice(0, i + 1).join('/'))} className={styles.breadcrumbLink}>
                    {segment}
                  </button>
                </span>)}
            </div>
            {sourceDir && <div className={styles.fileList}>
                {currentPath !== '' && <button className={styles.fileRow} onClick={() => {
            const parent = currentPath.split('/').slice(0, -1).join('/');
            browseDirectory(parent || undefined);
          }}>
                    <span className={styles.fileIcon}>📁</span>
                    <span className={styles.fileName}>..</span>
                  </button>}
                {sortedEntries().slice(0, 200).map((entry, i) => <button key={i} className={styles.fileRow} onClick={() => {
            if (entry.is_dir) {
              browseDirectory(entry.path);
            } else {
              viewFile(entry.path);
            }
          }}>
                    <span className={styles.fileIcon}>{getFileIcon(entry.name, entry.is_dir)}</span>
                    <span className={styles.fileName}>{entry.name}{entry.is_dir ? '/' : ''}</span>
                    <span className={styles.fileSize}>{entry.is_dir ? '' : formatBytes(entry.size)}</span>
                  </button>)}
                {sourceDir?.entries.length > 200 && <div className={styles.fileTruncated}>
                    {t("components.Linux.k17")} {sourceDir?.entries.length ?? 0} {t("components.Linux.k18")}
                  </div>}
              </div>}
            {fileContent && <div className={styles.filePanel}>
                <div className={styles.filePanelHeader}>
                  <span>📄 {fileContent?.file_name ?? ''}</span>
                  <span className={styles.fileMeta}>
                    {fileContent?.total_lines ?? 0} {t("components.Linux.k19")} {formatBytes(fileContent?.size_bytes ?? 0)}
                  </span>
                  <button className={styles.fileCloseBtn} onClick={() => setFileContent(null)}>
                    ×
                  </button>
                </div>
                <pre className={styles.fileContent}>
                  <code>{fileContent?.content ?? ''}</code>
                </pre>
              </div>}
          </div>}

        {activeTab === 'versions' && <div className={styles.versionsPanel}>
            <div className={styles.panelHeader}>
              <h3>{t("components.Linux.k20")}</h3>
              <button className={styles.btnSmall} onClick={loadVersions} disabled={isLoading}>
                {t("components.Linux.k21")}
              </button>
            </div>
            {envInfo && (envInfo.downloaded_versions?.length ?? 0) > 0 && <div className={styles.sectionLabel}>{t("components.Linux.k22")}</div>}
            {(envInfo?.downloaded_versions ?? []).map(v => <div key={v} className={`${styles.versionCard} ${v === activeVersion ? styles.versionCardActive : ''}`}>
                <div className={styles.versionInfo}>
                  <span className={styles.versionName}>v{v}</span>
                  {v === activeVersion && <span className={styles.versionActiveTag}>{t("components.Linux.k23")}</span>}
                </div>
                <div className={styles.versionActions}>
                  {v !== activeVersion && <button className={styles.btnSmall} onClick={() => handleSetActive(v)}>
                      {t("components.Linux.k24")}
                    </button>}
                  <button className={styles.btnDangerSmall} onClick={() => handleRemoveKernel(v)}>
                    {t("common.delete")}
                  </button>
                </div>
              </div>)}

            <div className={styles.sectionLabel}>
              {t("components.Linux.k25")}
              {downloadVersion && <span className={styles.downloadHint}> {t("components.Linux.k26")}{downloadVersion}</span>}
            </div>
            {versions.length === 0 && !isLoading && <div className={styles.emptySmall}>
                <p>{t("components.Linux.k27")}</p>
                <button className={styles.btnSmall} onClick={loadVersions}>
                  {t("components.Linux.k28")}
                </button>
              </div>}
            <div className={styles.versionGrid}>
              {(versions ?? []).slice(0, 30).map((v, i) => <div key={i} className={styles.versionCard}>
                  <div className={styles.versionHeader}>
                    <span className={styles.versionName}>v{v.version}</span>
                    {getReleaseBadge(v)}
                  </div>
                  <div className={styles.versionMeta}>
                    <span>{v.release_date ?? ''}</span>
                    <span>{v.size_mb?.toFixed(1) ?? '0.0'} MB</span>
                  </div>
                  <div className={styles.versionActions}>
                    <button className={styles.btnPrimary} onClick={() => handleDownload(v.version)} disabled={isLoading || (envInfo?.downloaded_versions ?? []).includes(v.version)}>
                      {(envInfo?.downloaded_versions ?? []).includes(v.version) ? t("components.Linux.k10") : t("common.download")}
                    </button>
                  </div>
                </div>)}
            </div>
          </div>}

        {activeTab === 'search' && activeVersion && <div className={styles.searchPanel}>
            <div className={styles.panelHeader}>
              <h3>{t("components.Linux.k29")}</h3>
              <span className={styles.versionTag}>v{activeVersion}</span>
            </div>
            <div className={styles.searchBar}>
              <input className={styles.searchInput} type="text" placeholder={t("components.Linux.k30")} value={searchQuery} onChange={e => setSearchQuery(e.target.value)} onKeyDown={e => e.key === 'Enter' && executeSearch()} />
              <button className={styles.btnPrimary} onClick={executeSearch} disabled={isLoading || !searchQuery.trim()}>
                {t("common.search")}
              </button>
            </div>
            {searchResults && <div className={styles.searchResults}>
                <div className={styles.searchSummary}>
                  {t("components.Linux.k31")} <strong>{searchResults?.total_matches ?? 0}</strong> {t("components.Linux.k32")} <strong>{searchResults?.files_searched ?? 0}</strong> {t("components.Linux.k33")} <strong>{searchResults?.search_time_ms ?? 0}ms</strong>
                </div>
                {searchResults?.results.length === 0 && <div className={styles.emptySmall}>{t("components.Linux.k34")}</div>}
                {(searchResults?.results ?? []).slice(0, 100).map((result, i) => <button key={i} className={styles.searchResultItem} onClick={() => viewFile(result?.file_path ?? '')}>
                    <div className={styles.searchResultHeader}>
                      <span className={styles.searchFileName}>{result?.file_name ?? ''}</span>
                      <span className={styles.searchLineNum}>{t("components.CodePreview.k1")} {result?.line_number ?? 0}</span>
                    </div>
                    <div className={styles.searchResultPath}>{result?.file_path ?? ''}</div>
                    {(result?.context_before?.length ?? 0) > 0 && <div className={styles.searchContext}>
                        {result.context_before!.map((line, j) => <div key={j} className={styles.contextLine}>{line}</div>)}
                      </div>}
                    <div className={styles.searchMatchLine}>{result?.line_content ?? ''}</div>
                    {(result?.context_after?.length ?? 0) > 0 && <div className={styles.searchContext}>
                        {result.context_after!.map((line, j) => <div key={j} className={styles.contextLine}>{line}</div>)}
                      </div>}
                  </button>)}
                {searchResults?.results.length > 100 && <div className={styles.fileTruncated}>
                    {t("components.Linux.k35")} {searchResults.results.length} {t("components.Linux.k18")}
                  </div>}
              </div>}
          </div>}

        {activeTab === 'info' && <div className={styles.infoPanel}>
            <div className={styles.panelHeader}>
              <h3>{t("components.Linux.k36")}</h3>
              <button className={styles.btnSmall} onClick={loadEnvironment} disabled={isLoading}>
                {t("components.Linux.k21")}
              </button>
            </div>
            {envInfo && <div className={styles.infoGrid}>
                <div className={styles.infoCard}>
                  <span className={styles.infoLabel}>{t("components.Linux.k37")}</span>
                  <span className={styles.infoValue}>{envInfo.data_dir ?? t("components.GroupChatOrchestrationPanel.k35")}</span>
                </div>
                <div className={styles.infoCard}>
                  <span className={styles.infoLabel}>{t("components.Linux.k8")}</span>
                  <span className={styles.infoValue}>{envInfo.active_version || t("components.Linux.k9")}</span>
                </div>
                <div className={styles.infoCard}>
                  <span className={styles.infoLabel}>{t("components.Linux.k38")}</span>
                  <span className={styles.infoValue}>{envInfo.available_versions ?? 0}</span>
                </div>
                <div className={styles.infoCard}>
                  <span className={styles.infoLabel}>{t("components.Linux.k12")}</span>
                  <span className={styles.infoValue}>{envInfo.total_disk_usage_mb?.toFixed(1) ?? '0.0'} MB</span>
                </div>
                {envInfo?.downloaded_versions?.length > 0 && <div className={styles.infoCardFull}>
                    <span className={styles.infoLabel}>{t("components.Linux.k39")}</span>
                    <div className={styles.versionList}>
                      {(envInfo?.downloaded_versions ?? []).map(v => <span key={v} className={styles.versionChip}>
                          v{v}
                          {v === envInfo.active_version ? t("components.Linux.k40") : ''}
                        </span>)}
                    </div>
                  </div>}
              </div>}
          </div>}
      </div>

      {/* 确认弹窗（删除内核） */}
      <ConfirmDialog isOpen={confirmDialog.isOpen} onClose={() => setConfirmDialog({
      ...confirmDialog,
      isOpen: false
    })} onConfirm={confirmDialog.onConfirm} targetName={confirmDialog.targetName} type="danger" message={t("components.Linux.k41", {
      targetName: confirmDialog.targetName
    })} />
    </div>;
}