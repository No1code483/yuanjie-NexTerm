import { t } from "i18next";
import { useRef, useState } from 'react';
import type { PersonalInfo, ResumeItem, ResumeTemplate } from './types';
import MarkdownRenderer from '@/components/MarkdownRenderer';
import styles from '../Profile.module.css';
interface ResumePanelProps {
  resumeSubPage: string | null;
  viewingResumeId: number | null;
  showAddResume: boolean;
  editingResumeId: number | null;
  resumeForm: {
    title: string;
    content: string;
  };
  personalInfo: PersonalInfo;
  aiOn: boolean;
  featureOn: (feature: string) => boolean;
  llmConfigured: boolean;
  resumePolishLoading: boolean;
  resumePolishResult: {
    text: string;
    changes: string[];
    suggestions: string[];
  } | null;
  isLoading: boolean;
  resumes: ResumeItem[];
  resumesLoading: boolean;
  selectedTemplate: string;
  resumeTemplates: ResumeTemplate[];
  templateSupplement: Partial<PersonalInfo>;
  isEditingResume: boolean;
  editingResumeContent: string;
  annotationLoading: boolean;
  inlineAnnotation: string | null;
  setResumeSubPage: React.Dispatch<React.SetStateAction<string | null>>;
  setShowAddResume: React.Dispatch<React.SetStateAction<boolean>>;
  setEditingResumeId: React.Dispatch<React.SetStateAction<number | null>>;
  setResumeForm: React.Dispatch<React.SetStateAction<{
    title: string;
    content: string;
  }>>;
  setSelectedTemplate: React.Dispatch<React.SetStateAction<string>>;
  setTemplateSupplement: React.Dispatch<React.SetStateAction<Partial<PersonalInfo>>>;
  setIsEditingResume: React.Dispatch<React.SetStateAction<boolean>>;
  setEditingResumeContent: React.Dispatch<React.SetStateAction<string>>;
  setViewingResumeId: React.Dispatch<React.SetStateAction<number | null>>;
  setInlineAnnotation: React.Dispatch<React.SetStateAction<string | null>>;
  savePersonalInfo: (info: PersonalInfo) => Promise<void>;
  generateResumeFromTemplate: () => Promise<void>;
  handleResumeSpellCheck: () => Promise<void>;
  handleResumePolish: () => Promise<void>;
  handleUpdateResume: () => Promise<void>;
  handleAddResume: () => Promise<void>;
  applyResumeResult: () => void;
  dismissResumeResult: () => void;
  saveEditedResume: () => Promise<void>;
  openResumeView: (id: number) => void;
  exportResumeLocal: (content: string, title: string) => Promise<void>;
  handleDeleteResume: (id: number, title?: string) => void;
  formatTime: (ts: number) => string;
}
export default function ResumePanel({
  resumeSubPage,
  viewingResumeId,
  showAddResume,
  editingResumeId,
  resumeForm,
  personalInfo,
  aiOn,
  featureOn,
  llmConfigured,
  resumePolishLoading,
  resumePolishResult,
  isLoading,
  resumes,
  resumesLoading,
  selectedTemplate,
  resumeTemplates,
  templateSupplement,
  isEditingResume,
  editingResumeContent,
  annotationLoading,
  inlineAnnotation,
  setResumeSubPage,
  setShowAddResume,
  setEditingResumeId,
  setResumeForm,
  setSelectedTemplate,
  setTemplateSupplement,
  setIsEditingResume,
  setEditingResumeContent,
  setViewingResumeId,
  setInlineAnnotation,
  savePersonalInfo,
  generateResumeFromTemplate,
  handleResumeSpellCheck,
  handleResumePolish,
  handleUpdateResume,
  handleAddResume,
  applyResumeResult,
  dismissResumeResult,
  saveEditedResume,
  openResumeView,
  exportResumeLocal,
  handleDeleteResume,
  formatTime
}: ResumePanelProps) {
  const editTextareaRef = useRef<HTMLTextAreaElement>(null);
  const [tabGhostText, setTabGhostText] = useState('');

  // 在 textarea 光标位置插入文本
  const insertAtCursor = (before: string, after: string = '', placeholder: string = '') => {
    const ta = editTextareaRef.current;
    if (!ta) return;
    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const selected = editingResumeContent.substring(start, end) || placeholder;
    const newText = editingResumeContent.substring(0, start) + before + selected + after + editingResumeContent.substring(end);
    setEditingResumeContent(newText);
    requestAnimationFrame(() => {
      ta.focus();
      const cursorPos = start + before.length + selected.length + after.length;
      ta.setSelectionRange(cursorPos, cursorPos);
    });
  };

  // 在行首插入（如标题、列表项）
  const insertAtLineStart = (prefix: string) => {
    const ta = editTextareaRef.current;
    if (!ta) return;
    const start = ta.selectionStart;
    const lineStart = editingResumeContent.lastIndexOf('\n', start - 1) + 1;
    const newText = editingResumeContent.substring(0, lineStart) + prefix + editingResumeContent.substring(lineStart);
    setEditingResumeContent(newText);
    requestAnimationFrame(() => {
      ta.focus();
      ta.setSelectionRange(start + prefix.length, start + prefix.length);
    });
  };

  // 从材料库插入模块
  const insertFromPersonalInfo = (section: 'education' | 'work' | 'projects' | 'skills') => {
    const info = personalInfo;
    let text = '';
    if (section === 'education' && info.education?.length) {
      text = t("profile.ResumePanel.k1") + info.education.map(e => t("profile.ResumePanel.k2", {
        school: e.school,
        major: e.major,
        degree: e.degree,
        startDate: e.startDate,
        endDate: e.endDate
      })).join('\n');
    } else if (section === 'work' && info.workExperience?.length) {
      text = t("profile.ResumePanel.k3") + info.workExperience.map(w => t("profile.ResumePanel.k4", {
        company: w.company,
        position: w.position,
        startDate: w.startDate,
        endDate: w.endDate,
        description: w.description
      })).join('\n');
    } else if (section === 'projects' && info.projects?.length) {
      text = t("profile.ResumePanel.k5") + info.projects.map(p => t("profile.ResumePanel.k6", {
        name: p.name,
        role: p.role,
        startDate: p.startDate,
        endDate: p.endDate,
        description: p.description,
        technologies: p.technologies
      })).join('\n');
    } else if (section === 'skills' && info.skills) {
      text = t("profile.ResumePanel.k7") + info.skills + '\n';
    }
    if (text) {
      setEditingResumeContent(prev => prev + text);
    }
  };
  const renderPersonalInfoPage = () => {
    const info = personalInfo;
    const update = (field: keyof PersonalInfo, value: any) => savePersonalInfo({
      ...info,
      [field]: value
    });
    return <div>
        <div className={styles.subPageHeader}>
          <button className={styles.backButton} onClick={() => setResumeSubPage(null)}>{t("Intelligence.k2")}</button>
          <div className={styles.infoTitle}>{t("profile.ResumePanel.k8")}</div>
        </div>

        <div className={styles.personalInfoGrid}>
          <div className={styles.formGroup}><label className={styles.formLabel}>{t("common.name")}</label><input type="text" value={info.name} onChange={e => update('name', e.target.value)} placeholder={t("profile.ResumePanel.k9")} className={styles.formInput} /></div>
          <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k10")}</label><select value={info.gender} onChange={e => update('gender', e.target.value)} className={styles.formInput}><option value="">{t("profile.ResumePanel.k11")}</option><option value={t("profile.ResumePanel.k12")}>{t("profile.ResumePanel.k12")}</option><option value={t("profile.ResumePanel.k13")}>{t("profile.ResumePanel.k13")}</option></select></div>
          <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k14")}</label><input type="date" value={info.birthDate} onChange={e => update('birthDate', e.target.value)} className={styles.formInput} /></div>
          <div className={styles.formGroup}><label className={styles.formLabel}>{t("common.phone")}</label><input type="tel" value={info.phone} onChange={e => update('phone', e.target.value)} placeholder={t("profile.ResumePanel.k15")} className={styles.formInput} /></div>
          <div className={styles.formGroup}><label className={styles.formLabel}>{t("common.email")}</label><input type="email" value={info.email} onChange={e => update('email', e.target.value)} placeholder={t("profile.ResumePanel.k16")} className={styles.formInput} /></div>
          <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k17")}</label><input type="text" value={info.location} onChange={e => update('location', e.target.value)} placeholder={t("profile.ResumePanel.k18")} className={styles.formInput} /></div>
        </div>

        <div className={styles.editSectionTitle}>{t("profile.ResumePanel.k19")}</div>
        {info.education.map((edu, idx) => <div key={idx} className={styles.nestedItem}>
            <div className={styles.nestedItemRow}>
              <div className={styles.formGroup} style={{
            flex: 2
          }}><label className={styles.formLabel}>{t("profile.ResumePanel.k20")}</label><input type="text" value={edu.school} onChange={e => {
              const arr = [...info.education];
              arr[idx] = {
                ...edu,
                school: e.target.value
              };
              update('education', arr);
            }} placeholder={t("profile.ResumePanel.k21")} className={styles.formInput} /></div>
              <div className={styles.formGroup} style={{
            flex: 1
          }}><label className={styles.formLabel}>{t("profile.ResumePanel.k22")}</label><select value={edu.degree} onChange={e => {
              const arr = [...info.education];
              arr[idx] = {
                ...edu,
                degree: e.target.value
              };
              update('education', arr);
            }} className={styles.formInput}><option value="">{t("profile.ResumePanel.k23")}</option><option value={t("profile.ResumePanel.k24")}>{t("profile.ResumePanel.k24")}</option><option value={t("profile.ResumePanel.k25")}>{t("profile.ResumePanel.k25")}</option><option value={t("profile.ResumePanel.k26")}>{t("profile.ResumePanel.k26")}</option><option value={t("profile.ResumePanel.k27")}>{t("profile.ResumePanel.k27")}</option><option value={t("profile.ResumePanel.k28")}>{t("profile.ResumePanel.k28")}</option></select></div>
            </div>
            <div className={styles.nestedItemRow}>
              <div className={styles.formGroup} style={{
            flex: 2
          }}><label className={styles.formLabel}>{t("profile.ResumePanel.k29")}</label><input type="text" value={edu.major} onChange={e => {
              const arr = [...info.education];
              arr[idx] = {
                ...edu,
                major: e.target.value
              };
              update('education', arr);
            }} placeholder={t("profile.ResumePanel.k30")} className={styles.formInput} /></div>
              <div className={styles.formGroup} style={{
            flex: 1
          }}><label className={styles.formLabel}>{t("common.time")}</label><input type="text" value={`${edu.startDate} - ${edu.endDate}`} onChange={e => {
              const [s, rest] = e.target.value.split(' - ');
              const end = rest || '';
              const arr = [...info.education];
              arr[idx] = {
                ...edu,
                startDate: s,
                endDate: end
              };
              update('education', arr);
            }} placeholder="2020.09 - 2024.06" className={styles.formInput} /></div>
            </div>
            <button className={styles.dangerButton} onClick={() => update('education', info.education.filter((_, i) => i !== idx))} style={{
          padding: '3px 12px',
          fontSize: 11
        }}>{t("profile.ResumePanel.k31")}</button>
          </div>)}
        <button className={styles.primaryButton} onClick={() => update('education', [...info.education, {
        school: '',
        major: '',
        degree: '',
        startDate: '',
        endDate: ''
      }])} style={{
        padding: '5px 14px',
        fontSize: 12,
        marginTop: 8
      }}>{t("profile.ResumePanel.k32")}</button>

        <div className={styles.editSectionTitle}>{t("profile.ResumePanel.k33")}</div>
        {info.workExperience.map((work, idx) => <div key={idx} className={styles.nestedItem}>
            <div className={styles.nestedItemRow}>
              <div className={styles.formGroup} style={{
            flex: 2
          }}><label className={styles.formLabel}>{t("profile.ResumePanel.k34")}</label><input type="text" value={work.company} onChange={e => {
              const arr = [...info.workExperience];
              arr[idx] = {
                ...work,
                company: e.target.value
              };
              update('workExperience', arr);
            }} placeholder={t("profile.ResumePanel.k35")} className={styles.formInput} /></div>
              <div className={styles.formGroup} style={{
            flex: 1
          }}><label className={styles.formLabel}>{t("profile.ResumePanel.k36")}</label><input type="text" value={work.position} onChange={e => {
              const arr = [...info.workExperience];
              arr[idx] = {
                ...work,
                position: e.target.value
              };
              update('workExperience', arr);
            }} placeholder={t("profile.ResumePanel.k37")} className={styles.formInput} /></div>
            </div>
            <div className={styles.formGroup}><label className={styles.formLabel}>{t("common.time")}</label><input type="text" value={`${work.startDate} - ${work.endDate}`} onChange={e => {
            const [s, rest] = e.target.value.split(' - ');
            const arr = [...info.workExperience];
            arr[idx] = {
              ...work,
              startDate: s,
              endDate: rest || ''
            };
            update('workExperience', arr);
          }} placeholder={t("profile.ResumePanel.k38")} className={styles.formInput} /></div>
            <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k39")}</label><textarea value={work.description} onChange={e => {
            const arr = [...info.workExperience];
            arr[idx] = {
              ...work,
              description: e.target.value
            };
            update('workExperience', arr);
          }} placeholder={t("profile.ResumePanel.k40")} className={styles.formInput} rows={2} style={{
            resize: 'vertical'
          }} /></div>
            <button className={styles.dangerButton} onClick={() => update('workExperience', info.workExperience.filter((_, i) => i !== idx))} style={{
          padding: '3px 12px',
          fontSize: 11
        }}>{t("profile.ResumePanel.k31")}</button>
          </div>)}
        <button className={styles.primaryButton} onClick={() => update('workExperience', [...info.workExperience, {
        company: '',
        position: '',
        startDate: '',
        endDate: '',
        description: ''
      }])} style={{
        padding: '5px 14px',
        fontSize: 12,
        marginTop: 8
      }}>{t("profile.ResumePanel.k41")}</button>

        <div className={styles.editSectionTitle}>{t("profile.ResumePanel.k42")}</div>
        {info.projects.map((proj, idx) => <div key={idx} className={styles.nestedItem}>
            <div className={styles.nestedItemRow}>
              <div className={styles.formGroup} style={{
            flex: 2
          }}><label className={styles.formLabel}>{t("profile.ResumePanel.k43")}</label><input type="text" value={proj.name} onChange={e => {
              const arr = [...info.projects];
              arr[idx] = {
                ...proj,
                name: e.target.value
              };
              update('projects', arr);
            }} placeholder={t("profile.ResumePanel.k44")} className={styles.formInput} /></div>
              <div className={styles.formGroup} style={{
            flex: 1
          }}><label className={styles.formLabel}>{t("profile.ResumePanel.k45")}</label><input type="text" value={proj.role} onChange={e => {
              const arr = [...info.projects];
              arr[idx] = {
                ...proj,
                role: e.target.value
              };
              update('projects', arr);
            }} placeholder={t("profile.ResumePanel.k46")} className={styles.formInput} /></div>
            </div>
            <div className={styles.nestedItemRow}>
              <div className={styles.formGroup} style={{
            flex: 1
          }}><label className={styles.formLabel}>{t("profile.ResumePanel.k47")}</label><input type="text" value={proj.technologies} onChange={e => {
              const arr = [...info.projects];
              arr[idx] = {
                ...proj,
                technologies: e.target.value
              };
              update('projects', arr);
            }} placeholder="React, Rust, ..." className={styles.formInput} /></div>
              <div className={styles.formGroup} style={{
            flex: 1
          }}><label className={styles.formLabel}>{t("common.time")}</label><input type="text" value={`${proj.startDate} - ${proj.endDate}`} onChange={e => {
              const [s, rest] = e.target.value.split(' - ');
              const arr = [...info.projects];
              arr[idx] = {
                ...proj,
                startDate: s,
                endDate: rest || ''
              };
              update('projects', arr);
            }} placeholder="2024.01 - 2024.06" className={styles.formInput} /></div>
            </div>
            <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k48")}</label><textarea value={proj.description} onChange={e => {
            const arr = [...info.projects];
            arr[idx] = {
              ...proj,
              description: e.target.value
            };
            update('projects', arr);
          }} placeholder={t("profile.ResumePanel.k49")} className={styles.formInput} rows={2} style={{
            resize: 'vertical'
          }} /></div>
            <button className={styles.dangerButton} onClick={() => update('projects', info.projects.filter((_, i) => i !== idx))} style={{
          padding: '3px 12px',
          fontSize: 11
        }}>{t("profile.ResumePanel.k31")}</button>
          </div>)}
        <button className={styles.primaryButton} onClick={() => update('projects', [...info.projects, {
        name: '',
        role: '',
        startDate: '',
        endDate: '',
        description: '',
        technologies: ''
      }])} style={{
        padding: '5px 14px',
        fontSize: 12,
        marginTop: 8
      }}>{t("profile.ResumePanel.k50")}</button>

        <div className={styles.editSectionTitle}>{t("profile.ResumePanel.k51")}</div>
        <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k52")}</label><textarea value={info.skills} onChange={e => update('skills', e.target.value)} placeholder={t("profile.ResumePanel.k53")} className={styles.formInput} rows={3} style={{
          resize: 'vertical'
        }} /></div>
        <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k54")}</label><textarea value={info.selfEvaluation} onChange={e => update('selfEvaluation', e.target.value)} placeholder={t("profile.ResumePanel.k55")} className={styles.formInput} rows={3} style={{
          resize: 'vertical'
        }} /></div>
        <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k56")}</label><textarea value={info.certificates} onChange={e => update('certificates', e.target.value)} placeholder={t("profile.ResumePanel.k57")} className={styles.formInput} rows={2} style={{
          resize: 'vertical'
        }} /></div>

        <div style={{
        marginTop: 20,
        textAlign: 'center'
      }}>
          <span className={styles.infoContent} style={{
          color: 'rgba(0,240,255,0.4)'
        }}>{t("profile.ResumePanel.k58")}</span>
        </div>
      </div>;
  };
  const renderTemplatePage = () => {
    return <div>
        <div className={styles.subPageHeader}>
          <button className={styles.backButton} onClick={() => setResumeSubPage(null)}>{t("Intelligence.k2")}</button>
          <div className={styles.infoTitle}>{t("profile.ResumePanel.k59")}</div>
        </div>

        <div className={styles.editSectionTitle}>{t("profile.ResumePanel.k60")}</div>
        <div className={styles.templateGrid}>
          {resumeTemplates.map(tpl => <div key={tpl.id} className={`${styles.templateCard} ${selectedTemplate === tpl.id ? styles.templateCardActive : ''}`} onClick={() => setSelectedTemplate(tpl.id)}>
              <div className={styles.templateThumbWrap}>
                <div className={`${styles.templateThumb} ${styles[`thumb_${tpl.id}`] || ''}`}>
                  <div className={styles.thumbName}>{t("profile.ResumePanel.k61")}</div>
                  <div className={styles.thumbRole}>{t("profile.ResumePanel.k62")}</div>
                  <div className={styles.thumbSection} />
                  <div className={styles.thumbLine} />
                  <div className={styles.thumbLine} style={{
                width: '70%'
              }} />
                  <div className={styles.thumbLine} />
                  <div className={styles.thumbSection} style={{
                marginTop: 4
              }} />
                  <div className={styles.thumbLine} style={{
                width: '90%'
              }} />
                  <div className={styles.thumbLine} style={{
                width: '60%'
              }} />
                </div>
              </div>
              <div className={styles.templateIcon}>{tpl.icon}</div>
              <div className={styles.templateName}>{tpl.name}</div>
              <div className={styles.templateDesc}>{tpl.description}</div>
            </div>)}
        </div>

        <div className={styles.editSectionTitle}>{t("profile.ResumePanel.k63")}</div>
        <div className={styles.supplementNote}>{t("profile.ResumePanel.k64")}</div>
        <div className={styles.personalInfoGrid}>
          <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k65")}</label><input type="text" value={templateSupplement.name || ''} onChange={e => setTemplateSupplement(prev => ({
            ...prev,
            name: e.target.value
          }))} placeholder={t("profile.ResumePanel.k66", {
            arg0: personalInfo.name || t("profile.ResumePanel.k67")
          })} className={styles.formInput} /></div>
          <div className={styles.formGroup}><label className={styles.formLabel}>{t("profile.ResumePanel.k68")}</label><input type="text" value={templateSupplement.selfEvaluation || ''} onChange={e => setTemplateSupplement(prev => ({
            ...prev,
            selfEvaluation: e.target.value
          }))} placeholder={t("profile.ResumePanel.k69")} className={styles.formInput} /></div>
        </div>

        <div style={{
        marginTop: 24,
        textAlign: 'center'
      }}>
          <button className={styles.editActionBtn} onClick={generateResumeFromTemplate}>
            <span className={styles.btnText}>{t("profile.ResumePanel.k70")}</span>
            <span className={styles.btnIcon}>✨</span>
          </button>
        </div>

        <div className={styles.supplementNote} style={{
        marginTop: 16
      }}>
          {t("profile.ResumePanel.k71")}
        </div>
      </div>;
  };
  const renderResumeListPage = () => <div>
      <div className={styles.subPageHeader}>
        <button className={styles.backButton} onClick={() => setResumeSubPage(null)}>{t("Intelligence.k2")}</button>
          <div className={styles.infoTitle}>{t("profile.ResumePanel.k72")}</div>
      </div>

      {resumesLoading && <div className={styles.infoContent}>{t("common.loading")}</div>}

      {!resumesLoading && resumes.length === 0 && <div style={{
      textAlign: 'center',
      padding: '48px 0'
    }}>
          <div className={styles.infoContent} style={{
        marginBottom: 16,
        fontSize: 14
      }}>{t("profile.ResumePanel.k73")}</div>
          <div style={{
        display: 'flex',
        gap: 10,
        justifyContent: 'center',
        flexWrap: 'wrap'
      }}>
            <button className={styles.primaryButton} onClick={() => {
          setResumeSubPage('template');
        }} style={{
          padding: '8px 18px',
          fontSize: 13
        }}>
              {t("profile.ResumePanel.k74")}
            </button>
            <button className={styles.primaryButton} onClick={() => {
          setShowAddResume(true);
          setEditingResumeId(null);
          setResumeForm({
            title: '',
            content: ''
          });
        }} style={{
          padding: '8px 18px',
          fontSize: 13
        }}>
              {t("profile.ResumePanel.k75")}
            </button>
          </div>
        </div>}

      <div className={styles.resumeFileList}>
        {resumes.map(resume => {
        const wordCount = resume.content.replace(/\s/g, '').length;
        const lineCount = resume.content.split('\n').length;
        const isDraft = wordCount < 50;
        return <div key={resume.id} className={styles.resumeFileCard} onClick={() => openResumeView(resume.id)}>
            <div className={styles.resumeFileIcon}>📄</div>
            <div className={styles.resumeFileInfo}>
              <div className={styles.resumeFileTitleRow}>
                <span className={styles.resumeFileName}>{resume.title}</span>
                <span className={`${styles.resumeStatusTag} ${isDraft ? styles.statusDraft : styles.statusDone}`}>
                  {isDraft ? t("profile.ResumePanel.k76") : t("home.TodoPanel.k2")}
                </span>
              </div>
              <div className={styles.resumeFileMeta}>
                {t("Knowledge.k235")} {formatTime(resume.updated_at)}
                <span className={styles.resumeFileStats}>
                  {wordCount} {t("profile.ResumePanel.k77")} {lineCount} {t("components.Linux.k19")} {(new Blob([resume.content]).size / 1024).toFixed(1)} KB
                </span>
              </div>
              <div className={styles.resumeFilePreview}>
                {resume.content.slice(0, 120).replace(/[#*`\-]/g, '').replace(/\n/g, ' ')}{resume.content.length > 120 ? '...' : ''}
              </div>
            </div>
            <div className={styles.resumeFileActions}>
              <button className={styles.actionBtn} onClick={e => {
              e.stopPropagation();
              exportResumeLocal(resume.content, resume.title);
            }} title={t("common.export")}>💾</button>
              <button className={styles.actionBtnDanger} onClick={e => {
              e.stopPropagation();
              handleDeleteResume(resume.id, resume.title);
            }} title={t("common.delete")}>✕</button>
            </div>
          </div>;
      })}
      </div>

      {!showAddResume && resumes.length > 0 && <div style={{
      marginTop: 20,
      textAlign: 'center'
    }}>
          <button className={styles.primaryButton} onClick={() => {
        setShowAddResume(true);
        setEditingResumeId(null);
        setResumeForm({
          title: '',
          content: ''
        });
      }} style={{
        padding: '8px 20px',
        fontSize: 13
      }}>
            {t("profile.ResumePanel.k75")}
          </button>
        </div>}
    </div>;
  const renderResumeViewer = () => {
    const resume = resumes.find(r => r.id === viewingResumeId);
    if (!resume) return null;
    if (isEditingResume) {
      return <div>
          <div className={styles.subPageHeader}>
            <button className={styles.backButton} onClick={() => {
            setIsEditingResume(false);
          }}>{t("Intelligence.k2")}</button>
            <div className={styles.infoTitle}>{t("profile.ResumePanel.k78")} {resume.title}</div>
          </div>
          <div className={styles.resumeEditSplit}>
            <div className={styles.resumeEditLeft}>
              <div className={styles.resumeEditPanelLabel}>{t("profile.ResumePanel.k79")}</div>
              <div className={styles.mdToolbar}>
                <button className={styles.mdToolBtn} onClick={() => insertAtLineStart('# ')} title={t("profile.ResumePanel.k80")}>H1</button>
                <button className={styles.mdToolBtn} onClick={() => insertAtLineStart('## ')} title={t("components.TextEditor.k3")}>H2</button>
                <button className={styles.mdToolBtn} onClick={() => insertAtLineStart('### ')} title={t("components.TextEditor.k4")}>H3</button>
                <span className={styles.mdToolDivider} />
                <button className={styles.mdToolBtn} onClick={() => insertAtCursor('**', '**', t("profile.ResumePanel.k81"))} title={t("profile.ResumePanel.k81")}>B</button>
                <button className={styles.mdToolBtn} onClick={() => insertAtCursor('*', '*', t("profile.ResumePanel.k82"))} title={t("profile.ResumePanel.k82")}>I</button>
                <button className={styles.mdToolBtn} onClick={() => insertAtLineStart('- ')} title={t("components.TextEditor.k5")}>•</button>
                <button className={styles.mdToolBtn} onClick={() => insertAtLineStart('1. ')} title={t("components.TextEditor.k6")}>1.</button>
                <span className={styles.mdToolDivider} />
                <button className={styles.mdToolBtn} onClick={() => insertAtCursor('\n---\n', '', '')} title={t("components.TextEditor.k10")}>―</button>
                <button className={styles.mdToolBtn} onClick={() => insertAtCursor('| ', ' |  | \n|---|---|\n| ', t("profile.ResumePanel.k83"))} title={t("profile.ResumePanel.k83")}>⊞</button>
              </div>
              <div className={styles.mdInsertBar}>
                <span className={styles.mdInsertLabel}>{t("profile.ResumePanel.k84")}</span>
                <button className={styles.mdInsertBtn} onClick={() => insertFromPersonalInfo('education')}>{t("profile.ResumePanel.k85")}</button>
                <button className={styles.mdInsertBtn} onClick={() => insertFromPersonalInfo('work')}>{t("profile.ResumePanel.k86")}</button>
                <button className={styles.mdInsertBtn} onClick={() => insertFromPersonalInfo('projects')}>{t("profile.ResumePanel.k87")}</button>
                <button className={styles.mdInsertBtn} onClick={() => insertFromPersonalInfo('skills')}>{t("layout.k26")}</button>
              </div>
              <div style={{ position: 'relative' }}>
                <textarea ref={editTextareaRef} value={editingResumeContent} onChange={e => {
                  setEditingResumeContent(e.target.value);
                  setTabGhostText('');
                  // 简单的 ghost text 建议逻辑
                  const val = e.target.value;
                  if (val.length > 2) {
                    const suggestions = ['## 教育背景', '## 工作经历', '## 项目经验', '## 技能特长', '## 自我评价'];
                    const lastLine = val.split('\n').pop() || '';
                    const match = suggestions.find(s => s.toLowerCase().includes(lastLine.toLowerCase()));
                    if (match && !val.endsWith(match)) {
                      setTabGhostText(match.slice(lastLine.length));
                    }
                  }
                }} onKeyDown={e => {
                  if (e.key === 'Tab' && tabGhostText) {
                    e.preventDefault();
                    const ta = editTextareaRef.current;
                    if (ta) {
                      const start = ta.selectionStart;
                      const end = ta.selectionEnd;
                      const newText = editingResumeContent.substring(0, start) + tabGhostText + editingResumeContent.substring(end);
                      setEditingResumeContent(newText);
                      setTabGhostText('');
                      requestAnimationFrame(() => {
                        ta.focus();
                        const cursorPos = start + tabGhostText.length;
                        ta.setSelectionRange(cursorPos, cursorPos);
                      });
                    }
                  } else if (e.key === 'Escape') {
                    setTabGhostText('');
                  }
                }} className={styles.resumeEditTextarea} spellCheck={false} placeholder={t("profile.ResumePanel.k88")} />
                {tabGhostText && <div style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  pointerEvents: 'none',
                  color: 'rgba(0, 240, 255, 0.4)',
                  fontSize: 'inherit',
                  fontFamily: 'inherit',
                  whiteSpace: 'pre-wrap',
                  wordWrap: 'break-word'
                }}>
                  <span style={{ visibility: 'hidden' }}>{editingResumeContent}</span>
                  <span>{tabGhostText}</span>
                </div>}
              </div>
              {annotationLoading && <div className={styles.aiChecking}>{t("profile.ResumePanel.k89")}</div>}
              {inlineAnnotation && <div className={styles.aiAnnotationInline}>
                  <span className={styles.aiAnnotationIcon}>🔍</span>
                  <pre>{inlineAnnotation}</pre>
                </div>}
            </div>
            <div className={styles.resumeEditRight}>
              <div className={styles.resumeEditPanelLabel}>{t("profile.ResumePanel.k90")}</div>
              <div className={styles.resumePreview}>
                <MarkdownRenderer content={editingResumeContent} />
              </div>
            </div>
          </div>
          <div className={styles.resumeViewerFooter}>
            <button className={styles.primaryButton} onClick={() => setIsEditingResume(false)} style={{
            padding: '8px 20px',
            fontSize: 13
          }}>{t("common.cancel")}</button>
            <button className={styles.primaryButton} onClick={saveEditedResume} disabled={isLoading} style={{
            padding: '8px 24px',
            fontSize: 13,
            marginLeft: 8
          }}>{isLoading ? t("components.AudioEditor.k6") : t("knowledge.TemplateModals.k16")}</button>
          </div>
        </div>;
    }
    return <div>
        <div className={`${styles.fileViewerHeader} noPrint`}>
          <button className={styles.backButton} onClick={() => {
          setViewingResumeId(null);
          setEditingResumeContent('');
        }}>{t("Intelligence.k2")}</button>
          <div className={styles.fileViewerTitle}>
            <span className={styles.fileViewerIcon}>📄</span>
            <span>{resume.title}.md</span>
          </div>
        </div>

        <div className={`${styles.fileViewerMeta} noPrint`}>
          <span>{t("Knowledge.k236")} {formatTime(resume.created_at)}</span>
          <span style={{
          margin: '0 10px'
        }}>·</span>
          <span>{t("Knowledge.k235")} {formatTime(resume.updated_at)}</span>
          <span style={{
          margin: '0 10px'
        }}>·</span>
          <span>{(new Blob([resume.content]).size / 1024).toFixed(1)} KB</span>
        </div>

        <div className={`${styles.fileViewerToolbar} noPrint`}>
          <button className={styles.toolbarBtn} onClick={() => exportResumeLocal(resume.content, resume.title)}>{t("profile.ResumePanel.k91")}</button>
          <button className={styles.toolbarBtn} onClick={() => window.print()}>{t("profile.ResumePanel.k92")}</button>
          <button className={styles.toolbarBtn} onClick={() => setIsEditingResume(true)}>{t("profile.ResumePanel.k93")}</button>
          <button className={styles.toolbarBtnDanger} onClick={() => handleDeleteResume(resume.id, resume.title)}>{t("profile.ResumePanel.k94")}</button>
          {aiOn && featureOn('smart_complete') ? <span className={styles.aiBadge} title={t("profile.ResumePanel.k95")}>
              {t("profile.ResumePanel.k96")}
            </span> : aiOn ? <span className={styles.aiBadge} title={t("Knowledge.k215")} style={{
          opacity: 0.5,
          cursor: 'default',
          background: '#1A1A1F'
        }}>
              {t("profile.ResumePanel.k97")}
            </span> : null}
        </div>

        {inlineAnnotation && <div className={`${styles.aiAnnotation} noPrint`}>
            <div className={styles.aiAnnotationIcon}>🔍</div>
            <pre className={styles.aiAnnotationText}>{inlineAnnotation}</pre>
            <button className={styles.aiAnnotationClose} onClick={() => setInlineAnnotation(null)}>✕</button>
          </div>}

        <div className={styles.fileViewerBody}>
          <div className={`${styles.resumePreview} printableResume`}>
            <MarkdownRenderer content={resume.content} />
          </div>
        </div>
      </div>;
  };
  if (resumeSubPage === 'personalInfo') return renderPersonalInfoPage();
  if (resumeSubPage === 'template') return renderTemplatePage();
  if (resumeSubPage === 'resumeList') {
    if (viewingResumeId !== null) return renderResumeViewer();
    return renderResumeListPage();
  }
  if (viewingResumeId !== null) return renderResumeViewer();
  return <div>
      <div className={styles.resumeSection}>
        <div className={styles.infoTitle}>{t("profile.ResumePanel.k98")}</div>

        <div className={styles.resumeNavCard} onClick={() => setResumeSubPage('personalInfo')}>
          <div className={styles.resumeNavIcon}>👤</div>
          <div className={styles.resumeNavInfo}>
            <div className={styles.resumeNavTitle}>{t("profile.ResumePanel.k8")}</div>
            <div className={styles.resumeNavDesc}>{t("profile.ResumePanel.k99")}</div>
          </div>
          <div className={styles.resumeNavArrow}>›</div>
          {personalInfo.name && <div className={styles.resumeNavBadge}>{t("profile.ResumePanel.k100")}</div>}
        </div>

        <div className={styles.resumeNavCard} onClick={() => setResumeSubPage('template')}>
          <div className={styles.resumeNavIcon}>📋</div>
          <div className={styles.resumeNavInfo}>
            <div className={styles.resumeNavTitle}>{t("profile.ResumePanel.k59")}</div>
            <div className={styles.resumeNavDesc}>{t("profile.ResumePanel.k101")}</div>
          </div>
          <div className={styles.resumeNavArrow}>›</div>
        </div>

        <div className={styles.resumeNavCard} onClick={() => setResumeSubPage('resumeList')}>
          <div className={styles.resumeNavIcon}>📄</div>
          <div className={styles.resumeNavInfo}>
            <div className={styles.resumeNavTitle}>{t("profile.ResumePanel.k72")}</div>
            <div className={styles.resumeNavDesc}>
              {resumes.length > 0 ? t("profile.ResumePanel.k102", {
              length: resumes.length
            }) : t("profile.ResumePanel.k103")}
            </div>
          </div>
          <div className={styles.resumeNavArrow}>›</div>
          {resumes.length > 0 && <div className={styles.resumeNavBadge} style={{
          background: 'rgba(176, 38, 255, 0.12)',
          borderColor: 'rgba(176, 38, 255, 0.3)',
          color: '#B026FF'
        }}>{resumes.length}</div>}
        </div>
      </div>

      {showAddResume && <div className={styles.addSourceForm}>
          <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
            <label className={styles.formLabel}>{t("common.title")}</label>
            <input type="text" value={resumeForm.title} onChange={e => setResumeForm(prev => ({
          ...prev,
          title: e.target.value
        }))} placeholder={t("profile.ResumePanel.k104")} className={styles.formInput} />
          </div>
          <div className={styles.formGroup} style={{
        marginBottom: 12
      }}>
            <label className={styles.formLabel}>{t("common.content")}</label>
            <textarea value={resumeForm.content} onChange={e => setResumeForm(prev => ({
          ...prev,
          content: e.target.value
        }))} placeholder={t("profile.ResumePanel.k105")} className={styles.formInput} rows={8} style={{
          resize: 'vertical',
          fontFamily: 'var(--nt-font-mono)',
          fontSize: 13
        }} />
          </div>
          {aiOn && featureOn('smart_complete') ? <div style={{
        display: 'flex',
        gap: 8,
        marginBottom: 12,
        flexWrap: 'wrap',
        alignItems: 'center'
      }}>
              <button onClick={handleResumeSpellCheck} disabled={resumePolishLoading || !resumeForm.content.trim()} style={{
          padding: '4px 12px',
          fontSize: 12,
          borderRadius: 4,
          border: '1px solid rgba(0,240,255,0.3)',
          background: 'rgba(0,240,255,0.08)',
          color: '#00F0FF',
          cursor: 'pointer'
        }}>
                {resumePolishLoading ? '⏳' : '🔍'} {t("profile.ResumePanel.k106")}
              </button>
              <button onClick={handleResumePolish} disabled={resumePolishLoading || !resumeForm.content.trim()} style={{
          padding: '4px 12px',
          fontSize: 12,
          borderRadius: 4,
          border: '1px solid #00FF00',
          background: '#000000',
          color: '#00FF00',
          cursor: 'pointer',
          fontFamily: 'monospace',
          fontWeight: 'bold'
        }}>
                {resumePolishLoading ? '⏳' : '⚡'} {t("profile.ResumePanel.k107")}
              </button>
            </div> : aiOn && !llmConfigured ? <span style={{
        fontSize: 11,
        color: '#6E6E7A',
        display: 'block',
        marginBottom: 12
      }}>{t("profile.ResumePanel.k108")}</span> : aiOn && !featureOn('smart_complete') ? <span style={{
        fontSize: 11,
        color: '#6E6E7A',
        display: 'block',
        marginBottom: 12
      }}>{t("profile.ResumePanel.k109")}</span> : null}
          {resumePolishResult && <div style={{
        marginBottom: 12,
        padding: 12,
        borderRadius: 6,
        border: '1px solid rgba(0,240,255,0.2)',
        background: 'rgba(0,240,255,0.04)'
      }}>
              <div style={{
          fontSize: 12,
          color: 'rgba(0,240,255,0.7)',
          marginBottom: 8
        }}>
                {t("components.intelligence.ActivityPanel.k17")} {resumePolishResult.changes.length > 0 ? t("profile.ResumePanel.k110", {
            length: resumePolishResult.changes.length
          }) : ''}
              </div>
              {resumePolishResult.suggestions.length > 0 && <div style={{
          fontSize: 11,
          color: 'rgba(255,255,255,0.5)',
          marginBottom: 6,
          maxHeight: 100,
          overflowY: 'auto',
          lineHeight: 1.8
        }}>
                  {resumePolishResult.suggestions.map((s, i) => <div key={i}>· {s}</div>)}
                </div>}
              <textarea value={resumePolishResult.text} readOnly className={styles.formInput} rows={4} style={{
          resize: 'vertical',
          fontFamily: 'var(--nt-font-mono)',
          fontSize: 12,
          marginBottom: 8
        }} />
              <div style={{
          display: 'flex',
          gap: 8
        }}>
                <button onClick={applyResumeResult} style={{
            padding: '4px 12px',
            fontSize: 12,
            borderRadius: 4,
            border: 'none',
            background: '#00F0FF',
            color: '#0A0A0A',
            cursor: 'pointer'
          }}>
                  {t("profile.ResumePanel.k111")}
                </button>
                <button onClick={dismissResumeResult} style={{
            padding: '4px 12px',
            fontSize: 12,
            borderRadius: 4,
            border: '1px solid rgba(255,255,255,0.2)',
            background: 'transparent',
            color: 'rgba(255,255,255,0.5)',
            cursor: 'pointer'
          }}>
                  {t("components.intelligence.SuggestionsPanel.k18")}
                </button>
              </div>
            </div>}
          <div style={{
        display: 'flex',
        gap: 10
      }}>
            <button className={styles.primaryButton} onClick={editingResumeId ? handleUpdateResume : handleAddResume} disabled={isLoading} style={{
          padding: '8px 20px',
          fontSize: 13
        }}>
              {isLoading ? t("components.AudioEditor.k6") : editingResumeId ? t("common.updated") : t("game3d.KnowledgeMapping.k22")}
            </button>
            <button className={styles.dangerButton} onClick={() => {
          setShowAddResume(false);
          setEditingResumeId(null);
          setResumeForm({
            title: '',
            content: ''
          });
        }} style={{
          padding: '8px 20px',
          fontSize: 13
        }}>{t("common.cancel")}</button>
          </div>
        </div>}
    </div>;
}