import { t } from "i18next";
import type { Memory, MoodEntry } from './types';
export function getMockMemories(): Memory[] {
  return [{
    id: 'm1',
    category: 'project',
    key: t("profile.ResumePanel.k44"),
    value: t("xin.mockData.k1"),
    importance: 5,
    created_at: new Date(Date.now() - 86400000 * 30).toISOString()
  }, {
    id: 'm2',
    category: 'knowledge',
    key: t("xin.mockData.k2"),
    value: t("xin.mockData.k3"),
    importance: 4,
    created_at: new Date(Date.now() - 86400000 * 20).toISOString()
  }, {
    id: 'm3',
    category: 'preference',
    key: t("xin.mockData.k4"),
    value: t("xin.mockData.k5"),
    importance: 3,
    created_at: new Date(Date.now() - 86400000 * 15).toISOString()
  }, {
    id: 'm4',
    category: 'habit',
    key: t("xin.mockData.k6"),
    value: t("xin.mockData.k7"),
    importance: 4,
    created_at: new Date(Date.now() - 86400000 * 10).toISOString()
  }, {
    id: 'm5',
    category: 'personal',
    key: t("xin.mockData.k8"),
    value: t("xin.mockData.k9"),
    importance: 3,
    created_at: new Date(Date.now() - 86400000 * 5).toISOString()
  }, {
    id: 'm6',
    category: 'note',
    key: t("AutoSaveDemo.k18"),
    value: t("xin.mockData.k10"),
    importance: 4,
    created_at: new Date(Date.now() - 86400000 * 2).toISOString()
  }];
}
export function getMockMoodTimeline(): MoodEntry[] {
  return [{
    id: 'e1',
    category: 'curious',
    emoji: '🤔',
    intensity: 0.7,
    context: t("xin.mockData.k11"),
    time: t("xin.mockData.k12")
  }, {
    id: 'e2',
    category: 'productive',
    emoji: '⚡',
    intensity: 0.85,
    context: t("xin.mockData.k13"),
    time: t("xin.mockData.k14")
  }, {
    id: 'e3',
    category: 'calm',
    emoji: '😌',
    intensity: 0.6,
    context: t("xin.mockData.k15"),
    time: t("xin.mockData.k16")
  }, {
    id: 'e4',
    category: 'inspired',
    emoji: '✨',
    intensity: 0.8,
    context: t("xin.mockData.k17"),
    time: t("common.yesterday")
  }, {
    id: 'e5',
    category: 'focused',
    emoji: '🎯',
    intensity: 0.9,
    context: t("xin.mockData.k18"),
    time: t("common.yesterday")
  }];
}
export function getMockBriefing() {
  return {
    date: new Date().toLocaleDateString('zh-CN'),
    greeting: t("xin.mockData.k19"),
    quote: t("xin.mockData.k20"),
    suggestions: [t("xin.mockData.k21"), t("xin.mockData.k22"), t("xin.mockData.k23")],
    memories: [{
      title: t("xin.mockData.k24"),
      content: t("xin.mockData.k25")
    }, {
      title: t("xin.mockData.k26"),
      content: t("xin.mockData.k3")
    }]
  };
}