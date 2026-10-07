import { Suspense, Component, createElement, type ReactNode } from 'react';
import { useRegistry } from '../registry/context'; // RegistryProvider 提供

/** 插槽渲染器：L1 板块页面中放置。
 *  隔离保证：每个组件独立 ErrorBoundary——单个 L2 崩溃仅显示该卡片错误态。 */
export function SlotRenderer({ slot, layout }: {
  slot: string;                     // 插槽 id，如 'home.widgets'
  layout?: 'grid' | 'stack';
}) {
  const registry = useRegistry();
  const items = registry.getSlotComponents(slot);
  if (items.length === 0) return null;
  return (
    <div className={`slot-${layout ?? 'stack'}`} data-slot={slot}>
      {items.map((it) => (
        <PluginBoundary key={`${slot}:${it.component.toString().slice(0, 40)}`}>
          <Suspense fallback={<SlotLoading slot={slot} />}>
            {createElement(it.component)}
          </Suspense>
        </PluginBoundary>
      ))}
    </div>
  );
}

class PluginBoundary extends Component<{ children: ReactNode }, { err: boolean }> {
  state = { err: false };
  static getDerivedStateFromError() { return { err: true }; }
  render() {
    if (this.state.err) return <SlotError />;   // 卡片级错误占位（不传染兄弟）
    return this.props.children;
  }
}

function SlotLoading({ slot }: { slot: string }) {
  return <div className="slot-loading" data-slot-loading={slot}>加载中…</div>;
}

function SlotError() {
  return <div className="slot-error" data-slot-error>组件加载失败</div>;
}
