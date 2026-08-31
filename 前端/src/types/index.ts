/**
 * RBAC 权限系统类型定义
 * 对齐后端 permissions 表和 07_安全体系.md
 */

// ==================== 权限维度 (四维) ====================
export type PermissionAction = 'read' | 'write' | 'delete' | 'modify'

// ==================== 资源标识符 (8大模块 + 子功能) ====================
export type ResourceName = 
  // ===== 主模块 (7大板块) =====
  | 'home'           // 首页
  | 'ai_chat'        // AI对话
  | 'knowledge'      // 知识库
  | 'terminal'       // 终端
  | 'game'           // 游戏
  | 'profile_settings' // 个人设置
  | 'recycle_bin'    // 回收站
  
  // ===== 首页子功能 =====
  | 'home_news'      // 首页-新闻
  | 'home_todo'      // 首页-待办
  | 'home_log'       // 首页-日志
  | 'home_timer'     // 首页-计时器

  // ===== 终端子功能 =====
  | 'terminal_manual'    // 终端-命令手册
  | 'terminal_yuancode'  // 终端-YuanCode
  | 'terminal_linux'     // 终端-Linux

  // ===== 其他独立模块 =====
  | 'search'         // 搜索
  | 'xin'            // 小欣
  | 'spyglass'       // 底层智能

// ==================== 角色定义 ====================
export type Role = 'admin' | 'user' | 'guest'

// ==================== 权限详情 (对齐后端 Permission 模型) ====================
export interface PermissionDetail {
  can_read: boolean
  can_write: boolean
  can_delete: boolean
  can_modify: boolean
}

/** 后端返回的原始权限记录 (auth_get_permissions 命令的返回格式) */
export interface BackendPermission {
  id: number
  role: string
  resource: string
  can_read: boolean
  can_write: boolean
  can_delete: boolean
  can_modify: boolean
}

// ==================== 完整权限矩阵 (Role × Resource → PermissionDetail) ====================
export type PermissionsMatrix = Record<ResourceName, PermissionDetail>

// ==================== 用户信息 (对齐后端 User 模型) ====================
export interface UserInfo {
  id: number
  username: string
  role: Role
  is_permanent: boolean
  permissions?: string[]
  expires_at?: number
  created_at?: number
  avatar_url?: string | null
  bio?: string | null
  display_name?: string | null
}

// ==================== API 响应格式 (统一) ====================
export interface ApiResponse<T = any> {
  code: number         // 0=成功, 其他=错误码
  message: string      // 错误/成功消息
  data?: T             // 数据载荷
}

// ==================== 登录响应 ====================
export interface LoginResponse {
  token: string
  user: UserInfo
  permissions?: PermissionsMatrix  // 可选：登录时一并返回权限
}

// ==================== AI 相关类型 ====================
export interface AIModel {
  id: number
  name: string
  provider: string
  api_endpoint?: string
  model_name: string
  type: 'local' | 'api'
  status: 'active' | 'inactive' | 'error'
  created_at?: string
}

export interface ChatMessage {
  id: number
  session_id: number
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: string
  token_count?: number
  model?: string
}

export interface ChatSession {
  id: number
  name: string
  type: 'single' | 'group' | 'agent'
  model_id: number
  model_name: string
  last_message?: string
  last_message_at?: string
  unread_count: number
  created_at: string
  updated_at: string
}

// v1.01 新增：群聊编排状态
export interface GroupChatOrchestration {
  session_id: number
  current_round: number
  max_rounds: number
  total_token_budget: number
  used_tokens: number
  status: 'discussing' | 'converged' | 'summarizing' | 'completed' | 'timeout'
  timeout_models: string[]
  summary?: string
  summarizer?: string
  started_at: string
  ended_at?: string
}

// ==================== 知识库类型 ====================
export interface KnowledgeCategory {
  id: number
  name: string
  parent_id?: number
  item_count: number
  level: number
  expanded: boolean
}

export interface KnowledgeItem {
  id: number
  category_id: number
  title: string
  content?: string
  source: 'file' | 'link' | 'text'
  file_path?: string
  url?: string
  preview?: string
  created_at: string
  updated_at: string
}

// ==================== 首页相关类型 ====================
export interface TodoItem {
  id: number
  user_id: number
  title: string
  description?: string
  completed: boolean
  priority: 'low' | 'medium' | 'high'
  due_date?: string
  created_at: string
  updated_at: string
}

