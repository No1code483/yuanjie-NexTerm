import {
  BOARD_ORDER,
  dotClass,
  isBoardPlugin,
  rootOf,
  usePluginManagerStore,
  type PluginInfo,
} from './pluginManagerStore';
import pmStyles from './PluginManagerPage.module.css';

/** F6 插件管理 · 全局侧边栏导航（手稿 20260926 ③：分组目录置于全局侧边栏，主内容区通栏）
 *  三大分类（手稿 20260926）：顶层分「板块插件」「扩展插件」两组，子插件（功能/扩展）
 *  由主内容区按父链钻取展示；只读共享 store（数据单一来源 = PluginManagerPage 的 kernel:plugin:list）；
 *  点击仅切换选中态，不导航。 */
export default function PluginManagerNav() {
  const plugins = usePluginManagerStore((s) => s.plugins);
  const selectedId = usePluginManagerStore((s) => s.selectedId);
  const setSelectedId = usePluginManagerStore((s) => s.setSelectedId);

  if (plugins === null) {
    return <div className={pmStyles.navLoading}>加载中…</div>;
  }

  const orderIndex = (id: string) => {
    const idx = BOARD_ORDER.indexOf(id);
    return idx === -1 ? Number.MAX_SAFE_INTEGER : idx;
  };
  const boards = plugins
    .filter(isBoardPlugin)
    .sort((a, b) => orderIndex(a.id) - orderIndex(b.id) || a.id.localeCompare(b.id));
  // 扩展插件：跨板块顶层定制级插件（无父级）；其余子插件在主内容区按父链展示
  const extensions = plugins
    .filter((p) => p.level === 'custom' && !p.parent)
    .sort((a, b) => a.id.localeCompare(b.id));
  // 选中项可能为钻取中的子插件 → 侧边栏保持其顶层项高亮
  const activeRoot = rootOf(plugins, selectedId);

  const renderItem = (p: PluginInfo) => (
    <button
      key={p.id}
      className={`${pmStyles.sidebarItem} ${activeRoot === p.id ? pmStyles.sidebarItemActive : ''}`}
      onClick={() => setSelectedId(p.id)}
      title={p.name}
    >
      <span className={`${pmStyles.stateDot} ${pmStyles[dotClass(p.state)]}`} aria-hidden="true" />
      <span className={pmStyles.sidebarItemName}>{p.name}</span>
    </button>
  );

  return (
    <div className={pmStyles.navRoot}>
      <div className={pmStyles.sidebarGroupTitle}>板块插件</div>
      {boards.map(renderItem)}
      {extensions.length > 0 && (
        <>
          <div className={pmStyles.sidebarGroupTitle}>扩展插件</div>
          {extensions.map(renderItem)}
        </>
      )}
    </div>
  );
}