import { t } from "i18next";
import styles from '../Knowledge.module.css';
import type { KbTemplate } from './types';
interface Props {
  showTemplateModal: boolean;
  showTemplateManager: boolean;
  editingTemplate: KbTemplate | null;
  newTemplate: {
    name: string;
    icon: string;
    description: string;
    entry_type: string;
    content: string;
  };
  templates: KbTemplate[];
  onClosePicker: () => void;
  onOpenManager: () => void;
  onCloseManager: () => void;
  onNewTemplateChange: (updates: Partial<Props['newTemplate']>) => void;
  onEditingTemplateChange: (tpl: KbTemplate | null) => void;
  onEditingTemplateField: (updates: Partial<KbTemplate>) => void;
  onInsertTemplate: (tpl: KbTemplate) => void;
  onCreateTemplate: () => void;
  onUpdateTemplate: () => void;
  onDeleteTemplate: (id: number) => void;
}
const ICON_OPTIONS = ['📄', '📝', '📊', '📖', '🚀', '🔗', '📌', '💡', '✅', '🎯', '📋', '🔬', '💻', '📧', '🗂'];
export default function TemplateModals({
  showTemplateModal,
  showTemplateManager,
  editingTemplate,
  newTemplate,
  templates,
  onClosePicker,
  onOpenManager,
  onCloseManager,
  onNewTemplateChange,
  onEditingTemplateChange,
  onEditingTemplateField,
  onInsertTemplate,
  onCreateTemplate,
  onUpdateTemplate,
  onDeleteTemplate
}: Props) {
  return <>
      {/* 模板选择器 */}
      {showTemplateModal && <div className={styles.modalOverlay} onClick={onClosePicker}>
          <div className={styles.modal} onClick={e => e.stopPropagation()} style={{
        maxWidth: 600
      }}>
            <h3 className={styles.modalTitle}>{t("knowledge.TemplateModals.k1")}</h3>
            <p className={styles.modalBody}>{t("knowledge.TemplateModals.k2")}</p>
            <div className={styles.templateGrid}>
              {templates.length === 0 && <span style={{
            color: '#666',
            fontSize: 13,
            gridColumn: '1 / -1'
          }}>
                  {t("knowledge.TemplateModals.k3")}
                </span>}
              {templates.map(tpl => <button key={tpl.id} className={styles.templateCard} onClick={() => onInsertTemplate(tpl)}>
                  <span className={styles.templateIcon}>{tpl.icon}</span>
                  <span className={styles.templateName}>{tpl.name}</span>
                  <span className={styles.templateDesc}>{tpl.description}</span>
                </button>)}
            </div>
            <div className={styles.modalActions}>
              <button className={styles.btnSm} onClick={onOpenManager}>{t("knowledge.TemplateModals.k4")}</button>
              <button className={styles.btnSm} onClick={onClosePicker}>{t("common.cancel")}</button>
            </div>
          </div>
        </div>}

      {/* 模板管理器 */}
      {showTemplateManager && <div className={styles.modalOverlay} onClick={onCloseManager}>
          <div className={styles.modal} onClick={e => e.stopPropagation()} style={{
        maxWidth: 640,
        maxHeight: '85vh',
        overflowY: 'auto'
      }}>
            <h3 className={styles.modalTitle}>
              {editingTemplate ? t("knowledge.TemplateModals.k5") : t("knowledge.TemplateModals.k6")}
            </h3>

            {!editingTemplate && <>
                <div style={{
            marginBottom: 16,
            display: 'flex',
            gap: 8
          }}>
                  <input value={newTemplate.name} onChange={e => onNewTemplateChange({
              name: e.target.value
            })} placeholder={t("knowledge.TemplateModals.k7")} className={styles.input} style={{
              flex: 1
            }} onKeyDown={e => e.key === 'Enter' && onCreateTemplate()} />
                  <select value={newTemplate.icon} onChange={e => onNewTemplateChange({
              icon: e.target.value
            })} style={{
              padding: '6px 8px',
              borderRadius: 4,
              background: '#1a1a2e',
              color: '#ccc',
              border: '1px solid rgba(255,255,255,0.08)',
              fontSize: 14
            }}>
                    {ICON_OPTIONS.map(icon => <option key={icon} value={icon}>{icon}</option>)}
                  </select>
                  <select value={newTemplate.entry_type} onChange={e => onNewTemplateChange({
              entry_type: e.target.value
            })} style={{
              padding: '6px 8px',
              borderRadius: 4,
              background: '#1a1a2e',
              color: '#ccc',
              border: '1px solid rgba(255,255,255,0.08)',
              fontSize: 14
            }}>
                    <option value="text">{t("knowledge.TemplateModals.k8")}</option>
                    <option value="link">{t("knowledge.TemplateModals.k9")}</option>
                  </select>
                </div>
                <input value={newTemplate.description} onChange={e => onNewTemplateChange({
            description: e.target.value
          })} placeholder={t("knowledge.TemplateModals.k10")} className={styles.input} style={{
            marginBottom: 8
          }} />
                <textarea value={newTemplate.content} onChange={e => onNewTemplateChange({
            content: e.target.value
          })} placeholder={t("knowledge.TemplateModals.k11")} rows={8} style={{
            width: '100%',
            padding: '8px 12px',
            borderRadius: 4,
            background: '#1a1a2e',
            color: '#ccc',
            border: '1px solid rgba(255,255,255,0.08)',
            fontSize: 13,
            fontFamily: 'monospace',
            resize: 'vertical',
            marginBottom: 12
          }} />
                <div className={styles.modalActions}>
                  <button className={styles.btnPrimary} onClick={onCreateTemplate}>{t("knowledge.TemplateModals.k12")}</button>
                </div>

                <div style={{
            borderTop: '1px solid rgba(255,255,255,0.06)',
            paddingTop: 16,
            marginTop: 8
          }}>
                  <p style={{
              color: '#666',
              fontSize: 12,
              marginBottom: 12
            }}>{t("knowledge.TemplateModals.k13")}{templates.length} {t("components.Linux.k18")}</p>
                  {templates.length === 0 && <span style={{
              color: '#555',
              fontSize: 13
            }}>{t("knowledge.TemplateModals.k14")}</span>}
                  {templates.map(tpl => <div key={tpl.id} style={{
              display: 'flex',
              alignItems: 'center',
              gap: 12,
              padding: '8px 12px',
              borderRadius: 4,
              background: 'rgba(255,255,255,0.03)',
              marginBottom: 6
            }}>
                      <span style={{
                fontSize: 20
              }}>{tpl.icon}</span>
                      <div style={{
                flex: 1,
                minWidth: 0
              }}>
                        <div style={{
                  color: '#ccc',
                  fontSize: 13,
                  fontWeight: 500
                }}>{tpl.name}</div>
                        <div style={{
                  color: '#555',
                  fontSize: 11
                }}>{tpl.description || tpl.entry_type}</div>
                      </div>
                      <button onClick={() => onEditingTemplateChange(tpl)} style={{
                padding: '3px 10px',
                borderRadius: 3,
                border: 'none',
                background: 'rgba(0,240,255,0.1)',
                color: '#00F0FF',
                fontSize: 12,
                cursor: 'pointer'
              }}>{t("common.edit")}</button>
                      <button onClick={() => onDeleteTemplate(tpl.id)} style={{
                padding: '3px 10px',
                borderRadius: 3,
                border: 'none',
                background: 'rgba(255,80,80,0.1)',
                color: '#ff5050',
                fontSize: 12,
                cursor: 'pointer'
              }}>{t("common.delete")}</button>
                    </div>)}
                </div>
                <div className={styles.modalActions} style={{
            marginTop: 12
          }}>
                  <button className={styles.btnSm} onClick={onCloseManager}>{t("common.close")}</button>
                </div>
              </>}

            {editingTemplate && <>
                <div style={{
            marginBottom: 12,
            display: 'flex',
            gap: 8
          }}>
                  <input value={editingTemplate.name} onChange={e => onEditingTemplateField({
              name: e.target.value
            })} className={styles.input} style={{
              flex: 1
            }} />
                  <select value={editingTemplate.icon} onChange={e => onEditingTemplateField({
              icon: e.target.value
            })} style={{
              padding: '6px 8px',
              borderRadius: 4,
              background: '#1a1a2e',
              color: '#ccc',
              border: '1px solid rgba(255,255,255,0.08)',
              fontSize: 14
            }}>
                    {ICON_OPTIONS.map(icon => <option key={icon} value={icon}>{icon}</option>)}
                  </select>
                  <select value={editingTemplate.entry_type} onChange={e => onEditingTemplateField({
              entry_type: e.target.value
            })} style={{
              padding: '6px 8px',
              borderRadius: 4,
              background: '#1a1a2e',
              color: '#ccc',
              border: '1px solid rgba(255,255,255,0.08)',
              fontSize: 14
            }}>
                    <option value="text">{t("knowledge.TemplateModals.k8")}</option>
                    <option value="link">{t("knowledge.TemplateModals.k9")}</option>
                  </select>
                </div>
                <input value={editingTemplate.description} onChange={e => onEditingTemplateField({
            description: e.target.value
          })} placeholder={t("knowledge.TemplateModals.k15")} className={styles.input} style={{
            marginBottom: 8
          }} />
                <textarea value={editingTemplate.content} onChange={e => onEditingTemplateField({
            content: e.target.value
          })} placeholder={t("knowledge.TemplateModals.k11")} rows={8} style={{
            width: '100%',
            padding: '8px 12px',
            borderRadius: 4,
            background: '#1a1a2e',
            color: '#ccc',
            border: '1px solid rgba(255,255,255,0.08)',
            fontSize: 13,
            fontFamily: 'monospace',
            resize: 'vertical',
            marginBottom: 12
          }} />
                <div className={styles.modalActions}>
                  <button className={styles.btnPrimary} onClick={onUpdateTemplate}>{t("knowledge.TemplateModals.k16")}</button>
                  <button className={styles.btnSm} onClick={() => onEditingTemplateChange(null)}>{t("knowledge.TemplateModals.k17")}</button>
                </div>
              </>}
          </div>
        </div>}
    </>;
}