export interface NewsItem {
  id: number
  title: string
  summary: string
  content: string
  source: string
  published_at: string
  read: boolean
  category: string
}

export interface LogEntry {
  id: number
  user_id: number
  date: string
  content: string
  created_at: string
  updated_at: string
}

// ==================== 回收站类型 ====================
export type RecyclableType = 'todo' | 'knowledge_item' | 'chat_session' | 'ai_model' | 'file'

export interface RecycledItem {
  id: number
  original_id: number
  type: RecyclableType
  title: string
  description?: string
  deleted_at: string
  deleted_by: number
  restoreable: boolean
  permanent_delete_at?: string
}

// ==================== 终端类型 ====================
export interface TerminalTab {
  id: number
  name: string
  type: 'terminal' | 'cmd' | 'xincode' | 'linux'
  active: boolean
}

// ==================== 计时器类型 ====================
export interface ShortTimer {
  id: number
  name: string
  duration: number      // 秒
  remaining: number      // 剩余秒数
  status: 'running' | 'paused' | 'completed'
  started_at?: string
  finished_at?: string
}

export interface LongTimer {
  id: number
  name: string
  target_date: string   // ISO日期字符串
  remaining_days: number
  completed: boolean
}

// ==================== 万能转接头接口 ====================
export interface NexTermAdapter<T = any> {
  type: string
  version: string
  data: T
  onMount?: () => void
  onUnmount?: () => void
  onMessage?: (msg: any) => void
}

// ==================== 底层智能引擎类型 (Engine 7 V2) ====================

export interface SuggestionV2 {
  id: string
  title: string
  description: string
  priority: number
  action_type: 'run_command' | 'open_file' | 'ai_help' | 'navigation' | 'info'
  category: string
  created_at: string
  action_payload?: Record<string, any>
}

export interface UserContext {
  active_file: string | null
  active_file_language: string | null
  active_terminal_sessions: string[]
  active_window_title: string | null
  recent_activities: {
    activity_type: string
    action: string
    detail: string
    file_path?: string
    timestamp: string
  }[]
  collected_at: string
}

export interface ContextSnapshot {
  id: string
  context: {
    active_file: string | null
  }
  captured_at: string
}

export interface OllamaStatus {
  is_running: boolean
  is_installed: boolean
  version?: string
  api_url?: string
  available_models?: { name: string; parameter_size?: string; quantization?: string; size_bytes: number }[]
}

export interface IntelligenceConfig {
  enabled: boolean
  auto_suggest_enabled: boolean
  context_collection_enabled: boolean
  ollama_url: string
  suggestion_interval_secs: number
  max_snapshots?: number
}

export interface BehaviorAnalysisResult {
  total_activities_analyzed: number
  dominant_category: string
  patterns: {
    pattern_name: string
    category: string
    frequency: number
    avg_duration_secs: number
    last_observed_at: string
    activities: string[]
  }[]
}

export interface WorkflowRecommendation {
  id: string
  title: string
  description: string
  steps: string[]
  relevance_score: number
  based_on_patterns: string[]
}

export interface ProjectTechStack {
  project_type: string
  confidence: number
  primary_language: string
  frameworks: string[]
  build_tools: string[]
  package_manager: string
  detected_files: string[]
}

export interface CognitiveLoadSnapshot {
  load_level: 'low' | 'medium' | 'high' | 'critical'
  load_score: number
  active_duration_secs: number
  activity_count: number
  context_switches: number
  suggestion?: string
}

export interface SmartNotification {
  id: string
  title: string
  message: string
  priority: 'high' | 'medium' | 'low'
  notification_type: string
  triggered_by: string
  timestamp: string
  action_type?: string
}

export interface UsageDashboard {
  total_activity: number
  total_activities?: number
  total_active_hours?: number
  daily_active: number
  suggestions_count: number
  score: number
  productivity_score?: number
  avg_sessions_per_day?: number
  top_activity_types: [string, number][]
  daily_trend: [string, number][]
  hourly_activity: number[]
  top_files: [string, number][]
  peak_hours: number[]
}

// ==================== 底层智能 V4 类型 (Engine 7 V4) ====================

export interface ActivityLog {
  id: number
  userId: string
  timestamp: string
  module: string
  operation: string
  detail?: string
  remark?: string
  durationSecs: number
  createdAt: string
}

export interface ActivityStats {
  totalOperations: number
  byModule: Record<string, number>
  byOperation: Record<string, number>
  byHour: number[]
  byDay: number[]
}

