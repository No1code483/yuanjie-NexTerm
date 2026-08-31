import { t } from "i18next";
// Home 模块共享工具函数

export function formatDateChinese(dateStr: string): string {
  if (!dateStr) return '';
  const d = new Date(dateStr);
  return t("home.utils.k1", {
    arg0: d.getFullYear(),
    arg1: d.getMonth() + 1,
    arg2: d.getDate()
  });
}
export function formatTimerTime(seconds: number): string {
  const hrs = Math.floor(seconds / 3600);
  const mins = Math.floor(seconds % 3600 / 60);
  const secs = seconds % 60;
  return `${String(hrs).padStart(2, '0')}:${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}`;
}
export function calculateRemainingDays(targetDate: string): number {
  const target = new Date(targetDate);
  const now = new Date();
  const diff = target.getTime() - now.getTime();
  return Math.ceil(diff / (1000 * 60 * 60 * 24));
}
export function getPriorityConfig(priority: string) {
  switch (priority) {
    case 'high':
      return {
        label: t("home.utils.k2"),
        color: '#FF0040',
        bg: 'rgba(255, 0, 64, 0.12)'
      };
    case 'medium':
      return {
        label: t("home.utils.k3"),
        color: '#FFD700',
        bg: 'rgba(255, 215, 0, 0.12)'
      };
    case 'low':
      return {
        label: t("home.utils.k4"),
        color: '#00F0FF',
        bg: 'rgba(0, 240, 255, 0.12)'
      };
    default:
      return {
        label: t("home.utils.k3"),
        color: '#FFD700',
        bg: 'rgba(255, 215, 0, 0.12)'
      };
  }
}
export function getCategoryLabel(cat?: string) {
  switch (cat) {
    case 'security':
    case 'vulnerability':
    case 'attack_defense':
      return {
        text: t("home.utils.k5"),
        color: '#FF4444'
      };
    case 'ai':
    case 'tech_innovation':
      return {
        text: 'AI',
        color: '#B026FF'
      };
    case 'programming':
    case 'tool_application':
    case 'cloud_native':
    case 'open_source':
      return {
        text: t("home.utils.k6"),
        color: '#00F0FF'
      };
    case 'github':
      return {
        text: 'GitHub',
        color: '#6e5494'
      };
    default:
      return {
        text: t("lib.xinChatEngine.k69"),
        color: '#8a8aaa'
      };
  }
}
export function isOverdue(dueDate: string | undefined, today: string): boolean {
  if (!dueDate) return false;
  return new Date(dueDate) < new Date(today);
}
export function isGithubItem(item: {
  category?: string;
  url?: string;
}): boolean {
  return item.category === 'github' || !!item.url && item.url.includes('github.com') && !item.url.includes('/blog');
}
export function formatDateFull(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const days = [t("home.utils.k7"), t("home.utils.k8"), t("home.utils.k9"), t("home.utils.k10"), t("home.utils.k11"), t("home.utils.k12"), t("home.utils.k13")];
  return `${year}-${month}-${day} ${days[date.getDay()]}`;
}
export function formatTime(date: Date): string {
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');
  const seconds = String(date.getSeconds()).padStart(2, '0');
  return `${hours}:${minutes}:${seconds}`;
}