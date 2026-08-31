import { memo } from 'react'
import styles from './Button.module.css'

interface ButtonProps {
  children: React.ReactNode
  variant?: 'default' | 'danger' | 'warning' | 'ghost'
  size?: 'small' | 'medium' | 'large'
  onClick?: () => void
  disabled?: boolean
  className?: string
  ariaLabel?: string
}

const Button = memo(function Button({
  children,
  variant = 'default',
  size = 'medium',
  onClick,
  disabled = false,
  className = '',
  ariaLabel
}: ButtonProps) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      aria-label={ariaLabel}
      aria-disabled={disabled || undefined}
      className={`
        ${styles.button}
        ${styles[variant]}
        ${styles[size]}
        ${disabled ? styles.disabled : ''}
        ${className}
      `}
    >
      {children}
    </button>
  )
})

export default Button
