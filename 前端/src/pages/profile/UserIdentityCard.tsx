import { t } from "i18next";
import { useState } from 'react';
import { profile } from '@/lib/ipc';
import { useAuthStore } from '@/stores/authStore';
import type { UserInfo } from '@/types';
import styles from '../Profile.module.css';
interface UserIdentityCardProps {
  user: UserInfo;
  isTempAccount: boolean;
  formatTime: (ts: number) => string;
}

/**
 * 用户身份卡 - 终端风格
 * 替代原 AccountPanel 顶部的静态信息列表
 * 浏览态：头像 / 用户名 / UID / 角色徽章 / 账号类型 / 签名 / 编辑按钮
 * 编辑态：头像 URL / 显示名 / 签名 输入框 + 保存/取消
 */
export default function UserIdentityCard({
  user,
  isTempAccount,
  formatTime
}: UserIdentityCardProps) {
  const updateUser = useAuthStore(s => s.updateUser);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');

  // 编辑表单本地态
  const [avatarUrl, setAvatarUrl] = useState(user.avatar_url ?? '');
  const [displayName, setDisplayName] = useState(user.display_name ?? '');
  const [bio, setBio] = useState(user.bio ?? '');
  const handleEnterEdit = () => {
    setAvatarUrl(user.avatar_url ?? '');
    setDisplayName(user.display_name ?? '');
    setBio(user.bio ?? '');
    setMessage('');
    setEditing(true);
  };
  const handleCancel = () => {
    setEditing(false);
    setMessage('');
  };
  const handleSave = async () => {
    setSaving(true);
    setMessage('');
    try {
      const res = await profile.updateProfile({
        avatar_url: avatarUrl.trim() || null,
        display_name: displayName.trim() || null,
        bio: bio.trim() || null
      });
      if (res.code === 0) {
        // 同步本地 store
        updateUser({
          avatar_url: avatarUrl.trim() || null,
          display_name: displayName.trim() || null,
          bio: bio.trim() || null
        });
        setMessage(t("components.TableEditor.k1"));
        setEditing(false);
      } else {
        setMessage(res.message || t("errors.saveFailed"));
      }
    } catch (err) {
      setMessage(t("profile.UserIdentityCard.k1", {
        err: err
      }));
    } finally {
      setSaving(false);
    }
  };

  // 角色文本
  const roleText = user.role === 'admin' ? t("profile.UserIdentityCard.k2") : user.role === 'user' ? t("profile.UserIdentityCard.k3") : t("profile.UserIdentityCard.k4");

  // 终端用户名：root@nexterm 风格
  const terminalUser = isTempAccount ? `guest@nexterm` : `${user.username}@nexterm`;
  return <div className={`${styles.infoCard} ${styles.identityCard}`}>
      <div className={styles.cardTitle}>{t("profile.UserIdentityCard.k5")}</div>

      {message && <div className={`${styles.identityMessage} ${message.includes(t("common.success")) ? styles.msgSuccess : styles.msgError}`}>
          {message}
        </div>}

      {!editing ? (/* 浏览态 */
    <div className={styles.identityBody}>
          {/* 头像区 */}
          <div className={styles.avatarBox}>
            {user.avatar_url ? <img src={user.avatar_url} alt="avatar" className={styles.avatarImg} onError={e => {
          // 加载失败回退到 ASCII
          (e.currentTarget as HTMLImageElement).style.display = 'none';
        }} /> : <pre className={styles.avatarAscii}>{` ░░░░░░░░░░░
░░██╗░░░██╗░░
░██░░███░░██╗░
██░░░███░░░██║
██║░░███░░░██║
░██╗░███░░██╔╝
░░███░░░███░░
░░░███████░░░
░░░░░███░░░░░
░░░░░███░░░░░
░░░░░░░░░░░░░`}</pre>}
          </div>

          {/* 信息区 */}
          <div className={styles.identityInfo}>
            <div className={styles.identityNameLine}>
              <span className={styles.terminalUser}>{terminalUser}</span>
              <button className={styles.editIdentityBtn} onClick={handleEnterEdit} disabled={isTempAccount} title={isTempAccount ? t("profile.UserIdentityCard.k6") : t("profile.UserIdentityCard.k7")}>
                {t("profile.ResumePanel.k93")}
              </button>
            </div>

            <div className={styles.identityBadges}>
              <span className={styles.badgeUid}>UID: {isTempAccount ? '-' : user.id}</span>
              <span className={`${styles.badgeRole} ${user.role === 'admin' ? styles.badgeAdmin : styles.badgeUser}`}>
                {roleText}
              </span>
              <span className={`${styles.badgeType} ${isTempAccount ? styles.badgeTemp : styles.badgePerm}`}>
                {isTempAccount ? t("LoginModal.k69") : t("profile.UserIdentityCard.k8")}
              </span>
              {isTempAccount && user.expires_at && <span className={styles.badgeExpire}>⏱ {formatTime(user.expires_at)}</span>}
            </div>

            {/* 显示名（如有） */}
            {user.display_name && <div className={styles.identityDisplayName}>
                <span className={styles.identityLabel}>display_name:</span>
                <span className={styles.identityValue}>{user.display_name}</span>
              </div>}

            {/* 签名 / Bio */}
            <div className={styles.identityBioLine}>
              <span className={styles.identityLabel}>bio:</span>
              <span className={styles.identityBioValue}>
                {user.bio || t("profile.UserIdentityCard.k9")}
              </span>
            </div>
          </div>
        </div>) : (/* 编辑态 */
    <div className={styles.identityEditForm}>
          <div className={styles.formGroup} style={{
        marginBottom: 14
      }}>
            <div style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
          marginBottom: 6
        }}>
              <label className={styles.formLabel} style={{
            marginBottom: 0
          }}>{t("profile.UserIdentityCard.k10")}</label>
              <button type="button" className={styles.uploadAvatarBtn} onClick={() => {
            const input = document.getElementById('avatarFileInput') as HTMLInputElement;
            if (input) input.click();
          }} disabled={saving}>
                {t("profile.UserIdentityCard.k11")}
              </button>
              <input id="avatarFileInput" type="file" accept="image/*" style={{
            display: 'none'
          }} onChange={e => {
            const file = e.target.files?.[0];
            if (!file) return;
            if (file.size > 2 * 1024 * 1024) {
              setMessage(t("profile.UserIdentityCard.k12"));
              return;
            }
            const reader = new FileReader();
            reader.onload = () => {
              setAvatarUrl(reader.result as string);
            };
            reader.readAsDataURL(file);
          }} />
            </div>
            <input type="text" value={avatarUrl} onChange={e => setAvatarUrl(e.target.value)} placeholder={t("profile.UserIdentityCard.k13")} className={styles.formInput} disabled={saving} />
            {avatarUrl && <div className={styles.avatarPreviewBox}>
                <span className={styles.avatarPreviewLabel}>{t("profile.UserIdentityCard.k14")}</span>
                <img src={avatarUrl.startsWith('data:') || avatarUrl.startsWith('http') ? avatarUrl : ''} alt={t("common.preview")} className={styles.avatarPreviewImg} onError={e => {
            (e.currentTarget as HTMLImageElement).style.display = 'none';
          }} />
              </div>}
          </div>
          <div className={styles.formGroup} style={{
        marginBottom: 14
      }}>
            <label className={styles.formLabel}>{t("profile.UserIdentityCard.k15")}</label>
            <input type="text" value={displayName} onChange={e => setDisplayName(e.target.value)} placeholder={t("profile.UserIdentityCard.k16")} maxLength={30} className={styles.formInput} disabled={saving} />
          </div>
          <div className={styles.formGroup} style={{
        marginBottom: 18
      }}>
            <label className={styles.formLabel}>{t("profile.UserIdentityCard.k17")}</label>
            <textarea value={bio} onChange={e => setBio(e.target.value)} placeholder={t("profile.UserIdentityCard.k18")} maxLength={120} rows={2} className={styles.formInput} disabled={saving} style={{
          resize: 'vertical',
          minHeight: 56
        }} />
            <div className={styles.charCount}>{bio.length}/120</div>
          </div>
          <div className={styles.identityEditActions}>
            <button className={styles.primaryButton} onClick={handleSave} disabled={saving} style={{
          padding: '8px 22px',
          fontSize: 13
        }}>
              {saving ? t("components.AudioEditor.k6") : t("common.save")}
            </button>
            <button className={styles.dangerButton} onClick={handleCancel} disabled={saving} style={{
          padding: '8px 22px',
          fontSize: 13
        }}>
              {t("common.cancel")}
            </button>
          </div>
        </div>)}
    </div>;
}