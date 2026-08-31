import { t } from "i18next";
/**
 * ReplayPanel - 时间轴回放主面板（T1.5）
 *
 * 11_时间轴回放.md §4：底部固定面板，包含
 *   - 时间轴滑块（左→右：minTime→maxTime）
 *   - 控制按钮：单步后退 / 播放-暂停 / 单步前进 / 速度切换（1x/2x/4x/8x）
 *   - 信息显示：当前时间 / 总事件数 / 当前场景建筑数
 *   - 退出回放按钮
 *
 * 拖动策略：
 *   - 拖动中：仅更新 store.replayCurrentTime（不触发 IPC）
 *   - 拖动松开（onMouseUp/onTouchEnd）：debounce 300ms 后调用 ipc.getWorldSnapshot
 *   - 相邻 100ms 内的时间戳由后端 LRU 缓存命中
 *
 * 播放策略：
 *   - 按 replaySpeed 倍率推进时间（1x = 实时，2x = 2 倍速...）
 *   - 到达 maxTime 时自动暂停
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { game } from '@/lib/ipc';
import { useGame3DStore } from '../../stores/gameStore';
import type { ReplaySpeed } from '../../stores/gameStore';

/** 播放速度选项。 */
const SPEED_OPTIONS: ReplaySpeed[] = [1, 2, 4, 8];

/** 播放时推进间隔（ms）。 */
const PLAY_TICK_MS = 100;

/** 拖动松开后触发场景重建的 debounce 时长（ms）。 */
const DEBOUNCE_MS = 300;

