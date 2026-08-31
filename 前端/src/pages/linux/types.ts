import { t } from "i18next";
export type TabId = 'status' | 'versions' | 'source' | 'build' | 'config' | 'modules' | 'logs' | 'perf' | 'benchmark' | 'stress' | 'docker' | 'network';
export const TABS: {
  id: TabId;
  label: string;
  icon: string;
}[] = [{
  id: 'status',
  label: t("linux.types.k1"),
  icon: '📡'
}, {
  id: 'versions',
  label: t("linux.types.k2"),
  icon: '📦'
}, {
  id: 'source',
  label: t("components.Linux.k16"),
  icon: '📂'
}, {
  id: 'build',
  label: t("linux.types.k3"),
  icon: '🔨'
}, {
  id: 'config',
  label: t("linux.types.k4"),
  icon: '⚙️'
}, {
  id: 'modules',
  label: t("linux.types.k5"),
  icon: '🧩'
}, {
  id: 'logs',
  label: t("linux.types.k6"),
  icon: '📋'
}, {
  id: 'perf',
  label: t("linux.types.k7"),
  icon: '⚡'
}, {
  id: 'benchmark',
  label: t("linux.types.k8"),
  icon: '📊'
}, {
  id: 'stress',
  label: t("linux.types.k9"),
  icon: '💥'
}, {
  id: 'docker',
  label: 'Docker',
  icon: '🐳'
}, {
  id: 'network',
  label: t("linux.types.k10"),
  icon: '🌐'
}];
export const LOG_LEVEL_COLORS: Record<string, string> = {
  emerg: '#FF006E',
  alert: '#FF4500',
  crit: '#FF6347',
  err: '#FF006E',
  warn: '#FFD700',
  notice: '#00F0FF',
  info: '#00F0FF',
  debug: '#888888'
};