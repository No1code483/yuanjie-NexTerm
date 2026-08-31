import { t } from "i18next";
import { useMemo } from 'react';
import styles from './IntelligenceCharts.module.css';
const ACTIVITY_COLORS = ['#00F0FF', '#7FD962', '#FFB454', '#E26D6D', '#D290E4', '#39BAE6', '#FF8C00', '#00E5D4'];
const HEATMAP_COLORS = ['rgba(0,240,255,0.03)', 'rgba(0,240,255,0.08)', 'rgba(0,240,255,0.18)', 'rgba(0,240,255,0.35)', 'rgba(0,240,255,0.6)', 'rgba(0,240,255,0.85)'];
interface ActivityTrendChartProps {
  data: [string, number][];
  width?: number;
  height?: number;
}
function getHeatLevel(count: number, max: number): number {
  if (max === 0 || count === 0) return 0;
  const ratio = count / max;
  if (ratio <= 0.1) return 1;
  if (ratio <= 0.3) return 2;
  if (ratio <= 0.5) return 3;
  if (ratio <= 0.7) return 4;
  return 5;
}
export function ActivityTrendLineChart({
  data,
  width = 720,
  height = 180
}: ActivityTrendChartProps) {
  const chart = useMemo(() => {
    if (!data || data.length === 0) return null;
    const pad = {
      top: 20,
      right: 20,
      bottom: 30,
      left: 45
    };
    const w = width - pad.left - pad.right;
    const h = height - pad.top - pad.bottom;
    const values = data.map(d => d[1]);
    const maxVal = Math.max(...values, 1);
    const minVal = 0;
    const yScale = (v: number) => pad.top + h - (v - minVal) / (maxVal - minVal) * h;
    const xScale = (i: number) => pad.left + i / Math.max(data.length - 1, 1) * w;
    const points = data.map((d, i) => `${xScale(i)},${yScale(d[1])}`).join(' ');
    const gridLines = [];
    const gridCount = 4;
    for (let i = 0; i <= gridCount; i++) {
      const val = minVal + (maxVal - minVal) * (i / gridCount);
      const y = yScale(val);
      gridLines.push(<g key={`grid-${i}`}>
          <line x1={pad.left} y1={y} x2={pad.left + w} y2={y} className={styles.chartGridLine} />
          <text x={pad.left - 8} y={y + 4} textAnchor="end" className={styles.chartAxisLabel}>
            {val}
          </text>
        </g>);
    }
    const xLabels = data.filter((_, i) => i % Math.max(Math.floor(data.length / 8), 1) === 0 || i === data.length - 1);
    return <svg viewBox={`0 0 ${width} ${height}`} className={styles.chartSvg} preserveAspectRatio="xMidYMid meet">
        {gridLines}
        <defs>
          <linearGradient id="trendGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="#00F0FF" stopOpacity="0.2" />
            <stop offset="100%" stopColor="#00F0FF" stopOpacity="0.01" />
          </linearGradient>
        </defs>
        <polygon points={`${xScale(0)},${pad.top + h} ${points} ${xScale(data.length - 1)},${pad.top + h}`} fill="url(#trendGrad)" />
        <polyline points={points} className={styles.chartLinePath} stroke="#00F0FF" />
        {data.map((d, i) => <circle key={`dot-${i}`} cx={xScale(i)} cy={yScale(d[1])} r={3} fill="#00F0FF" stroke="#0E1219" strokeWidth={1.5} className={styles.chartDot} />)}
        {xLabels.map(d => {
        const i = data.indexOf(d);
        return <text key={`xl-${i}`} x={xScale(i)} y={height - 6} textAnchor="middle" className={styles.chartAxisLabel}>
              {d[0]}
            </text>;
      })}
      </svg>;
  }, [data, width, height]);
  if (!chart) return null;
  return <div className={styles.trendCard}>
      <div className={styles.chartTitle}>
        {t("IntelligenceCharts.k1")}
        <span className={styles.chartSubtitle}>{data.length} {t("IntelligenceCharts.k2")}</span>
      </div>
      {chart}
    </div>;
}
interface DonutChartProps {
  data: [string, number][];
  size?: number;
}
export function ActivityDonutChart({
  data,
  size = 200
}: DonutChartProps) {
  const chart = useMemo(() => {
    if (!data || data.length === 0) return null;
    const total = data.reduce((s, d) => s + d[1], 0) || 1;
    const r = size / 2 - 10;
    const innerR = r * 0.6;
    const cx = size / 2;
    const cy = size / 2;
    let angle = -Math.PI / 2;
    const segments = data.map((d, i) => {
      const pct = d[1] / total;
      const sweep = pct * Math.PI * 2;
      const startAngle = angle;
      const endAngle = angle + sweep;
      const x1 = cx + r * Math.cos(startAngle);
      const y1 = cy + r * Math.sin(startAngle);
      const x2 = cx + r * Math.cos(endAngle);
      const y2 = cy + r * Math.sin(endAngle);
      const xi1 = cx + innerR * Math.cos(startAngle);
      const yi1 = cy + innerR * Math.sin(startAngle);
      const xi2 = cx + innerR * Math.cos(endAngle);
      const yi2 = cy + innerR * Math.sin(endAngle);
      const largeArc = sweep > Math.PI ? 1 : 0;
      const path = [`M ${x1} ${y1}`, `A ${r} ${r} 0 ${largeArc} 1 ${x2} ${y2}`, `L ${xi2} ${yi2}`, `A ${innerR} ${innerR} 0 ${largeArc} 0 ${xi1} ${yi1}`, 'Z'].join(' ');
      angle = endAngle;
      return {
        path,
        color: ACTIVITY_COLORS[i % ACTIVITY_COLORS.length],
        label: d[0],
        count: d[1],
        pct
      };
    });
    return {
      segments,
      total,
      cx,
      cy
    };
  }, [data, size]);
  if (!chart) return null;
  return <div className={styles.chartCard}>
      <div className={styles.chartTitle}>{t("IntelligenceCharts.k3")}</div>
      <div className={styles.donutContainer}>
        <svg viewBox={`0 0 ${size} ${size}`} width={size} height={size} className={styles.donutSvg}>
          {chart.segments.map((seg, i) => <path key={i} d={seg.path} fill={seg.color} opacity={0.85} stroke="#0E1219" strokeWidth={2} />)}
          <text x={chart.cx} y={chart.cy - 6} className={styles.donutCenter}>
            <tspan className={styles.donutCenterTotal}>{chart.total}</tspan>
          </text>
          <text x={chart.cx} y={chart.cy + 10} className={styles.donutCenter}>
            <tspan className={styles.donutCenterLabel}>{t("IntelligenceCharts.k4")}</tspan>
          </text>
        </svg>
        <div className={styles.donutLegend}>
          {chart.segments.map((seg, i) => <div key={i} className={styles.donutLegendItem}>
              <span className={styles.donutLegendLabel}>
                <span className={styles.chartLegendDot} style={{
              background: seg.color
            }} />
                {seg.label}
                <span className={styles.donutLegendPct}>{(seg.pct * 100).toFixed(1)}%</span>
              </span>
              <span className={styles.donutLegendCount}>{seg.count}</span>
            </div>)}
        </div>
      </div>
    </div>;
}
interface HeatmapProps {
  data: number[];
}
export function ActivityHeatmap({
  data
}: HeatmapProps) {
  const maxVal = Math.max(...data, 1);
  return <div className={styles.chartCard}>
      <div className={styles.chartTitle}>{t("IntelligenceCharts.k5")}</div>
      <div className={styles.heatmap}>
        {data.map((count, i) => <div key={i} className={styles.heatmapCell} style={{
        background: HEATMAP_COLORS[getHeatLevel(count, maxVal)]
      }} title={t("IntelligenceCharts.k6", {
        i: i,
        count: count
      })} />)}
      </div>
      <div className={styles.heatmapLabel}>
        <span>{t("IntelligenceCharts.k7")}</span>
        <span>{t("IntelligenceCharts.k8")}</span>
        <span>{t("IntelligenceCharts.k9")}</span>
        <span>{t("IntelligenceCharts.k10")}</span>
        <span>{t("IntelligenceCharts.k11")}</span>
      </div>
    </div>;
}
interface EnhancedBarChartProps {
  data: [string, number][];
  width?: number;
  height?: number;
}
export function EnhancedBarChart({
  data,
  width = 720,
  height = 200
}: EnhancedBarChartProps) {
  const chart = useMemo(() => {
    if (!data || data.length === 0) return null;
    const pad = {
      top: 10,
      right: 20,
      bottom: 35,
      left: 80
    };
    const w = width - pad.left - pad.right;
    const h = height - pad.top - pad.bottom;
    const maxVal = Math.max(...data.map(d => d[1]), 1);
    const barW = Math.min(48, w / data.length * 0.7);
    const gap = (w - barW * data.length) / (data.length + 1);
    return <svg viewBox={`0 0 ${width} ${height}`} className={styles.chartSvg} preserveAspectRatio="xMidYMid meet">
        {data.map((d, i) => {
        const barH = d[1] / maxVal * h;
        const x = pad.left + gap + i * (barW + gap) + barW / 2 - barW / 2;
        const y = pad.top + h - barH;
        const color = ACTIVITY_COLORS[i % ACTIVITY_COLORS.length];
        return <g key={i}>
              <rect x={x} y={y} width={barW} height={barH} rx={2} fill={color} opacity={0.8} className={styles.chartBar} />
              <text x={x + barW / 2} y={y - 6} textAnchor="middle" className={styles.chartBarValue}>
                {d[1]}
              </text>
              <text x={x + barW / 2} y={height - 8} textAnchor="middle" className={styles.chartAxisLabel}>
                {d[0].length > 5 ? d[0].substring(0, 5) + '...' : d[0]}
              </text>
            </g>;
      })}
      </svg>;
  }, [data, width, height]);
  if (!chart) return null;
  return <div className={styles.chartCard}>
      <div className={styles.chartTitle}>
        {t("IntelligenceCharts.k12")}
        <span className={styles.chartSubtitle}>{t("IntelligenceCharts.k13")}</span>
      </div>
      {chart}
    </div>;
}