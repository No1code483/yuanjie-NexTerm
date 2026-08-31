// ipc/core.ts — IPC 核心基础设施（从 ipc.ts 拆分，T2.1.6）
// 包含：IPCache / IPCService / ipc 单例 / ApiResponse 类型 / USE_MOCK

import { t } from "i18next";
import { getLocalizedErrorMessage } from '../errorCodeI18n';
/**
 * IPC 统一封装层
 * 
 * 功能:
 * 1. 统一 Tauri invoke 调用接口
 * 2. 支持 Mock 模式 (USE_MOCK = true)
 * 3. 自动错误处理和日志
 * 4. 统一返回格式 ApiResponse<T>
 * 
 * 使用方式:
 * import { ipc } from '@/lib/ipc'
 * const result = await ipc.invoke('login', { username, password })
 * 
 * 命令名必须与后端 Tauri Command 函数名一致
 */

// 扩展 Window 接口以支持 Tauri 环境检测
declare global {
  interface Window {
    __TAURI__?: unknown;
  }
}

// 是否使用 Mock 数据 (通过环境变量 VITE_MOCK 控制)
export const USE_MOCK = import.meta.env.VITE_MOCK === 'true';
import { ipcMockHandlers } from '../ipcMock';

// 统一响应格式 (对齐后端)
export interface ApiResponse<T = any> {
  code: number;
  message: string;
  data?: T;
}

// ============================================================================
// IPC 日志脱敏（安全审计修复发现 12，MEDIUM）
//
// 原实现 `console.log('[IPC] 🚀 开始调用: ${command}', args)` 直接打印所有参数到
// devtools，含 password / api_key / recovery_phrase / base64_data 等敏感字段，
// 任何能访问 devtools 的扩展或调试器即可读取。现引入 `sanitizeArgs` 在日志输出前
// 对敏感字段脱敏。
// ============================================================================

/** 敏感字段名匹配模式（小写匹配） */
const SENSITIVE_KEY_PATTERNS = [
  'password',
  'passwd',
  'pwd',
  'token',
  'secret',
  'api_key',
  'apikey',
  'api_key_plain',
  'private_key',
  'public_key', // 公钥虽非机密，但日志无需打印完整内容
  'recovery_phrase',
  'mnemonic',
  'seed',
  'passphrase',
  'credential',
  'authorization',
  'auth',
  'session_id', // 会话 ID 可被劫持
  'csrf_token',
];

/** 长字段截断阈值（base64_data / content 等可能很大） */
const TRUNCATE_THRESHOLD = 80;

/**
 * 判断字段名是否敏感（小写匹配，包含即视为敏感）
 */
function isSensitiveKey(key: string): boolean {
  const lower = key.toLowerCase();
  return SENSITIVE_KEY_PATTERNS.some((p) => lower.includes(p));
}

/**
 * 脱敏单个值：敏感字段返回 `***REDACTED***`，长字段截断
 */
function sanitizeValue(key: string, value: unknown): unknown {
  // 敏感字段直接脱敏
  if (isSensitiveKey(key)) {
    return '***REDACTED***';
  }
  // 字符串长字段截断（避免日志被 base64/大文本淹没）
  if (typeof value === 'string' && value.length > TRUNCATE_THRESHOLD) {
    return `${value.slice(0, TRUNCATE_THRESHOLD)}...(truncated, len=${value.length})`;
  }
  return value;
}

/**
 * 递归脱敏 args 对象（深拷贝，不修改原对象）
 *
 * - 对象/数组递归处理
 * - 敏感字段值替换为 `***REDACTED***`
 * - 长字符串字段截断
 * - 原始值（string/number/boolean）原样返回
 */
function sanitizeArgs(args: unknown): unknown {
  if (args === null || args === undefined) {
    return args;
  }
  if (typeof args !== 'object') {
    return args;
  }
  if (Array.isArray(args)) {
    return args.map((item, idx) => sanitizeValue(`[${idx}]`, sanitizeArgs(item)));
  }
  const sanitized: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(args as Record<string, unknown>)) {
    if (value !== null && typeof value === 'object') {
      // 嵌套对象：先标记是否敏感键，再递归
      if (isSensitiveKey(key)) {
        sanitized[key] = '***REDACTED***';
      } else {
        sanitized[key] = sanitizeArgs(value);
      }
    } else {
      sanitized[key] = sanitizeValue(key, value);
    }
  }
  return sanitized;
}

// ============================================================================
// 认证入口豁免清单（安全审计修复：1005 自动登出拦截的例外）
//
// 设计依据：
// - 后端 870 个 #[tauri::command] 入口中，11 个为「认证入口本身」或「启动期必需」，
//   不调用 require_auth（见 `安全审计报告/2026-07-25-代码安全审计报告.md` 附录二）。
// - 前端 IPC 拦截器在收到 code=1005 时会自动登出并跳转登录页。若不豁免认证入口本身，
//   会导致登录页/启动期会话恢复被拦截器误判为"token 失效"而自踢。
//
// 豁免范围（严格对齐后端 `auth_commands.rs` 无 require_auth 的入口 + perf 命令）：
//   1. 登录/注册/恢复入口：login / register / create_temp_account / recover_by_phrase
//   2. 2FA 登录入口：login_2fa / auth_2fa_login_verify
//   3. 会话恢复入口（启动期调用）：auth_verify_token / auth_restore_session
//   4. 启动期性能记录：record_perf_metric（设计要求登录前记录启动耗时）
//
// 注意：logout / auth_get_permissions / auth_reset_password / session_* /
//       auth_2fa_setup / auth_2fa_verify / auth_2fa_enable / auth_2fa_disable /
//       auth_2fa_status 等命令均需登录后调用，token 失效时应正常触发自动登出，
//       故不豁免。
// ============================================================================
const AUTH_COMMAND_ALLOWLIST = new Set<string>([
  'login',
  'register',
  'create_temp_account',
  'recover_by_phrase',
  'login_2fa',
  'auth_2fa_login_verify',
  'auth_verify_token',
  'auth_restore_session',
  'record_perf_metric',
]);

