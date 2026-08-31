import { t } from "i18next";
import { useState, useCallback, useEffect } from 'react';
import { ipc } from '@/lib/ipc';
import styles from '../YuanCode.module.css';
type SafetyLevel = 'strict' | 'balanced' | 'permissive';
interface ExecRecord {
  id: string;
  lang: string;
  risk: number;
  passed: boolean;
  duration: string;
  timestamp: string;
}
interface BackendExecRecord {
  id: string;
  sandbox_id: string;
  lang: string;
  risk: number;
  passed: boolean;
  duration: string;
  timestamp: string;
  exit_code: number;
  stdout: string;
  stderr: string;
}
interface ResourceLimit {
  key: string;
  label: string;
  value: string;
  step: number;
  range: [number, number];
}
const PRESETS: Record<SafetyLevel, {
  label: string;
  desc: string;
  color: string;
}> = {
  strict: {
    label: t("yuan-code.SandboxPanel.k1"),
    desc: t("yuan-code.SandboxPanel.k2"),
    color: 'var(--nt-accent)'
  },
  balanced: {
    label: t("yuan-code.SandboxPanel.k3"),
    desc: t("yuan-code.SandboxPanel.k4"),
    color: 'var(--nt-primary)'
  },
  permissive: {
    label: t("yuan-code.SandboxPanel.k5"),
    desc: t("yuan-code.SandboxPanel.k6"),
    color: 'var(--nt-warning)'
  }
};
const PRESET_PERMISSIONS: Record<SafetyLevel, Record<string, boolean>> = {
  strict: {
    network: false,
    filesystem: true,
    process: false,
    env: false,
    gpu: false
  },
  balanced: {
    network: true,
    filesystem: true,
    process: true,
    env: false,
    gpu: false
  },
  permissive: {
    network: true,
    filesystem: true,
    process: true,
    env: true,
    gpu: true
  }
};
const PERMISSION_LIST = [{
  key: 'network',
  label: t("yuan-code.SandboxPanel.k7"),
  desc: t("yuan-code.SandboxPanel.k8"),
  icon: '🌐'
}, {
  key: 'filesystem',
  label: t("yuan-code.SandboxPanel.k9"),
  desc: t("yuan-code.SandboxPanel.k10"),
  icon: '📁'
}, {
  key: 'process',
  label: t("yuan-code.SandboxPanel.k11"),
  desc: t("yuan-code.SandboxPanel.k12"),
  icon: '⚙️'
}, {
  key: 'env',
  label: t("yuan-code.SandboxPanel.k13"),
  desc: t("yuan-code.SandboxPanel.k14"),
  icon: '🔧'
}, {
  key: 'gpu',
  label: t("yuan-code.SandboxPanel.k15"),
  desc: t("yuan-code.SandboxPanel.k16"),
  icon: '🎮'
}];
const RESOURCE_LIMITS: ResourceLimit[] = [{
  key: 'memory_mb',
  label: t("yuan-code.SandboxPanel.k17"),
  value: '512',
  step: 128,
  range: [64, 8192]
}, {
  key: 'cpu_cores',
  label: t("yuan-code.SandboxPanel.k18"),
  value: '2',
  step: 1,
  range: [1, 16]
}, {
  key: 'timeout_s',
  label: t("yuan-code.SandboxPanel.k19"),
  value: '30',
  step: 5,
  range: [5, 300]
}, {
  key: 'disk_mb',
  label: t("yuan-code.SandboxPanel.k20"),
  value: '256',
  step: 64,
  range: [64, 4096]
}, {
  key: 'max_processes',
  label: t("yuan-code.SandboxPanel.k21"),
  value: '64',
  step: 8,
  range: [8, 512]
}];
const ALL_LANGUAGES = ['Python', 'JavaScript', 'TypeScript', 'Rust', 'Go', 'Java', 'C', 'C++', 'Bash', 'PowerShell', 'Ruby', 'PHP'];
const MOCK_HISTORY: ExecRecord[] = [{
  id: 'exec_001',
  lang: 'Python',
  risk: 12,
  passed: true,
  duration: '234ms',
  timestamp: '14:32:05'
}, {
  id: 'exec_002',
  lang: 'Rust',
  risk: 8,
  passed: true,
  duration: '1.2s',
  timestamp: '14:30:12'
}, {
  id: 'exec_003',
  lang: 'JavaScript',
  risk: 45,
  passed: false,
  duration: '56ms',
  timestamp: '14:28:44'
}, {
  id: 'exec_004',
  lang: 'Bash',
  risk: 22,
  passed: true,
  duration: '89ms',
  timestamp: '14:25:30'
}, {
  id: 'exec_005',
  lang: 'Python',
  risk: 5,
  passed: true,
  duration: '312ms',
  timestamp: '14:22:18'
}];
function getRiskColor(risk: number): string {
  if (risk < 20) return '#00F0FF';
  if (risk < 50) return '#FFD700';
  return '#FF006E';
}
function getRiskLabel(risk: number): string {
  if (risk < 20) return t("yuan-code.SandboxPanel.k22");
  if (risk < 50) return t("yuan-code.SandboxPanel.k23");
  return t("yuan-code.SandboxPanel.k24");
}
export default function SandboxPanel() {
  const [safetyLevel, setSafetyLevel] = useState<SafetyLevel>('balanced');
  const [permissions, setPermissions] = useState<Record<string, boolean>>(PRESET_PERMISSIONS.balanced);
  const [resources, setResources] = useState<Record<string, string>>(Object.fromEntries(RESOURCE_LIMITS.map(r => [r.key, r.value])));
  const [allowedLanguages, setAllowedLanguages] = useState<string[]>(['Python', 'JavaScript', 'TypeScript', 'Rust', 'Go', 'Bash']);
  const [allowedDomains, setAllowedDomains] = useState<string[]>(['api.github.com', 'pypi.org', 'registry.npmjs.org']);
  const [newDomain, setNewDomain] = useState('');
  const [execHistory, setExecHistory] = useState<ExecRecord[]>(MOCK_HISTORY);
  const [saving, setSaving] = useState(false);

  // 从后端加载执行历史
  useEffect(() => {
    const loadHistory = async () => {
      try {
        const res = await ipc.invoke<BackendExecRecord[]>('yuan_sandbox_history');
        if (res.code === 0 && res.data && res.data.length > 0) {
          setExecHistory(res.data.map(r => ({
            id: r.id,
            lang: r.lang,
            risk: r.risk,
            passed: r.passed,
            duration: r.duration,
            timestamp: r.timestamp
          })));
        }
      } catch {
        // 后端不可用，使用默认数据
      }
    };
    loadHistory();
  }, []);
  const handleSafetyChange = useCallback((level: SafetyLevel) => {
    setSafetyLevel(level);
    setPermissions({
      ...PRESET_PERMISSIONS[level]
    });
  }, []);
  const togglePermission = useCallback((key: string) => {
    setPermissions(prev => ({
      ...prev,
      [key]: !prev[key]
    }));
    setSafetyLevel('permissive' as SafetyLevel);
  }, []);
  const updateResource = useCallback((key: string, value: string) => {
    setResources(prev => ({
      ...prev,
      [key]: value
    }));
  }, []);
  const toggleLanguage = useCallback((lang: string) => {
    setAllowedLanguages(prev => prev.includes(lang) ? prev.filter(l => l !== lang) : [...prev, lang]);
  }, []);
  const addDomain = useCallback(() => {
    if (!newDomain.trim()) return;
    if (allowedDomains.includes(newDomain.trim())) return;
    setAllowedDomains(prev => [...prev, newDomain.trim()]);
    setNewDomain('');
  }, [newDomain, allowedDomains]);
  const removeDomain = useCallback((domain: string) => {
    setAllowedDomains(prev => prev.filter(d => d !== domain));
  }, []);
  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      await ipc.invoke('yuan_sandbox_save', {
        safety_level: safetyLevel,
        permissions,
        resources,
        allowed_languages: allowedLanguages,
        allowed_domains: allowedDomains
      });
    } catch {
      /* 后端未就绪时静默处理 */
    } finally {
      setSaving(false);
    }
  }, [safetyLevel, permissions, resources, allowedLanguages, allowedDomains]);
  const overallRisk = (() => {
    let score = 0;
    if (permissions.network) score += 25;
    if (permissions.process) score += 20;
    if (permissions.env) score += 15;
    if (permissions.gpu) score += 10;
    if (!permissions.filesystem) score += 5;
    if (safetyLevel === 'strict') score = Math.min(score, 15);
    if (safetyLevel === 'permissive') score += 20;
    return Math.min(score, 100);
  })();
  return <div className={styles.panelContainer}>
      <div className={styles.panelHeader}>
        <span className={styles.panelTitle}>{t("yuan-code.SandboxPanel.k25")}</span>
        <div style={{
        display: 'flex',
        gap: 8,
        alignItems: 'center'
      }}>
          <span className={styles.panelBadge}>
            {t("yuan-code.SandboxPanel.k26")} {overallRisk}/100
          </span>
          <button className={styles.btnPrimary} onClick={handleSave} disabled={saving} style={{
          padding: '4px 12px',
          fontSize: 12
        }}>
            {saving ? t("components.AudioEditor.k6") : t("yuan-code.SandboxPanel.k27")}
          </button>
        </div>
      </div>

      <div className={styles.panelBody}>
        {/* 安全等级预设 */}
        <div className={styles.sandboxConfig}>
          <div className={styles.configSection}>
            <h3 className={styles.configTitle}>{t("yuan-code.SandboxPanel.k28")}</h3>
            <div style={{
            display: 'flex',
            gap: 8
          }}>
              {(Object.entries(PRESETS) as [SafetyLevel, typeof PRESETS.strict][]).map(([key, preset]) => <div key={key} onClick={() => handleSafetyChange(key)} style={{
              flex: 1,
              padding: '10px 12px',
              border: `1px solid ${safetyLevel === key ? preset.color : 'rgba(0,240,255,0.12)'}`,
              borderRadius: 2,
              background: safetyLevel === key ? `${preset.color}0D` : 'rgba(0,240,255,0.02)',
              cursor: 'pointer',
              transition: 'all 0.2s'
            }}>
                  <div style={{
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 13,
                color: safetyLevel === key ? preset.color : 'var(--nt-text-secondary)',
                fontWeight: 600,
                marginBottom: 2
              }}>
                    {preset.label}
                  </div>
                  <div style={{
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 10,
                color: 'var(--nt-text-muted)'
              }}>
                    {preset.desc}
                  </div>
                </div>)}
            </div>
          </div>

          {/* 风险仪表 */}
          <div className={styles.configSection}>
            <h3 className={styles.configTitle}>{t("yuan-code.SandboxPanel.k29")}</h3>
            <div style={{
            padding: '12px 14px',
            background: 'rgba(0,240,255,0.03)',
            border: '1px solid rgba(0,240,255,0.1)',
            borderRadius: 2
          }}>
              <div style={{
              display: 'flex',
              justifyContent: 'space-between',
              marginBottom: 6
            }}>
                <span style={{
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 11,
                color: 'var(--nt-text-muted)'
              }}>
                  {t("yuan-code.SandboxPanel.k30")}
                </span>
                <span style={{
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 12,
                fontWeight: 600,
                color: getRiskColor(overallRisk)
              }}>
                  {getRiskLabel(overallRisk)}
                </span>
              </div>
              <div className={styles.progressBar} style={{
              height: 6
            }}>
                <div className={styles.progressFill} style={{
                width: `${overallRisk}%`,
                background: `linear-gradient(90deg, ${getRiskColor(20)} 0%, ${getRiskColor(50)} 50%, ${getRiskColor(80)} 100%)`
              }} />
              </div>
              <div style={{
              display: 'flex',
              justifyContent: 'space-between',
              marginTop: 4,
              fontFamily: 'var(--nt-font-mono)',
              fontSize: 9,
              color: 'var(--nt-text-muted)'
            }}>
                <span>{t("yuan-code.SandboxPanel.k31")}</span>
                <span>{t("yuan-code.SandboxPanel.k32")}</span>
                <span>{t("Linux.k134")}</span>
              </div>
            </div>
          </div>

          {/* 权限策略 */}
          <div className={styles.configSection}>
            <h3 className={styles.configTitle}>{t("yuan-code.SandboxPanel.k33")}</h3>
            <div className={styles.permissionGrid}>
              {PERMISSION_LIST.map(p => <div key={p.key} className={styles.permissionItem} style={{
              borderColor: permissions[p.key] ? 'rgba(0,240,255,0.25)' : 'rgba(0,240,255,0.08)'
            }}>
                  <div className={styles.permissionHeader}>
                    <span className={styles.permissionLabel}>
                      {p.icon} {p.label}
                    </span>
                    <div className={`${styles.toggleSwitch} ${permissions[p.key] ? styles.toggleSwitchOn : ''}`} onClick={() => togglePermission(p.key)}>
                      <div className={styles.toggleKnob} />
                    </div>
                  </div>
                  <span className={styles.permissionDesc}>{p.desc}</span>
                </div>)}
            </div>
          </div>

          {/* 资源限制 */}
          <div className={styles.configSection}>
            <h3 className={styles.configTitle}>{t("yuan-code.SandboxPanel.k34")}</h3>
            <div className={styles.resourceLimits}>
              {RESOURCE_LIMITS.map(r => <div key={r.key} className={styles.limitItem}>
                  <span className={styles.limitLabel}>{r.label}</span>
                  <div style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8
              }}>
                    <input className={styles.formInput} type="number" value={resources[r.key]} min={r.range[0]} max={r.range[1]} step={r.step} onChange={e => updateResource(r.key, e.target.value)} style={{
                  width: 80,
                  padding: '2px 6px',
                  fontSize: 12
                }} />
                  </div>
                </div>)}
            </div>
          </div>

          {/* 语言白名单 */}
          <div className={styles.configSection}>
            <h3 className={styles.configTitle}>{t("yuan-code.SandboxPanel.k35")}</h3>
            <div style={{
            display: 'flex',
            gap: 6,
            flexWrap: 'wrap'
          }}>
              {ALL_LANGUAGES.map(lang => {
              const active = allowedLanguages.includes(lang);
              return <span key={lang} onClick={() => toggleLanguage(lang)} className={styles.langTag} style={{
                cursor: 'pointer',
                opacity: active ? 1 : 0.4,
                borderColor: active ? 'rgba(0,240,255,0.4)' : 'rgba(0,240,255,0.1)',
                background: active ? 'rgba(0,240,255,0.08)' : 'rgba(0,240,255,0.02)'
              }}>
                    {lang}
                  </span>;
            })}
            </div>
          </div>

          {/* 网络策略 */}
          <div className={styles.configSection}>
            <h3 className={styles.configTitle}>{t("yuan-code.SandboxPanel.k36")}</h3>
            <div style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 8
          }}>
              <div style={{
              display: 'flex',
              gap: 6,
              flexWrap: 'wrap'
            }}>
                {allowedDomains.map(domain => <span key={domain} style={{
                fontFamily: 'var(--nt-font-mono)',
                fontSize: 11,
                color: 'var(--nt-text-secondary)',
                padding: '2px 8px',
                background: 'rgba(0,240,255,0.04)',
                border: '1px solid rgba(0,240,255,0.12)',
                borderRadius: 2,
                display: 'flex',
                alignItems: 'center',
                gap: 6
              }}>
                    {domain}
                    <span onClick={() => removeDomain(domain)} style={{
                  cursor: 'pointer',
                  color: 'var(--nt-accent)',
                  fontSize: 13,
                  lineHeight: 1
                }}>
                      ×
                    </span>
                  </span>)}
              </div>
              <div style={{
              display: 'flex',
              gap: 6
            }}>
                <input className={styles.formInput} placeholder={t("yuan-code.SandboxPanel.k37")} value={newDomain} onChange={e => setNewDomain(e.target.value)} onKeyDown={e => e.key === 'Enter' && addDomain()} style={{
                flex: 1,
                fontSize: 11
              }} />
                <button className={styles.btnPrimary} onClick={addDomain} style={{
                padding: '2px 10px',
                fontSize: 11
              }}>
                  {t("yuan-code.SandboxPanel.k38")}
                </button>
              </div>
            </div>
          </div>

          {/* 执行历史 */}
          <div className={styles.configSection}>
            <h3 className={styles.configTitle}>{t("yuan-code.SandboxPanel.k39")}</h3>
            <div className={styles.execHistory}>
              {execHistory.map(rec => <div key={rec.id} className={styles.execHistoryItem}>
                  <div style={{
                display: 'flex',
                justifyContent: 'space-between',
                alignItems: 'center'
              }}>
                    <div style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8
                }}>
                      <span style={{
                    color: rec.passed ? '#00F0FF' : '#FF006E',
                    fontSize: 10
                  }}>
                        {rec.passed ? '✓' : '✗'}
                      </span>
                      <span style={{
                    color: 'var(--nt-text-secondary)'
                  }}>{rec.lang}</span>
                      <span style={{
                    color: getRiskColor(rec.risk),
                    fontSize: 10,
                    padding: '0 4px',
                    border: `1px solid ${getRiskColor(rec.risk)}44`,
                    borderRadius: 2
                  }}>
                        {rec.risk}%
                      </span>
                    </div>
                    <div style={{
                  display: 'flex',
                  gap: 12
                }}>
                      <span style={{
                    color: 'var(--nt-text-muted)',
                    fontSize: 10
                  }}>{rec.duration}</span>
                      <span style={{
                    color: 'var(--nt-text-muted)',
                    fontSize: 10
                  }}>{rec.timestamp}</span>
                    </div>
                  </div>
                  <div className={styles.progressBar} style={{
                marginTop: 3,
                height: 2
              }}>
                    <div className={styles.progressFill} style={{
                  width: `${rec.risk}%`,
                  background: getRiskColor(rec.risk)
                }} />
                  </div>
                </div>)}
            </div>
          </div>
        </div>
      </div>
    </div>;
}