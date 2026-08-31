import { useState, ChangeEvent, KeyboardEvent, memo, useId } from 'react'
import styles from './Input.module.css'

interface InputProps {
  type?: 'text' | 'password' | 'email' | 'number' | 'search'
  placeholder?: string
  value?: string
  onChange?: (value: string) => void
  onEnter?: () => void
  error?: string
  disabled?: boolean
  size?: 'small' | 'medium' | 'large'
  className?: string
  ariaLabel?: string
}

const Input = memo(function Input({
  type = 'text',
  placeholder = '',
  value = '',
  onChange,
  onEnter,
  error,
  disabled = false,
  size = 'medium',
  className = '',
  ariaLabel
}: InputProps) {
  const [isFocused, setIsFocused] = useState(false)
  const errorId = useId()
  const inputState = disabled ? 'disabled' : error ? 'error' : isFocused ? 'focused' : 'normal'

  return (
    <div className={`${styles.container} ${className}`}>
      <input
        type={type}
        placeholder={placeholder}
        value={value}
        onChange={(e: ChangeEvent<HTMLInputElement>) => onChange?.(e.target.value)}
        onKeyDown={(e: KeyboardEvent<HTMLInputElement>) => {
          if (e.key === 'Enter') onEnter?.()
        }}
        onFocus={() => !disabled && setIsFocused(true)}
        onBlur={() => setIsFocused(false)}
        disabled={disabled}
        aria-label={ariaLabel}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errorId : undefined}
        className={`${styles.input} ${styles[size]} ${styles[inputState]}`}
      />
      {error && <div id={errorId} className={styles.errorMsg} role="alert">{error}</div>}
    </div>
  )
})

export default Input