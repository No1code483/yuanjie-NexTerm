import { t } from "i18next";
/**
 * IntroOverlay - 开场文案动画（Task 8.4）
 *
 * 参考 05_3D场景设计.md §15 开场文案动画：
 *   阶段1 0-1s   黑屏 + 加载提示
 *   阶段2 1-4s   文案逐行淡入（3 行核心文案，每行 0.6s 间隔）
 *   阶段3 4-5s   文案整体淡出
 *   阶段4 5s 后  调用 onComplete，3D 场景淡入
 *
 * 文案传递"知识塑造文明"核心理念，同时掩盖 3D 资源加载等待
 *
 * 跳过逻辑（05 文档 §15.5）：
 *   - sessionStorage 标记已观看，二次进入直接跳过
 *   - 由 Game3D.tsx 控制（本组件不读取 sessionStorage）
 *
 * change-id: game-3d-rebuild-refactor
 */
import { useEffect, useState } from 'react';

/** 开场文案 3 行 */
const INTRO_LINES = [t("game3d.components.UI.IntroOverlay.k1"), t("game3d.components.UI.IntroOverlay.k2"), t("game3d.components.UI.IntroOverlay.k3")];

/** 阶段时序（毫秒） */
const TIMING = {
  typingStart: 1000,
  // 1s 后开始打字
  fadeOutStart: 4000,
  // 4s 后开始淡出
  complete: 5000 // 5s 完成
} as const;
type Phase = 'loading' | 'typing' | 'fadeOut' | 'done';
interface Props {
  /** 动画完成回调，由父组件控制 Canvas 淡入与 UI 层显示 */
  onComplete: () => void;
}

/**
 * 开场动画组件
 * - loading：显示"加载中…"
 * - typing：3 行文案逐行淡入
 * - fadeOut：整体淡出
 * - done：返回 null（已卸载）
 */
export function IntroOverlay({
  onComplete
}: Props) {
  const [phase, setPhase] = useState<Phase>('loading');
  useEffect(() => {
    const timers = [setTimeout(() => setPhase('typing'), TIMING.typingStart), setTimeout(() => setPhase('fadeOut'), TIMING.fadeOutStart), setTimeout(() => {
      setPhase('done');
      onComplete();
    }, TIMING.complete)];
    return () => timers.forEach(clearTimeout);
  }, [onComplete]);
  if (phase === 'done') return null;
  return <div className={`game3d-intro game3d-intro--${phase}`}>
      {phase !== 'loading' && <div className="game3d-intro__content">
          {INTRO_LINES.map((line, i) => <p key={i} className="game3d-intro__line" style={{
        animationDelay: `${i * 0.6}s`
      }}>
              {line}
            </p>)}
        </div>}
      {phase === 'loading' && <div className="game3d-intro__loader">{t("game3d.components.UI.IntroOverlay.k4")}</div>}
    </div>;
}