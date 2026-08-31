import { t } from "i18next";
import { useState } from 'react';
import type { AIModel, GroupConversationForm } from './types';
import { getProviderLabel } from './utils';
import styles from '../AI.module.css';
interface Props {
  models: AIModel[];
  errorMsg: string;
  showError: (msg: string) => void;
  showSuccess: (msg: string) => void;
  onCreated: () => void;
  onClose: () => void;
}
export default function GroupModal({
  models,
  errorMsg,
  showError,
  showSuccess,
  onCreated,
  onClose
}: Props) {
  const [form, setForm] = useState<GroupConversationForm>({
    title: '',
    model_ids: []
  });
  const activeModels = models.filter(m => m.status !== 'offline');
  const toggleModel = (modelId: number) => {
    setForm(prev => ({
      ...prev,
      model_ids: prev.model_ids.includes(modelId) ? prev.model_ids.filter(id => id !== modelId) : [...prev.model_ids, modelId]
    }));
  };
  const handleCreate = async () => {
    if (!form.title.trim()) {
      showError(t("ai.ConversationModal.k1"));
      return;
    }
    if (form.model_ids.length === 0) {
      showError(t("ai.ConversationModal.k2"));
      return;
    }
    try {
      const {
        ai
      } = await import('@/lib/ipc');
      const participants = form.model_ids.map(id => ({
        model_id: id,
        agent_id: null,
        role: 'assistant'
      }));
      await (ai as any).createConversation({
        title: form.title.trim(),
        type: 'group',
        is_temp: false,
        token_budget: 50000,
        participants
      });
      onClose();
      onCreated();
      showSuccess(t("ai.GroupModal.k1"));
    } catch (e) {
      showError(t("ai.GroupModal.k2", {
        e: e
      }));
    }
  };
  return <div className={styles.modalOverlay} onClick={onClose}>
      <div className={styles.modal} onClick={e => e.stopPropagation()}>
        <div className={styles.modalHeader}>
          <h3>{t("ai.GroupModal.k3")}</h3>
          <button className={styles.modalClose} onClick={onClose}>✕</button>
        </div>
        <div className={styles.modalBody}>
          <div className={styles.formGroupC}>
            <label className={styles.formLabel}>{t("ai.GroupModal.k4")}</label>
            <input type="text" className={styles.formInput} placeholder={t("ai.GroupModal.k5")} value={form.title} onChange={e => setForm({
            ...form,
            title: e.target.value
          })} />
          </div>
          <div className={styles.formGroupC}>
            <label className={styles.formLabel}>{t("ai.ConversationModal.k8")} <span className={styles.hintText}>{t("ai.GroupModal.k6")} {form.model_ids.length} {t("ai.GroupModal.k7")}</span></label>
            {activeModels.length === 0 ? <div className={styles.emptyHint}>{t("ai.AgentManager.k18")}</div> : <div className={styles.modelCheckList}>
                {activeModels.map(model => <div key={model.id} className={`${styles.modelCheckItem} ${form.model_ids.includes(model.id) ? styles.modelCheckItemActive : ''}`} onClick={() => toggleModel(model.id)}>
                    <div className={styles.modelCheckBox}>{form.model_ids.includes(model.id) ? '☑' : '☐'}</div>
                    <div className={styles.modelCheckInfo}>
                      <span className={styles.modelCheckName}>{model.name}</span>
                      <span className={`${styles.providerTag} ${styles[`provider${model.provider}` as keyof typeof styles] || ''}`}>
                        {getProviderLabel(model.provider)}
                      </span>
                    </div>
                  </div>)}
              </div>}
          </div>
          <div className={styles.groupInfo}>
            <span className={styles.infoIcon}>ⓘ</span>
            <span className={styles.infoText}>{t("ai.GroupModal.k8")}</span>
          </div>
          {errorMsg && <div className={styles.errorMsg}>{errorMsg}</div>}
        </div>
        <div className={styles.modalFooter}>
          <button className={styles.cancelBtn} onClick={onClose}>{t("common.cancel")}</button>
          <button className={styles.primaryBtn} onClick={handleCreate} disabled={activeModels.length === 0}>{t("ai.GroupModal.k9")}</button>
        </div>
      </div>
    </div>;
}