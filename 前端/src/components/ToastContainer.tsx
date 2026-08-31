import { useNotifStore } from '@/stores/notifStore'
import styles from './ToastContainer.module.css'

const TYPE_ICONS: Record<string, string> = {
  info: 'ℹ',
  success: '✓',
  warning: '⚠',
  error: '✕',
}

export default function ToastContainer() {
  const toasts = useNotifStore(s => s.toasts)
  const removeToast = useNotifStore(s => s.removeToast)

  if (toasts.length === 0) return null

  return (
    <div className={styles.container}>
      {toasts.map(toast => (
        <div key={toast.id} className={`${styles.toast} ${styles[toast.type] || styles.info}`}>
          <span className={styles.icon}>{TYPE_ICONS[toast.type] || 'ℹ'}</span>
          <div className={styles.body}>
            <div className={styles.title}>{toast.title}</div>
            {toast.message && <div className={styles.message}>{toast.message}</div>}
            {toast.action && (
              <button className={styles.action} onClick={toast.action.onClick}>
                {toast.action.label}
              </button>
            )}
          </div>
          <button className={styles.close} onClick={() => removeToast(toast.id)}>✕</button>
        </div>
      ))}
    </div>
  )
}