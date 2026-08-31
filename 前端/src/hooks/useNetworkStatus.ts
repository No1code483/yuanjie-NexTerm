/**
 * useNetworkStatus — 网络在线状态检测（A5 离线同步 Phase 3 Task 1）
 *
 * 职责：
 * 1. 调用 sync_get_network_status 获取初始网络状态
 * 2. 监听 Tauri event 'network-status-changed' 订阅后端推送的状态变更
 * 3. 提供 checkNow() 立即触发检测
 *
 * 设计约束（项目核心设计意图 §八）：
 * - 独立于 syncStore，直接调用 sync IPC 层，避免循环依赖
 * - 非侵入式：非 Tauri 环境下事件订阅静默降级，不影响渲染
 *
 * 设计文档：功能展望/平台级增强/04_离线与同步机制.md §Phase 3
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { sync } from '@/lib/ipc';

export interface UseNetworkStatusReturn {
  isOnline: boolean;
  lastChecked: number | null; // 毫秒时间戳
  checkNow: () => void;
}

/**
 * 后端 last_checked 单位未在接口约定中明确（可能秒或毫秒）。
 * 统一归一化为毫秒：小于 1e12（约 2001-09-09）视为秒级，乘 1000。
 */
function normalizeTimestamp(ts: number): number {
  return ts > 0 && ts < 1e12 ? ts * 1000 : ts;
}

export function useNetworkStatus(): UseNetworkStatusReturn {
  // 乐观默认在线，避免首屏闪烁离线横幅
  const [isOnline, setIsOnline] = useState(true);
  const [lastChecked, setLastChecked] = useState<number | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);

  const applyStatus = useCallback(
    (status: { is_online: boolean; last_checked: number } | undefined | null) => {
      if (!status) return;
      setIsOnline(!!status.is_online);
      setLastChecked(normalizeTimestamp(status.last_checked));
    },
    [],
  );

  // 初始查询 + 事件订阅
  useEffect(() => {
    let cancelled = false;

    // 1. 获取初始状态
    sync
      .getNetworkStatus()
      .then((res) => {
        if (cancelled) return;
        if (res.code === 0 && res.data) {
          applyStatus(res.data);
        }
      })
      .catch((e) => {
        console.warn('[useNetworkStatus] 初始查询失败:', e);
      });

    // 2. 订阅后端事件
    const setupListener = async () => {
      if (typeof window === 'undefined' || !window.__TAURI__) return;
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const unlisten = await listen<{ is_online: boolean; last_checked: number }>(
          'network-status-changed',
          (event) => {
            applyStatus(event.payload);
          },
        );
        if (cancelled) {
          unlisten();
          return;
        }
        unlistenRef.current = unlisten;
      } catch (e) {
        console.warn('[useNetworkStatus] 事件订阅失败:', e);
      }
    };
    setupListener();

    return () => {
      cancelled = true;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, [applyStatus]);

  // 立即触发检测
  const checkNow = useCallback(() => {
    sync
      .checkNetworkNow()
      .then((res) => {
        if (res.code === 0 && res.data) {
          applyStatus(res.data);
        }
      })
      .catch((e) => {
        console.warn('[useNetworkStatus] 立即检测失败:', e);
      });
  }, [applyStatus]);

  return { isOnline, lastChecked, checkNow };
}

export default useNetworkStatus;
