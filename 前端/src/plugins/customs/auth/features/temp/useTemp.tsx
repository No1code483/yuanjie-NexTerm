// auth.temp L2 —— 临时账号功能域（temp_login 视图）。
// 物理迁入本目录：原 L1 LoginModal 的临时账号登录视图；共享表单状态经 core（AuthCore）注入。
import { t } from "i18next";
import { useState } from 'react';
import { auth } from '../../ipc/auth';
import type { AuthCore } from '../../core';
import styles from '../../LoginModal.module.css';
import ErrorHint from '../../ErrorHint';

export function useTemp(core: AuthCore) {
  // 临时账号状态
  const [tempUsername, setTempUsername] = useState('');
  const [tempExpires, setTempExpires] = useState<'1h' | '24h' | '7d'>('24h');

  /**
   * 处理临时账号登录
   */
  const handleTempLogin = async () => {
    if (!tempUsername.trim()) {
      core.setError(t("LoginModal.k16"));
      return;
    }
    core.setIsLoading(true);
    try {
      const response = await auth.createTempAccount(tempUsername, tempExpires);
      if (response.code === 0 && response.data) {
        console.log(`✅ 临时账号创建成功: ${response.data.username}`);
        const loginResponse = await auth.login(response.data.username, response.data.password);
        if (loginResponse.code === 0 && loginResponse.data) {
          console.log('✅ 临时账号登录成功');
          core.onLogin(loginResponse.data.user, loginResponse.data.token);
        } else {
          core.setError(loginResponse.message || t("LoginModal.k17"));
        }
      } else {
        core.setError(response.message || t("LoginModal.k18"));
      }
    } catch (err) {
      console.error('❌ 临时登录异常:', err);
      core.setError(t("LoginModal.k18"));
    } finally {
      core.setIsLoading(false);
    }
  };

  const renderTempLoginForm = () => <div>
      <div className={styles.infoBox}>
        {t("LoginModal.k56")}
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("LoginModal.k57")}</label>
        <input type="text" value={tempUsername} onChange={e => setTempUsername(e.target.value)} placeholder={t("LoginModal.k58")} className={styles.input} autoFocus />
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("LoginModal.k59")}</label>
        <select value={tempExpires} onChange={e => setTempExpires(e.target.value as '1h' | '24h' | '7d')} className={styles.select}>
          <option value="1h">{t("LoginModal.k60")}</option>
          <option value="24h">{t("LoginModal.k61")}</option>
          <option value="7d">{t("LoginModal.k62")}</option>
        </select>
      </div>

      {core.error && <ErrorHint message={core.error} />}

      <button type="button" onClick={handleTempLogin} disabled={core.isLoading} className={styles.primaryButton}>
        {core.isLoading ? t("game.GamePreview.k5") : t("LoginModal.k63")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        core.goToMode('login');
        core.clearError();
      }} className={styles.secondaryButton}>
          {t("LoginModal.k51")}
        </button>
      </div>
    </div>;

  return { renderTempLoginForm };
}
