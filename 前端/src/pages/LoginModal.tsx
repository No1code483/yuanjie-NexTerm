import { t } from "i18next";
/**
 * LoginModal - 登录/注册弹窗 (v1.01 增强)
 * 
 * 新增功能:
 * 1. 恢复短语展示：首次注册时显示12词恢复短语
 * 2. 忘记密码入口：登录页面"忘记密码?"链接
 * 3. 恢复短语验证流程
 * 4. 密码重置流程
 * 5. 对接 IPC 接口 (支持 Mock 模式)
 */

import { useState } from 'react';
import { auth, profile } from '@/lib/ipc';
import { copy } from '@/lib/utils';
import styles from './LoginModal.module.css';

// BIP39 英文词表 (简化版，实际应用中应使用完整2048词)
const BIP39_WORDS = ['abandon', 'ability', 'able', 'about', 'above', 'absence', 'absorb', 'abstract', 'absurd', 'abuse', 'access', 'accident', 'account', 'achieve', 'acid', 'acoustic', 'acquire', 'act', 'action', 'actor', 'actual', 'adapt', 'add', 'address', 'admin', 'admit', 'adult', 'advance', 'advice', 'aeroplane', 'affair', 'afford', 'afraid', 'again', 'age', 'agent', 'agree', 'ahead', 'aim', 'air', 'alarm', 'album', 'alcohol', 'alien', 'all', 'allow', 'almost', 'alone', 'already', 'also', 'alter', 'always', 'amateur', 'amazing', 'among', 'amount', 'amused', 'analyst', 'anchor', 'ancient', 'anger', 'angle', 'angry', 'animal', 'ankle', 'announce', 'annual', 'another', 'answer', 'antenna', 'antique', 'anxiety', 'any', 'apart', 'apology', 'appear', 'apple', 'approve', 'april', 'arch', 'area', 'argue', 'army', 'around', 'arrange', 'arrive', 'arrow', 'artefact'
// ... 完整列表应包含2048个单词
];
type AuthMode = 'login' | 'register' | 'show_recovery_phrase' | 'forgot_password' | 'verify_recovery' | 'reset_password' | 'temp_login' | '2fa_verify';
export default function LoginModal({
  onLogin
}: {
  onLogin: (user?: any, token?: string) => void;
}) {
  const [mode, setMode] = useState<AuthMode>('login');

  // 表单状态
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  // 临时账号状态
  const [tempUsername, setTempUsername] = useState('');
  const [tempExpires, setTempExpires] = useState<'1h' | '24h' | '7d'>('24h');

  // 恢复短语状态
  const [recoveryPhrase, setRecoveryPhrase] = useState<string[]>([]);
  const [recoveryInput, setRecoveryInput] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [phraseCopied, setPhraseCopied] = useState(false);
  const [forgotUsername, setForgotUsername] = useState('');

  // 待确认的认证数据 (注册后等待用户确认恢复短语时暂存)
  const [pendingAuth, setPendingAuth] = useState<{
    user: any;
    token: string;
  } | null>(null);

  // 2FA 状态
  const [twoFactorCode, setTwoFactorCode] = useState('');
  const [twoFactorToken, setTwoFactorToken] = useState('');

  // ==================== 工具函数 ====================

  /**
   * 生成随机恢复短语 (12个单词)
   */
  const generateRecoveryPhrase = (): string[] => {
    const words: string[] = [];
    for (let i = 0; i < 12; i++) {
      const randomIndex = Math.floor(Math.random() * BIP39_WORDS.length);
      words.push(BIP39_WORDS[randomIndex]);
    }
    return words;
  };

  /**
   * 复制恢复短语到剪贴板
   */
  const copyRecoveryPhrase = async () => {
    const ok = await copy(recoveryPhrase.join(' '));
    if (ok) {
      setPhraseCopied(true);
      setTimeout(() => setPhraseCopied(false), 2000);
    }
  };

  /**
   * 清除错误信息
   */
  const clearError = () => setError('');

  // ==================== 处理函数 ====================

  /**
   * 处理登录
   */
  const handleLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    clearError();
    console.log('🔐 [登录] 开始登录流程', {
      username,
      hasPassword: !!password
    });
    if (!username.trim() || !password) {
      setError(t("LoginModal.k1"));
      return;
    }
    setIsLoading(true);
    try {
      console.log('🔐 [登录] 调用 auth.login...', {
        username
      });
      const response = await auth.login(username, password);
      console.log('🔐 [登录] 收到响应:', {
        code: response.code,
        message: response.message,
        hasData: !!response.data,
        data: response.data
      });
      if (response.code === 0 && response.data) {
        const u = response.data.user;
        console.log('✅ 登录成功:', u.username, '| 角色:', u.role, '| 永久账号:', u.is_permanent);
        onLogin(response.data.user, response.data.token);
      } else if (response.code === 1002 && response.data?.token) {
        // 需要 2FA 验证
        console.log('🔐 需要2FA验证');
        setTwoFactorToken(response.data.token);
        setMode('2fa_verify');
      } else {
        console.error('❌ 登录失败 (服务端返回错误):', {
          code: response.code,
          message: response.message
        });
        setError(response.message || t("LoginModal.k2"));
      }
    } catch (err) {
      console.error('❌ 登录异常 (抛出异常):', err);
      setError(t("LoginModal.k3", {
        arg0: err instanceof Error ? err.message : t("errors.unknown")
      }));
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * 处理注册 (第一步：创建账号)
   */
  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    clearError();
    if (!username.trim()) {
      setError(t("LoginModal.k4"));
      return;
    }
    if (!password || password.length < 6) {
      setError(t("lib.ipcMock.k4"));
      return;
    }
    setIsLoading(true);
    try {
      // 生成恢复短语
      const phrase = generateRecoveryPhrase();
      setRecoveryPhrase(phrase);

      // 调用注册接口
      const response = await auth.register(username, password, phrase);
      if (response.code === 0 && response.data) {
        console.log('✅ 注册成功，展示恢复短语');
        setPendingAuth({
          user: response.data.user,
          token: response.data.token
        });
        setMode('show_recovery_phrase');
      } else {
        setError(response.message || t("LoginModal.k5"));
      }
    } catch (err) {
      console.error('❌ 注册异常:', err);

      // 即使后端失败，也可以继续展示恢复短语 (Mock模式或降级)
      if (!recoveryPhrase.length) {
        setRecoveryPhrase(generateRecoveryPhrase());
      }
      setMode('show_recovery_phrase');
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * 跳过恢复短语记录 (进入系统)
   */
  const handleSkipRecoveryPhrase = () => {
    console.log('⚠️ 用户跳过了恢复短语记录');
    if (pendingAuth) {
      onLogin(pendingAuth.user, pendingAuth.token);
    } else {
      onLogin();
    }
  };

  /**
   * 确认已记录恢复短语 (进入系统)
   */
  const handleConfirmRecoveryPhrase = () => {
    console.log('✅ 用户确认已记录恢复短语');
    if (pendingAuth) {
      onLogin(pendingAuth.user, pendingAuth.token);
    } else {
      onLogin();
    }
  };

  /**
   * 开始忘记密码流程
   */
  const handleForgotPassword = () => {
    setMode('verify_recovery');
    clearError();
  };

  /**
   * 验证恢复短语并重置密码 (合并为一步)
   */
  const handleVerifyRecoveryPhrase = async (e: React.FormEvent) => {
    e.preventDefault();
    clearError();
    if (!forgotUsername.trim()) {
      setError(t("LoginModal.k4"));
      return;
    }
    if (!recoveryInput.trim()) {
      setError(t("LoginModal.k6"));
      return;
    }
    const inputWords = recoveryInput.trim().split(/\s+/).filter(w => w.length > 0);
    if (inputWords.length !== 12) {
      setError(t("LoginModal.k7"));
      return;
    }
    if (!newPassword || newPassword.length < 6) {
      setError(t("LoginModal.k8"));
      return;
    }
    if (newPassword !== confirmPassword) {
      setError(t("LoginModal.k9"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await auth.verifyRecoveryPhrase(forgotUsername, inputWords, newPassword);
      if (response.code === 0 && response.data) {
        console.log('✅ 恢复短语验证通过，密码已重置');
        onLogin(response.data.user, response.data.token);
      } else {
        setError(response.message || t("LoginModal.k10"));
      }
    } catch (err) {
      console.error('❌ 验证异常:', err);
      setError(t("LoginModal.k11"));
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * 重置密码
   */
  const handleResetPassword = async (e: React.FormEvent) => {
    e.preventDefault();
    clearError();
    if (!newPassword || newPassword.length < 6) {
      setError(t("LoginModal.k8"));
      return;
    }
    if (newPassword !== confirmPassword) {
      setError(t("LoginModal.k9"));
      return;
    }
    setIsLoading(true);
    try {
      const inputWords = recoveryInput.trim().split(/\s+/).filter(w => w.length > 0);
      const response = await auth.verifyRecoveryPhrase(forgotUsername, inputWords, newPassword);
      if (response.code === 0 && response.data) {
        console.log('✅ 密码重置成功');
        onLogin(response.data.user, response.data.token);
      } else {
        setError(response.message || t("LoginModal.k12"));
      }
    } catch (err) {
      console.error('❌ 重置异常:', err);
      setError(t("LoginModal.k13"));
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * 处理 2FA 验证
   */
  const handle2faVerify = async (e: React.FormEvent) => {
    e.preventDefault();
    clearError();
    if (!twoFactorCode || twoFactorCode.length !== 6) {
      setError(t("LoginModal.k14"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await auth.auth2faLoginVerify(twoFactorToken, twoFactorCode);
      if (response.code === 0 && response.data) {
        console.log('✅ 2FA验证通过，登录成功');
        onLogin(response.data.user, response.data.token);
      } else {
        setError(response.message || t("LoginModal.k15"));
      }
    } catch (err) {
      console.error('❌ 2FA验证异常:', err);
      setError(t("LoginModal.k11"));
    } finally {
      setIsLoading(false);
    }
  };

  /**
   * 处理临时账号登录
   */
  const handleTempLogin = async () => {
    if (!tempUsername.trim()) {
      setError(t("LoginModal.k16"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await profile.createTempAccount(tempUsername, tempExpires);
      if (response.code === 0 && response.data) {
        console.log(`✅ 临时账号创建成功: ${response.data.username}`);
        const loginResponse = await auth.login(response.data.username, response.data.password);
        if (loginResponse.code === 0 && loginResponse.data) {
          console.log('✅ 临时账号登录成功');
          onLogin(loginResponse.data.user, loginResponse.data.token);
        } else {
          setError(loginResponse.message || t("LoginModal.k17"));
        }
      } else {
        setError(response.message || t("LoginModal.k18"));
      }
    } catch (err) {
      console.error('❌ 临时登录异常:', err);
      setError(t("LoginModal.k18"));
    } finally {
      setIsLoading(false);
    }
  };

  // ==================== 渲染函数 ====================

  const renderLoginForm = () => <form onSubmit={handleLogin}>
      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.username")}</label>
        <input type="text" value={username} onChange={e => setUsername(e.target.value)} placeholder="" className={styles.input} autoFocus />
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.password")}</label>
        <input type="password" value={password} onChange={e => setPassword(e.target.value)} placeholder="" className={styles.input} onKeyDown={e => e.key === 'Enter' && handleLogin(e)} />
      </div>

      {error && renderError()}

      <button type="submit" disabled={isLoading} className={styles.primaryButton}>
        {isLoading ? t("LoginModal.k19") : t("LoginModal.k20")}
      </button>
    </form>;
  const renderRegisterForm = () => <form onSubmit={handleRegister}>
      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.username")}</label>
        <input type="text" value={username} onChange={e => setUsername(e.target.value)} placeholder={t("LoginModal.k21")} className={styles.input} autoFocus />
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.password")}</label>
        <input type="password" value={password} onChange={e => setPassword(e.target.value)} placeholder={t("LoginModal.k22")} className={styles.input} />
      </div>

      <div className={styles.warningBox}>
        {t("LoginModal.k23")}<strong>{t("LoginModal.k24")}</strong>{t("LoginModal.k25")}
      </div>

      {error && renderError()}

      <button type="submit" disabled={isLoading} className={styles.primaryButton}>
        {isLoading ? t("LoginModal.k26") : t("LoginModal.k27")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        setMode('login');
        clearError();
      }} className={styles.secondaryButton}>
          {t("LoginModal.k28")}
        </button>
      </div>
    </form>;
  const renderRecoveryPhraseDisplay = () => <div>
      <div className={styles.warningBox}>
        <h3 className={styles.recoveryPhraseTitle}>
          {t("LoginModal.k29")}
        </h3>

        <div className={styles.recoveryPhraseBox}>
          {recoveryPhrase.map((word, index) => <span key={index}>
              {index + 1}. {word}
              {index < recoveryPhrase.length - 1 && '\n'}
            </span>)}
        </div>

        <div className={styles.copyButtonContainer}>
          <button type="button" onClick={copyRecoveryPhrase} className={styles.copyButton}>
            {phraseCopied ? t("LoginModal.k30") : t("LoginModal.k31")}
          </button>
        </div>

        <div className={styles.dangerBox}>
          ⚠️ <strong>{t("LoginModal.k32")}</strong><br />
          {t("LoginModal.k33")}<strong>{t("LoginModal.k34")}</strong><br />
          {t("LoginModal.k35")}<br />
          {t("LoginModal.k36")}<br />
          {t("LoginModal.k37")}
        </div>
      </div>

      <div className={styles.actionButtons}>
        <button type="button" onClick={handleConfirmRecoveryPhrase} className={styles.primaryButton}>
          {t("LoginModal.k38")}
        </button>
        
        <button type="button" onClick={handleSkipRecoveryPhrase} className={styles.secondaryButton}>
          {t("LoginModal.k39")}
        </button>
      </div>
    </div>;
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

      {error && renderError()}

      <button type="submit" disabled={isLoading} className={styles.primaryButton}>
        {isLoading ? t("LoginModal.k49") : t("LoginModal.k50")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        setMode('login');
        clearError();
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

      {error && renderError()}

      <button type="submit" disabled={isLoading} className={styles.primaryButton}>
        {isLoading ? t("LoginModal.k53") : t("LoginModal.k54")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        setMode('verify_recovery');
        clearError();
      }} className={styles.secondaryButton}>
          {t("LoginModal.k55")}
        </button>
      </div>
    </form>;
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

      {error && renderError()}

      <button type="button" onClick={handleTempLogin} disabled={isLoading} className={styles.primaryButton}>
        {isLoading ? t("game.GamePreview.k5") : t("LoginModal.k63")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        setMode('login');
        clearError();
      }} className={styles.secondaryButton}>
          {t("LoginModal.k51")}
        </button>
      </div>
    </div>;
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

      {error && renderError()}

      <button type="submit" disabled={isLoading || twoFactorCode.length !== 6} className={styles.primaryButton}>
        {isLoading ? t("LoginModal.k49") : t("LoginModal.k67")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        setMode('login');
        clearError();
        setTwoFactorCode('');
        setTwoFactorToken('');
      }} className={styles.secondaryButton}>
          {t("LoginModal.k51")}
        </button>
      </div>
    </form>;
  const renderError = () => <div className={styles.errorMessage}>
      {error}
    </div>;

  // ==================== 主渲染 ====================

  const getModeTitle = () => {
    switch (mode) {
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
  return <div className={styles.modalOverlay}>
      <div className={styles.modalContainer}>
        {mode !== 'login' && <h2 className={styles.title}>
            {getModeTitle()}
          </h2>}

        {mode === 'login' && renderLoginForm()}
        {mode === 'register' && renderRegisterForm()}
        {mode === 'show_recovery_phrase' && renderRecoveryPhraseDisplay()}
        {mode === 'verify_recovery' && renderVerifyRecoveryForm()}
        {mode === 'reset_password' && renderResetPasswordForm()}
        {mode === 'temp_login' && renderTempLoginForm()}
        {mode === '2fa_verify' && render2faVerifyForm()}

        {mode === 'login' && <div className={styles.bottomOptions}>
            <button type="button" onClick={() => {
          setMode('register');
          clearError();
        }} className={styles.optionLink}>
              {t("LoginModal.k27")}
            </button>
            <span className={styles.optionDivider}>•</span>
            <button type="button" onClick={handleForgotPassword} className={styles.optionLink}>
              {t("LoginModal.k71")}
            </button>
            <span className={styles.optionDivider}>•</span>
            <button type="button" onClick={() => {
          setMode('temp_login');
          clearError();
        }} className={styles.optionLink}>
              {t("LoginModal.k69")}
            </button>
          </div>}
      </div>
    </div>;
}