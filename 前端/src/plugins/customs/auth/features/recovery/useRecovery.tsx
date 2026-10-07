// auth.recovery L2 —— 找回密码功能域（verify_recovery 校验 + reset_password 重置）。
// 物理迁入本目录：原 L1 LoginModal 的恢复短语验证 / 密码重置视图；共享表单状态经 core（AuthCore）注入。
// 注：forgot_password 模式归此类（唯一入口 handleForgotPassword 由 L1 bottomOptions 调用）。
import { t } from "i18next";
import { useState } from 'react';
import { auth } from '../../ipc/auth';
import type { AuthCore } from '../../core';
import styles from '../../LoginModal.module.css';
import ErrorHint from '../../ErrorHint';

export function useRecovery(core: AuthCore) {
  // 找回密码状态
  const [forgotUsername, setForgotUsername] = useState('');
  const [recoveryInput, setRecoveryInput] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');

  /**
   * 开始忘记密码流程
   */
  const handleForgotPassword = () => {
    core.goToMode('verify_recovery');
    core.clearError();
  };

  /**
   * 验证恢复短语并重置密码 (合并为一步)
   */
  const handleVerifyRecoveryPhrase = async (e: React.FormEvent) => {
    e.preventDefault();
    core.clearError();
    if (!forgotUsername.trim()) {
      core.setError(t("LoginModal.k4"));
      return;
    }
    if (!recoveryInput.trim()) {
      core.setError(t("LoginModal.k6"));
      return;
    }
    const inputWords = recoveryInput.trim().split(/\s+/).filter(w => w.length > 0);
    if (inputWords.length !== 12) {
      core.setError(t("LoginModal.k7"));
      return;
    }
    if (!newPassword || newPassword.length < 6) {
      core.setError(t("LoginModal.k8"));
      return;
    }
    if (newPassword !== confirmPassword) {
      core.setError(t("LoginModal.k9"));
      return;
    }
    core.setIsLoading(true);
    try {
      const response = await auth.verifyRecoveryPhrase(forgotUsername, inputWords, newPassword);
      if (response.code === 0 && response.data) {
        console.log('✅ 恢复短语验证通过，密码已重置');
        core.onLogin(response.data.user, response.data.token);
      } else {
        core.setError(response.message || t("LoginModal.k10"));
      }
    } catch (err) {
      console.error('❌ 验证异常:', err);
      core.setError(t("LoginModal.k11"));
    } finally {
      core.setIsLoading(false);
    }
  };

  /**
   * 重置密码
   */
  const handleResetPassword = async (e: React.FormEvent) => {
    e.preventDefault();
    core.clearError();
    if (!newPassword || newPassword.length < 6) {
      core.setError(t("LoginModal.k8"));
      return;
    }
    if (newPassword !== confirmPassword) {
      core.setError(t("LoginModal.k9"));
      return;
    }
    core.setIsLoading(true);
    try {
      const inputWords = recoveryInput.trim().split(/\s+/).filter(w => w.length > 0);
      const response = await auth.verifyRecoveryPhrase(forgotUsername, inputWords, newPassword);
      if (response.code === 0 && response.data) {
        console.log('✅ 密码重置成功');
        core.onLogin(response.data.user, response.data.token);
      } else {
        core.setError(response.message || t("LoginModal.k12"));
      }
    } catch (err) {
      console.error('❌ 重置异常:', err);
      core.setError(t("LoginModal.k13"));
    } finally {
      core.setIsLoading(false);
    }
  };

  const renderVerifyRecoveryForm = () => <form onSubmit={handleVerifyRecoveryPhrase}>
      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.username")}</label>
        <input type="text" value={forgotUsername} onChange={e => setForgotUsername(e.target.value)} placeholder={t("LoginModal.k40")} className={styles.input} autoFocus />
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("LoginModal.k41")}</label>
        <textarea value={recoveryInput} onChange={e => setRecoveryInput(e.target.value)} placeholder={t("LoginModal.k42")} rows={3} className={styles.textarea} />
        <div className={styles.wordCount}>
          {t("LoginModal.k43")} {recoveryInput.trim().split(/\s+/).filter(w => w).length} {t("LoginModal.k44")}
        </div>
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("LoginModal.k45")}</label>
        <input type="password" value={newPassword} onChange={e => setNewPassword(e.target.value)} placeholder={t("LoginModal.k46")} className={styles.input} />
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("LoginModal.k47")}</label>
        <input type="password" value={confirmPassword} onChange={e => setConfirmPassword(e.target.value)} placeholder={t("LoginModal.k48")} className={styles.input} />
      </div>

      {core.error && <ErrorHint message={core.error} />}

      <button type="submit" disabled={core.isLoading} className={styles.primaryButton}>
        {core.isLoading ? t("LoginModal.k49") : t("LoginModal.k50")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        core.goToMode('login');
        core.clearError();
      }} className={styles.secondaryButton}>
          {t("LoginModal.k51")}
        </button>
      </div>
    </form>;

  const renderResetPasswordForm = () => <form onSubmit={handleResetPassword}>
      <div className={styles.warningBox}>
        {t("LoginModal.k52")}
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("LoginModal.k45")}</label>
        <input type="password" value={newPassword} onChange={e => setNewPassword(e.target.value)} placeholder={t("LoginModal.k46")} className={styles.input} autoFocus />
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("LoginModal.k47")}</label>
        <input type="password" value={confirmPassword} onChange={e => setConfirmPassword(e.target.value)} placeholder={t("LoginModal.k48")} className={styles.input} />
      </div>

      {core.error && <ErrorHint message={core.error} />}

      <button type="submit" disabled={core.isLoading} className={styles.primaryButton}>
        {core.isLoading ? t("LoginModal.k53") : t("LoginModal.k54")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        core.goToMode('verify_recovery');
        core.clearError();
      }} className={styles.secondaryButton}>
          {t("LoginModal.k55")}
        </button>
      </div>
    </form>;

  return { renderVerifyRecoveryForm, renderResetPasswordForm, handleForgotPassword };
}
