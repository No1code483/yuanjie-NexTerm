import { t } from "i18next";
/**
 * TitleBar 自定义标题栏（C3 窗口控制按钮）
 *
 * 功能：
 *   - 自定义标题栏替代系统原生标题栏（tauri.conf.json: decorations=false）
 *   - 拖动区域：mousedown 触发 startDragging，双击切换最大化
 *   - 应用标题/Logo 显示
 *   - WindowControls：最小化 / 最大化/还原 / 关闭
 *   - C3.2 扩展按钮：置顶 / 开发者工具 / 截图 / 录屏
 *   - C3.4 macOS 适配：左上角红绿灯按钮（close/minimize/maximize）
 *   - C3.5 设置面板已迁移至个人中心/设置列表（M3.②），见 TitleBarSettingsPanel 组件
 *   - C3.6 全局快捷键：F11/Ctrl+Shift+T/Ctrl+Shift+S/F12/Ctrl+Shift+R
 *   - 截图：区域选择 UI + getDisplayMedia 捕获 + plugin-fs 保存
 *   - 录屏：MediaRecorder + getDisplayMedia + plugin-fs 保存
 *   - 主题感知：所有颜色使用 CSS 变量，自动跟随主题
 *   - 无障碍：ARIA 标签 + 键盘聚焦
 *
 * 关联文档：功能展望/01_接下来可开发_55/体验深化_82/03_窗口控制按钮_自定义方案_100.md
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { windowControl, checkIsTauri } from '@/lib/tauri';
import { useTitleBarStore, type WindowButtonType, type ScreenshotRegion, DEFAULT_SHORTCUTS } from '@/stores/titleBarStore';
import styles from './TitleBar.module.css';

export function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);
  const isTauri = checkIsTauri();

  // C3：titleBarStore 状态
  const {
    style,
    buttonVisibility,
    buttonOrder,
    isPinned,
    isRecording,
    platform,
    isSelectingScreenshot,
    recordingElapsedSecs,
    togglePin,
    toggleRecording,
    setSelectingScreenshot,
    setLastScreenshotPath,
    tickRecording,
    resetRecordingTimer,
  } = useTitleBarStore();

  // 截图区域选择本地状态
  const [screenshotRegion, setScreenshotRegion] = useState<ScreenshotRegion | null>(null);
  const [screenshotFlash, setScreenshotFlash] = useState(false);
  const [toastMsg, setToastMsg] = useState<string | null>(null);
  // 截图区域选择用的拖动 ref（与窗口拖动无关）
  const dragStartRef = useRef<{ x: number; y: number } | null>(null);

  // 录屏相关 refs
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const recordedChunksRef = useRef<Blob[]>([]);
  const recordingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const toastTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  /** 显示 toast 通知（3 秒后自动消失） */
  const showToast = useCallback((msg: string) => {
    setToastMsg(msg);
    if (toastTimerRef.current) clearTimeout(toastTimerRef.current);
    toastTimerRef.current = setTimeout(() => setToastMsg(null), 3000);
  }, []);

  // 初始化 + 监听最大化状态变化
  useEffect(() => {
    if (!isTauri) return;
    const refreshMaximized = async () => {
      try {
        setIsMaximized(await windowControl.isMaximized());
      } catch { /* ignore */ }
    };
    refreshMaximized();
    // 监听窗口 resize（最大化/还原会触发）
    const handleResize = () => refreshMaximized();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [isTauri]);

  /** 最小化 */
  const handleMinimize = useCallback(() => {
    windowControl.minimize();
  }, []);

  /** 最大化/还原 */
  const handleMaximizeToggle = useCallback(() => {
    windowControl.maximize();
  }, []);

  /** 关闭 */
  const handleClose = useCallback(() => {
    windowControl.close();
  }, []);

  /** C3.2：切换窗口置顶 */
  const handleTogglePin = useCallback(async () => {
    togglePin();
    try {
      await invoke('window_set_always_on_top', { alwaysOnTop: useTitleBarStore.getState().isPinned });
    } catch { /* 非关键功能，静默失败 */ }
  }, [togglePin]);

  /** C3.2：打开开发者工具 */
  const handleDevTools = useCallback(async () => {
    try {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview');
      // Tauri 2.x：openDevTools 在 Webview 类型上未声明但运行时存在
      const webview = getCurrentWebview() as unknown as { openDevTools?: () => void };
      webview.openDevTools?.();
    } catch { /* 非关键功能 */ }
  }, []);

  /** C3.2：截图入口 — 调用后端 window_screenshot 命令（后端会 dispatch 'nexterm:screenshot' 事件） */
  const handleScreenshot = useCallback(async () => {
    try {
      await invoke('window_screenshot');
      // 后端会通过 eval 派发 'nexterm:screenshot' 事件，由下方 useEffect 接收
    } catch {
      // 后端命令失败时，直接进入区域选择模式（前端兜底）
      setSelectingScreenshot(true);
    }
  }, [setSelectingScreenshot]);

  /** C3.2：监听后端派发的 'nexterm:screenshot' 事件，进入区域选择模式 */
  useEffect(() => {
    const handleScreenshotEvent = () => setSelectingScreenshot(true);
    window.addEventListener('nexterm:screenshot', handleScreenshotEvent);
    return () => window.removeEventListener('nexterm:screenshot', handleScreenshotEvent);
  }, [setSelectingScreenshot]);

  /** 截图区域选择：鼠标按下记录起点 */
  const handleScreenshotMouseDown = useCallback((e: React.MouseEvent) => {
    dragStartRef.current = { x: e.clientX, y: e.clientY };
    setScreenshotRegion({ x: e.clientX, y: e.clientY, width: 0, height: 0 });
  }, []);

  /** 截图区域选择：鼠标移动更新选区 */
  const handleScreenshotMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragStartRef.current) return;
    const start = dragStartRef.current;
    const x = Math.min(start.x, e.clientX);
    const y = Math.min(start.y, e.clientY);
    const width = Math.abs(e.clientX - start.x);
    const height = Math.abs(e.clientY - start.y);
    setScreenshotRegion({ x, y, width, height });
  }, []);

  /** 截图区域选择：鼠标释放完成选区，执行截图 */
  const handleScreenshotMouseUp = useCallback(async () => {
    const region = screenshotRegion;
    dragStartRef.current = null;
    setSelectingScreenshot(false);
    setScreenshotRegion(null);
    // 选区太小则视为取消，整屏截图
    const useRegion = region && region.width > 4 && region.height > 4 ? region : null;
    try {
      await captureScreenshot(useRegion);
    } catch (e) {
      console.error('[TitleBar] 截图失败:', e);
      showToast(t('components.TitleBar.k19'));
    }
  }, [screenshotRegion, setSelectingScreenshot, showToast]);

  /** 截图取消（Esc 键） */
  const handleScreenshotCancel = useCallback(() => {
    dragStartRef.current = null;
    setScreenshotRegion(null);
    setSelectingScreenshot(false);
  }, [setSelectingScreenshot]);

  /**
   * C3.2：实际截图逻辑
   * 通过 getDisplayMedia 获取屏幕流 → 绘制到 canvas → 裁剪选区 → 保存到临时目录
   * 注：Tauri 2.11.1 的 WebviewWindow::capture() 在不稳定 API 中，前端走浏览器原生 API
   */
  const captureScreenshot = useCallback(async (region: ScreenshotRegion | null) => {
    const stream = await navigator.mediaDevices.getDisplayMedia({
      video: { frameRate: 30 } as MediaTrackConstraints,
      audio: false,
    });
    const video = document.createElement('video');
    video.srcObject = stream;
    await new Promise<void>((resolve) => {
      video.onloadedmetadata = () => { video.play(); resolve(); };
    });
    // 等一帧确保视频已渲染
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    const canvas = document.createElement('canvas');
    const videoWidth = video.videoWidth;
    const videoHeight = video.videoHeight;
    canvas.width = region ? region.width : videoWidth;
    canvas.height = region ? region.height : videoHeight;
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Canvas 2D context unavailable');
    // 若有选区，按视频原始分辨率比例裁剪
    if (region) {
      const scaleX = videoWidth / window.innerWidth;
      const scaleY = videoHeight / window.innerHeight;
      ctx.drawImage(
        video,
        region.x * scaleX, region.y * scaleY, region.width * scaleX, region.height * scaleY,
        0, 0, region.width, region.height
      );
    } else {
      ctx.drawImage(video, 0, 0, videoWidth, videoHeight);
    }
    stream.getTracks().forEach((track) => track.stop());
    // 触发白闪动画
    setScreenshotFlash(true);
    setTimeout(() => setScreenshotFlash(false), 250);
    // 转 Blob 并保存
    const blob: Blob = await new Promise((resolve, reject) => {
      canvas.toBlob((b) => b ? resolve(b) : reject(new Error('toBlob failed')), 'image/png');
    });
    await saveScreenshotFile(blob);
  }, [showToast]);

  /** 保存截图到临时目录（通过 @tauri-apps/plugin-fs），并复制到剪贴板 */
  const saveScreenshotFile = useCallback(async (blob: Blob) => {
    const ts = Date.now();
    const fileName = `nexterm_screenshot_${ts}.png`;
    try {
      const { writeFile } = await import('@tauri-apps/plugin-fs');
      const { tempDir } = await import('@tauri-apps/api/path');
      const tmpDir = await tempDir();
      const filePath = `${tmpDir}${fileName}`;
      const arrayBuffer = await blob.arrayBuffer();
      await writeFile(filePath, new Uint8Array(arrayBuffer));
      setLastScreenshotPath(filePath);
      showToast(t('components.TitleBar.k17', { arg0: filePath }));
    } catch (e) {
      // Tauri 文件系统不可用（浏览器环境）：降级到下载
      console.warn('[TitleBar] plugin-fs 不可用，降级到浏览器下载:', e);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = fileName;
      a.click();
      URL.revokeObjectURL(url);
      setLastScreenshotPath(fileName);
      showToast(t('components.TitleBar.k18', { arg0: fileName }));
    }
    // 同时尝试复制到剪贴板
    try {
      await navigator.clipboard.write([
        new ClipboardItem({ 'image/png': blob }),
      ]);
    } catch { /* 剪贴板写入失败不阻塞 */ }
  }, [setLastScreenshotPath, showToast]);

  /** C3.2：切换录屏（启动/停止） */
  const handleToggleRecord = useCallback(async () => {
    if (isRecording) {
      // 停止录屏
      if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
        mediaRecorderRef.current.stop();
      }
      if (recordingTimerRef.current) {
        clearInterval(recordingTimerRef.current);
        recordingTimerRef.current = null;
      }
    } else {
      // 启动录屏
      try {
        const stream = await navigator.mediaDevices.getDisplayMedia({
          video: { frameRate: 30 } as MediaTrackConstraints,
          audio: false,
        });
        // 用户在系统对话框中点了"停止共享"时，自动停止录屏
        stream.getVideoTracks()[0].addEventListener('ended', () => {
          if (mediaRecorderRef.current && mediaRecorderRef.current.state !== 'inactive') {
            mediaRecorderRef.current.stop();
          }
          if (recordingTimerRef.current) {
            clearInterval(recordingTimerRef.current);
            recordingTimerRef.current = null;
          }
          toggleRecording();
          resetRecordingTimer();
        });
        const mime = MediaRecorder.isTypeSupported('video/webm;codecs=vp9')
          ? 'video/webm;codecs=vp9'
          : MediaRecorder.isTypeSupported('video/webm;codecs=vp8')
            ? 'video/webm;codecs=vp8'
            : 'video/webm';
        const recorder = new MediaRecorder(stream, { mimeType: mime });
        recordedChunksRef.current = [];
        recorder.ondataavailable = (e) => {
          if (e.data.size > 0) recordedChunksRef.current.push(e.data);
        };
        recorder.onstop = async () => {
          stream.getTracks().forEach((tr) => tr.stop());
          const blob = new Blob(recordedChunksRef.current, { type: mime });
          await saveRecordingFile(blob);
        };
        recorder.start(1000); // 每秒收集一次数据
        mediaRecorderRef.current = recorder;
        toggleRecording();
        resetRecordingTimer();
        recordingTimerRef.current = setInterval(() => tickRecording(), 1000);
      } catch (e) {
        console.error('[TitleBar] 录屏启动失败:', e);
        showToast(t('components.TitleBar.k22'));
      }
    }
  }, [isRecording, toggleRecording, tickRecording, resetRecordingTimer, showToast]);

  /** 保存录屏文件到临时目录 */
  const saveRecordingFile = useCallback(async (blob: Blob) => {
    const ts = Date.now();
    const fileName = `nexterm_recording_${ts}.webm`;
    try {
      const { writeFile } = await import('@tauri-apps/plugin-fs');
      const { tempDir } = await import('@tauri-apps/api/path');
      const tmpDir = await tempDir();
      const filePath = `${tmpDir}${fileName}`;
      const arrayBuffer = await blob.arrayBuffer();
      await writeFile(filePath, new Uint8Array(arrayBuffer));
      showToast(t('components.TitleBar.k20', { arg0: filePath }));
    } catch (e) {
      console.warn('[TitleBar] plugin-fs 不可用，降级到浏览器下载:', e);
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = fileName;
      a.click();
      URL.revokeObjectURL(url);
      showToast(t('components.TitleBar.k21', { arg0: fileName }));
    }
  }, [showToast]);

  /** C3.6：全局快捷键注册 */
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Esc：截图选区取消
      if (e.key === 'Escape' && isSelectingScreenshot) {
        e.preventDefault();
        handleScreenshotCancel();
        return;
      }
      // F11：全屏切换
      if (e.key === 'F11') {
        e.preventDefault();
        if (isTauri) windowControl.maximize();
        return;
      }
      // F12：开发者工具
      if (e.key === 'F12') {
        e.preventDefault();
        handleDevTools();
        return;
      }
      // Ctrl+Shift+T：切换置顶
      if (e.ctrlKey && e.shiftKey && (e.key === 'T' || e.key === 't')) {
        e.preventDefault();
        handleTogglePin();
        return;
      }
      // Ctrl+Shift+S：截图
      if (e.ctrlKey && e.shiftKey && (e.key === 'S' || e.key === 's')) {
        e.preventDefault();
        handleScreenshot();
        return;
      }
      // Ctrl+Shift+R：切换录屏
      if (e.ctrlKey && e.shiftKey && (e.key === 'R' || e.key === 'r')) {
        e.preventDefault();
        handleToggleRecord();
        return;
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isTauri, isSelectingScreenshot, handleDevTools, handleTogglePin, handleScreenshot, handleToggleRecord, handleScreenshotCancel]);

  // 清理计时器（组件卸载时）
  useEffect(() => {
    return () => {
      if (recordingTimerRef.current) clearInterval(recordingTimerRef.current);
      if (toastTimerRef.current) clearTimeout(toastTimerRef.current);
    };
  }, []);

  // 非 Tauri 环境（如纯浏览器开发）不渲染标题栏
  if (!isTauri) return null;

  /** 格式化录屏计时 mm:ss */
  const formatRecordingTime = (secs: number) => {
    const m = Math.floor(secs / 60).toString().padStart(2, '0');
    const s = (secs % 60).toString().padStart(2, '0');
    return `${m}:${s}`;
  };

  return (
    <>
      {/* 合并标题栏后仅渲染右侧控制区（窗口控制 + 设置面板） */}
      <div className={styles.titleBarWrapper} data-style={style} data-platform={platform}>
        <div className={styles.rightSection}>
          {/* C3.2：录屏计时指示器 */}
          {isRecording && (
            <div className={styles.recordIndicator} aria-live="polite">
              <span className={styles.recordDot} />
              <span>{formatRecordingTime(recordingElapsedSecs)}</span>
            </div>
          )}

          <div className={styles.windowControls} role="group" aria-label={t('components.TitleBar.k1')}>
            {/* C3.3：按 buttonOrder 渲染按钮 */}
            {buttonOrder.map((btnType: WindowButtonType) => {
              switch (btnType) {
                case 'pin':
                  return buttonVisibility.pin ? (
                    <button key="pin" type="button"
                      className={`${styles.controlBtn} ${isPinned ? styles.activeBtn : ''}`}
                      onClick={handleTogglePin}
                      aria-label={t('components.TitleBar.k3')}
                      title={`${t('components.TitleBar.k3')} (${DEFAULT_SHORTCUTS.pin})`}
                    >
                      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                        <path d="M6 1 L7 5 L11 6 L7 7 L6 11 L5 7 L1 6 L5 5 Z" fill="currentColor" opacity={isPinned ? 1 : 0.5} />
                      </svg>
                    </button>
                  ) : null;
                case 'devtools':
                  return buttonVisibility.devtools ? (
                    <button key="devtools" type="button"
                      className={styles.controlBtn}
                      onClick={handleDevTools}
                      aria-label={t('components.TitleBar.k4')}
                      title={`${t('components.TitleBar.k4')} (${DEFAULT_SHORTCUTS.devtools})`}
                    >
                      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                        <path d="M3 2 H9 V10 H3 Z M5 5 H7 M5 7 H7" stroke="currentColor" fill="none" strokeWidth="1" />
                      </svg>
                    </button>
                  ) : null;
                case 'screenshot':
                  return buttonVisibility.screenshot ? (
                    <button key="screenshot" type="button"
                      className={styles.controlBtn}
                      onClick={handleScreenshot}
                      aria-label={t('components.TitleBar.k5')}
                      title={`${t('components.TitleBar.k5')} (${DEFAULT_SHORTCUTS.screenshot})`}
                    >
                      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                        <rect x="1.5" y="3.5" width="9" height="6" stroke="currentColor" fill="none" strokeWidth="1" />
                        <circle cx="6" cy="6.5" r="1.5" stroke="currentColor" fill="none" strokeWidth="1" />
                      </svg>
                    </button>
                  ) : null;
                case 'record':
                  return buttonVisibility.record ? (
                    <button key="record" type="button"
                      className={`${styles.controlBtn} ${isRecording ? styles.activeBtn : ''}`}
                      onClick={handleToggleRecord}
                      aria-label={t('components.TitleBar.k6')}
                      title={`${t('components.TitleBar.k6')} (${DEFAULT_SHORTCUTS.record})`}
                    >
                      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                        <circle cx="6" cy="6" r="3.5" fill={isRecording ? '#FF0000' : 'none'} stroke="currentColor" strokeWidth="1" />
                      </svg>
                    </button>
                  ) : null;
                case 'minimize':
                  return (
                    <button key="minimize" type="button"
                      className={`${styles.controlBtn} ${styles.minimizeBtn}`}
                      onClick={handleMinimize}
                      aria-label={t('common.minimize')}
                      title={t('common.minimize')}
                    >
                      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                        <rect x="2" y="5.5" width="8" height="1" fill="currentColor" />
                      </svg>
                    </button>
                  );
                case 'maximize':
                  return (
                    <button key="maximize" type="button"
                      className={`${styles.controlBtn} ${styles.maximizeBtn}`}
                      onClick={handleMaximizeToggle}
                      aria-label={isMaximized ? t('components.TitleBar.k2') : t('common.maximize')}
                      title={isMaximized ? t('components.TitleBar.k2') : t('common.maximize')}
                    >
                      {isMaximized ? (
                        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                          <rect x="2.5" y="4" width="5" height="5" stroke="currentColor" fill="none" strokeWidth="1" />
                          <path d="M4 4 V2 H9 V7 H7" stroke="currentColor" fill="none" strokeWidth="1" />
                        </svg>
                      ) : (
                        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                          <rect x="2" y="2" width="8" height="8" stroke="currentColor" fill="none" strokeWidth="1" />
                        </svg>
                      )}
                    </button>
                  );
                case 'close':
                  return (
                    <button key="close" type="button"
                      className={`${styles.controlBtn} ${styles.closeBtn}`}
                      onClick={handleClose}
                      aria-label={t('common.close')}
                      title={t('common.close')}
                    >
                      <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
                        <path d="M2 2 L10 10 M10 2 L2 10" stroke="currentColor" strokeWidth="1.2" />
                      </svg>
                    </button>
                  );
                default:
                  return null;
              }
            })}
          </div>
        </div>
      </div>

      {/* C3.2：截图区域选择覆盖层 */}
      {isSelectingScreenshot && (
        <div
          className={styles.screenshotOverlay}
          onMouseDown={handleScreenshotMouseDown}
          onMouseMove={handleScreenshotMouseMove}
          onMouseUp={handleScreenshotMouseUp}
        >
          <div className={styles.screenshotHint}>
            {t('components.TitleBar.k24')}
          </div>
          {screenshotRegion && screenshotRegion.width > 0 && (
            <div
              className={styles.screenshotSelection}
              style={{
                left: screenshotRegion.x,
                top: screenshotRegion.y,
                width: screenshotRegion.width,
                height: screenshotRegion.height,
              }}
            />
          )}
        </div>
      )}

      {/* C3.2：截图白闪动画 */}
      {screenshotFlash && <div className={styles.screenshotFlash} aria-hidden="true" />}

      {/* C3.2：通知 toast（截图/录屏结果） */}
      {toastMsg && (
        <div className={styles.notificationToast} role="status" aria-live="polite">
          {toastMsg}
        </div>
      )}
    </>
  );
}

export default TitleBar;