/** 格式化毫秒时间戳为 'MM-DD HH:mm'。 */
function formatTime(ts: number): string {
  const d = new Date(ts);
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const mi = String(d.getMinutes()).padStart(2, '0');
  return `${mm}-${dd} ${hh}:${mi}`;
}
export function ReplayPanel() {
  const world = useGame3DStore(s => s.world);
  const timeline = useGame3DStore(s => s.replayTimeline);
  const currentTime = useGame3DStore(s => s.replayCurrentTime);
  const minTime = useGame3DStore(s => s.replayMinTime);
  const maxTime = useGame3DStore(s => s.replayMaxTime);
  const playing = useGame3DStore(s => s.replayPlaying);
  const speed = useGame3DStore(s => s.replaySpeed);
  const currentScene = useGame3DStore(s => s.replayCurrentScene);
  const replayLoading = useGame3DStore(s => s.replayLoading);
  const setReplayCurrentTime = useGame3DStore(s => s.setReplayCurrentTime);
  const setReplayPlaying = useGame3DStore(s => s.setReplayPlaying);
  const setReplaySpeed = useGame3DStore(s => s.setReplaySpeed);
  const setReplayCurrentScene = useGame3DStore(s => s.setReplayCurrentScene);
  const setReplayLoading = useGame3DStore(s => s.setReplayLoading);
  const exitReplay = useGame3DStore(s => s.exitReplay);
  const stepForward = useGame3DStore(s => s.stepForward);
  const stepBackward = useGame3DStore(s => s.stepBackward);

  /** 拖动状态：拖动期间不触发场景重建。 */
  const [dragging, setDragging] = useState(false);
  /** debounce 计时器。 */
  const debounceRef = useRef<number | null>(null);
  /** 上次请求的时间戳，避免重复请求相同时间。 */
  const lastFetchedTsRef = useRef<number>(-1);

  /** 拉取场景重建（去重 + 防抖）。 */
  const fetchScene = useCallback(async (ts: number) => {
    if (!world) return;
    // 相同时间戳不重复请求
    if (lastFetchedTsRef.current === ts) return;
    lastFetchedTsRef.current = ts;
    setReplayLoading(true);
    try {
      const res = await game.getWorldSnapshot(world.id, ts);
      if (res.code !== 0) {
        console.error('[ReplayPanel] getWorldSnapshot failed:', res.message);
        return;
      }
      if (res.data) {
        setReplayCurrentScene(res.data);
      }
    } catch (err) {
      console.error('[ReplayPanel] getWorldSnapshot error:', err);
    } finally {
      setReplayLoading(false);
    }
  }, [world, setReplayCurrentScene, setReplayLoading]);

  /** debounce 触发：拖动松开后 300ms 调用。 */
  const scheduleFetch = useCallback((ts: number) => {
    if (debounceRef.current !== null) {
      window.clearTimeout(debounceRef.current);
    }
    debounceRef.current = window.setTimeout(() => {
      fetchScene(ts);
      debounceRef.current = null;
    }, DEBOUNCE_MS);
  }, [fetchScene]);

  /** 滑块值变化（拖动中）。 */
  const handleSliderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const ts = Number(e.target.value);
    setReplayCurrentTime(ts);
    if (playing) setReplayPlaying(false); // 拖动时暂停播放
  };

  /** 滑块拖动开始。 */
  const handleSliderMouseDown = () => {
    setDragging(true);
  };

  /** 滑块拖动结束。 */
  const handleSliderMouseUp = () => {
    if (dragging) {
      setDragging(false);
      scheduleFetch(currentTime);
    }
  };

  /** 单步前进/后退后立即拉取场景。 */
  const handleStepForward = () => {
    stepForward();
    // stepForward 修改的是 store，下一帧才能拿到最新值，用 setTimeout 0 推迟
    setTimeout(() => {
      const ts = useGame3DStore.getState().replayCurrentTime;
      fetchScene(ts);
    }, 0);
  };
  const handleStepBackward = () => {
    stepBackward();
    setTimeout(() => {
      const ts = useGame3DStore.getState().replayCurrentTime;
      fetchScene(ts);
    }, 0);
  };

  /** 进入面板时自动加载初始场景（定位到 maxTime 时拉取一次）。 */
  useEffect(() => {
    if (currentScene === null && currentTime > 0) {
      fetchScene(currentTime);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  /** 播放推进：按 speed 倍率推进时间。 */
  useEffect(() => {
    if (!playing) return;
    const timer = window.setInterval(() => {
      const state = useGame3DStore.getState();
      const next = state.replayCurrentTime + PLAY_TICK_MS * state.replaySpeed;
      if (next >= state.replayMaxTime) {
        // 到达终点：定位到 maxTime 并暂停
        state.setReplayCurrentTime(state.replayMaxTime);
        state.setReplayPlaying(false);
        fetchScene(state.replayMaxTime);
        return;
      }
      state.setReplayCurrentTime(next);
    }, PLAY_TICK_MS);
    return () => window.clearInterval(timer);
  }, [playing, speed, fetchScene]);

  /** 退出回放：清空后端缓存 + 前端状态。 */
  const handleExit = async () => {
    if (world) {
      try {
        await game.exitReplay(world.id);
      } catch (err) {
        console.error('[ReplayPanel] exitReplay error:', err);
      }
    }
    exitReplay();
  };

  /** 切换播放速度。 */
  const handleSpeedToggle = () => {
    const idx = SPEED_OPTIONS.indexOf(speed);
    const next = SPEED_OPTIONS[(idx + 1) % SPEED_OPTIONS.length];
    setReplaySpeed(next);
  };

  /** 滑块范围。 */
  const sliderMin = minTime;
  const sliderMax = Math.max(maxTime, minTime + 1); // 防止 min == max
  const sliderValue = Math.min(Math.max(currentTime, sliderMin), sliderMax);

  /** 当前时间附近的事件列表（取前 3 + 后 3 共 6 条）。 */
  const nearbyEvents = (() => {
    const idx = timeline.findIndex(e => e.timestamp >= currentTime);
    if (idx < 0) return timeline.slice(-3);
    return timeline.slice(Math.max(0, idx - 3), idx + 3);
  })();
  return <div className="ts-panel">
      {/* === 顶部：标题 + 退出按钮 === */}
      <div className="ts-panel__header">
        <div className="ts-panel__title">
          <span className="ts-panel__icon">⏱</span>
          <span>{t("game3d.components.UI.ReplayPanel.k1")}</span>
          {replayLoading && <span className="ts-panel__loading">{t("game3d.components.UI.ReplayPanel.k2")}</span>}
        </div>
        <button className="game3d-btn game3d-btn--back" onClick={handleExit} aria-label={t("game3d.components.UI.ReplayPanel.k3")}>
          {t("game3d.components.UI.ReplayPanel.k4")}
        </button>
      </div>

      {/* === 时间轴滑块 === */}
      <div className="ts-slider-wrapper">
        <span className="ts-slider__label">{formatTime(sliderMin)}</span>
        <input type="range" className="ts-slider" min={sliderMin} max={sliderMax} value={sliderValue} onChange={handleSliderChange} onMouseDown={handleSliderMouseDown} onMouseUp={handleSliderMouseUp} onTouchStart={handleSliderMouseDown} onTouchEnd={handleSliderMouseUp} />
        <span className="ts-slider__label">{formatTime(sliderMax)}</span>
      </div>

      {/* === 控制按钮组 === */}
      <div className="ts-controls">
        <button className="game3d-btn" onClick={handleStepBackward} disabled={currentTime <= minTime} aria-label={t("game3d.components.UI.ReplayPanel.k5")}>
          ⏮
        </button>
        <button className={`game3d-btn ${playing ? 'is-active' : ''}`} onClick={() => setReplayPlaying(!playing)} aria-label={playing ? t("common.pause") : t("game3d.components.UI.ReplayPanel.k6")}>
          {playing ? t("components.FocusMode.k23") : t("game3d.components.UI.ReplayPanel.k7")}
        </button>
        <button className="game3d-btn" onClick={handleStepForward} disabled={currentTime >= maxTime} aria-label={t("game3d.components.UI.ReplayPanel.k8")}>
          ⏭
        </button>
        <button className={`game3d-btn ts-controls__speed ${speed > 1 ? 'is-active' : ''}`} onClick={handleSpeedToggle} aria-label={t("game3d.components.UI.ReplayPanel.k9")}>
          {speed}x
        </button>
      </div>

      {/* === 信息面板 === */}
      <div className="ts-info">
        <div className="ts-info__row">
          <span className="ts-info__label">{t("game3d.components.UI.ReplayPanel.k10")}</span>
          <span className="ts-info__value">{formatTime(currentTime)}</span>
        </div>
        <div className="ts-info__row">
          <span className="ts-info__label">{t("game3d.components.UI.ReplayPanel.k11")}</span>
          <span className="ts-info__value">{timeline.length}</span>
        </div>
        <div className="ts-info__row">
          <span className="ts-info__label">{t("game3d.components.UI.ReplayPanel.k12")}</span>
          <span className="ts-info__value">
            {currentScene ? currentScene.stats.building_count : '—'}
            {currentScene && t("game3d.components.UI.ReplayPanel.k13", {
            total_levels: currentScene.stats.total_levels
          })}
          </span>
        </div>
        <div className="ts-info__row">
          <span className="ts-info__label">{t("game3d.components.UI.ReplayPanel.k14")}</span>
          <span className="ts-info__value">
            {currentScene ? t("game3d.components.UI.ReplayPanel.k15", {
            replayed_event_count: currentScene.replayed_event_count
          }) : '—'}
          </span>
        </div>
      </div>

      {/* === 当前时间附近的事件列表 === */}
      <div className="ts-events">
        <div className="ts-events__title">{t("game3d.components.UI.ReplayPanel.k16")}</div>
        <ul className="ts-events__list">
          {nearbyEvents.map(e => <li key={e.id} className={`ts-event ${Math.abs(e.timestamp - currentTime) < PLAY_TICK_MS ? 'is-current' : ''}`}>
              <span className="ts-event__time">{formatTime(e.timestamp)}</span>
              <span className="ts-event__type">{e.event_type}</span>
              <span className="ts-event__name">
                {e.building_name ?? '—'}
                {e.level !== null && ` Lv.${e.level}`}
              </span>
            </li>)}
        </ul>
      </div>
    </div>;
}
export default ReplayPanel;