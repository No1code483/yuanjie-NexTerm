// profile.settings L2 —— 设置功能域（局部 state + handlers + 全部设置子页 JSX）。
// 物理迁入本目录；核心共享项经 core（ProfileCore）注入，禁止直接修改 L1 核心 state。
// active 表示当前是否处于设置 tab（且已鉴权），用于惰性加载（等价于原壳 currentTab==='setting' 分支）。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { auth } from '@/plugins/customs/auth/ipc/auth';
// news_source 3 命令归属 home.news（阶段3 批次1b-2b-1，裁定 9-A）：改指该插件公开 client
import { news } from '@/plugins/boards/home/features/news/ipc';
import { profile } from '../../ipc/profile';
import type { ProfileCore } from '../../core';
import type { NewsSourceItem } from '../../types';
import ConfirmDialog from '@/components/ConfirmDialog';
import ThemeSelector from '@/components/ThemeSelector/ThemeSelector';
import LanguageSelector from '@/components/LanguageSelector/LanguageSelector';
import { TitleBarSettingsPanel } from '@/components/TitleBar/TitleBarSettingsPanel';
import styles from '../../Profile.module.css';

export function useSettings(core: ProfileCore, active: boolean) {
  const { user, isTempAccount, showNotify, confirmDialog, setConfirmDialog, formatTime } = core;
  const navigate = useNavigate();

  const [editUsername, setEditUsername] = useState('');
  const [oldPassword, setOldPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [newsSources, setNewsSources] = useState<NewsSourceItem[]>([]);
  const [newsSourcesLoading, setNewsSourcesLoading] = useState(false);
  const [showAddSource, setShowAddSource] = useState(false);
  const [newSource, setNewSource] = useState({
    name: '',
    url: '',
    category: 'security',
    feedType: 'rss'
  });
  const [settingSubPage, setSettingSubPage] = useState<string | null>(null);
  const [exportingData, setExportingData] = useState(false);
  const [tempForm, setTempForm] = useState({
    username: '',
    duration: '24h' as '1h' | '24h' | '7d'
  });

  // 会话管理状态
  const [sessions, setSessions] = useState<any[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [sessionsMessage, setSessionsMessage] = useState('');

  // 2FA 状态
  const [fa2Loading, setFa2Loading] = useState(false);
  const [fa2SetupData, setFa2SetupData] = useState<{
    secret: string;
    qr_code_url: string;
  } | null>(null);
  const [fa2VerifyCode, setFa2VerifyCode] = useState('');
  const [fa2Enabled, setFa2Enabled] = useState(false);
  const [fa2Message, setFa2Message] = useState('');

  useEffect(() => {
    if (user?.username) {
      setEditUsername(user.username);
    }
  }, [user?.username]);

  const handleExportData = async () => {
    setExportingData(true);
    try {
      const response = await profile.exportUserData();
      if (response.code === 0 && response.data) {
        const jsonStr = JSON.stringify(response.data, null, 2);
        try {
          const {
            save
          } = await import('@tauri-apps/plugin-dialog');
          const {
            writeTextFile
          } = await import('@tauri-apps/plugin-fs');
          const filePath = await save({
            defaultPath: `nexterm-data-export-${Date.now()}.json`,
            filters: [{
              name: 'JSON',
              extensions: ['json']
            }]
          });
          if (filePath) {
            await writeTextFile(filePath, jsonStr);
            showNotify('success', t("Profile.k10", {
              filePath: filePath
            }));
          }
        } catch {
          // fallback: copy to clipboard
          const {
            copy
          } = await import('@/lib/utils');
          await copy(jsonStr);
          showNotify('success', t("Profile.k11"));
        }
      } else {
        showNotify('error', response.message || t("components.TableEditor.k3"));
      }
    } catch (err) {
      showNotify('error', t("Profile.k12", {
        err: err
      }));
    } finally {
      setExportingData(false);
    }
  };

  const fetchNewsSources = async () => {
    setNewsSourcesLoading(true);
    try {
      const response = await news.getNewsSources();
      if (response.code === 0 && response.data) {
        setNewsSources(response.data);
      }
    } catch (err) {
      console.error('获取新闻源失败:', err);
    } finally {
      setNewsSourcesLoading(false);
    }
  };

  // 设置 tab 激活且位于新闻源子页时加载（等价原壳 currentTab==='setting' && settingSubPage==='newsSource' 分支）
  useEffect(() => {
    if (active && settingSubPage === 'newsSource') {
      fetchNewsSources();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, settingSubPage]);

  const handleAddNewsSource = async () => {
    if (!newSource.name.trim() || !newSource.url.trim()) {
      showNotify('error', t("Profile.k50"));
      return;
    }
    if (!newSource.url.startsWith('http://') && !newSource.url.startsWith('https://')) {
      showNotify('error', t("Profile.k51"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await news.addNewsSource(newSource.name, newSource.url, newSource.category, newSource.feedType);
      if (response.code === 0) {
        setNewSource({
          name: '',
          url: '',
          category: 'security',
          feedType: 'rss'
        });
        setShowAddSource(false);
        await fetchNewsSources();
        showNotify('success', t("Profile.k52"));
      } else {
        showNotify('error', response.message || t("components.PptEditor.k4"));
      }
    } catch (err) {
      console.error('添加新闻源失败:', err);
      showNotify('error', t("Profile.k17"));
    } finally {
      setIsLoading(false);
    }
  };
  const handleDeleteNewsSource = async (id: number, name: string) => {
    setConfirmDialog({
      isOpen: true,
      targetName: name,
      onConfirm: async () => {
        setIsLoading(true);
        try {
          const response = await news.deleteNewsSource(id);
          if (response.code === 0) {
            await fetchNewsSources();
            showNotify('success', t("Profile.k53"));
          } else {
            showNotify('error', response.message || t("errors.deleteFailed"));
          }
        } catch (err) {
          console.error('删除新闻源失败:', err);
          showNotify('error', t("Profile.k23"));
        } finally {
          setIsLoading(false);
        }
      }
    });
  };
  const getCategoryLabel = (cat: string) => {
    const map: Record<string, {
      text: string;
      color: string;
    }> = {
      // ===== 网安 =====
      security: {
        text: t("home.utils.k5"),
        color: 'rgba(255, 68, 68, 0.85)'
      },
      vulnerability: {
        text: t("home.utils.k5"),
        color: 'rgba(255, 68, 68, 0.85)'
      },
      attack_defense: {
        text: t("home.utils.k5"),
        color: 'rgba(255, 68, 68, 0.85)'
      },
      // ===== AI =====
      ai: {
        text: 'AI',
        color: 'rgba(176, 38, 255, 0.85)'
      },
      tech_innovation: {
        text: 'AI',
        color: 'rgba(176, 38, 255, 0.85)'
      },
      // ===== 编程 =====
      programming: {
        text: t("home.utils.k6"),
        color: 'rgba(0, 240, 255, 0.85)'
      },
      tool_application: {
        text: t("home.utils.k6"),
        color: 'rgba(0, 240, 255, 0.85)'
      },
      cloud_native: {
        text: t("home.utils.k6"),
        color: 'rgba(0, 240, 255, 0.85)'
      },
      open_source: {
        text: t("home.utils.k6"),
        color: 'rgba(0, 240, 255, 0.85)'
      },
      // ===== GitHub =====
      github: {
        text: 'GitHub',
        color: 'rgba(110, 84, 148, 0.85)'
      }
    };
    return map[cat] || {
      text: cat,
      color: '#6a6a8a'
    };
  };
  const handlePasswordChange = async () => {
    if (newPassword !== confirmPassword) {
      showNotify('error', t("Profile.k58"));
      return;
    }
    if (newPassword.length < 6) {
      showNotify('error', t("Profile.k59"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await auth.changePassword(oldPassword, newPassword);
      if (response.code === 0) {
        showNotify('success', t("lib.ipcMock.k108"));
        setOldPassword('');
        setNewPassword('');
        setConfirmPassword('');
      } else {
        showNotify('error', response.message || t("Profile.k60"));
      }
    } catch (err) {
      console.error('修改密码失败:', err);
      showNotify('error', t("Profile.k61"));
    } finally {
      setIsLoading(false);
    }
  };
  const handleUpdateUsername = async () => {
    if (!editUsername.trim() || editUsername === user?.username) {
      showNotify('error', t("Profile.k62"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await auth.changeUsername(editUsername);
      if (response.code === 0) {
        showNotify('success', t("lib.ipcMock.k109"));
      } else {
        showNotify('error', response.message || t("Profile.k63"));
      }
    } catch (err) {
      console.error('修改用户名失败:', err);
      showNotify('error', t("Profile.k64"));
    } finally {
      setIsLoading(false);
    }
  };
  const createTempAccount = async () => {
    if (!tempForm.duration || !['1h', '24h', '7d'].includes(tempForm.duration)) {
      showNotify('error', t("Profile.k65"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await auth.createTempAccount(tempForm.username || '', tempForm.duration);
      if (response.code === 0 && response.data) {
        setTempForm({
          username: '',
          duration: '24h'
        });
        showNotify('success', t("Profile.k66", {
          username: response.data.username,
          arg0: formatTime(response.data.expires_at)
        }));
      } else {
        showNotify('error', response.message || t("Knowledge.k10"));
      }
    } catch (err) {
      console.error('创建临时账号失败:', err);
      showNotify('error', t("Profile.k67"));
    } finally {
      setIsLoading(false);
    }
  };
  const handleSessionList = async () => {
    setSessionsLoading(true);
    setSessionsMessage('');
    try {
      const res = await auth.sessionList();
      if (res.code === 0 && res.data) {
        setSessions(res.data);
      } else {
        setSessionsMessage(res.message || t("Profile.k68"));
      }
    } catch (err) {
      setSessionsMessage(t("profile.ActivityTimeline.k4", {
        err: err
      }));
    } finally {
      setSessionsLoading(false);
    }
  };
  const handleSessionRevoke = async (sessionId: string) => {
    setSessionsLoading(true);
    setSessionsMessage('');
    try {
      const res = await auth.sessionRevoke(sessionId);
      if (res.code === 0) {
        setSessions(prev => prev.filter(s => s.session_id !== sessionId));
        setSessionsMessage(t("Profile.k69"));
      } else {
        setSessionsMessage(res.message || t("Profile.k70"));
      }
    } catch (err) {
      setSessionsMessage(t("Profile.k71", {
        err: err
      }));
    } finally {
      setSessionsLoading(false);
    }
  };
  const handle2faSetup = async () => {
    setFa2Loading(true);
    setFa2Message('');
    try {
      const res = await auth.auth2faSetup();
      if (res.code === 0 && res.data) {
        setFa2SetupData(res.data);
      } else {
        setFa2Message(res.message || t("Profile.k72"));
      }
    } catch (err) {
      setFa2Message(t("Profile.k73", {
        err: err
      }));
    } finally {
      setFa2Loading(false);
    }
  };
  const handle2faVerify = async () => {
    if (!fa2VerifyCode) return;
    setFa2Loading(true);
    setFa2Message('');
    try {
      const res = await auth.auth2faVerify(fa2VerifyCode);
      if (res.code === 0 && res.data?.verified) {
        setFa2Message(t("Profile.k74"));
        setFa2Enabled(true);
      } else {
        setFa2Message(res.message || t("LoginModal.k15"));
      }
    } catch (err) {
      setFa2Message(t("Profile.k75", {
        err: err
      }));
    } finally {
      setFa2Loading(false);
    }
  };
  const handle2faDisable = async () => {
    setFa2Loading(true);
    setFa2Message('');
    try {
      const res = await auth.auth2faDisable(fa2VerifyCode);
      if (res.code === 0) {
        setFa2Enabled(false);
        setFa2SetupData(null);
        setFa2VerifyCode('');
        setFa2Message(t("Profile.k76"));
      } else {
        setFa2Message(res.message || t("Profile.k77"));
      }
    } catch (err) {
      setFa2Message(t("Profile.k78", {
        err: err
      }));
    } finally {
      setFa2Loading(false);
    }
  };

  const renderSettingList = () => <div className={styles.infoCard}>
      <div className={styles.cardTitle}>{t("Profile.k80")}</div>

      <div className={styles.settingNavCard} style={{
      marginBottom: 0,
      marginTop: 0
    }} onClick={() => setSettingSubPage('account')}>
        <div className={styles.settingNavIcon}>👤</div>
        <div className={styles.settingNavInfo}>
          <div className={styles.settingNavTitle}>{t("Profile.k81")}</div>
          <div className={styles.settingNavDesc}>{t("Profile.k82")}</div>
        </div>
        <div className={styles.settingNavArrow}>›</div>
      </div>

      <div className={styles.settingNavCard} style={{
      marginBottom: 0,
      marginTop: 0
    }} onClick={() => setSettingSubPage('newsSource')}>
        <div className={styles.settingNavIcon}>📰</div>
        <div className={styles.settingNavInfo}>
          <div className={styles.settingNavTitle}>{t("Profile.k83")}</div>
          <div className={styles.settingNavDesc}>{t("Profile.k84")}</div>
        </div>
        <div className={styles.settingNavArrow}>›</div>
      </div>

      <div className={styles.settingNavCard} style={{
      marginBottom: 0,
      marginTop: 0
    }} onClick={() => setSettingSubPage('theme')}>
        <div className={styles.settingNavIcon}>🎨</div>
        <div className={styles.settingNavInfo}>
          <div className={styles.settingNavTitle}>{t("Profile.k85")}</div>
          <div className={styles.settingNavDesc}>{t("Profile.k86")}</div>
        </div>
        <div className={styles.settingNavArrow}>›</div>
      </div>

      <div className={styles.settingNavCard} style={{
      marginBottom: 0,
      marginTop: 0
    }} onClick={() => setSettingSubPage('windowControl')}>
        <div className={styles.settingNavIcon}>⚙</div>
        <div className={styles.settingNavInfo}>
          <div className={styles.settingNavTitle}>{t("Profile.k149")}</div>
          <div className={styles.settingNavDesc}>{t("Profile.k150")}</div>
        </div>
        <div className={styles.settingNavArrow}>›</div>
      </div>

      <div className={styles.settingNavCard} style={{
      marginBottom: 0,
      marginTop: 0
    }} onClick={handleExportData}>
        <div className={styles.settingNavIcon}>📤</div>
        <div className={styles.settingNavInfo}>
          <div className={styles.settingNavTitle}>{t("Profile.k87")}</div>
          <div className={styles.settingNavDesc}>{exportingData ? t("Profile.k88") : t("Profile.k89")}</div>
        </div>
        <div className={styles.settingNavArrow}>›</div>
      </div>

      <div className={styles.settingNavCard} style={{
      marginBottom: 0,
      marginTop: 0
    }} onClick={() => navigate('/spyglass')}>
        <div className={styles.settingNavIcon}>
          <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#00F0FF" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
            <rect x="3" y="8" width="18" height="12" rx="2.5" />
            <circle cx="9" cy="13" r="1.5" fill="#00F0FF" />
            <circle cx="15" cy="13" r="1.5" fill="#00F0FF" />
            <path d="M10 17h4" />
            <line x1="12" y1="4" x2="12" y2="8" />
            <circle cx="12" cy="3.5" r="1.5" stroke="#00F0FF" />
          </svg>
        </div>
        <div className={styles.settingNavInfo}>
          <div className={styles.settingNavTitle}>{t("components.intelligence.DashboardPanel.k103")}</div>
          <div className={styles.settingNavDesc}>{t("Profile.k90")}</div>
        </div>
        <div className={styles.settingNavArrow}>›</div>
      </div>
    </div>;
  const renderAccountSettingsPage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => setSettingSubPage(null)}>
          {t("Intelligence.k2")}
        </button>
        <div className={styles.infoTitle}>{t("Profile.k81")}</div>
      </div>

      <div className={styles.infoCard}>
        <div className={styles.settingNavCard} style={{
        marginBottom: 0,
        marginTop: 0
      }} onClick={() => setSettingSubPage('account-edit')}>
          <div className={styles.settingNavIcon}>✎</div>
          <div className={styles.settingNavInfo}>
            <div className={styles.settingNavTitle}>{t("Profile.k91")}</div>
            <div className={styles.settingNavDesc}>{t("Profile.k92")}</div>
          </div>
          <div className={styles.settingNavArrow}>›</div>
        </div>

        <div className={styles.settingNavCard} style={{
        marginBottom: 0,
        marginTop: 0
      }} onClick={() => setSettingSubPage('account-temp')}>
          <div className={styles.settingNavIcon}>⏱</div>
          <div className={styles.settingNavInfo}>
            <div className={styles.settingNavTitle}>{t("Profile.k93")}</div>
            <div className={styles.settingNavDesc}>{t("Profile.k94")}</div>
          </div>
          <div className={styles.settingNavArrow}>›</div>
        </div>

        <div className={styles.settingNavCard} style={{
        marginBottom: 0,
        marginTop: 0
      }} onClick={() => setSettingSubPage('account-2fa')}>
          <div className={styles.settingNavIcon}>🔒</div>
          <div className={styles.settingNavInfo}>
            <div className={styles.settingNavTitle}>{t("LoginModal.k70")}</div>
            <div className={styles.settingNavDesc}>{t("Profile.k95")}</div>
          </div>
          <div className={styles.settingNavArrow}>›</div>
        </div>

        <div className={styles.settingNavCard} style={{
        marginBottom: 0,
        marginTop: 0
      }} onClick={() => setSettingSubPage('account-sessions')}>
          <div className={styles.settingNavIcon}>📱</div>
          <div className={styles.settingNavInfo}>
            <div className={styles.settingNavTitle}>{t("Linux.k15")}</div>
            <div className={styles.settingNavDesc}>{t("Profile.k96")}</div>
          </div>
          <div className={styles.settingNavArrow}>›</div>
        </div>
      </div>
    </div>;
  const renderAccountEditPage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => setSettingSubPage('account')}>
          {t("Intelligence.k2")}
        </button>
        <div className={styles.infoTitle}>{t("Profile.k91")}</div>
      </div>

      {isTempAccount && <div className={styles.tempAccountInfo}>
          <div className={styles.tempAccountTitle}>{t("Profile.k97")}</div>
          <div className={styles.tempAccountText}>
            {t("Profile.k98")}
          </div>
        </div>}

      <div className={styles.editSectionTitle}>{t("Profile.k99")}</div>
      <div className={styles.formGroup} style={{
      marginBottom: 16
    }}>
        <label className={styles.formLabel}>{t("Profile.k100")}</label>
        <input type="text" value={user?.username ?? ''} disabled className={styles.formInput} style={{
        opacity: 0.5
      }} />
      </div>
      <div className={styles.formGroup} style={{
      marginBottom: 14
    }}>
        <label className={styles.formLabel}>{t("Profile.k101")}</label>
        <input type="text" value={editUsername} onChange={e => setEditUsername(e.target.value)} placeholder={t("Profile.k102")} className={styles.formInput} disabled={isTempAccount || isLoading} />
      </div>
      <button className={styles.primaryButton} onClick={handleUpdateUsername} disabled={isTempAccount || isLoading} style={{
      padding: '8px 20px',
      fontSize: 13,
      marginBottom: 28
    }}>
        {isLoading ? t("components.AudioEditor.k6") : t("Profile.k103")}
      </button>

      <div className={styles.editSectionDivider} />

      <div className={styles.editSectionTitle}>{t("Profile.k104")}</div>
      <div className={styles.formGroup} style={{
      marginBottom: 12
    }}>
        <label className={styles.formLabel}>{t("Profile.k105")}</label>
        <input type="password" value={oldPassword} onChange={e => setOldPassword(e.target.value)} placeholder={t("Profile.k106")} className={styles.formInput} disabled={isTempAccount || isLoading} />
      </div>
      <div className={styles.formGroup} style={{
      marginBottom: 12
    }}>
        <label className={styles.formLabel}>{t("Profile.k107")}</label>
        <input type="password" value={newPassword} onChange={e => setNewPassword(e.target.value)} placeholder={t("Profile.k108")} className={styles.formInput} disabled={isTempAccount || isLoading} />
      </div>
      <div className={styles.formGroup} style={{
      marginBottom: 14
    }}>
        <label className={styles.formLabel}>{t("LoginModal.k47")}</label>
        <input type="password" value={confirmPassword} onChange={e => setConfirmPassword(e.target.value)} placeholder={t("LoginModal.k48")} className={styles.formInput} disabled={isTempAccount || isLoading} />
      </div>
      <button className={styles.primaryButton} onClick={handlePasswordChange} disabled={isTempAccount || isLoading} style={{
      padding: '8px 20px',
      fontSize: 13
    }}>
        {isLoading ? t("Profile.k109") : t("Profile.k110")}
      </button>
    </div>;
  const renderAccountTempPage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => setSettingSubPage('account')}>
          {t("Intelligence.k2")}
        </button>
        <div className={styles.infoTitle}>{t("Profile.k93")}</div>
      </div>

      <div className={styles.editSectionTitle}>{t("LoginModal.k63")}</div>
      <div className={styles.addSourceForm} style={{
      animation: 'none'
    }}>
        <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
          <label className={styles.formLabel}>{t("Profile.k111")}</label>
          <input type="text" value={tempForm.username} onChange={e => setTempForm(prev => ({
          ...prev,
          username: e.target.value
        }))} placeholder={t("Profile.k112")} className={styles.formInput} />
        </div>
        <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
          <label className={styles.formLabel}>{t("LoginModal.k59")}</label>
          <select value={tempForm.duration} onChange={e => setTempForm(prev => ({
          ...prev,
          duration: e.target.value as '1h' | '24h' | '7d'
        }))} className={styles.formInput}>
            <option value="1h">{t("Profile.k113")}</option>
            <option value="24h">{t("Profile.k114")}</option>
            <option value="7d">{t("Profile.k115")}</option>
          </select>
        </div>
        <button className={styles.primaryButton} onClick={createTempAccount} disabled={isLoading} style={{
        padding: '10px 24px',
        fontSize: 13
      }}>
          {isLoading ? t("game.GamePreview.k5") : t("Profile.k116")}
        </button>
      </div>
    </div>;
  const renderAccount2faPage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => setSettingSubPage('account')}>
          {t("Intelligence.k2")}
        </button>
        <div className={styles.infoTitle}>{t("Profile.k117")}</div>
      </div>

      {fa2Message && <div style={{
      padding: '8px 12px',
      marginBottom: 12,
      background: fa2Message.includes(t("common.success")) ? 'rgba(0,255,0,0.1)' : 'rgba(255,0,0,0.1)',
      border: `1px solid ${fa2Message.includes(t("common.success")) ? '#00FF00' : '#FF0000'}40`,
      borderRadius: 4,
      color: fa2Message.includes(t("common.success")) ? '#00FF00' : '#FF0000',
      fontSize: 13
    }}>
          {fa2Message}
        </div>}

      {!fa2SetupData && !fa2Enabled && <div>
          <div className={styles.editSectionTitle}>{t("Profile.k118")}</div>
          <p style={{
        color: '#888',
        fontSize: 13,
        marginBottom: 14
      }}>
            {t("Profile.k119")}
          </p>
          <button className={styles.primaryButton} onClick={handle2faSetup} disabled={fa2Loading} style={{
        padding: '10px 24px',
        fontSize: 13
      }}>
            {fa2Loading ? t("components.intelligence.DashboardPanel.k67") : t("Profile.k120")}
          </button>
        </div>}

      {fa2SetupData && !fa2Enabled && <div>
          <div className={styles.editSectionTitle}>{t("Profile.k121")}</div>
          <div style={{
        background: '#fff',
        padding: 16,
        borderRadius: 8,
        display: 'inline-block',
        marginBottom: 14
      }}>
            <img src={fa2SetupData.qr_code_url} alt="2FA QR Code" style={{
          width: 180,
          height: 180
        }} />
          </div>
          <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
            <label className={styles.formLabel}>{t("Profile.k122")}</label>
            <input type="text" value={fa2SetupData.secret} readOnly className={styles.formInput} style={{
          opacity: 0.8
        }} />
          </div>
          <div className={styles.formGroup} style={{
        marginBottom: 14
      }}>
            <label className={styles.formLabel}>{t("Profile.k123")}</label>
            <input type="text" value={fa2VerifyCode} onChange={e => setFa2VerifyCode(e.target.value)} placeholder={t("LoginModal.k66")} maxLength={6} className={styles.formInput} autoFocus />
          </div>
          <button className={styles.primaryButton} onClick={handle2faVerify} disabled={fa2Loading || fa2VerifyCode.length !== 6} style={{
        padding: '10px 24px',
        fontSize: 13
      }}>
            {fa2Loading ? t("LoginModal.k49") : t("Profile.k124")}
          </button>
        </div>}

      {fa2Enabled && <div>
          <div className={styles.editSectionTitle}>{t("Profile.k125")}</div>
          <p style={{
        color: '#00FF00',
        fontSize: 13,
        marginBottom: 14
      }}>
            {t("Profile.k126")}
          </p>
          <div className={styles.formGroup} style={{
        marginBottom: 14
      }}>
            <label className={styles.formLabel}>{t("Profile.k127")}</label>
            <input type="text" value={fa2VerifyCode} onChange={e => setFa2VerifyCode(e.target.value)} placeholder={t("LoginModal.k66")} maxLength={6} className={styles.formInput} />
          </div>
          <button className={styles.dangerButton} onClick={handle2faDisable} disabled={fa2Loading || fa2VerifyCode.length !== 6} style={{
        padding: '10px 24px',
        fontSize: 13
      }}>
            {fa2Loading ? t("Profile.k109") : t("Profile.k128")}
          </button>
        </div>}
    </div>;
  const renderAccountSessionsPage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => {
        setSettingSubPage('account');
        setSessions([]);
        setSessionsMessage('');
      }}>
          {t("Intelligence.k2")}
        </button>
        <div className={styles.infoTitle}>{t("Profile.k129")}</div>
      </div>

      {sessionsMessage && <div style={{
      padding: '8px 12px',
      marginBottom: 12,
      background: sessionsMessage.includes(t("common.undo")) || sessionsMessage.includes(t("common.success")) ? 'rgba(0,255,0,0.1)' : 'rgba(255,0,0,0.1)',
      border: `1px solid ${sessionsMessage.includes(t("common.undo")) || sessionsMessage.includes(t("common.success")) ? '#00FF00' : '#FF0000'}40`,
      borderRadius: 4,
      color: sessionsMessage.includes(t("common.undo")) || sessionsMessage.includes(t("common.success")) ? '#00FF00' : '#FF0000',
      fontSize: 13
    }}>
          {sessionsMessage}
        </div>}

      <button className={styles.primaryButton} onClick={handleSessionList} disabled={sessionsLoading} style={{
      padding: '8px 20px',
      fontSize: 13,
      marginBottom: 16
    }}>
        {sessionsLoading ? t("common.loading") : t("Profile.k130")}
      </button>

      {sessions.length === 0 && !sessionsLoading && <div style={{
      color: '#888',
      fontSize: 13,
      textAlign: 'center',
      padding: 20
    }}>
          {t("Profile.k131")}
        </div>}

      {sessions.map((s: any) => <div key={s.session_id || s.id} style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '10px 12px',
      marginBottom: 8,
      background: 'rgba(0,255,0,0.03)',
      border: '1px solid rgba(0,255,0,0.15)',
      borderRadius: 6
    }}>
          <div>
            <div style={{
          color: '#00FF00',
          fontSize: 13,
          fontWeight: 600
        }}>
              {s.device || t("Profile.k132")}
            </div>
            <div style={{
          color: '#888',
          fontSize: 11,
          marginTop: 2
        }}>
              IP: {s.ip || '127.0.0.1'} {t("Profile.k133")} {s.session_id || s.id}
            </div>
            <div style={{
          color: '#666',
          fontSize: 10,
          marginTop: 2
        }}>
              {t("Profile.k134")} {new Date((s.created_at || 0) * 1000).toLocaleString()}
              {s.expires_at && t("Profile.k135", {
            arg0: new Date((s.expires_at || 0) * 1000).toLocaleString()
          })}
            </div>
          </div>
          <button onClick={() => handleSessionRevoke(s.session_id || s.id)} disabled={sessionsLoading} style={{
        padding: '4px 14px',
        background: 'rgba(255,0,0,0.1)',
        border: '1px solid rgba(255,0,0,0.3)',
        color: '#FF0000',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12,
        fontFamily: 'inherit'
      }}>
            {t("common.undo")}
          </button>
        </div>)}
    </div>;
  const renderNewsSourcePage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => setSettingSubPage(null)}>
          {t("Intelligence.k2")}
        </button>
        <div className={styles.infoTitle}>{t("Profile.k136")}</div>
      </div>

      <button className={styles.primaryButton} onClick={() => setShowAddSource(!showAddSource)} style={{
      padding: '8px 16px',
      fontSize: 13,
      marginBottom: 14
    }}>
        {showAddSource ? t("common.cancel") : t("Profile.k137")}
      </button>

      {showAddSource && <div className={styles.addSourceForm}>
          <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
            <label className={styles.formLabel}>{t("game3d.components.UI.BuildingDetail.k13")}</label>
            <input type="text" value={newSource.name} onChange={e => setNewSource(prev => ({
          ...prev,
          name: e.target.value
        }))} placeholder={t("Profile.k138")} className={styles.formInput} />
          </div>
          <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
            <label className={styles.formLabel}>RSS/Atom URL</label>
            <input type="text" value={newSource.url} onChange={e => setNewSource(prev => ({
          ...prev,
          url: e.target.value
        }))} placeholder="https://example.com/feed" className={styles.formInput} />
          </div>
          <div style={{
        display: 'flex',
        gap: 12
      }}>
            <div className={styles.formGroup} style={{
          flex: 1,
          marginBottom: 0
        }}>
              <label className={styles.formLabel}>{t("CommandManual.k223")}</label>
              <select value={newSource.category} onChange={e => setNewSource(prev => ({
            ...prev,
            category: e.target.value
          }))} className={styles.formInput}>
                <option value="security">{t("home.utils.k5")}</option>
                <option value="ai">AI</option>
                <option value="programming">{t("home.utils.k6")}</option>
              </select>
            </div>
            <div className={styles.formGroup} style={{
          flex: 1,
          marginBottom: 0
        }}>
              <label className={styles.formLabel}>{t("common.type")}</label>
              <select value={newSource.feedType} onChange={e => setNewSource(prev => ({
            ...prev,
            feedType: e.target.value
          }))} className={styles.formInput}>
                <option value="rss">RSS</option>
                <option value="atom">Atom</option>
              </select>
            </div>
          </div>
          <div style={{
        marginTop: 14,
        display: 'flex',
        gap: 10
      }}>
            <button className={styles.primaryButton} onClick={handleAddNewsSource} disabled={isLoading} style={{
          padding: '8px 20px',
          fontSize: 13
        }}>
              {isLoading ? t("components.AudioEditor.k6") : t("profile.QuotePanel.k12")}
            </button>
            <button className={styles.dangerButton} onClick={() => {
          setShowAddSource(false);
          setNewSource({
            name: '',
            url: '',
            category: 'security',
            feedType: 'rss'
          });
        }} style={{
          padding: '8px 20px',
          fontSize: 13
        }}>
              {t("common.cancel")}
            </button>
          </div>
        </div>}

      {newsSourcesLoading && <div className={styles.infoContent}>{t("common.loading")}</div>}

      <div className={styles.newsSourceList}>
        {newsSources.map(source => {
        const catInfo = getCategoryLabel(source.category);
        return <div key={source.id} className={styles.newsSourceItem}>
              <div className={styles.newsSourceInfo}>
                <div className={styles.newsSourceNameRow}>
                  <span className={styles.newsSourceName}>{source.name}</span>
                  <span className={styles.newsSourceCategoryBadge} style={{
                background: catInfo.color
              }}>
                    {catInfo.text}
                  </span>
                  <span className={styles.newsSourceType}>{source.feed_type.toUpperCase()}</span>
                </div>
                <div className={styles.newsSourceUrl}>{source.url}</div>
              </div>
              <button className={styles.deleteSourceBtn} onClick={() => handleDeleteNewsSource(source.id, source.name)} title={t("common.delete")}>
                ✕
              </button>
            </div>;
      })}
        {!newsSourcesLoading && newsSources.length === 0 && <div className={styles.emptySources}>{t("Profile.k139")}</div>}
      </div>

      <div className={styles.newsSourceFooter}>
        {t("components.GroupChatOrchestrationPanel.k26")} {newsSources.length} {t("Profile.k140")}
      </div>

      {/* 统一确认弹窗（多场景复用） */}
      <ConfirmDialog isOpen={confirmDialog.isOpen} onClose={() => setConfirmDialog({
      ...confirmDialog,
      isOpen: false
    })} onConfirm={confirmDialog.onConfirm} targetName={confirmDialog.targetName} type="danger" />
    </div>;
  const renderThemePage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => setSettingSubPage(null)}>
          {t("Intelligence.k2")}
        </button>
        <div className={styles.infoTitle}>{t("Profile.k85")}</div>
      </div>
      <ThemeSelector />
      <LanguageSelector />
    </div>;
  const renderWindowControlPage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => setSettingSubPage(null)}>
          {t("Intelligence.k2")}
        </button>
        <div className={styles.infoTitle}>{t("Profile.k149")}</div>
      </div>
      <TitleBarSettingsPanel />
    </div>;

  const renderSettings = () => {
    if (settingSubPage === 'account') {
      return renderAccountSettingsPage();
    }
    if (settingSubPage === 'account-edit') {
      return renderAccountEditPage();
    }
    if (settingSubPage === 'account-temp') {
      return renderAccountTempPage();
    }
    if (settingSubPage === 'account-2fa') {
      return renderAccount2faPage();
    }
    if (settingSubPage === 'account-sessions') {
      return renderAccountSessionsPage();
    }
    if (settingSubPage === 'newsSource') {
      return renderNewsSourcePage();
    }
    if (settingSubPage === 'theme') {
      return renderThemePage();
    }
    if (settingSubPage === 'windowControl') {
      return renderWindowControlPage();
    }
    return renderSettingList();
  };

  return { settingSubPage, renderSettings };
}
