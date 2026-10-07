import { t } from "i18next";
/**
 * LoginModal (L1 壳) —— 登录/注册弹窗。
 *
 * 认证「一切皆插件」物理迁移后本文件仅保留：弹窗外壳 + 共享表单状态 + 视图分发 + 启停门控。
 * 4 个功能域已物理拆分至 features/<dir>/ 各 L2 插件（login/register/recovery/temp），
 * 各 L2 自持其局部 state + handlers + JSX，经 core（AuthCore）注入共享状态。
 */
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import styles from './LoginModal.module.css';
import type { AuthCore, AuthMode } from './core';
import { useLogin } from './features/login/useLogin';
import { useRegister } from './features/register/useRegister';
import { useRecovery } from './features/recovery/useRecovery';
import { useTemp } from './features/temp/useTemp';

/** mode → 对应子插件 id（用于启停门控：停用时回退必备的 auth.login） */
const MODE_PLUGIN_ID: Record<AuthMode, string> = {
  login: 'auth.login',
  '2fa_verify': 'auth.login',
  register: 'auth.register',
  show_recovery_phrase: 'auth.register',
  verify_recovery: 'auth.recovery',
  reset_password: 'auth.recovery',
  forgot_password: 'auth.recovery',
  temp_login: 'auth.temp'
};

export default function LoginModal({
  onLogin
}: {
  onLogin: (user?: any, token?: string) => void;
}) {
  const [mode, setMode] = useState<AuthMode>('login');

  // 表单状态（各认证视图共享）
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const clearError = () => setError('');
  const goToMode = (m: AuthMode) => setMode(m);

  // ===== 子插件启停门控（参考 profile/Home.tsx；null 为加载中，放行） =====
  const [enabledIds, setEnabledIds] = useState<string[] | null>(null);
  useEffect(() => {
    invoke<Array<{ id: string }>>('plugin:kernel|kernel_dispatch', {
      cmd: 'kernel:plugin:get_enabled',
      args: {}
    }).then((list) => {
      setEnabledIds(list.map((p) => p.id));
    }).catch(() => setEnabledIds(null));
  }, []);
  const isModeEnabled = (m: AuthMode) => enabledIds === null || enabledIds.includes(MODE_PLUGIN_ID[m]);
  // 当前视图对应子插件被停用时回退到必备的 auth.login
  const displayMode: AuthMode = isModeEnabled(mode) ? mode : 'login';

  // ===== 核心共享契约（注入各功能域插件） =====
  const core: AuthCore = {
    username,
    setUsername,
    password,
    setPassword,
    error,
    setError,
    isLoading,
    setIsLoading,
    clearError,
    goToMode,
    onLogin
  };

  // ===== 功能域 hook（各 L2 自持其视图局部 state + handlers） =====
  const login = useLogin(core);
  const register = useRegister(core);
  const recovery = useRecovery(core);
  const temp = useTemp(core);

  const getModeTitle = () => {
    switch (displayMode) {
      case 'login':
        return '';
      case 'register':
        return t("LoginModal.k27");
      case 'show_recovery_phrase':
        return t("LoginModal.k24");
      case 'verify_recovery':
        return t("LoginModal.k68");
      case 'reset_password':
        return t("LoginModal.k54");
      case 'temp_login':
        return t("LoginModal.k69");
      case '2fa_verify':
        return t("LoginModal.k70");
      default:
        return '';
    }
  };

  const showBottomOptions = isModeEnabled('register') || isModeEnabled('verify_recovery') || isModeEnabled('temp_login');
  return <div className={styles.modalOverlay}>
      {/* BUG-026：为 E2E 测试提供稳定定位锚点（E2E选择器规范：testid 优先） */}
      <div className={styles.modalContainer} role="dialog" data-testid="login-modal">
        {displayMode !== 'login' && <h2 className={styles.title}>
            {getModeTitle()}
          </h2>}

        {displayMode === 'login' && login.renderLoginForm()}
        {displayMode === 'register' && register.renderRegisterForm()}
        {displayMode === 'show_recovery_phrase' && register.renderRecoveryPhraseDisplay()}
        {displayMode === 'verify_recovery' && recovery.renderVerifyRecoveryForm()}
        {displayMode === 'reset_password' && recovery.renderResetPasswordForm()}
        {displayMode === 'temp_login' && temp.renderTempLoginForm()}
        {displayMode === '2fa_verify' && login.render2faVerifyForm()}

        {displayMode === 'login' && showBottomOptions && <div className={styles.bottomOptions}>
            {isModeEnabled('register') && <button type="button" onClick={() => {
          goToMode('register');
          clearError();
        }} className={styles.optionLink}>
              {t("LoginModal.k27")}
            </button>}
            {isModeEnabled('register') && isModeEnabled('verify_recovery') && <span className={styles.optionDivider}>•</span>}
            {isModeEnabled('verify_recovery') && <button type="button" onClick={recovery.handleForgotPassword} className={styles.optionLink}>
              {t("LoginModal.k71")}
            </button>}
            {isModeEnabled('verify_recovery') && isModeEnabled('temp_login') && <span className={styles.optionDivider}>•</span>}
            {isModeEnabled('temp_login') && <button type="button" onClick={() => {
          goToMode('temp_login');
          clearError();
        }} className={styles.optionLink}>
              {t("LoginModal.k69")}
            </button>}
          </div>}
      </div>
    </div>;
}
