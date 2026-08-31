import { t } from "i18next";
import { useState } from 'react';
import type { AIModel, BackendAgent, AgentForm } from './types';
import { getProviderLabel } from './utils';
import { ai } from '@/lib/ipc';
import styles from '../AI.module.css';
interface Props {
  agents: BackendAgent[];
  models: AIModel[];
  errorMsg: string;
  showError: (msg: string) => void;
  showSuccess: (msg: string) => void;
  onAgentsChanged: () => void;
}
export default function AgentManager({
  agents,
  models,
  errorMsg,
  showError,
  showSuccess,
  onAgentsChanged
}: Props) {
  const [showModal, setShowModal] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [form, setForm] = useState<AgentForm>({
    name: '',
    description: '',
    system_prompt: '',
    model_id: 0
  });
  const openAdd = () => {
    setEditingId(null);
    setForm({
      name: '',
      description: '',
      system_prompt: '',
      model_id: 0
    });
    setShowModal(true);
  };
  const openEdit = (agent: BackendAgent) => {
    setEditingId(agent.id);
    setForm({
      name: agent.name,
      description: agent.description || '',
      system_prompt: agent.system_prompt || '',
      model_id: agent.model_id || 0
    });
    setShowModal(true);
  };
  const handleSubmit = async () => {
    const autoName = form.name.trim() || form.description.trim().slice(0, 20) || `Agent-${Date.now()}`;
    try {
      if (editingId) {
        await (ai as any).updateAgent({
          id: editingId,
          name: autoName,
          description: form.description.trim() || null,
          system_prompt: form.system_prompt.trim() || null,
          model_id: form.model_id > 0 ? form.model_id : null
        });
        showSuccess(t("ai.AgentManager.k1"));
      } else {
        await ai.createAgent({
          name: autoName,
          description: form.description.trim() || null,
          system_prompt: form.system_prompt.trim() || null,
          model_id: form.model_id > 0 ? form.model_id : null
        });
        showSuccess(t("ai.AgentManager.k2"));
      }
      setShowModal(false);
      setEditingId(null);
      setForm({
        name: '',
        description: '',
        system_prompt: '',
        model_id: 0
      });
      onAgentsChanged();
    } catch (e) {
      showError(t("ai.AgentManager.k3", {
        arg0: editingId ? t("common.updated") : t("common.add"),
        e: e
      }));
    }
  };
  const handleDelete = async (id: number) => {
    try {
      await ai.deleteAgent(id);
      onAgentsChanged();
      showSuccess(t("ai.AgentManager.k4"));
    } catch (e) {
      showError(t("ai.AgentManager.k5", {
        e: e
      }));
    }
  };
  const activeModels = models.filter(m => m.status !== 'offline');
  return <>
      <div className={styles.sidebarList}>
        <button className={styles.addBtn} onClick={openAdd}>{t("ai.AgentManager.k6")}</button>
        {agents.length === 0 ? <div className={styles.emptyHint}>{t("ai.AgentManager.k7")}</div> : agents.map(agent => <div key={agent.id} className={styles.sidebarItem}>
              <div className={styles.sidebarItemHeader}>
                <span className={styles.sidebarItemName}>{agent.name}</span>
                <div className={styles.sidebarItemActions}>
                  <button className={styles.editBtnSmall} onClick={() => openEdit(agent)}>✎</button>
                  <button className={styles.deleteBtnSmall} onClick={() => handleDelete(agent.id)}>✕</button>
                </div>
              </div>
              <div className={styles.sidebarItemMeta}>
                <span>{agent.description?.slice(0, 30) || t("ai.AgentManager.k8")}</span>
                {agent.model_id && <span className={styles.agentModelTag}>
                    🔗 {models.find(m => m.id === agent.model_id)?.name || t("ai.AgentManager.k9", {
              model_id: agent.model_id
            })}
                  </span>}
              </div>
            </div>)}
      </div>

      {showModal && <div className={styles.modalOverlay} onClick={() => {
      setShowModal(false);
      setEditingId(null);
    }}>
          <div className={styles.modal} onClick={e => e.stopPropagation()}>
            <div className={styles.modalHeader}>
              <h3>{editingId ? t("ai.AgentManager.k10") : t("ai.AgentManager.k11")}</h3>
              <button className={styles.modalClose} onClick={() => {
            setShowModal(false);
            setEditingId(null);
          }}>✕</button>
            </div>

            <div className={styles.modalBody}>
              <div className={styles.formGroupC}>
                <label className={styles.formLabel}>{t("ai.AgentManager.k12")}</label>
                <input type="text" className={styles.formInput} placeholder={t("ai.AgentManager.k13")} value={form.name} onChange={e => setForm({
              ...form,
              name: e.target.value
            })} />
              </div>
              <div className={styles.formGroupC}>
                <label className={styles.formLabel}>{t("common.description")}</label>
                <input type="text" className={styles.formInput} placeholder={t("ai.AgentManager.k14")} value={form.description} onChange={e => setForm({
              ...form,
              description: e.target.value
            })} />
              </div>
              <div className={styles.formGroupC}>
                <label className={styles.formLabel}>{t("ai.AgentManager.k15")}</label>
                <textarea className={`${styles.formInput} ${styles.textareaInput}`} placeholder={t("ai.AgentManager.k16")} value={form.system_prompt} onChange={e => setForm({
              ...form,
              system_prompt: e.target.value
            })} rows={5} />
              </div>
              <div className={styles.formGroupC}>
                <label className={styles.formLabel}>{t("ai.AgentManager.k17")}</label>
                {activeModels.length === 0 ? <div className={styles.emptyHint}>{t("ai.AgentManager.k18")}</div> : <select className={styles.formInput} value={form.model_id} onChange={e => setForm({
              ...form,
              model_id: Number(e.target.value)
            })}>
                    <option value={0}>{t("ai.AgentManager.k19")}</option>
                    {activeModels.map(m => <option key={m.id} value={m.id}>{m.name} ({getProviderLabel(m.provider)})</option>)}
                  </select>}
              </div>
              {errorMsg && <div className={styles.errorMsg}>{errorMsg}</div>}
            </div>

            <div className={styles.modalFooter}>
              <button className={styles.cancelBtn} onClick={() => {
            setShowModal(false);
            setEditingId(null);
          }}>{t("common.cancel")}</button>
              <button className={styles.primaryBtn} onClick={handleSubmit}>{editingId ? t("ai.AgentManager.k20") : t("ai.AgentManager.k11")}</button>
            </div>
          </div>
        </div>}
    </>;
}