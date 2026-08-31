import { ReactNode } from 'react'
import styles from './NexTermModal.module.css'

interface NexTermModalProps {
  isOpen: boolean
  onClose: () => void
  title?: string
  children: ReactNode
  type?: 'default' | 'danger' | 'warning'
  showCloseButton?: boolean
}

export default function NexTermModal({
  isOpen,
  onClose,
  title,
  children,
  type = 'default',
  showCloseButton = true
}: NexTermModalProps) {
  if (!isOpen) return null

  return (
    <div className={styles.overlay}>
      <div className={`${styles.content} ${styles[type]}`}>
        {title && (
          <div className={`${styles.header} ${styles[type]}`}>
            <h3 className={`${styles.title} ${styles[type]}`}>
              {title}
            </h3>
            {showCloseButton && (
              <button
                className={`${styles.closeButton} ${styles[type]}`}
                onClick={onClose}
              >
                ×
              </button>
            )}
          </div>
        )}
        <div className={styles.body}>
          {children}
        </div>
      </div>
    </div>
  )
}