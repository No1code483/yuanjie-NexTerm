import { t } from "i18next";
import { useState } from 'react';
import { NexTermAdapter, BUILTIN_EXTENSION_TYPES } from '../types/extensions';
import { extensionManager } from '../utils/extensionManager';
import styles from './NexTermModal.module.css';
interface AddExtensionModalProps {
  isOpen: boolean;
  onClose: () => void;
  onExtensionAdded: (adapter: NexTermAdapter) => void;
}
export default function AddExtensionModal({
  isOpen,
  onClose,
  onExtensionAdded
}: AddExtensionModalProps) {
  const [formData, setFormData] = useState({
    id: '',
    name: '',
    type: BUILTIN_EXTENSION_TYPES.UTILITY,
    version: '1.0.0',
    description: '',
    author: '',
    category: '',
    tags: ''
  });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const handleInputChange = (field: string, value: string) => {
    setFormData(prev => ({
      ...prev,
      [field]: value
    }));
    setError('');
  };
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError('');
    try {
      // 验证表单数据
      if (!formData.id || !formData.name || !formData.description) {
        throw new Error(t("components.AddExtensionModal.k1"));
      }

      // 创建扩展适配器
      const adapter: NexTermAdapter = {
        id: formData.id,
        name: formData.name,
        type: formData.type,
        version: formData.version,
        description: formData.description,
        author: formData.author || undefined,
        category: formData.category || undefined,
        tags: formData.tags ? formData.tags.split(',').map(tag => tag.trim()) : undefined,
        data: {},
        config: {
          autoStart: false,
          enabled: true
        },
        onMount: async () => {
          console.log(`扩展 ${formData.name} 已挂载`);
        },
        onUnmount: async () => {
          console.log(`扩展 ${formData.name} 已卸载`);
        },
        onMessage: async msg => {
          console.log(`收到扩展 ${formData.name} 消息:`, msg);
        }
      };

      // 注册扩展
      await extensionManager.register(adapter);

      // 通知父组件
      onExtensionAdded(adapter);

      // 重置表单并关闭模态框
      setFormData({
        id: '',
        name: '',
        type: BUILTIN_EXTENSION_TYPES.UTILITY,
        version: '1.0.0',
        description: '',
        author: '',
        category: '',
        tags: ''
      });
      onClose();
    } catch (error) {
      setError(error instanceof Error ? error.message : t("components.AddExtensionModal.k2"));
    } finally {
      setIsLoading(false);
    }
  };
  const handleClose = () => {
    setFormData({
      id: '',
      name: '',
      type: BUILTIN_EXTENSION_TYPES.UTILITY,
      version: '1.0.0',
      description: '',
      author: '',
      category: '',
      tags: ''
    });
    setError('');
    onClose();
  };
  if (!isOpen) return null;
  return <div className={styles.modalOverlay}>
      <div className={styles.modal} style={{
      maxWidth: '500px'
    }}>
        <div className={styles.modalHeader}>
          <h3 style={{
          color: '#00FF00',
          margin: 0
        }}>{t("components.AddExtensionModal.k3")}</h3>
          <button onClick={handleClose} className={styles.closeButton}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M18 6L6 18M6 6L18 18" />
            </svg>
          </button>
        </div>

        <form onSubmit={handleSubmit} className={styles.modalBody}>
          {error && <div style={{
          padding: '10px',
          backgroundColor: '#330000',
          border: '1px solid #FF0000',
          color: '#FF0000',
          fontSize: '14px',
          borderRadius: '4px',
          marginBottom: '15px'
        }}>
              {error}
            </div>}

          <div style={{
          display: 'grid',
          gap: '15px'
        }}>
            <div>
              <label style={{
              display: 'block',
              color: '#00FF00',
              fontSize: '14px',
              marginBottom: '5px'
            }}>
                {t("components.AddExtensionModal.k4")}
              </label>
              <input type="text" value={formData.id} onChange={e => handleInputChange('id', e.target.value)} placeholder={t("components.AddExtensionModal.k5")} style={{
              width: '100%',
              padding: '8px 12px',
              backgroundColor: '#000000',
              border: '1px solid #00FF00',
              color: '#00FF00',
              fontSize: '14px',
              outline: 'none'
            }} required />
            </div>

            <div>
              <label style={{
              display: 'block',
              color: '#00FF00',
              fontSize: '14px',
              marginBottom: '5px'
            }}>
                {t("components.AddExtensionModal.k6")}
              </label>
              <input type="text" value={formData.name} onChange={e => handleInputChange('name', e.target.value)} placeholder={t("components.AddExtensionModal.k7")} style={{
              width: '100%',
              padding: '8px 12px',
              backgroundColor: '#000000',
              border: '1px solid #00FF00',
              color: '#00FF00',
              fontSize: '14px',
              outline: 'none'
            }} required />
            </div>

            <div>
              <label style={{
              display: 'block',
              color: '#00FF00',
              fontSize: '14px',
              marginBottom: '5px'
            }}>
                {t("components.AddExtensionModal.k8")}
              </label>
              <select value={formData.type} onChange={e => handleInputChange('type', e.target.value)} style={{
              width: '100%',
              padding: '8px 12px',
              backgroundColor: '#000000',
              border: '1px solid #00FF00',
              color: '#00FF00',
              fontSize: '14px',
              outline: 'none'
            }}>
                <option value={BUILTIN_EXTENSION_TYPES.UTILITY}>{t("components.AddExtensionModal.k9")}</option>
                <option value={BUILTIN_EXTENSION_TYPES.TOOL}>{t("components.AddExtensionModal.k10")}</option>
                <option value={BUILTIN_EXTENSION_TYPES.GAME}>{t("components.AddExtensionModal.k11")}</option>
                <option value={BUILTIN_EXTENSION_TYPES.AI}>{t("components.AddExtensionModal.k12")}</option>
                <option value={BUILTIN_EXTENSION_TYPES.COMMUNICATION}>{t("components.AddExtensionModal.k13")}</option>
                <option value={BUILTIN_EXTENSION_TYPES.PRODUCTIVITY}>{t("components.AddExtensionModal.k14")}</option>
                <option value={BUILTIN_EXTENSION_TYPES.SYSTEM}>{t("components.AddExtensionModal.k15")}</option>
              </select>
            </div>

            <div>
              <label style={{
              display: 'block',
              color: '#00FF00',
              fontSize: '14px',
              marginBottom: '5px'
            }}>
                {t("components.AddExtensionModal.k16")}
              </label>
              <input type="text" value={formData.version} onChange={e => handleInputChange('version', e.target.value)} placeholder={t("components.AddExtensionModal.k17")} style={{
              width: '100%',
              padding: '8px 12px',
              backgroundColor: '#000000',
              border: '1px solid #00FF00',
              color: '#00FF00',
              fontSize: '14px',
              outline: 'none'
            }} required />
            </div>

            <div>
              <label style={{
              display: 'block',
              color: '#00FF00',
              fontSize: '14px',
              marginBottom: '5px'
            }}>
                {t("components.AddExtensionModal.k18")}
              </label>
              <textarea value={formData.description} onChange={e => handleInputChange('description', e.target.value)} placeholder={t("components.AddExtensionModal.k19")} rows={3} style={{
              width: '100%',
              padding: '8px 12px',
              backgroundColor: '#000000',
              border: '1px solid #00FF00',
              color: '#00FF00',
              fontSize: '14px',
              outline: 'none',
              resize: 'vertical'
            }} required />
            </div>

            <div>
              <label style={{
              display: 'block',
              color: '#00E0E0',
              fontSize: '14px',
              marginBottom: '5px'
            }}>
                {t("components.AddExtensionModal.k20")}
              </label>
              <input type="text" value={formData.author} onChange={e => handleInputChange('author', e.target.value)} placeholder={t("components.AddExtensionModal.k21")} style={{
              width: '100%',
              padding: '8px 12px',
              backgroundColor: '#000000',
              border: '1px solid #00E0E0',
              color: '#00E0E0',
              fontSize: '14px',
              outline: 'none'
            }} />
            </div>

            <div>
              <label style={{
              display: 'block',
              color: '#00E0E0',
              fontSize: '14px',
              marginBottom: '5px'
            }}>
                {t("components.AddExtensionModal.k22")}
              </label>
              <input type="text" value={formData.category} onChange={e => handleInputChange('category', e.target.value)} placeholder={t("components.AddExtensionModal.k23")} style={{
              width: '100%',
              padding: '8px 12px',
              backgroundColor: '#000000',
              border: '1px solid #00E0E0',
              color: '#00E0E0',
              fontSize: '14px',
              outline: 'none'
            }} />
            </div>

            <div>
              <label style={{
              display: 'block',
              color: '#00E0E0',
              fontSize: '14px',
              marginBottom: '5px'
            }}>
                {t("components.AddExtensionModal.k24")}
              </label>
              <input type="text" value={formData.tags} onChange={e => handleInputChange('tags', e.target.value)} placeholder={t("components.AddExtensionModal.k25")} style={{
              width: '100%',
              padding: '8px 12px',
              backgroundColor: '#000000',
              border: '1px solid #00E0E0',
              color: '#00E0E0',
              fontSize: '14px',
              outline: 'none'
            }} />
            </div>
          </div>

          <div className={styles.modalFooter} style={{
          marginTop: '20px'
        }}>
            <button type="button" onClick={handleClose} style={{
            padding: '8px 16px',
            backgroundColor: 'transparent',
            border: '1px solid #888888',
            color: '#888888',
            cursor: 'pointer',
            fontSize: '14px'
          }}>
              {t("common.cancel")}
            </button>
            <button type="submit" disabled={isLoading} style={{
            padding: '8px 16px',
            backgroundColor: '#00FF00',
            border: '1px solid #00FF00',
            color: '#000000',
            cursor: isLoading ? 'not-allowed' : 'pointer',
            fontSize: '14px',
            fontWeight: 'bold',
            opacity: isLoading ? 0.6 : 1
          }}>
              {isLoading ? t("components.AddExtensionModal.k26") : t("components.AddExtensionModal.k27")}
            </button>
          </div>
        </form>
      </div>
    </div>;
}