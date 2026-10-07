// customs.sync L1 壳 + 视图分发 + 启停门控（同步「一切皆插件」）
import { useState, useEffect } from 'react';
import { useSearchParams } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import styles from './SyncPage.module.css';
import DeviceManager from '../features/devices/DeviceManager';
import ConflictResolver from '../features/conflicts/ConflictResolver';

/** tab → 子插件 id（停用回退必备的 sync.devices） */
const TAB_PLUGIN_ID: Record<string, string> = {
  devices: 'sync.devices',
  conflicts: 'sync.conflicts',
};

export default function SyncPage() {
  const [searchParams] = useSearchParams();
  const tab = searchParams.get('tab') || 'devices';
  const [enabledIds, setEnabledIds] = useState<string[] | null>(null);
  useEffect(() => {
    invoke<Array<{ id: string }>>('plugin:kernel|kernel_dispatch', { cmd: 'kernel:plugin:get_enabled', args: {} })
      .then((list) => setEnabledIds(list.map((p) => p.id)))
      .catch(() => setEnabledIds(null));
  }, []);
  const displayTab = enabledIds !== null && TAB_PLUGIN_ID[tab] && !enabledIds.includes(TAB_PLUGIN_ID[tab]) ? 'devices' : tab;
  return (
    <div className={styles.page}>
      {displayTab === 'devices' && <DeviceManager visible />}
      {displayTab === 'conflicts' && <ConflictResolver visible />}
    </div>
  );
}
