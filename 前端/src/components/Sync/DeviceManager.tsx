// DeviceManager.tsx — A5.2.6.7 设备管理 UI
//
// 设计文档：功能展望/平台级增强/04_离线与同步机制.md §2.6.2 / §2.6.6 / §2.6.7
//
// 功能：
// - 列出所有已注册的同步设备
// - 显示设备名、类型、注册时间、最后同步时间、公钥指纹
// - 撤销非当前设备（清除其同步权限）
// - 注册新设备（生成 ECDH 密钥对并上传公钥）

import { useEffect, useState } from 'react';
import { useSyncStore } from '@/stores/syncStore';
import { sync as syncIpc } from '@/lib/ipc';
import styles from './DeviceManager.module.css';

const MAX_DEVICES = 5;

/** 检测当前设备类型 */
function detectDeviceType(): 'windows' | 'macos' | 'linux' {
  if (typeof navigator === 'undefined') return 'windows';
  const platform = navigator.platform.toLowerCase();
  if (platform.includes('mac')) return 'macos';
  if (platform.includes('linux')) return 'linux';
  return 'windows';
}

/** 生成简易设备 ID（UUID v4 风格，crypto 不可用时降级） */
function generateDeviceId(): string {
  try {
    if (typeof crypto !== 'undefined' && crypto.randomUUID) {
      return crypto.randomUUID();
    }
  } catch {
    /* ignore */
  }
  return 'dev-' + Date.now().toString(36) + '-' + Math.random().toString(36).slice(2, 10);
}

/** 公钥指纹（前 16 字符）用于显示 */
function publicKeyFingerprint(publicKey: string): string {
  if (!publicKey) return '—';
  return publicKey.length > 24 ? `${publicKey.slice(0, 12)}...${publicKey.slice(-8)}` : publicKey;
}

interface DeviceManagerProps {
  /** 是否可见（默认 true，用于在父容器中条件渲染） */
  visible?: boolean;
}

export default function DeviceManager({ visible = true }: DeviceManagerProps) {
  const { devices, refreshDevices, revokeDevice, registerCurrentDevice, lastError } =
    useSyncStore();

  const [registering, setRegistering] = useState(false);
  const [newDeviceName, setNewDeviceName] = useState('');

  useEffect(() => {
    if (visible) {
      void refreshDevices();
    }
  }, [visible, refreshDevices]);

  const handleRegister = async () => {
    if (!newDeviceName.trim()) return;
    setRegistering(true);
    try {
      // 1. 生成 ECDH 密钥对（后端返回公钥，私钥在内存中保持）
      const keypairRes = await syncIpc.ecdhGenerateKeypair();
      if (keypairRes.code !== 0 || !keypairRes.data) {
        return;
      }
      // 2. 注册设备（标记为当前设备）
      const deviceId = generateDeviceId();
      // 当前 UI 只采集操作系统（BUG-018 修复：OS 归 device_os）；设备形态暂缺省 'desktop'
      const deviceOs = detectDeviceType();
      const ok = await registerCurrentDevice(
        deviceId,
        newDeviceName.trim(),
        'desktop',
        deviceOs,
        keypairRes.data.public_key_b64,
      );
      if (ok) {
        setNewDeviceName('');
      }
    } finally {
      setRegistering(false);
    }
  };

  const handleRevoke = async (deviceId: string, deviceName: string) => {
    const confirmed = window.confirm(
      `确定要撤销设备「${deviceName}」的同步权限吗？\n撤销后该设备将无法再同步数据。`,
    );
    if (!confirmed) return;
    await revokeDevice(deviceId);
  };

  if (!visible) return null;

  const canRegisterMore = devices.length < MAX_DEVICES;

  return (
    <div className={styles.panel}>
      {/* 标题栏 */}
      <div className={styles.header}>
        <h3 className={styles.title}>
          <span>{'<Device Sync />'}</span>
          <span className={styles.badge}>
            {devices.length} / {MAX_DEVICES}
          </span>
        </h3>
        <div className={styles.toolbar}>
          <button
            className={styles.refreshBtn}
            onClick={() => void refreshDevices()}
          >
            ↻ 刷新
          </button>
        </div>
      </div>

      {lastError && <div className={styles.errorMsg}>{lastError}</div>}

      {/* 设备列表 */}
      {devices.length === 0 ? (
        <div className={styles.empty}>
          尚未注册任何同步设备。在下方注册当前设备以启用同步。
        </div>
      ) : (
        <div className={styles.deviceList}>
          {devices.map((device) => {
            const isCurrent = device.is_current_device === 1;
            // BUG-018 修复：OS 归属 device_os 用于样式色，device_type 为设备形态
            const deviceOs = (device.device_os || device.device_type || 'windows').toLowerCase();
            return (
              <div
                key={device.id}
                className={`${styles.deviceItem} ${isCurrent ? styles.current : ''}`}
              >
                <div className={styles.deviceHeader}>
                  <span className={styles.deviceName}>
                    {device.device_name}
                    {isCurrent && <span className={styles.currentTag}>当前</span>}
                  </span>
                  <span className={`${styles.deviceType} ${styles[deviceOs] || ''}`}>
                    {deviceOs}
                  </span>
                </div>
                <div className={styles.deviceMeta}>
                  <span className={styles.metaLabel}>设备 ID</span>
                  <span className={styles.metaValue}>{device.id}</span>
                  <span className={styles.metaLabel}>公钥指纹</span>
                  <span className={styles.metaValue}>
                    {publicKeyFingerprint(device.public_key)}
                  </span>
                  <span className={styles.metaLabel}>注册时间</span>
                  <span className={styles.metaValue}>
                    {new Date(device.registered_at).toLocaleString()}
                  </span>
                  <span className={styles.metaLabel}>最后同步</span>
                  <span className={styles.metaValue}>
                    {device.last_sync_at
                      ? new Date(device.last_sync_at).toLocaleString()
                      : '从未同步'}
                  </span>
                </div>
                <div className={styles.deviceActions}>
                  <button
                    className={styles.revokeBtn}
                    onClick={() => void handleRevoke(device.id, device.device_name)}
                    disabled={isCurrent}
                    title={isCurrent ? '不能撤销当前设备' : '撤销此设备的同步权限'}
                  >
                    撤销
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* 注册新设备 */}
      {canRegisterMore && (
        <div className={styles.registerSection}>
          <div className={styles.registerTitle}>注册当前设备</div>
          <div className={styles.registerForm}>
            <input
              className={styles.input}
              type="text"
              placeholder="设备名称（如：工作笔记本）"
              value={newDeviceName}
              onChange={(e) => setNewDeviceName(e.target.value)}
              maxLength={32}
              disabled={registering}
            />
            <input
              className={styles.input}
              type="text"
              value={detectDeviceType()}
              readOnly
              title="自动检测的设备类型"
            />
            <button
              className={styles.registerBtn}
              onClick={() => void handleRegister()}
              disabled={registering || !newDeviceName.trim()}
            >
              {registering ? '注册中...' : '生成密钥并注册'}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
