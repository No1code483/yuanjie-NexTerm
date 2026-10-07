// xin.skill L2 —— 技能功能域（局部 state + handlers + JSX）。
import { t } from "i18next";
import { useState, useEffect } from 'react';
import { xinOrchestration } from '../../ipc';
import styles from '../../Xin.module.css';
import type { SkillInfo, SkillResult } from '../../xin/types';
import type { XinCore } from '../../core';

export function useSkill(_core: XinCore, active: boolean) {
  const [skills, setSkills] = useState<SkillInfo[]>([]);
  const [skillInput, setSkillInput] = useState('');
  const [skillResult, setSkillResult] = useState<SkillResult | null>(null);
  const [skillExecLoading, setSkillExecLoading] = useState(false);

  const loadSkills = async () => {
    try {
      const res = await xinOrchestration.listSkills();
      if (res?.data) setSkills(res.data);
    } catch {/* silent */}
  };
  const executeSkill = async (skillId: string) => {
    if (!skillInput.trim()) return;
    setSkillExecLoading(true);
    try {
      const res = await xinOrchestration.executeSkill(skillId, skillInput);
      if (res?.data) setSkillResult(res.data);
    } catch {/* silent */} finally {
      setSkillExecLoading(false);
    }
  };

  useEffect(() => {
    if (active && skills.length === 0) loadSkills();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active]);

  const renderSkill = () => <div className={styles.tabContent}>
      {skills.length === 0 ? <div className={styles.loading}>{t("Xin.k156")}</div> : <>
          <div className={styles.card}>
            <div className={styles.panelTitle}>{t("Xin.k157")}</div>
            <div style={{
        display: 'flex',
        gap: '8px',
        marginBottom: '12px'
      }}>
              <input className={styles.input} placeholder={t("Xin.k158")} value={skillInput} onChange={e => setSkillInput(e.target.value)} style={{
          flex: '1'
        }} />
            </div>
            <div className={styles.skillGrid}>
              {skills.map(s => <div key={s.id} className={styles.skillCard} style={{
          opacity: s.available ? 1 : 0.4
        }}>
                  <div style={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center'
          }}>
                    <span style={{
              fontSize: '12px',
              color: '#00F0FF'
            }}>{s.name}</span>
                    <span className={styles.skillCategory}>{s.category}</span>
                  </div>
                  <p style={{
            fontSize: '10px',
            color: '#8a8aaa',
            margin: '4px 0'
          }}>{s.description}</p>
                  <button className={styles.btnSmall} onClick={() => executeSkill(s.id)} disabled={!s.available || skillExecLoading}>
                    {skillExecLoading ? t("Xin.k159") : t("Xin.k160")}
                  </button>
                </div>)}
            </div>
          </div>

          {skillResult && <div className={styles.card}>
              <div className={styles.panelTitle}>{t("Xin.k161")}</div>
              <div style={{
        fontSize: '11px',
        color: '#c8c8dd',
        marginBottom: '4px'
      }}>
                <span style={{
          color: '#6a6a8a'
        }}>{t("Xin.k162")} {skillResult.skill_id}</span>
                <span style={{
          marginLeft: '12px',
          color: skillResult.success ? '#00F0FF' : '#FF006E'
        }}>
                  {skillResult.success ? t("Xin.k163") : t("Xin.k164")}
                </span>
              </div>
              {skillResult.output && <pre className={styles.resultPre}>{skillResult.output}</pre>}
              {skillResult.error && <div style={{
        fontSize: '10px',
        color: '#FF006E'
      }}>{skillResult.error}</div>}
            </div>}
        </>}
    </div>;

  return { renderSkill };
}
