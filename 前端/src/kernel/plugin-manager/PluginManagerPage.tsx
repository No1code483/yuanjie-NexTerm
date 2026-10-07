import { useCallback, useEffect, useState, Fragment } from 'react';
import { invoke } from '@tauri-apps/api/core';
import ConfirmDialog from '@/components/ConfirmDialog';
import { bus } from '@/kernel/event-bus';
import type { PluginId } from '@/kernel/types';
import {
  PLUGIN_MANAGER_ENTRY,
  ancestorsOf,
  childrenOf,
  stateClass,
  stateLabel,
  usePluginManagerStore,
  type PluginInfo,
} from './pluginManagerStore';
import styles from './PluginManagerPage.module.css';

/** 内核唯一传输命令（B7 落地决策，与 buildApp.ts 同源） */
const KERNEL_DISPATCH = 'plugin:kernel|kernel_dispatch';

/** 与后端 plugin.rs StartupReportItem 对齐 */
interface ReportItem {
  plugin: PluginId;
  state: string;
  note: string | null;
}

function invokeKernel<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return invoke<T>(KERNEL_DISPATCH, { cmd, args: args ?? {} });
}

/** 行图标（终端风级别符号，与 Layout 应用标 ◈ 同源） */
const LEVEL_ICON: Record<string, string> = { board: '◈', feature: '▣', custom: '◧' };

/** F6 插件管理页 v1.2（手稿 20260926 ③层级化改造 + 三大分类）
 *  分组目录置于全局侧边栏（PluginManagerNav，经 Layout isPluginManagerPage 特例渲染），
 *  本页主内容区通栏：面包屑（n 级钻取）+ 所选节点控制行（启停/必备守卫）+ 一级子插件列表
 *  （随父停用联动显示；有下级的子插件可钻入）；
 *  三大分类（手稿 20260926）：板块/功能/扩展 = level（board/feature/custom）；
 *  交互：启停（kernel:plugin:set_enabled）+ 停用二次确认 + state-changed 自刷新；
 *  展示：行内只留「图标 + 中文名 + 状态徽标 + 开关」（2026-09-29 简化——插件 ID / 类型徽标 /
 *  版本号等开发信息不上页面，启动报告面板入口移除；失败原因仍按行内 errorNote 展示） */
