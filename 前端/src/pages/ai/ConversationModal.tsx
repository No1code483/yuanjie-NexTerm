import { t } from "i18next";
import { useState } from 'react';
import type { AIModel, NewConversationForm } from './types';
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
export default function ConversationModal({
  models,
  errorMsg,
  showError,
  showSuccess,
  onCreated,
  onClose
}: Props) {
  const [form, setForm] = useState<NewConversationForm>({
    title: '',
    model_id: 0,
    model_ids: []
  });
  const availableModels = models.filter(m => m.status !== 'offline');
  const toggleModel = (id: number) => {
    setForm(prev => {
      const exists = prev.model_ids.includes(id);
      const next = exists ? prev.model_ids.filter(mid => mid !== id) : [...prev.model_ids, id];
      return {
        ...prev,
        model_ids: next,
        model_id: next.length === 1 ? next[0] : 0
      };
    });
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
      const participants = form.model_ids.map(model_id => ({
        model_id,
        agent_id: null,
        role: 'assistant'
      }));
      await (ai as any).createConversation({
        title: form.title.trim(),
        type: 'single',
        is_temp: false,
        token_budget: 50000,
        participants
      });
      onClose();
      onCreated();
      showSuccess(t("ai.ConversationModal.k3", {
        length: form.model_ids.length
      }));
    } catch (e) {
      showError(t("ai.ConversationModal.k4", {
        e: e
      }));
    }
  };
  return <div className={styles.modalOverlay} onClick={onClose}>
      <div className={styles.modal} onClick={e => e.stopPropagation()}>
        <div className={styles.modalHeader}>
          <h3>{t("ai.ConversationModal.k5")}</h3>
          <button className={styles.modalClose} onClick={onClose}>✕</button>
        </div>
        <div className={styles.modalBody}>
          <div className={styles.formGroupC}>
            <label className={styles.formLabel}>{t("ai.ConversationModal.k6")}</label>
            <input type="text" className={styles.formInput} placeholder={t("ai.ConversationModal.k7")} value={form.title} onChange={e => setForm({
            ...form,
            title: e.target.value
          })} />
          </div>
          <div className={styles.formGroupC}>
            <label className={styles.formLabel}>
              {t("ai.ConversationModal.k8")}
              <span className={styles.hintText}>{t("ai.ConversationModal.k9")}</span>
            </label>
            {availableModels.length === 0 ? <div className={styles.emptyHint}>{t("ai.ConversationModal.k10")}</div> : <div className={styles.modelSelectList}>
                {availableModels.map(model => {
              const isSelected = form.model_ids.includes(model.id);
              return <div key={model.id} className={`${styles.modelSelectItem} ${isSelected ? styles.modelSelectItemActive : ''}`} onClick={() => toggleModel(model.id)}>
                      <span className={styles.modelCheckIcon}>{isSelected ? '☑' : '☐'}</span>
                      <span className={styles.modelSelectName}>{model.name}</span>
                      <span className={`${styles.providerTag} ${styles[`provider${model.provider}` as keyof typeof styles] || ''}`}>
                        {getProviderLabel(model.provider)}
                      </span>
                      <span className={styles.modelSelectType}>{model.model_type === 'local' ? t("ai.ConversationModal.k11") : 'API'}</span>
                    </div>;
            })}
              </div>}
          </div>
          {form.model_ids.length > 0 && <div className={styles.selectedCount}>
              {t("ai.ConversationModal.k12")} <strong>{form.model_ids.length}</strong> {t("ai.ConversationModal.k13")}
              {form.model_ids.length > 1 && <span className={styles.multiHint}> {t("ai.ConversationModal.k14")}</span>}
            </div>}
          {errorMsg && <div className={styles.errorMsg}>{errorMsg}</div>}
        </div>
        <div className={styles.modalFooter}>
          <button className={styles.cancelBtn} onClick={onClose}>{t("common.cancel")}</button>
          <button className={styles.primaryBtn} onClick={handleCreate} disabled={availableModels.length === 0}>
            {t("ai.ConversationModal.k15")}
          </button>
        </div>
      </div>
    </div>;
}