export interface TopFile {
  file_name: string
  kb_path: string
  local_path: string
  access_count: number
  total_duration_secs: number
}

export interface HourlyActivity {
  hour: string
  count: number
  label: string
}

export interface DailyTrend {
  date: string
  active_secs: number
  operation_count: number
}

export interface ProductivityBreakdown {
  focus_ratio: number
  todo_completion_ratio: number
  knowledge_regularity: number
  focus_weight: number
  todo_weight: number
  knowledge_weight: number
  raw_score: number
}

export interface DashboardData {
  period: string
  total_active_secs: number
  total_active_hours: number
  total_operations: number
  top_files: TopFile[]
  top_modules: [string, number][]
  hourly_heatmap: HourlyActivity[]
  period_labels: string[]
  productivity_score: number
  productivity_breakdown: ProductivityBreakdown
  peak_hours: string[]
  daily_trend: DailyTrend[]
  generated_at: string
}

export interface RealtimeStats {
  active_duration_today_secs: number
  operations_today: number
  current_productivity_score: number
  modules_used_today: number
  current_focus_module: string | null
  last_activity_at: string | null
}

export interface SuggestionV4 {
  id: number
  user_id: string
  category: string
  title: string
  description: string
  priority: string
  source: string
  status: string
  created_at: string | null
  updated_at: string | null
}

export type Suggestion = SuggestionV4

export interface BehaviorReport {
  date: string
  focus_score: number
  focus_verdict: string
  distraction_count: number
  distraction_verdict: string
  kb_avg_duration_secs: number
  kb_avg_duration_human: string
  kb_entry_count: number
  error_rate: number
  error_verdict: string
  active_period: string
  peak_hour: number | null
  peak_hour_label: string | null
  module_diversity: number
  module_diversity_verdict: string
  consistency_score: number
  consistency_verdict: string
  summary: string
  comparison: BehaviorComparison | null
  generated_at: string
}

export interface BehaviorComparison {
  period_label: string
  focus_change: number
  distraction_change: number
  error_change: number
  consistency_change: number
  overall_trend: string
}

// ==================== Linux 内核类型 (Engine 5) ====================

export interface LinuxEnvironmentInfo {
  os_name: string
  os_version: string
  kernel: string
  architecture: string
  cpu_model: string
  cpu_cores: number
  total_memory: string
  available_memory: string
  total_disk: string
  available_disk: string
  [key: string]: any
}

export interface LinuxKernelVersion {
  version: string
  major: number
  minor: number
  patch: number
  release: string
  arch: string
  build_time: string
  compiler: string
  [key: string]: any
}

export interface LinuxSourceFile {
  name: string
  path: string
  is_dir: boolean
  children?: any[]
  size?: number
  modified_at?: string
  [key: string]: any
}

export interface LinuxSearchResponse {
  total: number
  results: any[]
  [key: string]: any
}

export interface KernelBuildResult {
  success: boolean
  duration_secs: number
  output: string
  errors: any[]
  warnings: any[]
  [key: string]: any
}

export interface KernelConfigResult {
  [key: string]: any
}

export interface KernelModuleInfo {
  name: string
  size: number
  used_by: string[]
  status: string
  description: string
  parameters: any[]
  [key: string]: any
}

export interface KernelLogAnalysis {
  critical_events: any[]
  summary: any
  entries: any[]
  [key: string]: any
}

export interface PerfProfileResult {
  total_samples: number
  duration_secs: number
  top_functions: any[]
  top_symbols: any[]
  [key: string]: any
}

export interface LinuxShellStatus {
  is_installed: boolean
  is_running: boolean
  [key: string]: any
}

export interface LinuxSystemInfo {
  hostname: string
  uptime_secs: number
  load_average: [number, number, number]
  processes: number
  cpu_usage: number
  memory_usage: number
  disk_usage: number
  [key: string]: any
}

export interface LinuxIsoProgress {
  is_running: boolean
  progress: number
  current_step: string
  speed_mbps: number
  estimated_remaining_secs: number
  [key: string]: any
}

export interface LinuxStatusPanel {
  system: LinuxSystemInfo
  environment: LinuxEnvironmentInfo
  shell: LinuxShellStatus
  last_updated?: string
  [key: string]: any
}

export interface LinuxBenchmarkResult {
  categories: any[]
  [key: string]: any
}

export interface LinuxStressResult {
  scenarios: any[]
  [key: string]: any
}