/**
 * 判断命令是否为认证入口（豁免 1005 自动登出拦截）
 *
 * @param command Tauri 命令名
 * @returns true 表示该命令不应触发自动登出（认证入口本身或启动期必需）
 */
function isAuthCommand(command: string): boolean {
  return AUTH_COMMAND_ALLOWLIST.has(command);
}

// ============================================================================
// IPC LRU 缓存层（A1 §2.6.2）
// 规范：功能展望/平台级增强/01_性能优化_首屏200ms计划.md §2.6.2
//
// 功能：
// - TTL 30s 自动过期
// - LRU 淘汰策略（容量 100）
// - 仅对 get_*/list_*/search_*/count_*/kb_get_*/game_get_*/ai_get_*/auth_get_*
//   等读取类命令自动启用
// - 写操作（add_/update_/delete_/set_/create_/remove_/save_/toggle_/record_ 等）
//   自动清空缓存（保守策略，避免脏数据）
// - Mock 模式不启用缓存（保证 Mock 数据灵活性）
// ============================================================================

const IPC_CACHE_TTL_MS = 30_000; // 30 秒
const IPC_CACHE_MAX_SIZE = 100;  // 最多 100 条

interface CacheEntry {
  value: ApiResponse<any>;
  expireAt: number; // 过期时间戳（毫秒）
}

class IPCache {
  private cache = new Map<string, CacheEntry>();
  private maxSize: number;
  private ttlMs: number;

  constructor(maxSize: number = IPC_CACHE_MAX_SIZE, ttlMs: number = IPC_CACHE_TTL_MS) {
    this.maxSize = maxSize;
    this.ttlMs = ttlMs;
  }

  /** 生成缓存 key：command:JSON.stringify(args) */
  static buildKey(command: string, args?: any): string {
    if (!args || (typeof args === 'object' && Object.keys(args).length === 0)) {
      return command;
    }
    return `${command}:${JSON.stringify(args)}`;
  }

  /** 判断命令是否可缓存（仅读取类命令） */
  static isCacheable(command: string): boolean {
    return /^(get_|list_|search_|count_|kb_get_|game_get_|game_story_get|game_story_list|ai_get_|auth_get_|profile_get_|perf_get_|yuan_get_|yuan_list_|yuan_search_|sync_get_|sync_list_|sync_fetch_pending)/.test(command);
  }

  /** 判断命令是否为写操作（需要清空缓存） */
  static isWriteOp(command: string): boolean {
    return /^(add_|update_|delete_|set_|create_|remove_|save_|toggle_|record_|export_|branch_|reorder_|move_|start_|submit_|upgrade_|import_|batch_|clear_|cleanup_|restore_|recycle_|mark_|send_|run_|kb_add_|kb_delete_|kb_update_|kb_move_|kb_toggle_|kb_record_|game_start_|game_submit_|game_remove_|game_upgrade_|game_move_|game_story_generate|game_story_advance|ai_add_|ai_update_|ai_delete_|ai_send_|ai_run_|sync_enqueue|sync_mark_|sync_register_device|sync_unregister_device|sync_resolve_conflict|sync_run_once|sync_e2ee_)/.test(command);
  }

  /** 获取缓存值（不存在或已过期返回 null，命中时更新 LRU 顺序） */
  get(key: string): ApiResponse<any> | null {
    const entry = this.cache.get(key);
    if (!entry) return null;
    if (Date.now() > entry.expireAt) {
      this.cache.delete(key);
      return null;
    }
    // LRU：删除后重新插入到 Map 末尾（Map 保持插入顺序，最早在头部）
    this.cache.delete(key);
    this.cache.set(key, entry);
    return entry.value;
  }

  /** 写入缓存（超过容量时淘汰最早的） */
  set(key: string, value: ApiResponse<any>): void {
    if (this.cache.size >= this.maxSize) {
      const oldestKey = this.cache.keys().next().value;
      if (oldestKey !== undefined) {
        this.cache.delete(oldestKey);
      }
    }
    this.cache.set(key, {
      value,
      expireAt: Date.now() + this.ttlMs,
    });
  }

  /** 失效指定 key */
  invalidate(key: string): void {
    this.cache.delete(key);
  }

  /** 失效匹配前缀的所有 key（例：invalidatePattern('get_kb_entries') 失效 'get_kb_entries:...' 等） */
  invalidatePattern(prefix: string): void {
    for (const key of Array.from(this.cache.keys())) {
      if (key === prefix || key.startsWith(`${prefix}:`)) {
        this.cache.delete(key);
      }
    }
  }

  /** 清空全部缓存 */
  clear(): void {
    this.cache.clear();
  }

  /** 当前缓存条数 */
  size(): number {
    return this.cache.size;
  }
}

class IPCService {
  private useMock: boolean;
  private cache: IPCache;

  constructor(useMock: boolean = USE_MOCK) {
    this.useMock = useMock;
    this.cache = new IPCache();
    if (useMock) {
      console.log('[IPC] 🎭 Mock模式已启用 - 使用模拟数据');
    } else {
      console.log('[IPC] 🔗 真实模式已启用 - 连接后端');
    }
  }

