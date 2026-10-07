// auth.register L2 —— 注册功能域（register 表单 + 恢复短语展示）。
// 物理迁入本目录：原 L1 LoginModal 的注册视图与恢复短语展示；共享表单状态经 core（AuthCore）注入。
import { t } from "i18next";
import { useState } from 'react';
import { auth } from '../../ipc/auth';
import { copy } from '@/lib/utils';
import type { AuthCore } from '../../core';
import styles from '../../LoginModal.module.css';
import ErrorHint from '../../ErrorHint';

// BIP39 英文词表 (简化版，实际应用中应使用完整2048词)
const BIP39_WORDS = ['abandon', 'ability', 'able', 'about', 'above', 'absence', 'absorb', 'abstract', 'absurd', 'abuse', 'access', 'accident', 'account', 'achieve', 'acid', 'acoustic', 'acquire', 'act', 'action', 'actor', 'actual', 'adapt', 'add', 'address', 'admin', 'admit', 'adult', 'advance', 'advice', 'aeroplane', 'affair', 'afford', 'afraid', 'again', 'age', 'agent', 'agree', 'ahead', 'aim', 'air', 'alarm', 'album', 'alcohol', 'alien', 'all', 'allow', 'almost', 'alone', 'already', 'also', 'alter', 'always', 'amateur', 'amazing', 'among', 'amount', 'amused', 'analyst', 'anchor', 'ancient', 'anger', 'angle', 'angry', 'animal', 'ankle', 'announce', 'annual', 'another', 'answer', 'antenna', 'antique', 'anxiety', 'any', 'apart', 'apology', 'appear', 'apple', 'approve', 'april', 'arch', 'area', 'argue', 'army', 'around', 'arrange', 'arrive', 'arrow', 'artefact'
// ... 完整列表应包含2048个单词
];

export function useRegister(core: AuthCore) {
  // 恢复短语状态
  const [recoveryPhrase, setRecoveryPhrase] = useState<string[]>([]);
  const [phraseCopied, setPhraseCopied] = useState(false);

  // 待确认的认证数据 (注册后等待用户确认恢复短语时暂存)
  const [pendingAuth, setPendingAuth] = useState<{
    user: any;
    token: string;
  } | null>(null);

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
   * 处理注册 (第一步：创建账号)
   */
  const handleRegister = async (e: React.FormEvent) => {
    e.preventDefault();
    core.clearError();
    if (!core.username.trim()) {
      core.setError(t("LoginModal.k4"));
      return;
    }
    if (!core.password || core.password.length < 6) {
      core.setError(t("lib.ipcMock.k4"));
      return;
    }
    core.setIsLoading(true);
    try {
      // 生成恢复短语（仅作后端不可用时的降级展示，权威短语由后端生成并落库）
      const phrase = generateRecoveryPhrase();
      setRecoveryPhrase(phrase);

      // 调用注册接口
      const response = await auth.register(core.username, core.password, phrase);
      if (response.code === 0 && response.data) {
        console.log('✅ 注册成功，展示恢复短语');
        // 必须展示后端返回的权威短语，否则用户记下的词无法通过「助记词找回」验证
        const authoritativePhrase = response.data.recovery_phrase;
        if (authoritativePhrase) {
          setRecoveryPhrase(authoritativePhrase.trim().split(/\s+/));
        }
        setPendingAuth({
          user: response.data.user,
          token: response.data.token
        });
        core.goToMode('show_recovery_phrase');
      } else {
        core.setError(response.message || t("LoginModal.k5"));
      }
    } catch (err) {
      console.error('❌ 注册异常:', err);

      // 即使后端失败，也可以继续展示恢复短语 (Mock模式或降级)
      if (!recoveryPhrase.length) {
        setRecoveryPhrase(generateRecoveryPhrase());
      }
      core.goToMode('show_recovery_phrase');
    } finally {
      core.setIsLoading(false);
    }
  };

  /**
   * 跳过恢复短语记录 (进入系统)
   */
  const handleSkipRecoveryPhrase = () => {
    console.log('⚠️ 用户跳过了恢复短语记录');
    if (pendingAuth) {
      core.onLogin(pendingAuth.user, pendingAuth.token);
    } else {
      core.onLogin();
    }
  };

  /**
   * 确认已记录恢复短语 (进入系统)
   */
  const handleConfirmRecoveryPhrase = () => {
    console.log('✅ 用户确认已记录恢复短语');
    if (pendingAuth) {
      core.onLogin(pendingAuth.user, pendingAuth.token);
    } else {
      core.onLogin();
    }
  };

  const renderRegisterForm = () => <form onSubmit={handleRegister}>
      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.username")}</label>
        <input type="text" value={core.username} onChange={e => core.setUsername(e.target.value)} placeholder={t("LoginModal.k21")} className={styles.input} autoFocus />
      </div>

      <div className={styles.formGroup}>
        <label className={styles.label}>{t("common.password")}</label>
        <input type="password" value={core.password} onChange={e => core.setPassword(e.target.value)} placeholder={t("LoginModal.k22")} className={styles.input} />
      </div>

      <div className={styles.warningBox}>
        {t("LoginModal.k23")}<strong>{t("LoginModal.k24")}</strong>{t("LoginModal.k25")}
      </div>

      {core.error && <ErrorHint message={core.error} />}

      <button type="submit" disabled={core.isLoading} className={styles.primaryButton}>
        {core.isLoading ? t("LoginModal.k26") : t("LoginModal.k27")}
      </button>

      <div className={styles.secondaryButtonContainer}>
        <button type="button" onClick={() => {
        core.goToMode('login');
        core.clearError();
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

  return { renderRegisterForm, renderRecoveryPhraseDisplay };
}
