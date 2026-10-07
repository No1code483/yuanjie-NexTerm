// profile.resume L2 —— 简历功能域（局部 state + handlers + JSX）。
// 物理迁入本目录：ResumePanel 组件与本 useResume hook；核心共享项经 core（ProfileCore）注入，
// 需要刷新/通知走 core 回调，禁止直接修改 L1 核心 state。
// active 表示当前是否处于简历 tab（且已鉴权），用于惰性加载（等价于原壳 currentTab==='resume' 分支）。
import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
import { intelligence } from '@/lib/ipc';
import { profile } from '../../ipc/profile';
import type { ProfileCore } from '../../core';
import type { ResumeItem, PersonalInfo, ResumeTemplate } from '../../types';
import ResumePanel from './ResumePanel';

export function useResume(core: ProfileCore, active: boolean) {
  const { showNotify, setConfirmDialog, setResumeCount, aiOn, featureOn, llmConfigured, formatTime } = core;

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
  const [isLoading, setIsLoading] = useState(false);

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
  const annotationTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const fetchResumes = useCallback(async () => {
    setResumesLoading(true);
    try {
      const response = await profile.getResumes();
      if (response.code === 0 && response.data) {
        setResumes(response.data);
        setResumeCount(response.data.length);
      }
    } catch (err) {
      console.error('获取简历失败:', err);
    } finally {
      setResumesLoading(false);
    }
  }, [setResumeCount]);

  // 简历 tab 激活时加载（等价原壳 currentTab==='resume' 分支）
  useEffect(() => {
    if (active) {
      fetchResumes();
      fetchPersonalInfo();
    }
  }, [active, fetchResumes, fetchPersonalInfo]);

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

  const renderResume = () => <ResumePanel resumeSubPage={resumeSubPage} viewingResumeId={viewingResumeId} showAddResume={showAddResume} editingResumeId={editingResumeId} resumeForm={resumeForm} personalInfo={personalInfo} aiOn={aiOn} featureOn={featureOn} llmConfigured={llmConfigured} resumePolishLoading={resumePolishLoading} resumePolishResult={resumePolishResult} isLoading={isLoading} resumes={resumes} resumesLoading={resumesLoading} selectedTemplate={selectedTemplate} resumeTemplates={resumeTemplates} templateSupplement={templateSupplement} isEditingResume={isEditingResume} editingResumeContent={editingResumeContent} annotationLoading={annotationLoading} inlineAnnotation={inlineAnnotation} setResumeSubPage={setResumeSubPage} setShowAddResume={setShowAddResume} setEditingResumeId={setEditingResumeId} setResumeForm={setResumeForm} setSelectedTemplate={setSelectedTemplate} setTemplateSupplement={setTemplateSupplement} setIsEditingResume={setIsEditingResume} setEditingResumeContent={setEditingResumeContent} setViewingResumeId={setViewingResumeId} setInlineAnnotation={setInlineAnnotation} savePersonalInfo={savePersonalInfo} generateResumeFromTemplate={generateResumeFromTemplate} handleResumeSpellCheck={handleResumeSpellCheck} handleResumePolish={handleResumePolish} handleUpdateResume={handleUpdateResume} handleAddResume={handleAddResume} applyResumeResult={applyResumeResult} dismissResumeResult={dismissResumeResult} saveEditedResume={saveEditedResume} openResumeView={openResumeView} exportResumeLocal={exportResumeLocal} handleDeleteResume={handleDeleteResume} formatTime={formatTime} />;

  return { resumeSubPage, viewingResumeId, renderResume };
}