  /**
   * 统一调用接口
   * @param command 命令名 (必须与后端 Tauri Command 函数名一致)
   * @param args 参数对象
   * @param options.forceRefresh 强制刷新（绕过缓存，默认 false）
   * @returns Promise<ApiResponse<T>>
   */
  async invoke<T = any>(command: string, args?: any, options?: { forceRefresh?: boolean }): Promise<ApiResponse<T>> {
    const startTime = Date.now();
    const useCache = IPCache.isCacheable(command) && !options?.forceRefresh && !this.useMock;

    // 1. 缓存命中检查（仅读操作 + 真实模式 + 未强制刷新）
    if (useCache) {
      const cacheKey = IPCache.buildKey(command, args);
      const cached = this.cache.get(cacheKey);
      if (cached) {
        console.log(`[IPC] 💾 缓存命中: ${command} (${Date.now() - startTime}ms)`);
        return cached as ApiResponse<T>;
      }
    }

    console.log(`[IPC] 🚀 开始调用: ${command}`, sanitizeArgs(args));
    try {
      let result: ApiResponse<T>;
      if (this.useMock) {
        console.log(`[IPC] 🎭 使用 Mock 模式: ${command}`);
        result = await this.invokeMock<T>(command, args);
      } else {
        console.log(`[IPC] 🔗 使用 Tauri 真实模式: ${command}`);
        result = await this.invokeTauri<T>(command, args);
      }
      const duration = Date.now() - startTime;
      this.logCommand(command, duration, result.code === 0);
      console.log(`[IPC] ✅ 调用完成: ${command}`, {
        code: result.code,
        message: result.message,
        hasData: !!result.data
      });

      // 2. 写操作清空缓存（保守策略，避免脏数据）
      if (IPCache.isWriteOp(command) && this.cache.size() > 0) {
        const invalidated = this.cache.size();
        this.cache.clear();
        console.log(`[IPC] 🧹 写操作 ${command} 触发缓存清空（${invalidated} 条）`);
      }

      // 3. 读操作成功结果写入缓存
      if (useCache && result.code === 0) {
        const cacheKey = IPCache.buildKey(command, args);
        this.cache.set(cacheKey, result);
      }

      // C2.5：错误码 i18n 化 — 后端返回业务错误码(>0)时，查找本地化消息替换
      if (result.code > 0) {
        result.message = getLocalizedErrorMessage(result.code, result.message, t);
      }

      // 安全审计修复：认证失败统一拦截（code=1005）
      // 当 require_auth 失败（token 过期/无效/会话失效）时，自动登出并跳转登录页，
      // 避免用户卡在错误状态。例外：auth_commands 自身（login/register/restoreSession 等）
      // 不触发自动登出，否则会导致登录页本身被踢出。
      if (result.code === 1005 && !isAuthCommand(command)) {
        console.warn('[IPC] 🔐 认证失败，自动登出并跳转登录页');
        // 动态导入避免循环依赖
        import('../../stores/authStore').then(({ useAuthStore }) => {
          useAuthStore.getState().logout();
        }).catch((e) => {
          console.error('[IPC] 自动登出失败:', e);
        });
      }

      return result;
    } catch (error) {
      const duration = Date.now() - startTime;
      console.error(`[IPC] ❌ 命令执行失败 [${command}] (${duration}ms):`, error);
      console.error(`[IPC] ❌ 错误详情:`, error instanceof Error ? error.stack : error);
      const errorResponse: ApiResponse<T> = {
        code: -1,
        message: t("lib.ipc.k1", {
          arg0: error instanceof Error ? error.message : String(error)
        }),
        data: undefined as T
      };
      console.log(`[IPC] ⚠️ 返回错误响应:`, errorResponse);
      return errorResponse;
    }
  }

  // ===== 缓存控制 API（A1 §2.6.2）=====

  /** 失效指定命令 + 参数组合的缓存 */
  invalidateCache(command: string, args?: any): void {
    this.cache.invalidate(IPCache.buildKey(command, args));
  }

  /** 失效匹配前缀的所有缓存（例：invalidatePattern('get_kb_entries') 失效 'get_kb_entries:...' 等） */
  invalidateCachePattern(prefix: string): void {
    this.cache.invalidatePattern(prefix);
  }

  /** 清空全部 IPC 缓存 */
  clearCache(): void {
    this.cache.clear();
  }

  /** 获取当前缓存条数（调试用） */
  getCacheSize(): number {
    return this.cache.size();
  }

  /**
   * Mock 模式调用
   */
  private async invokeMock<T>(command: string, args: any): Promise<ApiResponse<T>> {
    const moduleName = this.getMockModule(command);
    const handler = (ipcMockHandlers as Record<string, any>)[moduleName];
    if (!handler) {
      console.warn(`[IPC] ⚠️ 未找到 Mock 处理器: ${command}, 返回空响应`);
      return {
        code: 0,
        message: t("lib.ipc.k2"),
        data: undefined as T
      };
    }
    const methodName = this.getMockMethodName(command);
    const methodFn = (handler as any)[methodName];
    if (typeof methodFn !== 'function') {
      console.warn(`[IPC] ⚠️ Mock 处理器缺少方法: ${moduleName}.${methodName}`);
      return {
        code: 0,
        message: t("lib.ipc.k2"),
        data: undefined as T
      };
    }
    return await methodFn.call(handler, args || {});
  }

