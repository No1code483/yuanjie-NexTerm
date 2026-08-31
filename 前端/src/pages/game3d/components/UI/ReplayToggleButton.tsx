import { t } from "i18next";
/**
 * ReplayToggleButton - 进入时间轴回放模式入口按钮（T1.5）
 *
 * 11_时间轴回放.md §4.1：在 Game3D 顶部 UI 层增加「时间轴」按钮，
 *   点击后加载时间轴数据并进入回放模式。
 *
 * 流程：
 *   1. 调用 ipc.getBuildTimeline(worldId) 拉取最近 7 天事件
 *   2. 调用 store.enterReplay() 切换到回放模式
 *   3. 调用 store.setReplayTimeline(events) 写入时间轴数据
 *   4. 定位到最新时间（maxTime），由 ReplayPanel 接管后续交互
 *
 * 失败处理：错误时弹出 alert 并不进入回放模式
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useState } from 'react';
import { game } from '@/lib/ipc';
import { useGame3DStore } from '../../stores/gameStore';
export function ReplayToggleButton() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const enterReplay = useGame3DStore(s => s.enterReplay);
  const setReplayTimeline = useGame3DStore(s => s.setReplayTimeline);
  const world = useGame3DStore(s => s.world);
  const handleClick = async () => {
    if (!world) {
      setError(t("game3d.components.UI.ReplayToggleButton.k1"));
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const res = await game.getBuildTimeline(world.id);
      if (res.code !== 0) throw new Error(res.message);
      const events = res.data ?? [];
      if (events.length === 0) {
        setError(t("game3d.components.UI.ReplayToggleButton.k2"));
        setLoading(false);
        return;
      }
      enterReplay();
      setReplayTimeline(events);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };
  return <button className="game3d-btn" onClick={handleClick} disabled={loading} aria-label={t("game3d.components.UI.ReplayToggleButton.k3")} title={error ? t("game3d.components.UI.ReplayToggleButton.k4", {
    error: error
  }) : t("game3d.components.UI.ReplayPanel.k1")}>
      {loading ? t("game3d.components.UI.ReplayToggleButton.k5") : t("game3d.components.UI.ReplayToggleButton.k6")}
    </button>;
}
export default ReplayToggleButton;