import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { useAuthStore } from '@/stores/authStore';
import { useNotifStore } from '@/stores/notifStore';
import { useIntelligence } from '@/hooks/useIntelligence';
import { auth, profile, newsSource, intelligence } from '@/lib/ipc';
import { time } from '@/lib/utils';
import ConfirmDialog from '@/components/ConfirmDialog';
import ThemeSelector from '@/components/ThemeSelector/ThemeSelector';
import LanguageSelector from '@/components/LanguageSelector/LanguageSelector';
import { TitleBarSettingsPanel } from '@/components/TitleBar/TitleBarSettingsPanel';
import styles from './Profile.module.css';
import type { ResumeItem, PersonalInfo, ResumeTemplate, QuoteItem, NewsSourceItem } from './profile/types';
import AccountPanel from './profile/AccountPanel';
import ResumePanel from './profile/ResumePanel';
import QuotePanel from './profile/QuotePanel';
export default function Profile() {
  const location = useLocation();
  const navigate = useNavigate();
  const pathname = location.pathname;
  const searchParams = new URLSearchParams(location.search);
  const queryTab = searchParams.get('tab');
  // 优先使用 ?tab= 查询参数（Layout 全局侧边栏导航），其次用路径分段
  const rawTab = queryTab || (pathname === '/profile' ? 'account' : pathname.split('/').pop() || 'account');
  const validTabs = ['account', 'resume', 'quote', 'setting'];
  const currentTab = validTabs.includes(rawTab) ? rawTab : 'account';
  const {
    user,
    isAuthenticated
  } = useAuthStore();
  const isTempAccount = user?.is_permanent === false;
  const [editUsername, setEditUsername] = useState('');
  const [oldPassword, setOldPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const addToast = useNotifStore(s => s.addToast);

  /** 统一通知：桥接旧 showNotification → 全局 notifStore */
  const showNotify = (type: 'success' | 'error' | 'info', message: string) => {
    addToast({
      type,
      title: message
    });
  };
  useEffect(() => {
    if (user?.username) {
      setEditUsername(user.username);
    }
  }, [user?.username]);
  const [resumes, setResumes] = useState<ResumeItem[]>([]);
  const [resumesLoading, setResumesLoading] = useState(false);
  const [showAddResume, setShowAddResume] = useState(false);
  const [editingResumeId, setEditingResumeId] = useState<number | null>(null);
  const [resumeForm, setResumeForm] = useState({
    title: '',
    content: ''
  });
  const [resumeSubPage, setResumeSubPage] = useState<string | null>(null);
  const [viewingResumeId, setViewingResumeId] = useState<number | null>(null);
  const [editingResumeContent, setEditingResumeContent] = useState('');
  const [isEditingResume, setIsEditingResume] = useState(false);
  const defaultPersonalInfo: PersonalInfo = {
    name: '',
    gender: '',
    birthDate: '',
    phone: '',
    email: '',
    location: '',
    education: [],
    workExperience: [],
    projects: [],
    skills: '',
    selfEvaluation: '',
    certificates: ''
  };
  const [personalInfo, setPersonalInfo] = useState<PersonalInfo>(defaultPersonalInfo);
  const savePersonalInfo = async (info: PersonalInfo) => {
    // BUG-008 v1: 邮箱/电话格式验证
    if (info.email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(info.email)) {
      showNotify('error', t("Profile.k50") || '邮箱格式不正确');
      return;
    }
    if (info.phone && !/^[\d\s\-+()]{7,20}$/.test(info.phone)) {
      showNotify('error', t("Profile.k51") || '电话格式不正确');
      return;
    }
    setPersonalInfo(info);
    try {
      await profile.savePersonalInfo(JSON.stringify(info));
    } catch (err) {
      console.error('保存个人信息失败:', err);
    }
  };
  const fetchPersonalInfo = useCallback(async () => {
    try {
      const response = await profile.getPersonalInfo();
      if (response.code === 0 && response.data) {
        setPersonalInfo(JSON.parse(response.data));
      }
    } catch (err) {
      console.error('获取个人信息失败:', err);
    }
  }, []);
  const [selectedTemplate, setSelectedTemplate] = useState<string>('professional');
  const [templateSupplement, setTemplateSupplement] = useState<Partial<PersonalInfo>>({});
  const resumeTemplates: ResumeTemplate[] = [{
    id: 'professional',
    name: t("Profile.k1"),
    description: t("Profile.k2"),
    icon: '💼'
  }, {
    id: 'creative',
    name: t("Profile.k3"),
    description: t("Profile.k4"),
    icon: '🎨'
  }, {
    id: 'academic',
    name: t("Profile.k5"),
    description: t("Profile.k6"),
    icon: '📚'
  }, {
    id: 'technical',
    name: t("Profile.k7"),
    description: t("Profile.k8"),
    icon: '⚙️'
  }];
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
  const [quotes, setQuotes] = useState<QuoteItem[]>([]);
  const [quotesLoading, setQuotesLoading] = useState(false);
  const [showAddQuote, setShowAddQuote] = useState(false);
  const [quoteForm, setQuoteForm] = useState({
    content: '',
    source: ''
  });
  const [duplicateResults, setDuplicateResults] = useState<any[]>([]);
  const [showDuplicateDialog, setShowDuplicateDialog] = useState(false);

  // ===== 统一确认弹窗（多场景复用） =====
  const [confirmDialog, setConfirmDialog] = useState<{
    isOpen: boolean;
    targetName: string;
    onConfirm: () => void | Promise<void>;
  }>({
    isOpen: false,
    targetName: '',
    onConfirm: () => {}
  });
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
  const {
    aiOn,
    featureOn,
    llmConfigured
  } = useIntelligence();

  // 简历智能辅助
  const [resumePolishLoading, setResumePolishLoading] = useState(false);
  const [resumePolishResult, setResumePolishResult] = useState<{
    text: string;
    changes: string[];
    suggestions: string[];
  } | null>(null);
  const handleResumeSpellCheck = useCallback(async () => {
    if (!resumeForm.content.trim()) return;
    setResumePolishLoading(true);
    setResumePolishResult(null);
    try {
      const res = await intelligence.resumeSpellCheck(resumeForm.content);
      if (res?.data) {
        setResumePolishResult({
          text: res.data.polished,
          changes: res.data.changes,
          suggestions: res.data.suggestions
        });
      }
    } catch {
      // ignore
    } finally {
      setResumePolishLoading(false);
    }
  }, [resumeForm.content]);
  const handleResumePolish = useCallback(async () => {
    if (!resumeForm.content.trim()) return;
    setResumePolishLoading(true);
    setResumePolishResult(null);
    try {
      // Use selection if available, otherwise polish entire content
      const textEl = document.querySelector('textarea') as HTMLTextAreaElement | null;
      const selectedText = textEl && textEl.selectionStart !== textEl.selectionEnd ? textEl.value.substring(textEl.selectionStart, textEl.selectionEnd) : null;
      const text = selectedText || resumeForm.content;
      const res = await intelligence.resumePolish(text);
      if (res?.data) {
        const polished = res.data.polished;
        setResumePolishResult({
          text: selectedText ? resumeForm.content.substring(0, textEl!.selectionStart) + polished + resumeForm.content.substring(textEl!.selectionEnd) : polished,
          changes: res.data.changes?.length ? res.data.changes : [selectedText ? t("Profile.k13") : t("Profile.k14")],
          suggestions: res.data.suggestions || []
        });
      }
    } catch {
      // ignore
    } finally {
      setResumePolishLoading(false);
    }
  }, [resumeForm.content]);
  const applyResumeResult = useCallback(() => {
    if (resumePolishResult) {
      setResumeForm(prev => ({
        ...prev,
        content: resumePolishResult.text
      }));
      setResumePolishResult(null);
    }
  }, [resumePolishResult]);
  const dismissResumeResult = useCallback(() => {
    setResumePolishResult(null);
  }, []);
  const [inlineAnnotation, setInlineAnnotation] = useState<string | null>(null);
  const [annotationLoading, setAnnotationLoading] = useState(false);
  const [quoteGhostText, setQuoteGhostText] = useState('');
  const [ghostLoading, setGhostLoading] = useState(false);
  // 语录智能辅助
  const [quoteCheckResult, setQuoteCheckResult] = useState<{
    field: string;
    issue: string;
    suggestion: string | null;
    severity: string;
  } | null>(null);
  const [quoteCheckLoading, setQuoteCheckLoading] = useState(false);
  const handleQuoteSpellCheck = useCallback(async () => {
    if (!quoteForm.content.trim()) return;
    setQuoteCheckLoading(true);
    setQuoteCheckResult(null);
    try {
      const res = await intelligence.quoteSpellCheck(quoteForm.content);
      if (res?.data) setQuoteCheckResult(res.data);
    } catch {/* ignore */} finally {
      setQuoteCheckLoading(false);
    }
  }, [quoteForm.content]);
  const handleQuoteSourceVerify = useCallback(async () => {
    if (!quoteForm.source.trim()) return;
    setQuoteCheckLoading(true);
    setQuoteCheckResult(null);
    try {
      const res = await intelligence.quoteSourceVerify(quoteForm.source, undefined);
      if (res?.data) setQuoteCheckResult(res.data);
    } catch {/* ignore */} finally {
      setQuoteCheckLoading(false);
    }
  }, [quoteForm.source]);
  const handleQuoteSmartComplete = useCallback(async () => {
    if (!quoteForm.content.trim()) return;
    setQuoteCheckLoading(true);
    setQuoteCheckResult(null);
    try {
      const res = await intelligence.quoteSmartComplete(quoteForm.content, quoteForm.source || undefined, undefined);
      if (res?.data) setQuoteCheckResult(res.data);
    } catch {/* ignore */} finally {
      setQuoteCheckLoading(false);
    }
  }, [quoteForm.content, quoteForm.source]);
  const annotationTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const ghostTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fetchResumes = useCallback(async () => {
    setResumesLoading(true);
    try {
      const response = await profile.getResumes();
      if (response.code === 0 && response.data) {
        setResumes(response.data);
      }
    } catch (err) {
      console.error('获取简历失败:', err);
    } finally {
      setResumesLoading(false);
    }
  }, []);
  const fetchNewsSources = useCallback(async () => {
    setNewsSourcesLoading(true);
    try {
      const response = await newsSource.getNewsSources();
      if (response.code === 0 && response.data) {
        setNewsSources(response.data);
      }
    } catch (err) {
      console.error('获取新闻源失败:', err);
    } finally {
      setNewsSourcesLoading(false);
    }
  }, []);
  const fetchAllQuotes = useCallback(async () => {
    setQuotesLoading(true);
    try {
      const response = await profile.getAllQuotes();
      if (response.code === 0 && response.data) {
        setQuotes(response.data);
      }
    } catch (err) {
      console.error('获取语录列表失败:', err);
    } finally {
      setQuotesLoading(false);
    }
  }, []);
  useEffect(() => {
    if (currentTab === 'resume') {
      fetchResumes();
      fetchPersonalInfo();
    } else if (currentTab === 'setting' && settingSubPage === 'newsSource') {
      fetchNewsSources();
    } else if (currentTab === 'quote') {
      profile.seedDefaultQuotes().then(() => fetchAllQuotes());
    }
  }, [currentTab, settingSubPage, fetchResumes, fetchNewsSources, fetchAllQuotes, fetchPersonalInfo]);
  const handleAddResume = async () => {
    if (!resumeForm.title.trim() || !resumeForm.content.trim()) {
      showNotify('error', t("Profile.k15"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await profile.addResume(resumeForm.title, resumeForm.content);
      if (response.code === 0) {
        setResumeForm({
          title: '',
          content: ''
        });
        setShowAddResume(false);
        await fetchResumes();
        showNotify('success', t("Profile.k16"));
      } else {
        showNotify('error', response.message || t("components.PptEditor.k4"));
      }
    } catch (err) {
      console.error('添加简历失败:', err);
      showNotify('error', t("Profile.k17"));
    } finally {
      setIsLoading(false);
    }
  };
  const handleDeleteQuote = async (id: number) => {
    try {
      const response = await profile.deleteQuote(id);
      if (response.code === 0) {
        await fetchAllQuotes();
        showNotify('success', t("Profile.k18"));
      }
    } catch (err) {
      console.error('删除语录失败:', err);
    }
  };
  const handleUpdateResume = async () => {
    if (!resumeForm.title.trim() || !resumeForm.content.trim() || editingResumeId === null) {
      showNotify('error', t("Profile.k15"));
      return;
    }
    setIsLoading(true);
    try {
      const response = await profile.updateResume(editingResumeId, resumeForm.title, resumeForm.content);
      if (response.code === 0) {
        setResumeForm({
          title: '',
          content: ''
        });
        setEditingResumeId(null);
        setShowAddResume(false);
        await fetchResumes();
        showNotify('success', t("Profile.k19"));
      } else {
        showNotify('error', response.message || t("Knowledge.k40"));
      }
    } catch (err) {
      console.error('更新简历失败:', err);
      showNotify('error', t("Profile.k20"));
    } finally {
      setIsLoading(false);
    }
  };
  const handleDeleteResume = async (id: number, title?: string) => {
    setConfirmDialog({
      isOpen: true,
      targetName: title || t("Profile.k21"),
      onConfirm: async () => {
        setIsLoading(true);
        try {
          const response = await profile.deleteResume(id);
          if (response.code === 0) {
            await fetchResumes();
            showNotify('success', t("Profile.k22"));
          } else {
            showNotify('error', response.message || t("errors.deleteFailed"));
          }
        } catch (err) {
          console.error('删除简历失败:', err);
          showNotify('error', t("Profile.k23"));
        } finally {
          setIsLoading(false);
        }
      }
    });
  };
  const generateResumeFromTemplate = async () => {
    const info = {
      ...personalInfo,
      ...templateSupplement
    };
    const template = resumeTemplates.find(t => t.id === selectedTemplate);
    let content = '';
    if (selectedTemplate === 'professional') {
      content = t("Profile.k24", {
        arg0: info.name || t("common.name"),
        arg1: info.phone ? ` | ${info.phone}` : '',
        arg2: info.email ? ` | ${info.email}` : '',
        arg3: info.location || '',
        arg4: info.education.map(e => `${e.school} | ${e.major} | ${e.degree}\n  ${e.startDate} - ${e.endDate}`).join('\n\n') || t("Profile.k25"),
        arg5: info.workExperience.map(w => `${w.company} | ${w.position}\n  ${w.startDate} - ${w.endDate}\n  ${w.description}`).join('\n\n') || t("Profile.k25"),
        arg6: info.projects.map(p => t("Profile.k26", {
          name: p.name,
          role: p.role,
          startDate: p.startDate,
          endDate: p.endDate,
          description: p.description,
          technologies: p.technologies
        })).join('\n\n') || t("Profile.k25"),
        arg7: info.skills || t("Profile.k25"),
        arg8: info.selfEvaluation || t("Profile.k25"),
        arg9: info.certificates ? t("Profile.k27", {
          certificates: info.certificates
        }) : ''
      });
    } else if (selectedTemplate === 'creative') {
      content = t("Profile.k28", {
        arg0: info.name || t("Profile.k29"),
        arg1: info.location || '',
        arg2: info.phone || '',
        arg3: info.email || '',
        arg4: info.selfEvaluation || t("Profile.k30"),
        arg5: info.education.map(e => t("Profile.k31", {
          school: e.school,
          major: e.major,
          degree: e.degree,
          startDate: e.startDate,
          endDate: e.endDate
        })).join('\n\n') || t("Profile.k32"),
        arg6: info.workExperience.map(w => t("Profile.k33", {
          company: w.company,
          position: w.position,
          startDate: w.startDate,
          endDate: w.endDate,
          description: w.description
        })).join('\n\n') || t("Profile.k32"),
        arg7: info.projects.map(p => `◈ ${p.name} — ${p.role}\n   ${p.technologies}\n   ${p.description}`).join('\n\n') || t("Profile.k32"),
        arg8: info.skills || t("Profile.k34"),
        arg9: info.certificates ? t("Profile.k35", {
          certificates: info.certificates
        }) : ''
      });
    } else if (selectedTemplate === 'academic') {
      content = t("Profile.k36", {
        arg0: info.name || t("common.name"),
        arg1: info.email || '',
        arg2: info.phone || '',
        arg3: info.selfEvaluation || t("Profile.k37"),
        arg4: info.education.map(e => t("Profile.k38", {
          school: e.school,
          major: e.major,
          degree: e.degree,
          startDate: e.startDate,
          endDate: e.endDate
        })).join('\n\n') || t("Profile.k25"),
        arg5: info.workExperience.map(w => `${w.company} | ${w.position}\n${w.startDate} — ${w.endDate}\n${w.description}`).join('\n\n') || t("Profile.k25"),
        arg6: info.projects.map(p => t("Profile.k39", {
          name: p.name,
          role: p.role,
          startDate: p.startDate,
          endDate: p.endDate,
          description: p.description,
          technologies: p.technologies
        })).join('\n\n') || t("Profile.k25"),
        arg7: info.skills || t("Profile.k25"),
        arg8: info.certificates ? t("Profile.k40", {
          certificates: info.certificates
        }) : ''
      });
    } else {
      content = t("Profile.k41", {
        arg0: info.name || t("common.name"),
        arg1: info.phone || '',
        arg2: info.location || '',
        arg3: info.skills.split(/[,，、]/).map(s => s.trim()).filter(Boolean).map(s => `• ${s}`).join('\n') || t("Profile.k37"),
        arg4: info.workExperience.map(w => `${w.company} // ${w.position}\n[${w.startDate} — ${w.endDate}]\n${w.description}`).join('\n\n') || t("Profile.k25"),
        arg5: info.projects.map(p => `${p.name} (${p.role})\n[${p.startDate} — ${p.endDate}]\nTech: ${p.technologies}\n${p.description}`).join('\n\n') || t("Profile.k25"),
        arg6: info.education.map(e => `${e.school} — ${e.major} (${e.degree}) [${e.startDate}—${e.endDate}]`).join('\n') || t("Profile.k25"),
        arg7: info.selfEvaluation ? t("Profile.k42", {
          selfEvaluation: info.selfEvaluation
        }) : '',
        arg8: info.certificates ? t("Profile.k43", {
          certificates: info.certificates
        }) : ''
      });
    }
    const title = `${template?.name || t("components.intelligence.ActivityPanel.k5")} - ${info.name || t("Knowledge.k94")}`;
    setIsLoading(true);
    try {
      const response = await profile.addResume(title, content);
      if (response.code === 0) {
        await fetchResumes();
        showNotify('success', t("Profile.k44"));
      } else {
        showNotify('error', response.message || t("Profile.k45"));
      }
    } catch (err) {
      console.error('生成简历失败:', err);
      showNotify('error', t("Profile.k46"));
    } finally {
      setIsLoading(false);
    }
  };
  const openResumeView = (id: number) => {
    const resume = resumes.find(r => r.id === id);
    if (resume) {
      setViewingResumeId(id);
      setEditingResumeContent(resume.content);
      setIsEditingResume(false);
    }
  };
  const saveEditedResume = async () => {
    if (viewingResumeId === null) return;
    setIsLoading(true);
    try {
      await profile.updateResume(viewingResumeId, resumes.find(r => r.id === viewingResumeId)?.title || '', editingResumeContent);
      await fetchResumes();
      showNotify('success', t("Profile.k47"));
    } catch {
      showNotify('error', t("errors.saveFailed"));
    } finally {
      setIsLoading(false);
    }
  };

  // === Passive AI: auto-spell-check when editing resume (debounced 1.5s) ===
  useEffect(() => {
    if (!aiOn || !featureOn('smart_complete') || !isEditingResume || !editingResumeContent.trim()) {
      setInlineAnnotation(null);
      return;
    }
    if (annotationTimerRef.current) clearTimeout(annotationTimerRef.current);
    setAnnotationLoading(true);
    annotationTimerRef.current = setTimeout(async () => {
      try {
        const resume = resumes.find(r => r.id === viewingResumeId);
        if (!resume) return;
        const res = await profile.aiSpellCheckResume(resume.id, editingResumeContent);
        if (res.code === 0 && res.data) {
          setInlineAnnotation(res.data.content);
        } else {
          setInlineAnnotation(null);
        }
      } catch {
        setInlineAnnotation(null);
      } finally {
        setAnnotationLoading(false);
      }
    }, 1500);
    return () => {
      if (annotationTimerRef.current) clearTimeout(annotationTimerRef.current);
    };
  }, [editingResumeContent, isEditingResume, aiOn, featureOn]);

  // === Passive AI: auto-complete ghost text when typing quote (debounced 800ms) ===
  useEffect(() => {
    if (!aiOn || !featureOn('smart_complete') || !showAddQuote || quoteForm.content.length < 4) {
      setQuoteGhostText('');
      return;
    }
    if (ghostTimerRef.current) clearTimeout(ghostTimerRef.current);
    setGhostLoading(true);
    ghostTimerRef.current = setTimeout(async () => {
      try {
        const res = await profile.aiQuoteComplete(quoteForm.content);
        if (res.code === 0 && res.data?.completion) {
          setQuoteGhostText(res.data.completion);
        } else {
          setQuoteGhostText('');
        }
      } catch {
        setQuoteGhostText('');
      } finally {
        setGhostLoading(false);
      }
    }, 800);
    return () => {
      if (ghostTimerRef.current) clearTimeout(ghostTimerRef.current);
    };
  }, [quoteForm.content, showAddQuote, aiOn, featureOn]);
  const exportResumeLocal = async (content: string, title: string) => {
    try {
      const {
        save
      } = await import('@tauri-apps/plugin-dialog');
      const {
        writeTextFile
      } = await import('@tauri-apps/plugin-fs');
      const filePath = await save({
        defaultPath: `${title}.md`,
        filters: [{
          name: 'Markdown',
          extensions: ['md']
        }]
      });
      if (filePath) {
        await writeTextFile(filePath, content);
        showNotify('success', t("Profile.k48", {
          filePath: filePath
        }));
      }
    } catch (err) {
      console.error('导出失败:', err);
      showNotify('error', t("Profile.k49"));
    }
  };
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
      const response = await newsSource.addNewsSource(newSource.name, newSource.url, newSource.category, newSource.feedType);
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
          const response = await newsSource.deleteNewsSource(id);
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
  const handleAddQuote = async () => {
    if (!quoteForm.content.trim()) {
      showNotify('error', t("Profile.k54"));
      return;
    }
    setIsLoading(true);
    try {
      const dupRes = await profile.checkQuoteDuplicate(quoteForm.content);
      if (dupRes.code === 0 && dupRes.data && dupRes.data.length > 0) {
        setDuplicateResults(dupRes.data);
        setShowDuplicateDialog(true);
        return;
      }
      const response = await profile.addQuote(quoteForm.content, quoteForm.source || undefined, 'daily');
      if (response.code === 0) {
        setQuoteForm({
          content: '',
          source: ''
        });
        setShowAddQuote(false);
        await fetchAllQuotes();
        showNotify('success', t("Profile.k55"));
      } else {
        showNotify('error', response.message || t("components.PptEditor.k4"));
      }
    } catch (err) {
      console.error('添加语录失败:', err);
      showNotify('error', t("Profile.k17"));
    } finally {
      setIsLoading(false);
    }
  };
  const handleDuplicateKeepExisting = () => {
    setShowDuplicateDialog(false);
    setDuplicateResults([]);
  };
  const handleDuplicateReplaceAll = async () => {
    setIsLoading(true);
    try {
      for (const dup of duplicateResults) {
        await profile.deleteQuote(dup.existing.id);
      }
      const response = await profile.addQuote(quoteForm.content, quoteForm.source || undefined, 'daily');
      if (response.code === 0) {
        setQuoteForm({
          content: '',
          source: ''
        });
        setShowAddQuote(false);
        await fetchAllQuotes();
        showNotify('success', t("Profile.k56"));
      } else {
        showNotify('error', response.message || t("Knowledge.k96"));
      }
    } catch (err) {
      console.error('替换语录失败:', err);
      showNotify('error', t("Profile.k57"));
    } finally {
      setShowDuplicateDialog(false);
      setDuplicateResults([]);
      setIsLoading(false);
    }
  };
  const handleDuplicateAddAnyway = async () => {
    setShowDuplicateDialog(false);
    setDuplicateResults([]);
    setIsLoading(true);
    try {
      const response = await profile.addQuote(quoteForm.content, quoteForm.source || undefined, 'daily');
      if (response.code === 0) {
        setQuoteForm({
          content: '',
          source: ''
        });
        setShowAddQuote(false);
        await fetchAllQuotes();
        showNotify('success', t("Profile.k55"));
      } else {
        showNotify('error', response.message || t("components.PptEditor.k4"));
      }
    } catch (err) {
      console.error('添加语录失败:', err);
      showNotify('error', t("Profile.k17"));
    } finally {
      setIsLoading(false);
    }
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
  const formatTime = (ts: number) => {
    return time.formatUtcToLocal(ts);
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
      const response = await profile.changePassword(oldPassword, newPassword);
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
      const response = await profile.changeUsername(editUsername);
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
      const response = await profile.createTempAccount(tempForm.username || '', tempForm.duration);
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
  if (!isAuthenticated || !user) {
    return <div className={styles.container}>
        <main className={styles.main}>
          <div className={styles.contentSection}>
            <h3 className={styles.sectionTitle}>{t("layout.k9")}</h3>
            <div className={styles.infoCard}>
              <div className={styles.infoContent}>
                <p style={{
                color: '#FF0000'
              }}>{t("Profile.k79")}</p>
              </div>
            </div>
          </div>
        </main>
      </div>;
  }
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
        <input type="text" value={user.username} disabled className={styles.formInput} style={{
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
  const renderSettingContent = () => {
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
  const isInSubPage = resumeSubPage !== null || settingSubPage !== null || viewingResumeId !== null;
  return <div className={`${styles.container} profile-scrollable`}>
      <main className={styles.main}>
        <div className={`${styles.contentSection} profile-scrollable`}>
          {!isInSubPage && <h3 className={styles.sectionTitle} style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between'
        }}>
            {currentTab === 'account' && <span>{t("layout.k9")}</span>}
            {currentTab === 'resume' && <span>{t("components.intelligence.ActivityPanel.k5")}</span>}
            {currentTab === 'quote' && <>
                <span>{t("components.intelligence.ActivityPanel.k6")}</span>
                <button className={styles.editActionBtn} onClick={() => setShowAddQuote(!showAddQuote)} style={{
              padding: '4px 14px',
              height: 32,
              fontSize: 12,
              minWidth: 80
            }}>
                  <span className={styles.btnText}>{showAddQuote ? t("common.cancel") : t("Profile.k148")}</span>
                  <span className={styles.btnIcon}>{showAddQuote ? '✕' : '✎'}</span>
                </button>
              </>}
            {currentTab === 'setting' && <span>{t("common.settings")}</span>}
          </h3>}
          <div className={`${styles.contentText} profile-scrollable`}>
            {currentTab === 'account' && <AccountPanel isTempAccount={isTempAccount} user={user} formatTime={formatTime} resumeCount={resumes.length} quoteCount={quotes.length} kbCount={0} />}
            {currentTab === 'resume' && <ResumePanel resumeSubPage={resumeSubPage} viewingResumeId={viewingResumeId} showAddResume={showAddResume} editingResumeId={editingResumeId} resumeForm={resumeForm} personalInfo={personalInfo} aiOn={aiOn} featureOn={featureOn} llmConfigured={llmConfigured} resumePolishLoading={resumePolishLoading} resumePolishResult={resumePolishResult} isLoading={isLoading} resumes={resumes} resumesLoading={resumesLoading} selectedTemplate={selectedTemplate} resumeTemplates={resumeTemplates} templateSupplement={templateSupplement} isEditingResume={isEditingResume} editingResumeContent={editingResumeContent} annotationLoading={annotationLoading} inlineAnnotation={inlineAnnotation} setResumeSubPage={setResumeSubPage} setShowAddResume={setShowAddResume} setEditingResumeId={setEditingResumeId} setResumeForm={setResumeForm} setSelectedTemplate={setSelectedTemplate} setTemplateSupplement={setTemplateSupplement} setIsEditingResume={setIsEditingResume} setEditingResumeContent={setEditingResumeContent} setViewingResumeId={setViewingResumeId} setInlineAnnotation={setInlineAnnotation} savePersonalInfo={savePersonalInfo} generateResumeFromTemplate={generateResumeFromTemplate} handleResumeSpellCheck={handleResumeSpellCheck} handleResumePolish={handleResumePolish} handleUpdateResume={handleUpdateResume} handleAddResume={handleAddResume} applyResumeResult={applyResumeResult} dismissResumeResult={dismissResumeResult} saveEditedResume={saveEditedResume} openResumeView={openResumeView} exportResumeLocal={exportResumeLocal} handleDeleteResume={handleDeleteResume} formatTime={formatTime} />}
            {currentTab === 'quote' && <QuotePanel showDuplicateDialog={showDuplicateDialog} duplicateResults={duplicateResults} showAddQuote={showAddQuote} quoteForm={quoteForm} quoteCheckLoading={quoteCheckLoading} quoteCheckResult={quoteCheckResult} ghostLoading={ghostLoading} quoteGhostText={quoteGhostText} quotesLoading={quotesLoading} quotes={quotes} isLoading={isLoading} aiOn={aiOn} featureOn={featureOn} setQuoteForm={setQuoteForm} setShowAddQuote={setShowAddQuote} setQuoteGhostText={setQuoteGhostText} setQuoteCheckResult={setQuoteCheckResult} handleDuplicateKeepExisting={handleDuplicateKeepExisting} handleDuplicateReplaceAll={handleDuplicateReplaceAll} handleDuplicateAddAnyway={handleDuplicateAddAnyway} handleQuoteSpellCheck={handleQuoteSpellCheck} handleQuoteSourceVerify={handleQuoteSourceVerify} handleQuoteSmartComplete={handleQuoteSmartComplete} handleAddQuote={handleAddQuote} handleDeleteQuote={handleDeleteQuote} />}
            {currentTab === 'setting' && renderSettingContent()}
          </div>
        </div>
      </main>
    </div>;
}