  /**
   * 从命令名推断 Mock 模块名
   */
  private getMockModule(command: string): string {
    const mapping: Record<string, string> = {
      'login': 'auth',
      'register': 'auth',
      'logout': 'auth',
      'recover_by_phrase': 'auth',
      'auth_verify_token': 'auth',
      'auth_get_permissions': 'auth',
      'auth_reset_password': 'auth',
      'get_todos': 'home',
      'add_todo': 'home',
      'toggle_todo': 'home',
      'delete_todo': 'home',
      'get_news': 'home',
      'get_news_by_category': 'home',
      'fetch_news': 'home',
      'add_news': 'home',
      'mark_news_read': 'home',
      'clear_old_news': 'home',
      'get_journal': 'home',
      'save_journal': 'home',
      'delete_journal': 'home',
      'get_timers': 'home',
      'create_timer': 'home',
      'update_timer_state': 'home',
      'delete_timer': 'home',
      'timer_action': 'home',
      'get_ai_models': 'ai',
      'add_ai_model': 'ai',
      'update_ai_model': 'ai',
      'delete_ai_model': 'ai',
      'get_ai_agents': 'ai',
      'add_ai_agent': 'ai',
      'delete_ai_agent': 'ai',
      'get_conversations': 'ai',
      'get_messages': 'ai',
      'get_participants': 'ai',
      'send_message': 'ai',
      'run_orchestrator': 'ai',
      'ai_get_orchestration_status': 'ai',
      'ai_end_group_chat': 'ai',
      'get_kb_categories': 'knowledge',
      'add_kb_category': 'knowledge',
      'delete_kb_category': 'knowledge',
      'get_kb_entries': 'knowledge',
      'add_kb_entry': 'knowledge',
      'delete_kb_entry': 'knowledge',
      'search_kb_entries': 'knowledge',
      'kb_import_folder': 'knowledge',
      'kb_import_multi_folders': 'knowledge',
      'get_kb_category_counts': 'knowledge',
      'get_kb_tags': 'knowledge',
      'add_kb_tag': 'knowledge',
      'update_kb_tag': 'knowledge',
      'delete_kb_tag': 'knowledge',
      'get_kb_tag_stats': 'knowledge',
      'get_kb_entry_tags': 'knowledge',
      'set_kb_entry_tags': 'knowledge',
      'get_kb_entries_by_tag': 'knowledge',
      'toggle_kb_favorite': 'knowledge',
      'get_kb_favorites': 'knowledge',
      'record_kb_access': 'knowledge',
      'get_kb_recent': 'knowledge',
      'batch_delete_kb_entries': 'knowledge',
      'batch_move_kb_entries': 'knowledge',
      'batch_add_kb_tag': 'knowledge',
      'batch_remove_kb_tag': 'knowledge',
      'kb_add_tracked_path': 'knowledge',
      'kb_get_tracked_paths': 'knowledge',
      'kb_remove_tracked_path': 'knowledge',
      'kb_add_scanned_files': 'knowledge',
      'kb_scan_directory': 'knowledge',
      'kb_check_paths': 'knowledge',
      'kb_get_templates': 'knowledge',
      'kb_create_template': 'knowledge',
      'kb_update_template': 'knowledge',
      'kb_delete_template': 'knowledge',
      'kb_get_backlinks': 'knowledge',
      'kb_get_outgoing_links': 'knowledge',
      'kb_get_snapshots': 'knowledge',
      'kb_restore_snapshot': 'knowledge',
      'intelligence_v4_kb_classify': 'intelligence',
      'intelligence_v4_log_activity': 'intelligence',
      'intelligence_v4_batch_log_activity': 'intelligence',
      'intelligence_v4_query_activity_logs': 'intelligence',
      'intelligence_v4_activity_stats': 'intelligence',
      'intelligence_v4_clean_activity_logs': 'intelligence',
      'intelligence_v4_dashboard': 'intelligence',
      'intelligence_v4_realtime_stats': 'intelligence',
      'intelligence_v4_generate_suggestions': 'intelligence',
      'intelligence_v4_get_suggestions': 'intelligence',
      'intelligence_v4_mark_suggestion': 'intelligence',
      'intelligence_v4_clean_suggestions': 'intelligence',
      'intelligence_v4_analyze_behavior': 'intelligence',
      'intelligence_v4_behavior_trend': 'intelligence',
      'intelligence_v4_behavior_history': 'intelligence',
      'intelligence_v4_get_settings': 'intelligence',
      'intelligence_v4_save_settings': 'intelligence',
      'intelligence_v4_reset_settings': 'intelligence',
      'intelligence_v4_export_activity_logs': 'intelligence',
      'intelligence_v4_get_operation_templates': 'intelligence',
      'intelligence_v4_log_with_template': 'intelligence',
      'intelligence_v4_resume_spell_check': 'intelligence',
      'intelligence_v4_resume_polish': 'intelligence',
      'intelligence_v4_resume_generate': 'intelligence',
      'intelligence_v4_quote_spell_check': 'intelligence',
      'intelligence_v4_quote_source_verify': 'intelligence',
      'intelligence_v4_quote_smart_complete': 'intelligence',
      'intelligence_v4_news_summarize': 'intelligence',
      'intelligence_v4_todo_enhance': 'intelligence',
      'intelligence_v4_journal_fill': 'intelligence',
      'intelligence_v4_timer_remind': 'intelligence',
      'intelligence_v4_terminal_complete': 'intelligence',
      'intelligence_v4_game_recommend': 'intelligence',
      'intelligence_v4_search_analyze': 'intelligence',
      'intelligence_get_enabled': 'intelligence',
      'intelligence_set_enabled': 'intelligence',
      'intelligence_trigger_proactive_actions': 'intelligence',
      'recycle_list': 'recycle',
      'recycle_move_to': 'recycle',
      'recycle_restore': 'recycle',
      'recycle_delete_permanently': 'recycle',
      'recycle_empty_all': 'recycle',
      'cleanup_expired_recycle': 'recycle',
      'terminal_create_session': 'terminal',
      'terminal_write_input': 'terminal',
      'terminal_resize': 'terminal',
      'terminal_kill_session': 'terminal',
      'get_profile': 'profile',
      'set_profile': 'profile',
      'get_resumes': 'profile',
      'add_resume': 'profile',
      'update_resume': 'profile',
      'delete_resume': 'profile',
      'get_random_quote': 'profile',
      'add_quote': 'profile',
      'get_all_quotes': 'profile',
      'batch_add_quotes': 'profile',
      'seed_default_quotes': 'profile',
      'profile_change_password': 'profile',
      'profile_change_username': 'profile',
      'profile_update_profile': 'profile',
      'save_personal_info': 'profile',
      'get_personal_info': 'profile',
      'create_temp_account': 'profile',
      'get_news_sources': 'profile',
      'add_news_source': 'profile',
      'delete_news_source': 'profile',
      'get_system_config': 'system',
      'set_system_config': 'system',
      'get_all_system_configs': 'system',
      'system_open_file': 'system',
      'system_open_url': 'system',
      'system_get_app_info': 'system',
      'extension_get_entry': 'extension',
      'extension_list_modules': 'extension',
      'yuan_list_files': 'yuanCode',
      'yuan_read_file': 'yuanCode',
      'yuan_write_file': 'yuanCode',
      'yuan_create_item': 'yuanCode',
      'yuan_execute': 'yuanCode',
      'yuan_io_cancel': 'yuanCode',
      'yuan_io_kill': 'yuanCode',
      'yuan_io_stdin': 'yuanCode',
      'yuan_agent_deploy': 'yuanCode',
      'yuan_agent_execute': 'yuanCode',
      'yuan_sandbox_save': 'yuanCode',
      'yuan_skill_export': 'yuanCode',
      'yuan_settings_save': 'yuanCode',
      'yuan_delete_item': 'yuanCode',
      'yuan_rename_item': 'yuanCode',
      'yuan_highlight': 'yuanCode',
      'yuan_complete': 'yuanCode',
      'yuan_complete_stream': 'yuanCode',
      'yuan_analyze': 'yuanCode',
      'yuan_save_snippet': 'yuanCode',
      'yuan_get_snippets': 'yuanCode',
      'yuan_update_snippet': 'yuanCode',
      'yuan_delete_snippet': 'yuanCode',
      'yuan_search_snippets': 'yuanCode',
      'yuan_compute_diff': 'yuanCode',
      'yuan_replace_files': 'yuanCode',
      'yuan_copy_move': 'yuanCode',
      'yuan_get_file_info': 'yuanCode',
      'yuan_save_workspace': 'yuanCode',
      'yuan_load_workspace': 'yuanCode',
      'yuan_list_workspaces': 'yuanCode',
      'yuan_delete_workspace': 'yuanCode',
      'yuan_prompt_list_templates': 'yuanCode',
      'yuan_prompt_get_template': 'yuanCode',
      'yuan_prompt_render': 'yuanCode',
      'yuan_prompt_set_custom_template': 'yuanCode',
      'yuan_prompt_remove_custom_template': 'yuanCode',
      'yuan_prompt_set_variable_default': 'yuanCode',
      'yuan_agents_discover': 'yuanCode',
      'yuan_agents_sources': 'yuanCode',
      'yuan_agents_assemble': 'yuanCode',
      'yuan_agents_set_max_bytes': 'yuanCode',
      'yuan_agents_get_max_bytes': 'yuanCode',
      'yuan_prompt_assemble': 'yuanCode',
      // D1 v3.1 Task 3.5: 云端 API Key 管理
      'cloud_api_list': 'cloudApi',
      'cloud_api_upsert': 'cloudApi',
      'cloud_api_set_enabled': 'cloudApi',
      'cloud_api_delete': 'cloudApi',
      'cloud_api_providers': 'cloudApi',
      'cloud_api_test_connection': 'cloudApi',
      // D1 v3.1 Task 3.1: Agent 化（7 种类型 + Plan/Execute/Review）
      'yuan_v3_agent_types': 'agentV3',
      'yuan_v3_agent_create': 'agentV3',
      'yuan_v3_agent_plan': 'agentV3',
      'yuan_v3_agent_execute': 'agentV3',
      'yuan_v3_agent_review': 'agentV3',
      'yuan_v3_agent_safety_check': 'agentV3',
      'yuan_v3_agent_status': 'agentV3',
      'yuan_v3_agent_list': 'agentV3',
      'yuan_v3_agent_destroy': 'agentV3',
      // D1 v3.1 Task 3.2: MCP 生态（内置服务器 + 调用历史）
      'yuan_mcp_list_builtin_servers': 'mcp',
      'yuan_mcp_list_builtin_tools': 'mcp',
      'yuan_mcp_call_builtin_tool': 'mcp',
      'yuan_mcp_configure_builtin': 'mcp',
      'yuan_mcp_list_call_history': 'mcp',
      // A5 离线与同步机制（Phase 1-4）
      'sync_enqueue': 'sync',
      'sync_fetch_pending': 'sync',
      'sync_mark_synced': 'sync',
      'sync_mark_failed': 'sync',
      'sync_get_queue_stats': 'sync',
      'sync_register_device': 'sync',
      'sync_list_devices': 'sync',
      'sync_unregister_device': 'sync',
      'sync_test_transport': 'sync',
      'sync_run_once': 'sync',
      'sync_get_status': 'sync',
      'sync_list_conflicts': 'sync',
      'sync_resolve_conflict': 'sync',
      'sync_ecdh_generate_keypair': 'sync',
      'sync_e2ee_encrypt': 'sync',
      'sync_e2ee_validate': 'sync',
      'sync_get_network_status': 'sync',
      'sync_check_network_now': 'sync',
      // D1 v3.2 Task 3.4.1: 模型路由配置（按任务类型选模型，强制云端 API 用于编程）
      'model_routing_list': 'modelRouting',
      'model_routing_upsert': 'modelRouting',
      'model_routing_set_enabled': 'modelRouting',
      'model_routing_delete': 'modelRouting',
      'model_routing_resolve': 'modelRouting',
      'model_routing_task_types': 'modelRouting',
      // D1 v3.2 Task 3.4.2: Yuan Code 底层智能监测（非侵入式、可关闭）
      'yuan_code_monitor_record_event': 'yuanCodeMonitor',
      'yuan_code_monitor_status': 'yuanCodeMonitor',
      'yuan_code_monitor_summary': 'yuanCodeMonitor',
      // D1 v3.2 Task 3.4.3 / 3.4.4: 协作会话管理（Yjs 接口骨架 + 多人协作）
      'collab_session_create': 'collabSession',
      'collab_session_list': 'collabSession',
      'collab_session_get': 'collabSession',
      'collab_session_join': 'collabSession',
      'collab_session_leave': 'collabSession',
      'collab_session_close': 'collabSession',
      'collab_session_update_cursor': 'collabSession'
    };
    return mapping[command] || command.split('_')[0];
  }