export default function PluginManagerPage() {
  const plugins = usePluginManagerStore((s) => s.plugins);
  const setPlugins = usePluginManagerStore((s) => s.setPlugins);
  const selectedId = usePluginManagerStore((s) => s.selectedId);
  const setSelectedId = usePluginManagerStore((s) => s.setSelectedId);
  const [report, setReport] = useState<ReportItem[]>([]);
  const [pending, setPending] = useState<PluginInfo | null>(null);
  const [busyId, setBusyId] = useState<PluginId | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    try {
      const [list, rpt] = await Promise.all([
        invokeKernel<PluginInfo[]>('kernel:plugin:list'),
        // 启动报告面板已移除（2026-09-29 简化）；数据仅作错误行的失败原因 errorNote
        invokeKernel<ReportItem[]>('kernel:startup_report'),
      ]);
      setPlugins(list);
      setReport(rpt);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [setPlugins]);

  // 挂载加载 + 订阅 state-changed 自刷新（桥接单一来源：本页负责 bridge，避免叠加监听）
  useEffect(() => {
    void reload();
    let unlisten: (() => void) | undefined;
    let disposed = false;
    bus.bridge('kernel:*').then((fn) => {
      if (disposed) fn();
      else unlisten = fn;
    });
    const off = bus.on('kernel:plugin.state-changed', () => { void reload(); });
    return () => {
      disposed = true;
      off();
      unlisten?.();
    };
  }, [reload]);

  // 启停：停用先二次确认（危险操作），启用直接执行（恢复性操作，wezterm confirm.rs 只对危险操作确认）
  const requestToggle = useCallback((p: PluginInfo) => {
    if (p.state === 'enabled') setPending(p);
    else void doSetEnabled(p, true);
  }, []);

  const doSetEnabled = useCallback(async (p: PluginInfo, enabled: boolean) => {
    setBusyId(p.id);
    try {
      await invokeKernel('kernel:plugin:set_enabled', { id: p.id, enabled });
      await reload();
    } catch (e) {
      setError(`启停 ${p.id} 失败: ${String(e)}`);
    } finally {
      setBusyId(null);
      setPending(null);
    }
  }, [reload]);

  const noteById = new Map(report.map((r) => [r.plugin, r.note]));

  if (plugins === null) {
    return <div className={styles.page}><div className={styles.empty}>加载中…</div></div>;
  }

  const byId = new Map(plugins.map((p) => [p.id, p]));
  const selectedPlugin = byId.get(selectedId) ?? null;
  // 面包屑祖先链（n 级钻取返回路径）
  const ancestors = selectedPlugin ? ancestorsOf(plugins, selectedPlugin.id) : [];
  // 一级子插件（含停用；首页在必备组之后追加内核级「插件管理」条目）
  let rows: PluginInfo[] = selectedPlugin ? childrenOf(plugins, selectedPlugin.id) : [];
  if (selectedPlugin?.id === 'boards.home') {
    const requiredCount = rows.filter((p) => p.required).length;
    rows = [...rows.slice(0, requiredCount), PLUGIN_MANAGER_ENTRY, ...rows.slice(requiredCount)];
  }
  const hasChildren = (p: PluginInfo) => plugins.some((c) => c.parent === p.id);

  /** 父插件是否已启用（无父级恒为 true；用于行内「随父停用」联动） */
  const parentEnabledOf = (p: PluginInfo) => {
    if (!p.parent) return true;
    return byId.get(p.parent)?.state === 'enabled';
  };

  /** 行：启停开关（必备守卫 + 随父停用守卫） */
  const renderToggle = (p: PluginInfo) => {
    const parentEnabled = parentEnabledOf(p);
    const lockedRequired = p.required;
    const lockedByParent = !parentEnabled && !p.required;
    const locked = lockedRequired || lockedByParent;
    const tip = lockedRequired
      ? '不可单独停用（只随其父插件同步停用）'
      : lockedByParent
        ? '父插件未启用，子插件随父停用'
        : undefined;
    return (
      <button
        role="switch"
        aria-checked={p.state === 'enabled'}
        aria-label={`${p.state === 'enabled' ? '停用' : '启用'} ${p.name}`}
        title={tip}
        className={`${styles.toggle} ${p.state === 'enabled' ? styles.toggleOn : ''}`}
        disabled={busyId === p.id || locked}
        onClick={() => { if (!locked) requestToggle(p); }}
      >
        <span className={styles.toggleThumb} />
      </button>
    );
  };

  /** 状态徽标：父插件未启用时子插件显示「随父插件停用」（手稿 20260926：必备/可选子插件只随父同步停用） */
  const renderState = (p: PluginInfo) => {
    if (!parentEnabledOf(p) && !p.required && p.state !== 'error') {
      return <span className={`${styles.stateBadge} ${styles.stateDisabled}`}>🔶 随父插件停用</span>;
    }
    return <span className={stateClass(p.state, styles.stateBadge, styles)}>{stateLabel(p.state)}</span>;
  };

  /** 行左区：图标 + 中文名（有下级可钻入；ID / 类型徽标等开发信息不上页面，2026-09-29 简化） */
  const renderRowLeft = (p: PluginInfo, drillable: boolean) => (
    <div className={styles.rowLeft}>
      <span className={styles.pluginIcon} aria-hidden="true">{LEVEL_ICON[p.level]}</span>
      {drillable ? (
        <button
          className={`${styles.pluginName} ${styles.drillName}`}
          onClick={() => setSelectedId(p.id)}
          title={`查看 ${p.name} 的子插件`}
        >
          {p.name} <span aria-hidden="true">▸</span>
        </button>
      ) : (
        <span className={styles.pluginName}>{p.name}</span>
      )}
    </div>
  );

  return (
    <div className={styles.page}>
      <div className={styles.header}>
        <div>
          <h1 className={styles.title}>插件管理</h1>
          <p className={styles.subtitle}>插件分级管理 · 启停即时生效</p>
        </div>
      </div>

      {error && <div className={styles.errorBar} role="alert">{error}</div>}

      {/* 面包屑（n 级层级钻取：祖先可点返回） */}
      {selectedPlugin && (
        <div className={styles.breadcrumb}>
          {ancestors.map((a) => (
            <Fragment key={a.id}>
              <button className={styles.breadcrumbLink} onClick={() => setSelectedId(a.id)}>
                {a.name}
              </button>
              <span className={styles.breadcrumbSep} aria-hidden="true">/</span>
            </Fragment>
          ))}
          <span className={styles.breadcrumbCurrent}>{selectedPlugin.name}</span>
        </div>
      )}

      {/* 所选节点控制行（板块/功能/扩展同级控制；子插件行随父停用联动显示） */}
      {selectedPlugin && (
        <div className={`${styles.row} ${styles.boardHeader}`}>
          {renderRowLeft(selectedPlugin, false)}
          <div className={styles.rowMid}>
            {renderState(selectedPlugin)}
            {renderToggle(selectedPlugin)}
          </div>
        </div>
      )}

      {selectedPlugin && rows.length === 0 ? (
        selectedPlugin.level === 'board' ? <div className={styles.empty}>该板块暂无子插件</div> : null
      ) : (
        <div className={styles.listBox}>
          {rows.map((p) => (
            <div key={p.id}>
              <div className={styles.row}>
                {renderRowLeft(p, hasChildren(p))}
                <div className={styles.rowMid}>
                  {renderState(p)}
                  {renderToggle(p)}
                </div>
              </div>
              {p.state === 'error' && noteById.get(p.id) && (
                <div className={styles.errorNote}>{noteById.get(p.id)}</div>
              )}
            </div>
          ))}
        </div>
      )}

      {/* 停用二次确认（复用全局 ConfirmDialog danger 语义；wezterm：Esc=取消、默认 focus 取消） */}
      <ConfirmDialog
        isOpen={pending !== null}
        onClose={() => setPending(null)}
        onConfirm={() => { if (pending) void doSetEnabled(pending, false); }}
        type="danger"
        message={
          pending
            ? plugins.some((p) => p.parent === pending.id)
              ? `停用插件 ${pending.name}？\n其导航项、路由与 IPC 命令将立即失效，子插件将随父插件一并停用，可随时重新启用。`
              : `停用插件 ${pending.name}？\n其导航项、路由与 IPC 命令将立即失效，可随时重新启用。`
            : ''
        }
      />
    </div>
  );
}