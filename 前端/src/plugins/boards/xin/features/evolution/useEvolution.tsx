// xin.evolution L2 —— 人格进化可视化功能域（数据为硬编码 SVG，仅局部 evolutionTab 切换）。
import { t } from "i18next";
import { useState } from 'react';
import styles from '../../Xin.module.css';
import type { XinCore } from '../../core';

export function useEvolution(_core: XinCore, _active: boolean) {
  const [evolutionTab, setEvolutionTab] = useState<'favorability' | 'radar'>('favorability');

  const renderEvolution = () => <div className={styles.tabContent}>
      <div className={styles.card}>
        <div className={styles.panelTitle}>{t("Xin.k178")}</div>
        <div style={{
      display: 'flex',
      gap: '8px',
      marginBottom: '12px'
    }}>
          <button className={evolutionTab === 'favorability' ? styles.btn : styles.btnSmall} onClick={() => setEvolutionTab('favorability')}>
            {t("Xin.k179")}
          </button>
          <button className={evolutionTab === 'radar' ? styles.btn : styles.btnSmall} onClick={() => setEvolutionTab('radar')}>
            {t("Xin.k180")}
          </button>
        </div>

        {/* 好感度变化曲线 */}
        {evolutionTab === 'favorability' && <div className={styles.evoChartContainer}>
            <svg viewBox="0 0 400 200" className={styles.evoChartSvg}>
              {/* 网格线 */}
              {[0, 1, 2, 3, 4].map(i => <line key={`h${i}`} x1={50} y1={20 + i * 40} x2={380} y2={20 + i * 40} stroke="rgba(0,255,100,0.08)" strokeWidth="0.5" />)}
              {[0, 1, 2, 3, 4, 5, 6].map(i => <line key={`v${i}`} x1={50 + i * 55} y1={20} x2={50 + i * 55} y2={180} stroke="rgba(0,255,100,0.08)" strokeWidth="0.5" />)}
              {/* Y轴标签 */}
              {[100, 75, 50, 25, 0].map((v, i) => <text key={`yl${i}`} x={40} y={24 + i * 40} fill="rgba(0,255,100,0.4)" fontSize="8" textAnchor="end">{v}</text>)}
              {/* X轴标签 */}
              {[t("Xin.k181"), t("Xin.k182"), t("Xin.k183"), t("Xin.k184"), t("Xin.k185"), t("Xin.k186"), t("Xin.k187")].map((d, i) => <text key={`xl${i}`} x={50 + i * 55} y={195} fill="rgba(0,255,100,0.4)" fontSize="7" textAnchor="middle">{d}</text>)}
              {/* 曲线 */}
              <polyline points="50,140 105,120 160,100 215,80 270,60 325,50 380,40" fill="none" stroke="#00FF64" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
              {/* 发光效果 */}
              <polyline points="50,140 105,120 160,100 215,80 270,60 325,50 380,40" fill="none" stroke="rgba(0,255,100,0.3)" strokeWidth="6" strokeLinecap="round" strokeLinejoin="round" />
              {/* 数据点 */}
              {[[50, 140], [105, 120], [160, 100], [215, 80], [270, 60], [325, 50], [380, 40]].map(([cx, cy], i) => <circle key={`dp${i}`} cx={cx} cy={cy} r="3" fill="#00FF64" />)}
            </svg>
            <div className={styles.evoChartLabel}>{t("Xin.k188")}</div>
          </div>}

        {/* 人格特质雷达图 */}
        {evolutionTab === 'radar' && <div className={styles.evoChartContainer}>
            <svg viewBox="0 0 300 280" className={styles.evoChartSvg}>
              {/* 同心五边形 */}
              {[1, 2, 3, 4].map(level => {
          const r = 25 + level * 25;
          const traits = [t("Xin.k189"), t("Xin.k190"), t("Xin.k191"), t("Xin.k192"), t("Xin.k193")];
          const points = traits.map((_, i) => {
            const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
            return `${150 + r * Math.cos(angle)},${140 + r * Math.sin(angle)}`;
          }).join(' ');
          return <polygon key={`grid${level}`} points={points} fill="none" stroke="rgba(0,255,100,0.08)" strokeWidth="0.5" />;
        })}
              {/* 轴线 */}
              {[0, 0.2, 0.4, 0.6, 0.8, 1.0].map((_, i) => {
          const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
          return <line key={`axis${i}`} x1={150} y1={140} x2={150 + 125 * Math.cos(angle)} y2={140 + 125 * Math.sin(angle)} stroke="rgba(0,255,100,0.1)" strokeWidth="0.5" />;
        })}
              {/* 数据多边形 */}
              <polygon points={(() => {
          const traits = [t("Xin.k189"), t("Xin.k190"), t("Xin.k191"), t("Xin.k192"), t("Xin.k193")];
          const values = [0.85, 0.7, 0.6, 0.75, 0.9];
          return traits.map((_, i) => {
            const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
            const r = 25 + values[i] * 100;
            return `${150 + r * Math.cos(angle)},${140 + r * Math.sin(angle)}`;
          }).join(' ');
        })()} fill="rgba(0,255,100,0.1)" stroke="#00FF64" strokeWidth="1.5" />
              {/* 发光多边形 */}
              <polygon points={(() => {
          const traits = [t("Xin.k189"), t("Xin.k190"), t("Xin.k191"), t("Xin.k192"), t("Xin.k193")];
          const values = [0.85, 0.7, 0.6, 0.75, 0.9];
          return traits.map((_, i) => {
            const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
            const r = 25 + values[i] * 100;
            return `${150 + r * Math.cos(angle)},${140 + r * Math.sin(angle)}`;
          }).join(' ');
        })()} fill="none" stroke="rgba(0,255,100,0.3)" strokeWidth="5" />
              {/* 数据点 */}
              {[0.85, 0.7, 0.6, 0.75, 0.9].map((v, i) => {
          const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
          const r = 25 + v * 100;
          return <circle key={`rp${i}`} cx={150 + r * Math.cos(angle)} cy={140 + r * Math.sin(angle)} r="3" fill="#00FF64" />;
        })}
              {/* 标签 */}
              {[t("Xin.k189"), t("Xin.k190"), t("Xin.k191"), t("Xin.k192"), t("Xin.k193")].map((trait, i) => {
          const angle = Math.PI * 2 * i / 5 - Math.PI / 2;
          const lr = 140;
          const x = 150 + lr * Math.cos(angle);
          const y = 140 + lr * Math.sin(angle);
          return <text key={`tl${i}`} x={x} y={y} fill="rgba(0,255,100,0.6)" fontSize="9" textAnchor="middle" dominantBaseline="middle">
                    {trait}
                  </text>;
        })}
            </svg>
            <div className={styles.evoChartLabel}>{t("Xin.k180")}</div>
          </div>}
      </div>
    </div>;

  return { renderEvolution };
}