  /**
   * 从命令名推断 Mock 方法名
   */
  private getMockMethodName(command: string): string {
    const mapping: Record<string, string> = {
      'login': 'login',
      'register': 'register',
      'logout': 'logout',
      'recover_by_phrase': 'verifyRecoveryPhrase',
      'auth_verify_token': 'verifyToken',
      'auth_get_permissions': 'getPermissions',
      'auth_reset_password': 'resetPassword',
      'get_todos': 'getTodos',
      'add_todo': 'createTodo',
      'toggle_todo': 'updateTodo',
      'delete_todo': 'deleteTodo',
      'get_news': 'getNews',
      'get_news_by_category': 'getNewsByCategory',
      'fetch_news': 'fetchNews',
      'add_news': 'addNews',
      'mark_news_read': 'markNewsRead',
      'clear_old_news': 'clearOldNews',
      'get_journal': 'getLogs',
      'save_journal': 'saveLog',
      'delete_journal': 'deleteLog',
      'get_timers': 'getTimers',
      'create_timer': 'createTimer',
      'update_timer_state': 'updateTimerState',
      'delete_timer': 'deleteTimer',
      'timer_action': 'timerAction',
      'get_ai_models': 'getModels',
      'add_ai_model': 'createModel',
      'update_ai_model': 'updateModel',
      'delete_ai_model': 'deleteModel',
      'get_ai_agents': 'getAgents',
      'add_ai_agent': 'createAgent',
      'delete_ai_agent': 'deleteAgent',
      'get_conversations': 'getSessions',
      'get_messages': 'getMessages',
      'get_participants': 'getParticipants',
      'send_message': 'sendMessage',
      'run_orchestrator': 'startGroupChat',
      'ai_get_orchestration_status': 'getOrchestrationStatus',
      'ai_end_group_chat': 'endGroupChat',
      'get_kb_categories': 'getCategories',
      'add_kb_category': 'addCategory',
      'delete_kb_category': 'deleteCategory',
      'get_kb_entries': 'getItems',
      'add_kb_entry': 'createItem',
      'delete_kb_entry': 'deleteItem',
      'search_kb_entries': 'search',
      'kb_import_folder': 'importFolder',
      'kb_import_multi_folders': 'importFolders',
      'get_kb_category_counts': 'getCategoryCounts',
      'get_kb_tags': 'getTags',
      'add_kb_tag': 'addTag',
      'update_kb_tag': 'updateTag',
      'delete_kb_tag': 'deleteTag',
      'get_kb_tag_stats': 'getTagStats',
      'get_kb_entry_tags': 'getEntryTags',
      'set_kb_entry_tags': 'setEntryTags',
      'get_kb_entries_by_tag': 'getEntriesByTag',
      'toggle_kb_favorite': 'toggleFavorite',
      'get_kb_favorites': 'getFavorites',
      'record_kb_access': 'recordAccess',
      'get_kb_recent': 'getRecent',
      'batch_delete_kb_entries': 'batchDeleteEntries',
      'batch_move_kb_entries': 'batchMoveEntries',
      'batch_add_kb_tag': 'batchAddTag',
      'batch_remove_kb_tag': 'batchRemoveTag',
      'kb_add_tracked_path': 'addTrackedPath',
      'kb_get_tracked_paths': 'getTrackedPaths',
      'kb_remove_tracked_path': 'removeTrackedPath',
      'kb_add_scanned_files': 'addScannedFiles',
      'kb_scan_directory': 'scanDirectory',
      'kb_check_paths': 'checkPaths',
      'kb_get_templates': 'getTemplates',
      'kb_create_template': 'createTemplate',
      'kb_update_template': 'updateTemplate',
      'kb_delete_template': 'deleteTemplate',
      'kb_get_backlinks': 'getBacklinks',
      'kb_get_outgoing_links': 'getOutgoingLinks',
      'kb_get_snapshots': 'getSnapshots',
      'kb_restore_snapshot': 'restoreSnapshot',
      'intelligence_v4_kb_classify': 'classifyKbEntry',
      'intelligence_v4_kb_summarize': 'kbSummarize',
      'intelligence_v4_kb_tags': 'kbTags',
      'intelligence_v4_log_activity': 'logActivity',
      'intelligence_v4_batch_log_activity': 'batchLogActivity',
      'intelligence_v4_query_activity_logs': 'queryActivityLogs',
      'intelligence_v4_activity_stats': 'getActivityStats',
      'intelligence_v4_clean_activity_logs': 'cleanActivityLogs',
      'intelligence_v4_clear_all_activity_logs': 'clearAllActivityLogs',
      'intelligence_v4_dashboard': 'getDashboard',
      'intelligence_v4_realtime_stats': 'getRealtimeStats',
      'intelligence_v4_generate_suggestions': 'generateSuggestions',
      'intelligence_v4_get_suggestions': 'getSuggestions',
      'intelligence_v4_mark_suggestion': 'markSuggestion',
      'intelligence_v4_clean_suggestions': 'cleanSuggestions',
      'intelligence_v4_analyze_behavior': 'analyzeBehavior',
      'intelligence_v4_behavior_trend': 'getBehaviorTrend',
      'intelligence_v4_behavior_history': 'getBehaviorHistory',
      'intelligence_v4_get_settings': 'getSettings',
      'intelligence_v4_save_settings': 'saveSettings',
      'intelligence_v4_reset_settings': 'resetSettings',
      'intelligence_v4_export_activity_logs': 'exportActivityLogs',
      'intelligence_v4_get_operation_templates': 'getOperationTemplates',
      'intelligence_v4_log_with_template': 'logWithTemplate',
      'intelligence_v4_resume_spell_check': 'resumeSpellCheck',
      'intelligence_v4_resume_polish': 'resumePolish',
      'intelligence_v4_resume_generate': 'resumeGenerate',
      'intelligence_v4_quote_spell_check': 'quoteSpellCheck',
      'intelligence_v4_quote_source_verify': 'quoteSourceVerify',
      'intelligence_v4_quote_smart_complete': 'quoteSmartComplete',
      'intelligence_v4_news_summarize': 'newsSummarize',
      'intelligence_v4_todo_enhance': 'todoEnhance',
      'intelligence_v4_journal_fill': 'journalFill',
      'intelligence_v4_timer_remind': 'timerRemind',
      'intelligence_v4_terminal_complete': 'terminalComplete',
      'intelligence_v4_game_recommend': 'gameRecommend',
      'intelligence_v4_search_analyze': 'searchAnalyze',
      'get_recycle_items': 'getItems',
      'move_to_recycle': 'moveToRecycle',
      'restore_recycle_item': 'restore',
      'delete_recycle_permanently': 'permanentDelete',
      'cleanup_expired_recycle': 'cleanupExpired',
      'clear_recycle_bin': 'empty',
      'terminal_create_session': 'createSession',
      'terminal_write_input': 'executeCommand',
      'terminal_resize': 'resize',
      'terminal_kill_session': 'closeSession',
      'get_profile': 'getProfile',
      'set_profile': 'setProfile',
      'get_resumes': 'getResumes',
      'add_resume': 'addResume',
      'update_resume': 'updateResume',
      'delete_resume': 'deleteResume',
      'get_random_quote': 'getRandomQuote',
      'add_quote': 'addQuote',
      'get_all_quotes': 'getAllQuotes',
      'batch_add_quotes': 'batchAddQuotes',
      'seed_default_quotes': 'seedDefaultQuotes',
      'delete_quote': 'deleteQuote',
      'profile_change_password': 'changePassword',
      'profile_change_username': 'changeUsername',
      'profile_update_profile': 'updateProfile',
      'save_personal_info': 'savePersonalInfo',
      'get_personal_info': 'getPersonalInfo',
      'create_temp_account': 'createTempAccount',
      'get_news_sources': 'getNewsSources',
      'add_news_source': 'addNewsSource',
      'delete_news_source': 'deleteNewsSource',
      'get_system_config': 'getInfo',
      'set_system_config': 'setConfig',
      'get_all_system_configs': 'getAllConfigs',
      'system_open_file': 'openFile',
      'system_open_url': 'openUrl',
      'system_get_app_info': 'getAppInfo',
      'extension_get_entry': 'getEntry',
      'extension_list_modules': 'listModules',
      'yuan_list_files': 'listFiles',
      'yuan_read_file': 'readFile',
      'yuan_write_file': 'writeFile',
      'yuan_create_item': 'createItem',
      'yuan_execute': 'execute',
      'yuan_io_cancel': 'cancelExecution',
      'yuan_io_kill': 'killExecution',
      'yuan_io_stdin': 'sendStdin',
      'yuan_agent_deploy': 'deployAgent',
      'yuan_agent_execute': 'agentExecute',
      'yuan_sandbox_save': 'saveSandboxConfig',
      'yuan_skill_export': 'exportSkills',
      'yuan_settings_save': 'saveSettings',
      'yuan_delete_item': 'deleteItem',
      'yuan_rename_item': 'renameItem',
      'yuan_highlight': 'highlight',
      'yuan_complete': 'complete',
      'yuan_complete_stream': 'completeStream',
      'yuan_analyze': 'analyze',
      'yuan_save_snippet': 'saveSnippet',
      'yuan_get_snippets': 'getSnippets',
      'yuan_update_snippet': 'updateSnippet',
      'yuan_delete_snippet': 'deleteSnippet',
      'yuan_search_snippets': 'searchSnippets',
      'yuan_compute_diff': 'computeDiff',
      'yuan_replace_files': 'replaceFiles',
      'yuan_copy_move': 'copyMove',
      'yuan_get_file_info': 'getFileInfo',
      'yuan_save_workspace': 'saveWorkspace',
      'yuan_load_workspace': 'loadWorkspace',
      'yuan_list_workspaces': 'listWorkspaces',
      'yuan_delete_workspace': 'deleteWorkspace',
      'yuan_prompt_list_templates': 'listTemplates',
      'yuan_prompt_get_template': 'getTemplate',
      'yuan_prompt_render': 'renderTemplate',
      'yuan_prompt_set_custom_template': 'setCustomTemplate',
      'yuan_prompt_remove_custom_template': 'removeCustomTemplate',
      'yuan_prompt_set_variable_default': 'setVariableDefault',
      'yuan_agents_discover': 'discoverAgentsMd',
      'yuan_agents_sources': 'getInstructionSources',
      'yuan_agents_assemble': 'assembleInstructions',
      'yuan_agents_set_max_bytes': 'setMaxBytes',
      'yuan_agents_get_max_bytes': 'getMaxBytes',
      'yuan_prompt_assemble': 'assembleSystemPrompt',
      // D1 v3.1 Task 3.5: 云端 API Key 管理
      'cloud_api_list': 'list',
      'cloud_api_upsert': 'upsert',
      'cloud_api_set_enabled': 'setEnabled',
      'cloud_api_delete': 'delete',
      'cloud_api_providers': 'providers',
      'cloud_api_test_connection': 'testConnection',
      // D1 v3.1 Task 3.1: Agent 化
      'yuan_v3_agent_types': 'types',
      'yuan_v3_agent_create': 'create',
      'yuan_v3_agent_plan': 'plan',
      'yuan_v3_agent_execute': 'execute',
      'yuan_v3_agent_review': 'review',
      'yuan_v3_agent_safety_check': 'safetyCheck',
      'yuan_v3_agent_status': 'status',
      'yuan_v3_agent_list': 'list',
      'yuan_v3_agent_destroy': 'destroy',
      // D1 v3.1 Task 3.2: MCP 生态（内置服务器 + 调用历史）
      'yuan_mcp_list_builtin_servers': 'listBuiltinServers',
      'yuan_mcp_list_builtin_tools': 'listBuiltinTools',
      'yuan_mcp_call_builtin_tool': 'callBuiltinTool',
      'yuan_mcp_configure_builtin': 'configureBuiltin',
      'yuan_mcp_list_call_history': 'listCallHistory',
      // A5 离线与同步机制（Phase 1-4）
      'sync_enqueue': 'enqueue',
      'sync_fetch_pending': 'fetchPending',
      'sync_mark_synced': 'markSynced',
      'sync_mark_failed': 'markFailed',
      'sync_get_queue_stats': 'getQueueStats',
      'sync_register_device': 'registerDevice',
      'sync_list_devices': 'listDevices',
      'sync_unregister_device': 'unregisterDevice',
      'sync_test_transport': 'testTransport',
      'sync_run_once': 'runOnce',
      'sync_get_status': 'getStatus',
      'sync_list_conflicts': 'listConflicts',
      'sync_resolve_conflict': 'resolveConflict',
      'sync_ecdh_generate_keypair': 'ecdhGenerateKeypair',
      'sync_e2ee_encrypt': 'e2eeEncrypt',
      'sync_e2ee_validate': 'e2eeValidate',
      'sync_get_network_status': 'getNetworkStatus',
      'sync_check_network_now': 'checkNetworkNow',
      // D1 v3.2 Task 3.4.1: 模型路由配置（按任务类型选模型，强制云端 API 用于编程）
      'model_routing_list': 'list',
      'model_routing_upsert': 'upsert',
      'model_routing_set_enabled': 'setEnabled',
      'model_routing_delete': 'delete',
      'model_routing_resolve': 'resolve',
      'model_routing_task_types': 'taskTypes',
      // D1 v3.2 Task 3.4.2: Yuan Code 底层智能监测（非侵入式、可关闭）
      'yuan_code_monitor_record_event': 'recordEvent',
      'yuan_code_monitor_status': 'status',
      'yuan_code_monitor_summary': 'summary',
      // D1 v3.2 Task 3.4.3 / 3.4.4: 协作会话管理（Yjs 接口骨架 + 多人协作）
      'collab_session_create': 'create',
      'collab_session_list': 'list',
      'collab_session_get': 'get',
      'collab_session_join': 'join',
      'collab_session_leave': 'leave',
      'collab_session_close': 'close',
      'collab_session_update_cursor': 'updateCursor'
    };
    return mapping[command] || command;
  }

