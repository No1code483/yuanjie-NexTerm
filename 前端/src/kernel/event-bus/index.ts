import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { KernelEvent } from '../types';

type Handler = (e: KernelEvent) => void;

/** 前端事件总线（mitt 语义自建，零依赖）
 *  与后端总线同构三域；桥接白名单在 manifest permissions.events.subscribe */
class EventBus {
  private map = new Map<string, Set<Handler>>(); // key: 精确名或 'home:*'

  on(pattern: string, h: Handler): () => void {
    // 支持 'home:*' 通配；返回退订函数
    let set = this.map.get(pattern);
    if (!set) {
      set = new Set();
      this.map.set(pattern, set);
    }
    set.add(h);
    return () => {
      const s = this.map.get(pattern);
      if (!s) return;
      s.delete(h);
      if (s.size === 0) this.map.delete(pattern);
    };
  }

  /** 本域分发：按各订阅 pattern 匹配事件名 */
  emit(e: KernelEvent): void {
    for (const [pattern, handlers] of this.map) {
      if (!matchPattern(pattern, e.name)) continue;
      for (const h of [...handlers]) h(e);
    }
  }

  /** 桥接声明：该事件来自 Rust 总线（Tauri event），自动转发本端订阅者 */
  async bridge(pattern: string): Promise<UnlistenFn> {
    return listen<KernelEvent>('k://event', (ev) => {
      if (matchPattern(pattern, ev.payload.name)) this.emit(ev.payload);
    });
  }

  /** 仅测试使用：清空全部订阅（单例隔离） */
  clearAllForTest(): void {
    this.map.clear();
  }
}

/** 通配匹配：精确名 / '前缀:*' / '*'（全量） */
export function matchPattern(pattern: string, name: string): boolean {
  if (pattern === '*') return true;
  if (pattern.endsWith(':*')) return name.startsWith(pattern.slice(0, -1));
  return pattern === name;
}

export const bus = new EventBus();
