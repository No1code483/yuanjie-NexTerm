import { t } from "i18next";
export interface Persona {
  id: string;
  name: string;
  description: string;
  emoji: string;
  traits: {
    name: string;
    value: number;
  }[];
  style: {
    formality: number;
    verbosity: number;
    humor: number;
    technical_depth: number;
    empathy: number;
  };
}
export interface ChatMessage {
  id: string;
  role: 'user' | 'xin';
  content: string;
  timestamp: string;
}
export interface Conversation {
  id: string;
  persona_id: string;
  persona_name: string;
  model_id: string;
  title: string;
  message_count: number;
  total_tokens: number;
  summary: string | null;
  created_at: string;
  updated_at: string;
}
export interface Memory {
  id: string;
  category: string;
  key: string;
  value: string;
  importance: number;
  created_at: string;
}
export interface MoodEntry {
  id: string;
  category: string;
  emoji: string;
  intensity: number;
  context: string;
  time: string;
}
export interface Reminder {
  id: string;
  title: string;
  description: string;
  time: string;
  repeat: string;
  active: boolean;
}
export interface Habit {
  id: string;
  name: string;
  category: string;
  streak: number;
  total: number;
  checked: boolean;
}
export interface CompactionRecord {
  id: string;
  conversation_id: string;
  trigger_type: string;
  pre_message_count: number;
  pre_token_count: number;
  post_message_count: number;
  post_token_count: number;
  summary_text: string;
  memory_flush_count: number;
  guidance_text: string | null;
  created_at: string;
}
export interface CompactionConfig {
  enabled: boolean;
  trigger_threshold_ratio: number;
  keep_recent_tokens: number;
  model_override: string | null;
  memory_flush_enabled: boolean;
  notify_user: boolean;
  compaction_mode: string;
}
export interface DreamConfig {
  enabled: boolean;
  timezone: string | null;
  verbose: boolean;
  phases: DreamPhasesConfig;
}
export interface DreamPhasesConfig {
  light: {
    enabled: boolean;
    interval_hours: number;
    lookback_hours: number;
    max_candidates: number;
  };
  deep: {
    enabled: boolean;
    interval_hours: number;
    max_promotions: number;
    min_score: number;
  };
  rem: {
    enabled: boolean;
    interval_hours: number;
    lookback_days: number;
    min_pattern_strength: number;
  };
}
export interface DreamCandidate {
  key: string;
  value: string;
  category: string;
  source: string;
  confidence: number;
  importance: number;
  keywords: string[];
  timestamp_ms: number;
}
export interface DreamResult {
  performed_at: string;
  duration_ms: number;
  candidates?: DreamCandidate[];
  deduped_count?: number;
  sources_processed?: number;
  promoted?: number;
  clusters?: any[];
  health_score?: number;
  health_status?: string;
  patterns?: any[];
  pattern_count?: number;
  cross_domain_links?: any[];
}
export interface DreamState {
  last_light_at: string | null;
  last_deep_at: string | null;
  last_rem_at: string | null;
  light_candidates_count: number;
  deep_promotions_count: number;
  rem_patterns_count: number;
}
export interface CheckpointSummary {
  id: string;
  conversation_id: string;
  checkpoint_type: string;
  message_count: number;
  total_tokens: number;
  title: string | null;
  has_partial_response: boolean;
  created_at: string;
}
export interface SearchResult {
  conversation_id: string;
  title: string;
  message_count: number;
  total_tokens: number;
  summary: string | null;
  created_at: string;
  matched_snippets: string[];
  relevance_score: number;
}
export interface ConversationReview {
  period_type: string;
  period_label: string;
  date_from: string;
  date_to: string;
  conversation_count: number;
  total_messages: number;
  total_tokens: number;
  dominant_topics: {
    topic: string;
    count: number;
  }[];
  generated_summary: string;
  most_active_hours: number[];
}
export interface TopicTrend {
  topic: string;
  data_points: {
    date_label: string;
    value: number;
  }[];
}
export interface GrowthTrajectory {
  date_from: string;
  date_to: string;
  emotion_trend: {
    date_label: string;
    value: number;
  }[];
  empathy_trend: {
    date_label: string;
    value: number;
  }[];
  creativity_trend: {
    date_label: string;
    value: number;
  }[];
  conversation_frequency: {
    date_label: string;
    value: number;
  }[];
  token_usage_trend: {
    date_label: string;
    value: number;
  }[];
}
export interface HeatmapCell {
  day_of_week: number;
  hour: number;
  count: number;
  intensity: number;
}
export interface HeatmapData {
  cells: HeatmapCell[];
  max_intensity: number;
}
export interface SkillInfo {
  id: string;
  name: string;
  description: string;
  category: string;
  available: boolean;
}
export interface SkillResult {
  skill_id: string;
  success: boolean;
  output: string;
  error: string | null;
}
export interface ToolDef {
  name: string;
  description: string;
  parameters: {
    name: string;
    description: string;
    param_type: string;
    required: boolean;
  }[];
  category: string;
}
export interface ToolCall {
  tool_name: string;
  arguments: Record<string, string>;
  raw: string;
}
export interface ToolExecResult {
  tool_name: string;
  success: boolean;
  output: string;
  error: string | null;
  duration_ms: number;
}
export interface FusionResult {
  source: string;
  score: number;
  content: string;
  metadata: Record<string, string>;
}
export interface AiModelOption {
  id: number;
  name: string;
  provider: string;
  model_name: string;
}
export const BUILTIN_PERSONAS: Persona[] = [{
  id: 'code_assistant',
  name: t("xin.types.k1"),
  emoji: '💻',
  description: t("xin.types.k2"),
  traits: [{
    name: t("xin.types.k3"),
    value: 95
  }, {
    name: t("lib.xinChatEngine.k68"),
    value: 60
  }, {
    name: t("xin.types.k4"),
    value: 85
  }, {
    name: t("xin.types.k5"),
    value: 90
  }],
  style: {
    formality: 0.5,
    verbosity: 0.7,
    humor: 0.2,
    technical_depth: 0.95,
    empathy: 0.5
  }
}, {
  id: 'knowledge_mentor',
  name: t("xin.types.k6"),
  emoji: '📚',
  description: t("xin.types.k7"),
  traits: [{
    name: t("xin.types.k3"),
    value: 90
  }, {
    name: t("lib.xinChatEngine.k68"),
    value: 65
  }, {
    name: t("xin.types.k4"),
    value: 95
  }, {
    name: t("xin.types.k5"),
    value: 85
  }],
  style: {
    formality: 0.6,
    verbosity: 0.8,
    humor: 0.2,
    technical_depth: 0.85,
    empathy: 0.7
  }
}, {
  id: 'creative_partner',
  name: t("xin.types.k8"),
  emoji: '🎨',
  description: t("xin.types.k9"),
  traits: [{
    name: t("xin.types.k3"),
    value: 55
  }, {
    name: t("lib.xinChatEngine.k68"),
    value: 95
  }, {
    name: t("xin.types.k4"),
    value: 80
  }, {
    name: t("xin.types.k5"),
    value: 50
  }],
  style: {
    formality: 0.2,
    verbosity: 0.6,
    humor: 0.7,
    technical_depth: 0.4,
    empathy: 0.85
  }
}, {
  id: 'caring_friend',
  name: t("xin.types.k10"),
  emoji: '☕',
  description: t("xin.types.k11"),
  traits: [{
    name: t("xin.types.k3"),
    value: 50
  }, {
    name: t("lib.xinChatEngine.k68"),
    value: 70
  }, {
    name: t("xin.types.k4"),
    value: 95
  }, {
    name: t("xin.types.k5"),
    value: 45
  }],
  style: {
    formality: 0.15,
    verbosity: 0.5,
    humor: 0.55,
    technical_depth: 0.3,
    empathy: 0.95
  }
}];
export const MOOD_EMOJI: Record<string, string> = {
  happy: '😊',
  productive: '⚡',
  calm: '😌',
  curious: '🤔',
  tired: '😴',
  frustrated: '😤',
  excited: '🎉',
  focused: '🎯',
  anxious: '😰',
  inspired: '✨'
};
export const CATEGORY_LABELS: Record<string, string> = {
  knowledge: t("xin.types.k12"),
  preference: t("xin.types.k13"),
  habit: t("xin.types.k14"),
  project: t("profile.ResumePanel.k87"),
  personal: t("xin.types.k15"),
  note: t("Recycle.k3")
};