  /**
   * Tauri 真实模式调用
   */
  private async invokeTauri<T>(command: string, args: any): Promise<ApiResponse<T>> {
    console.log(`[IPC] 🔍 检测 Tauri 环境...`);
    console.log(`[IPC] 🔍 window.__TAURI__ 存在: ${!!window.__TAURI__}`);
    console.log(`[IPC] 🔍 window.__TAURI_INTERNALS__ 存在: ${!!(window as any).__TAURI_INTERNALS__}`);
    try {
      const {
        invoke
      } = await import('@tauri-apps/api/core');
      console.log(`[IPC] ✅ Tauri API 加载成功, invoke 函数存在: ${!!invoke}`);
      if (!invoke) {
        throw new Error(t("lib.ipc.k3"));
      }
      const result = await invoke<T>(command, args);
      return result as ApiResponse<T>;
    } catch (importError) {
      console.error('[IPC] ❌ Tauri 调用失败:', command, importError);
      throw importError;
    }
  }

  /**
   * 命令日志
   */
  private logCommand(command: string, duration: number, success: boolean) {
    const icon = success ? '✅' : '❌';
    const color = success ? '#00FF00' : '#FF0000';
    console.log(`%c[IPC] ${icon} ${command} (${duration}ms)`, `color: ${color}; font-weight: 600;`);
    if (duration > 1000) {
      console.warn(`[IPC] ⚠️ 慢请求警告: ${command} 耗时 ${duration}ms`);
    }
  }
  setMockMode(enabled: boolean) {
    this.useMock = enabled;
    console.log(`[IPC] 模式切换为: ${enabled ? '🎭 Mock' : t("lib.ipc.k4")}`);
  }
  get isMockMode(): boolean {
    return this.useMock;
  }
}

// 导出单例实例
export const ipc = new IPCService();

// ============================================================
// 便捷方法导出 - 命令名必须与后端 Tauri Command 函数名一致
// ============================================================


export default ipc;
