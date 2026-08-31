import { t } from "i18next";
import { useState, useCallback, useMemo, useEffect } from 'react';
import { ipc } from '@/lib/ipc';
import styles from '../YuanCode.module.css';
import SkillMarket from './SkillMarket';
type SkillScope = 'system' | 'user' | 'workspace' | 'plugin';
type SkillType = 'implicit' | 'explicit';
interface BackendSkill {
  name: string;
  description: string;
  short_description: string | null;
  interface: {
    display_name: string | null;
    short_description: string | null;
    icon_small: string | null;
    icon_large: string | null;
    brand_color: string | null;
    default_prompt: string | null;
  } | null;
  dependencies: {
    tools: Array<{
      tool_type: string;
      value: string;
      description: string | null;
    }>;
  } | null;
  policy: {
    allow_implicit_invocation: boolean | null;
    auto_discover: boolean | null;
    trigger_patterns: string[] | null;
    file_patterns: string[] | null;
    priority: number | null;
  } | null;
  path: string;
  scope: string;
  plugin_id: string | null;
  enabled: boolean;
  file_size: number;
  created_at: string | null;
}
interface Skill {
  id: string;
  name: string;
  desc: string;
  scope: SkillScope;
  type: SkillType;
  enabled: boolean;
  trigger?: string;
  tools?: string[];
  version: string;
  author: string;
}
function mapBackendSkill(b: BackendSkill): Skill {
  const scopeMap: Record<string, SkillScope> = {
    system: 'system',
    user: 'user',
    project: 'workspace',
    plugin: 'system'
  };
  const isImplicit = b.policy?.allow_implicit_invocation === true;
  return {
    id: b.name,
    name: b.name,
    desc: b.short_description || b.description,
    scope: scopeMap[b.scope] || 'system',
    type: isImplicit ? 'implicit' : 'explicit',
    enabled: b.enabled,
    trigger: b.policy?.trigger_patterns?.join(', ') || undefined,
    tools: b.dependencies?.tools.map(t => t.value) || [],
    version: '1.0.0',
    author: b.plugin_id || 'NexTerm'
  };
}
interface SkillGroup {
  scope: SkillScope;
  label: string;
  skills: Skill[];
}
const DEFAULT_SKILLS: SkillGroup[] = [{
  scope: 'system',
  label: t("yuan-code.SkillsPanel.k1"),
  skills: [{
    id: 'sys_001',
    name: 'web_fetch',
    desc: t("yuan-code.SkillsPanel.k2"),
    scope: 'system',
    type: 'implicit',
    enabled: true,
    trigger: t("yuan-code.SkillsPanel.k3"),
    tools: ['fetch', 'html_parser'],
    version: '1.2.0',
    author: 'NexTerm'
  }, {
    id: 'sys_002',
    name: 'search_docs',
    desc: t("yuan-code.SkillsPanel.k4"),
    scope: 'system',
    type: 'implicit',
    enabled: true,
    trigger: t("yuan-code.SkillsPanel.k5"),
    tools: ['vector_search', 'keyword_match'],
    version: '1.1.0',
    author: 'NexTerm'
  }, {
    id: 'sys_003',
    name: 'code_review',
    desc: t("yuan-code.SkillsPanel.k6"),
    scope: 'system',
    type: 'implicit',
    enabled: true,
    trigger: t("yuan-code.SkillsPanel.k7"),
    tools: ['lint_runner', 'security_scan'],
    version: '1.3.0',
    author: 'NexTerm'
  }, {
    id: 'sys_004',
    name: 'context_compress',
    desc: t("yuan-code.SkillsPanel.k8"),
    scope: 'system',
    type: 'implicit',
    enabled: true,
    trigger: t("yuan-code.SkillsPanel.k9"),
    tools: ['summarizer', 'priority_filter'],
    version: '1.0.0',
    author: 'NexTerm'
  }]
}, {
  scope: 'user',
  label: t("yuan-code.SkillsPanel.k10"),
  skills: [{
    id: 'usr_001',
    name: 'deploy_script',
    desc: t("yuan-code.SkillsPanel.k11"),
    scope: 'user',
    type: 'explicit',
    enabled: true,
    trigger: t("yuan-code.SkillsPanel.k12"),
    tools: ['docker_gen', 'k8s_gen'],
    version: '1.0.0',
    author: t("profile.UserIdentityCard.k3")
  }, {
    id: 'usr_002',
    name: 'migrate_db',
    desc: t("yuan-code.SkillsPanel.k13"),
    scope: 'user',
    type: 'explicit',
    enabled: true,
    trigger: t("yuan-code.SkillsPanel.k14"),
    tools: ['sql_gen', 'orm_mapper'],
    version: '1.0.0',
    author: t("profile.UserIdentityCard.k3")
  }]
}, {
  scope: 'workspace',
  label: t("yuan-code.SkillsPanel.k15"),
  skills: [{
    id: 'ws_001',
    name: 'lint_check',
    desc: t("yuan-code.SkillsPanel.k16"),
    scope: 'workspace',
    type: 'implicit',
    enabled: true,
    trigger: t("yuan-code.SkillsPanel.k17"),
    tools: ['eslint', 'clippy', 'pylint'],
    version: '1.0.0',
    author: t("profile.ResumePanel.k87")
  }, {
    id: 'ws_002',
    name: 'generate_doc',
    desc: t("yuan-code.SkillsPanel.k18"),
    scope: 'workspace',
    type: 'explicit',
    enabled: false,
    trigger: t("yuan-code.SkillsPanel.k19"),
    tools: ['openapi_gen', 'jsdoc_parser'],
    version: '1.0.0',
    author: t("profile.ResumePanel.k87")
  }, {
    id: 'ws_003',
    name: 'test_gen',
    desc: t("yuan-code.SkillsPanel.k20"),
    scope: 'workspace',
    type: 'explicit',
    enabled: false,
    trigger: t("yuan-code.SkillsPanel.k21"),
    tools: ['pytest_gen', 'jest_gen'],
    version: '0.9.0',
    author: t("profile.ResumePanel.k87")
  }]
}];
const SCOPE_LABELS: Record<SkillScope, string> = {
  system: t("yuan-code.SkillsPanel.k1"),
  user: t("yuan-code.SkillsPanel.k10"),
  workspace: t("yuan-code.SkillsPanel.k15"),
  plugin: t("yuan-code.SkillsPanel.k22")
};
const SCOPE_COLORS: Record<SkillScope, string> = {
  system: 'var(--nt-secondary)',
  user: 'var(--nt-primary)',
  workspace: 'var(--nt-warning)',
  plugin: 'var(--nt-secondary)'
};
interface SkillFormData {
  name: string;
  desc: string;
  scope: SkillScope;
  type: SkillType;
  trigger: string;
  tools: string;
}
const EMPTY_FORM: SkillFormData = {
  name: '',
  desc: '',
  scope: 'user',
  type: 'explicit',
  trigger: '',
  tools: ''
};
export default function SkillsPanel() {
  const [skillGroups, setSkillGroups] = useState<SkillGroup[]>(DEFAULT_SKILLS);
  const [activeScope, setActiveScope] = useState<SkillScope | 'all'>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [showImportModal, setShowImportModal] = useState(false);
  const [importText, setImportText] = useState('');
  const [formData, setFormData] = useState<SkillFormData>(EMPTY_FORM);
  const [expandedSkill, setExpandedSkill] = useState<string | null>(null);
  const [view, setView] = useState<'mine' | 'market'>('mine');

  // 切换到市场视图时由市场组件回调：刷新"我的技能"列表
  const refreshMySkills = useCallback(() => {
    // 触发 useEffect 重新加载：通过简单的 reload 标记实现
    // 这里采用重新调用 yuan_skill_list 的方式
    (async () => {
      try {
        const res = await ipc.invoke<BackendSkill[]>('yuan_skill_list');
        if (res.code === 0 && res.data) {
          const groups: SkillGroup[] = [];
          const scopeLabels: Record<string, string> = {
            system: t("yuan-code.SkillsPanel.k1"),
            user: t("yuan-code.SkillsPanel.k10"),
            project: t("yuan-code.SkillsPanel.k15"),
            plugin: t("yuan-code.SkillsPanel.k22")
          };
          const byScope: Record<string, Skill[]> = {};
          for (const b of res.data) {
            const scope = b.scope;
            const label = scopeLabels[scope] || scope;
            if (!byScope[label]) byScope[label] = [];
            byScope[label].push(mapBackendSkill(b));
          }
          for (const [label, skills] of Object.entries(byScope)) {
            const scopeKey = skills[0]?.scope || 'system';
            groups.push({ scope: scopeKey, label, skills });
          }
          if (groups.length > 0) {
            setSkillGroups(groups);
          }
        }
      } catch {
        // 后端不可用，忽略
      }
    })();
  }, []);

  // 从后端加载 Skill 列表
  useEffect(() => {
    let cancelled = false;
    const loadSkills = async () => {
      try {
        const res = await ipc.invoke<BackendSkill[]>('yuan_skill_list');
        if (res.code === 0 && res.data && !cancelled) {
          const groups: SkillGroup[] = [];
          const scopeLabels: Record<string, string> = {
            system: t("yuan-code.SkillsPanel.k1"),
            user: t("yuan-code.SkillsPanel.k10"),
            project: t("yuan-code.SkillsPanel.k15"),
            plugin: t("yuan-code.SkillsPanel.k22")
          };
          const byScope: Record<string, Skill[]> = {};
          for (const b of res.data) {
            const scope = b.scope;
            const label = scopeLabels[scope] || scope;
            if (!byScope[label]) byScope[label] = [];
            byScope[label].push(mapBackendSkill(b));
          }
          for (const [label, skills] of Object.entries(byScope)) {
            const scopeKey = skills[0]?.scope || 'system';
            groups.push({
              scope: scopeKey,
              label,
              skills
            });
          }
          if (groups.length > 0) {
            setSkillGroups(groups);
          }
        }
      } catch {
        // 后端不可用，使用默认数据
      }
    };
    loadSkills();
    return () => {
      cancelled = true;
    };
  }, []);
  const allSkills = useMemo(() => skillGroups.flatMap(g => g.skills), [skillGroups]);
  const filteredGroups = useMemo(() => {
    let groups = skillGroups;
    if (activeScope !== 'all') {
      groups = groups.filter(g => g.scope === activeScope);
    }
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      groups = groups.map(g => ({
        ...g,
        skills: g.skills.filter(s => s.name.toLowerCase().includes(q) || s.desc.toLowerCase().includes(q) || (s.tools || []).some(t => t.toLowerCase().includes(q)))
      })).filter(g => g.skills.length > 0);
    }
    return groups;
  }, [skillGroups, activeScope, searchQuery]);
  const scopeCounts = useMemo(() => {
    const counts: Record<string, number> = {
      all: allSkills.length
    };
    for (const g of skillGroups) {
      counts[g.scope] = g.skills.length;
    }
    return counts;
  }, [skillGroups, allSkills]);
  const toggleSkill = useCallback((id: string) => {
    setSkillGroups(prev => prev.map(g => ({
      ...g,
      skills: g.skills.map(s => s.id === id ? {
        ...s,
        enabled: !s.enabled
      } : s)
    })));
  }, []);
  const toggleExpand = useCallback((id: string) => {
    setExpandedSkill(prev => prev === id ? null : id);
  }, []);
  const deleteSkill = useCallback((id: string) => {
    const skill = allSkills.find(s => s.id === id);
    if (!skill || skill.scope === 'system') return;
    setSkillGroups(prev => prev.map(g => ({
      ...g,
      skills: g.skills.filter(s => s.id !== id)
    })));
  }, [allSkills]);
  const handleCreate = useCallback(() => {
    if (!formData.name.trim() || !formData.desc.trim()) return;
    const newSkill: Skill = {
      id: `${formData.scope.slice(0, 3)}_${Date.now()}`,
      name: formData.name.trim(),
      desc: formData.desc.trim(),
      scope: formData.scope,
      type: formData.type,
      enabled: true,
      trigger: formData.trigger.trim() || undefined,
      tools: formData.tools ? formData.tools.split(',').map(t => t.trim()).filter(Boolean) : [],
      version: '1.0.0',
      author: t("profile.UserIdentityCard.k3")
    };
    setSkillGroups(prev => prev.map(g => g.scope === formData.scope ? {
      ...g,
      skills: [...g.skills, newSkill]
    } : g));
    setShowCreateModal(false);
    setFormData(EMPTY_FORM);
  }, [formData]);
  const handleImport = useCallback(async () => {
    try {
      const parsed = JSON.parse(importText) as Skill;
      if (!parsed.name || !parsed.scope) return;
      setSkillGroups(prev => prev.map(g => g.scope === parsed.scope ? {
        ...g,
        skills: [...g.skills, {
          ...parsed,
          id: `imp_${Date.now()}`
        }]
      } : g));
      setImportText('');
      setShowImportModal(false);
    } catch {
      /* JSON 格式无效 */
    }
  }, [importText]);
  const handleExport = useCallback(async () => {
    const exportData = JSON.stringify(skillGroups, null, 2);
    try {
      await ipc.invoke('yuan_skill_export', {
        data: exportData
      });
    } catch {
      const blob = new Blob([exportData], {
        type: 'application/json'
      });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'skills_export.json';
      a.click();
      URL.revokeObjectURL(url);
    }
  }, [skillGroups]);
  const renderModal = (title: string, onClose: () => void, children: React.ReactNode) => <div style={{
    position: 'fixed',
    inset: 0,
    zIndex: 100,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center'
  }}>
      <div style={{
      position: 'absolute',
      inset: 0,
      background: 'rgba(0,0,0,0.7)'
    }} onClick={onClose} />
      <div style={{
      position: 'relative',
      width: 480,
      maxHeight: '80vh',
      overflow: 'auto',
      background: 'rgba(10,0,20,0.98)',
      border: '1px solid rgba(0,240,255,0.25)',
      borderRadius: 2,
      padding: 20,
      zIndex: 101
    }}>
        <div style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        marginBottom: 16
      }}>
          <span className={styles.panelTitle}>{title}</span>
          <button onClick={onClose} style={{
          background: 'none',
          border: 'none',
          color: 'var(--nt-text-muted)',
          cursor: 'pointer',
          fontSize: 16
        }}>
            ✕
          </button>
        </div>
        {children}
      </div>
    </div>;
  const enabledCount = allSkills.filter(s => s.enabled).length;

  // D1.8 Skill 市场视图：完全切换到 SkillMarket 组件
  if (view === 'market') {
    return <SkillMarket onBack={() => setView('mine')} onSkillListChanged={refreshMySkills} />;
  }

  return <div className={styles.panelContainer}>
      <div className={styles.panelHeader}>
        <span className={styles.panelTitle}>{t("yuan-code.SkillsPanel.k23")}</span>
        <div style={{
        display: 'flex',
        gap: 8,
        alignItems: 'center'
      }}>
          <span className={styles.panelBadge}>
            {enabledCount}/{allSkills.length} {t("common.enable")}
          </span>
          <button className={styles.btnPurple} onClick={() => setShowCreateModal(true)} style={{
          padding: '4px 12px',
          fontSize: 12
        }}>
            {t("yuan-code.SkillsPanel.k24")}
          </button>
          <button className={styles.btnGold} onClick={() => setShowImportModal(true)} style={{
          padding: '4px 12px',
          fontSize: 12
        }}>
            {t("common.import")}
          </button>
          <button className={styles.btnPrimary} onClick={handleExport} style={{
          padding: '4px 12px',
          fontSize: 12
        }}>
            {t("common.export")}
          </button>
          <button className={styles.btnPurple} onClick={() => setView('market')} style={{
          padding: '4px 12px',
          fontSize: 12
        }} title={t("yuan-code.SkillsPanel.k47")}>
            🛒 {t("yuan-code.SkillsPanel.k45")}
          </button>
        </div>
      </div>

      <div className={styles.panelBody}>
        {/* 搜索行 */}
        <div style={{
        display: 'flex',
        gap: 10,
        marginBottom: 14
      }}>
          <input className={styles.formInput} placeholder={t("yuan-code.SkillsPanel.k25")} value={searchQuery} onChange={e => setSearchQuery(e.target.value)} style={{
          flex: 1
        }} />
        </div>

        {/* Scope 标签切换 */}
        <div className={styles.skillScopeTabs}>
          {([['all', t("common.all")], ...Object.entries(SCOPE_LABELS)] as [SkillScope | 'all', string][]).map(([key, label]) => <div key={key} className={`${styles.skillScopeTab} ${activeScope === key ? styles.skillScopeTabActive : ''}`} onClick={() => setActiveScope(key as SkillScope | 'all')} style={{
          borderColor: activeScope === key && key !== 'all' ? SCOPE_COLORS[key as SkillScope] : undefined
        }}>
              <span style={{
            fontWeight: 600,
            fontSize: 13,
            color: activeScope === key ? key === 'all' ? 'var(--nt-primary)' : SCOPE_COLORS[key as SkillScope] : 'var(--nt-text-secondary)',
            marginBottom: 4,
            display: 'block'
          }}>
                {label}
              </span>
              <span style={{
            fontSize: 11,
            color: 'var(--nt-text-muted)'
          }}>
                {scopeCounts[key]} {t("yuan-code.SkillsPanel.k26")}
              </span>
            </div>)}
        </div>

        {/* 技能列表 */}
        {filteredGroups.length === 0 ? <div style={{
        textAlign: 'center',
        padding: '40px 0',
        color: 'var(--nt-text-muted)',
        fontFamily: 'var(--nt-font-mono)',
        fontSize: 13
      }}>
            {searchQuery ? t("yuan-code.SkillsPanel.k27") : t("yuan-code.SkillsPanel.k28")}
          </div> : filteredGroups.map(group => <div key={group.scope} style={{
        marginTop: 20
      }}>
              <h4 style={{
          color: SCOPE_COLORS[group.scope],
          fontSize: 13,
          fontFamily: 'var(--nt-font-mono)',
          marginBottom: 12,
          paddingBottom: 6,
          borderBottom: `1px solid ${SCOPE_COLORS[group.scope]}22`
        }}>
                {group.label}
              </h4>
              <div className={styles.cardGrid}>
                {group.skills.map(skill => {
            const isExpanded = expandedSkill === skill.id;
            return <div key={skill.id} className={styles.skillCard} style={{
              opacity: skill.enabled ? 1 : 0.5,
              borderColor: isExpanded ? `${SCOPE_COLORS[skill.scope]}55` : undefined
            }}>
                      {/* 卡片头部 */}
                      <div onClick={() => toggleExpand(skill.id)} style={{
                cursor: 'pointer'
              }}>
                        <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  marginBottom: 4
                }}>
                          <div className={`${styles.toggleSwitch} ${skill.enabled ? styles.toggleSwitchOn : ''}`} onClick={e => {
                    e.stopPropagation();
                    toggleSkill(skill.id);
                  }} style={{
                    flexShrink: 0
                  }}>
                            <div className={styles.toggleKnob} />
                          </div>
                          <span className={styles.skillName}>{skill.name}</span>
                          <span className={`${styles.skillTag} ${skill.type === 'implicit' ? styles.skillTagImplicit : styles.skillTagExplicit}`}>
                            {skill.type === 'implicit' ? t("yuan-code.SkillsPanel.k29") : t("yuan-code.SkillsPanel.k30")}
                          </span>
                        </div>
                        <p className={styles.skillDesc}>{skill.desc}</p>
                        <div className={styles.skillMeta}>
                          <span className={styles.skillScope}>{group.label}</span>
                          <span style={{
                    color: 'var(--nt-text-muted)',
                    fontSize: 10
                  }}>
                            v{skill.version}
                          </span>
                          <span style={{
                    color: 'var(--nt-text-muted)',
                    fontSize: 10,
                    transform: isExpanded ? 'rotate(90deg)' : 'none',
                    transition: 'transform 0.15s'
                  }}>
                            ▸
                          </span>
                        </div>
                      </div>

                      {/* 展开详情 */}
                      {isExpanded && <div style={{
                marginTop: 10,
                paddingTop: 10,
                borderTop: '1px solid rgba(0,240,255,0.08)'
              }}>
                          <div style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 6
                }}>
                            <div className={styles.formGroup}>
                              <label className={styles.formLabel}>{t("yuan-code.SkillsPanel.k31")}</label>
                              <span style={{
                      fontFamily: 'var(--nt-font-mono)',
                      fontSize: 11,
                      color: 'var(--nt-text-secondary)'
                    }}>
                                {skill.trigger || t("components.Linux.k9")}
                              </span>
                            </div>
                            {skill.tools && skill.tools.length > 0 && <div className={styles.formGroup}>
                                <label className={styles.formLabel}>{t("yuan-code.SkillsPanel.k32")}</label>
                                <div style={{
                      display: 'flex',
                      gap: 4,
                      flexWrap: 'wrap'
                    }}>
                                  {skill.tools.map(t => <span key={t} style={{
                        fontFamily: 'var(--nt-font-mono)',
                        fontSize: 10,
                        color: 'var(--nt-secondary)',
                        padding: '1px 6px',
                        border: '1px solid rgba(176,38,255,0.2)',
                        borderRadius: 2
                      }}>
                                      {t}
                                    </span>)}
                                </div>
                              </div>}
                            <div className={styles.formGroup}>
                              <label className={styles.formLabel}>{t("yuan-code.SkillsPanel.k33")}</label>
                              <span style={{
                      fontFamily: 'var(--nt-font-mono)',
                      fontSize: 11,
                      color: 'var(--nt-text-muted)'
                    }}>
                                {skill.author} · {SCOPE_LABELS[skill.scope]}
                              </span>
                            </div>
                          </div>

                          {skill.scope !== 'system' && <div style={{
                  marginTop: 10,
                  display: 'flex',
                  gap: 6
                }}>
                              <button className={styles.btnDanger} onClick={() => deleteSkill(skill.id)} style={{
                    padding: '2px 8px',
                    fontSize: 10
                  }}>
                                {t("common.delete")}
                              </button>
                            </div>}
                        </div>}
                    </div>;
          })}
              </div>
            </div>)}
      </div>

      {/* 新建技能弹窗 */}
      {showCreateModal && renderModal(t("yuan-code.SkillsPanel.k34"), () => {
      setShowCreateModal(false);
      setFormData(EMPTY_FORM);
    }, <>
          <div className={styles.formGroup}>
            <label className={styles.formLabel}>{t("yuan-code.SkillsPanel.k35")}</label>
            <input className={styles.formInput} placeholder={t("yuan-code.SkillsPanel.k36")} value={formData.name} onChange={e => setFormData(prev => ({
          ...prev,
          name: e.target.value
        }))} />
          </div>
          <div className={styles.formGroup}>
            <label className={styles.formLabel}>{t("common.description")}</label>
            <input className={styles.formInput} placeholder={t("yuan-code.SkillsPanel.k37")} value={formData.desc} onChange={e => setFormData(prev => ({
          ...prev,
          desc: e.target.value
        }))} />
          </div>
          <div style={{
        display: 'grid',
        gridTemplateColumns: '1fr 1fr',
        gap: 10
      }}>
            <div className={styles.formGroup}>
              <label className={styles.formLabel}>{t("yuan-code.SkillsPanel.k38")}</label>
              <select className={styles.formSelect} value={formData.scope} onChange={e => setFormData(prev => ({
            ...prev,
            scope: e.target.value as SkillScope
          }))}>
                <option value="user">{t("yuan-code.SkillsPanel.k10")}</option>
                <option value="workspace">{t("yuan-code.SkillsPanel.k15")}</option>
              </select>
            </div>
            <div className={styles.formGroup}>
              <label className={styles.formLabel}>{t("common.type")}</label>
              <select className={styles.formSelect} value={formData.type} onChange={e => setFormData(prev => ({
            ...prev,
            type: e.target.value as SkillType
          }))}>
                <option value="implicit">{t("yuan-code.SkillsPanel.k29")}</option>
                <option value="explicit">{t("yuan-code.SkillsPanel.k30")}</option>
              </select>
            </div>
          </div>
          <div className={styles.formGroup}>
            <label className={styles.formLabel}>{t("yuan-code.SkillsPanel.k39")}</label>
            <input className={styles.formInput} placeholder={t("yuan-code.SkillsPanel.k40")} value={formData.trigger} onChange={e => setFormData(prev => ({
          ...prev,
          trigger: e.target.value
        }))} />
          </div>
          <div className={styles.formGroup}>
            <label className={styles.formLabel}>{t("yuan-code.SkillsPanel.k41")}</label>
            <input className={styles.formInput} placeholder={t("yuan-code.SkillsPanel.k42")} value={formData.tools} onChange={e => setFormData(prev => ({
          ...prev,
          tools: e.target.value
        }))} />
          </div>
          <div style={{
        display: 'flex',
        gap: 8,
        marginTop: 16,
        justifyContent: 'flex-end'
      }}>
            <button className={styles.btnDanger} onClick={() => {
          setShowCreateModal(false);
          setFormData(EMPTY_FORM);
        }} style={{
          padding: '6px 16px',
          fontSize: 12
        }}>
              {t("common.cancel")}
            </button>
            <button className={styles.btnPrimary} onClick={handleCreate} disabled={!formData.name.trim() || !formData.desc.trim()} style={{
          padding: '6px 16px',
          fontSize: 12
        }}>
              {t("common.created")}
            </button>
          </div>
        </>)}

      {/* 导入弹窗 */}
      {showImportModal && renderModal(t("yuan-code.SkillsPanel.k43"), () => {
      setShowImportModal(false);
      setImportText('');
    }, <>
          <div className={styles.formGroup}>
            <label className={styles.formLabel}>{t("yuan-code.SkillsPanel.k44")}</label>
            <textarea className={styles.formInput} placeholder={`{\n  "name": "my_skill",\n  "desc": "...",\n  "scope": "workspace",\n  "type": "explicit"\n}`} value={importText} onChange={e => setImportText(e.target.value)} style={{
          height: 160,
          resize: 'vertical',
          fontFamily: 'var(--nt-font-mono)',
          fontSize: 11
        }} />
          </div>
          <div style={{
        display: 'flex',
        gap: 8,
        marginTop: 16,
        justifyContent: 'flex-end'
      }}>
            <button className={styles.btnDanger} onClick={() => {
          setShowImportModal(false);
          setImportText('');
        }} style={{
          padding: '6px 16px',
          fontSize: 12
        }}>
              {t("common.cancel")}
            </button>
            <button className={styles.btnPurple} onClick={handleImport} disabled={!importText.trim()} style={{
          padding: '6px 16px',
          fontSize: 12
        }}>
              {t("common.import")}
            </button>
          </div>
        </>)}
    </div>;
}