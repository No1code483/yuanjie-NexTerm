/**
 * 错误码 i18n 映射（C2.5 / v1.51.8）
 *
 * 将后端 AppError::error_code() 返回的数字错误码映射到 i18n key，
 * 让前端根据错误码显示本地化的错误消息。
 *
 * 用法：
 *   const i18nKey = getErrorI18nKey(1005);
 *   const localizedMsg = i18nKey ? t(i18nKey) : fallbackMessage;
 *
 * 关联文档：功能展望/体验深化/02_多语言切换_i18n体系_未来展望.md §C2.5
 */

/**
 * 错误码 → i18n key 映射表
 *
 * key: 后端 AppError::error_code() 返回的数字
 * value: i18n key（在 errors namespace 下）
 *
 * 后端错误码定义见：后端/src-tauri/src/error/app_error.rs
 */
const ERROR_CODE_TO_I18N_KEY: Record<number, string> = {
  // ===== 认证类（1000-1999）=====
  1002: 'errors.permissionDenied',        // Permission { resource, action }
  1003: 'errors.validationFailed',        // Validation(String)
  1005: 'errors.authFailed',              // Auth(String)
  1006: 'errors.recoveryPhraseInvalid',   // RecoveryPhraseInvalid
  1007: 'errors.tempAccountExpired',      // TempAccountExpired

  // ===== 数据库类（2000-2999）=====
  2001: 'errors.databaseError',           // Database / Conflict
  2002: 'errors.notFound',                // NotFound

  // ===== 文件系统类（3000-3999）=====
  3001: 'errors.fileSystemError',         // FileSystem

  // ===== 终端类（4000-4999）=====
  4001: 'errors.terminalError',           // TerminalError

  // ===== AI 类（5000-5999）=====
  5001: 'errors.aiApiError',              // AiApi
  5003: 'errors.roundLimitReached',       // RoundLimitReached
  5004: 'errors.tokenBudgetExhausted',    // TokenBudgetExhausted
  5005: 'errors.modelTimeout',            // ModelTimeout

  // ===== 加密类（6000-6999）=====
  6001: 'errors.cryptoError',             // Crypto
  6002: 'errors.mekDecryptionFailed',     // MekDecryption

  // ===== 备份类（7000-7999）=====
  7001: 'errors.backupError',             // Backup

  // ===== 内部错误（9000-9999）=====
  9001: 'errors.internalError',           // Internal
};

/**
 * 根据错误码获取 i18n key
 *
 * @param code 后端返回的数字错误码
 * @returns i18n key，若未映射则返回 null
 */
export function getErrorI18nKey(code: number): string | null {
  return ERROR_CODE_TO_I18N_KEY[code] ?? null;
}

/**
 * 根据错误码获取本地化的错误消息
 *
 * @param code 后端返回的数字错误码
 * @param fallbackMessage 后端返回的原始错误消息（作为兜底）
 * @param tFunc i18n 的 t 函数
 * @returns 本地化的错误消息
 */
export function getLocalizedErrorMessage(
  code: number,
  fallbackMessage: string,
  tFunc: (key: string, options?: Record<string, unknown>) => string,
): string {
  const i18nKey = getErrorI18nKey(code);
  if (i18nKey) {
    const translated = tFunc(i18nKey);
    // 如果 i18n 返回的是 key 本身（未翻译），回退到 fallbackMessage
    if (translated && translated !== i18nKey) {
      return translated;
    }
  }
  return fallbackMessage;
}
