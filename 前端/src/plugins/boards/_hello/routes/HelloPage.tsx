import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { bus } from '@/kernel/event-bus';
import type { KernelEvent } from '@/kernel/types';

/** 与 buildApp.ts 同名常量（F7 kernel IPC 客户端落地后统一收口） */
const KERNEL_DISPATCH = 'plugin:kernel|kernel_dispatch';

/** Hello 板块页：显示版本 + 调用 hello_ping 验证内核 IPC 全链路 */
export default function HelloPage() {
  const [pong, setPong] = useState('调用中…');
  const [event, setEvent] = useState<KernelEvent | null>(null);
  const [publishState, setPublishState] = useState('待发送');

  useEffect(() => {
    invoke<string>(KERNEL_DISPATCH, { cmd: '_hello:plugin:hello_ping', args: {} })
      .then((r) => setPong(String(r)))
      .catch((e) => setPong(`调用失败: ${String(e)}`));

    const off = bus.on('_hello:*', setEvent);
    let unlisten: (() => void) | undefined;
    bus.bridge('_hello:*').then((fn) => {
      unlisten = fn;
    });
    return () => {
      off();
      unlisten?.();
    };
  }, []);

  const publishEvent = async () => {
    setPublishState('发送中…');
    try {
      await invoke(KERNEL_DISPATCH, { cmd: '_hello:plugin:hello_publish', args: {} });
      setPublishState('已发送');
    } catch (error) {
      setPublishState(`发送失败: ${String(error)}`);
    }
  };

  return (
    <div
      style={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 16,
        color: '#e0e0e0',
      }}
    >
      <h1 style={{ fontSize: 28, margin: 0 }}>
        Hello 插件化 <span style={{ color: '#00ff9d' }}>v0.1.0</span>
      </h1>
      <p style={{ fontFamily: 'Consolas, monospace' }} data-hello-pong>
        hello_ping → {pong}
      </p>
      <button type="button" onClick={publishEvent} data-hello-publish>
        发事件
      </button>
      <p style={{ fontFamily: 'Consolas, monospace' }} data-hello-event>
        {publishState}；收到事件 → {event?.name ?? '等待中'}
      </p>
    </div>
  );
}
