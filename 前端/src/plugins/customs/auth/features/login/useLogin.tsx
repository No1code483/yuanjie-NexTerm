// auth.login L2 —— 登录功能域（login 表单 + 2FA 校验）。
// 物理迁入本目录：原 L1 LoginModal 的登录视图与 2FA 视图；共享表单状态经 core（AuthCore）注入。
import { t } from "i18next";
import { useState } from 'react';
import { auth } from '../../ipc/auth';
import type { AuthCore } from '../../core';
import styles from '../../LoginModal.module.css';
import ErrorHint from '../../ErrorHint';

export function useLogin(core: AuthCore) {
  // 2FA 状态
  const [twoFactorCode, setTwoFactorCode] = useState('');
  const [twoFactorToken, setTwoFactorToken] = useState('');

  /**
   * 处理登录
   */
  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    core.clearError();
    console.log('🔐 [登录] 开始登录流程', {
      username: core.username,
      hasPassword: !!core.password
    });
    if (!core.username.trim() || !core.password) {
      core.setError(t("LoginModal.k1"));
      return;
    }
    core.setIsLoading(true);
    try {
      console.log('🔐 [登录] 调用 auth.login...', {
        username: core.username
      });
      const response = await auth.login(core.username, core.password);
      console.log('🔐 [登录] 收到响应:', {
        code: response.code,
        message: response.message,
        hasData: !!response.data,
        data: response.data
      });
      if (response.code === 0 && response.data) {
        const u = response.data.user;
        console.log('✅ 登录成功:', u.username, '| 角色:', u.role, '| 永久账号:', u.is_permanent);
        core.onLogin(response.data.user, response.data.token);
      } else if (response.code === 1002 && response.data?.token) {
        // 需要 2FA 验证
        console.log('🔐 需要2FA验证');
        setTwoFactorToken(response.data.token);
        core.goToMode('2fa_verify');
      } else {
        console.error('❌ 登录失败 (服务端返回错误):', {
          code: response.code,
          message: response.message
        });
        core.setError(response.message || t("LoginModal.k2"));
      }
    } catch (err) {
      console.error('❌ 登录异常 (抛出异常):', err);
      core.setError(t("LoginModal.k3", {
        arg0: err instanceof Error ? err.message : t("errors.unknown")
      }));
    } finally {
      core.setIsLoading(false);
    }
  };

  /**
   * 处理 2FA 验证
   */
  const handle2faVerify = async (e: React.FormEvent) => {
    e.preventDefault();
    core.clearError();
    if (!twoFactorCode || twoFactorCode.length !== 6) {
      core.setError(t("LoginModal.k14"));
      return;
    }
    core.setIsLoading(true);
    try {
      const response = await auth.auth2faLoginVerify(twoFactorToken, twoFactorCode);
      if (response.code === 0 && response.data) {
        console.log('✅ 2FA验证通过，登录成功');
        core.onLogin(response.data.user, response.data.token);
      } else {
        core.setError(response.message || t("LoginModal.k15"));
      }
    } catch (err) {
      console.error('❌ 2FA验证异常:', err);
      core.setError(t("LoginModal.k11"));
    } finally {
      core.setIsLoading(false);
    }
  };

  const renderLoginForm = () => <form onSubmit={handleLogin} data-testid="login-form">
      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.username")}</label>
        <input type="text" value={core.username} onChange={e => core.setUsername(e.target.value)} placeholder="" className={styles.input} autoFocus data-testid="login-username-input" />
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.password")}</label>
        <input type="password" value={core.password} onChange={e => core.setPassword(e.target.value)} placeholder="" className={styles.input} onKeyDown={e => e.key === 'Enter' && handleLogin(e)} data-testid="login-password-input" />
      </div>

      {core.error && <ErrorHint message={core.error} />}

      <button type="submit" disabled={core.isLoading} className={styles.primaryButton} data-testid="login-submit-button">
        {core.isLoading ? t("LoginModal.k19") : t("LoginModal.k20")}
      </button>
    </form>;

  const render2faVerifyForm = () => <form onSubmit={handle2faVerify}>
      <div className={styles.warningBox}>
        {t("LoginModal.k64")}
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("LoginModal.k65")}</label>
        <input type="text" value={twoFactorCode} onChange={e => setTwoFactorCode(e.target.value.replace(/\D/g, '').substring(0, 6))} placeholder={t("LoginModal.k66")} className={styles.input} autoFocus maxLength={6} inputMode="numeric" pattern="[0-9]*" style={{
        fontSize: 20,
        letterSpacing: 8,
        textAlign: 'center'
      }} />
      </div>

      {core.error && <ErrorHint message={core.error} />}

      <button type="submit" disabled={core.isLoading || twoFactorCode.length !== 6} className={styles.primaryButton}>
        {core.isLoading ? t("LoginModal.k49") : t("LoginModal.k67")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        core.goToMode('login');
        core.clearError();
        setTwoFactorCode('');
        setTwoFactorToken('');
      }} className={styles.secondaryButton}>
          {t("LoginModal.k51")}
        </button>
      </div>
    </form>;

  return { renderLoginForm, render2faVerifyForm };
}
