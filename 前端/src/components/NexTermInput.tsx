import { useState, ChangeEvent, KeyboardEvent } from 'react'
import styles from './NexTermInput.module.css'

interface NexTermInputProps {
  type?: 'text' | 'password' | 'email' | 'number'
  placeholder?: string
  value?: string
  onChange?: (value: string) => void
  onEnter?: () => void
  error?: string
  disabled?: boolean
  size?: 'small' | 'medium' | 'large'
}

export default function NexTermInput({
  type = 'text',
  placeholder = '',
  value = '',
  onChange,
  onEnter,
  error,
  disabled = false,
  size = 'medium'
}: NexTermInputProps) {
  const [isFocused, setIsFocused] = useState(false)

  const getInputState = () => {
    if (disabled) return 'disabled'
    if (error) return 'error'
    if (isFocused) return 'focused'
    return 'normal'
  }

  const handleChange = (e: ChangeEvent<HTMLInputElement>) => {
    if (onChange) {
      onChange(e.target.value)
    }
  }

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter' && onEnter) {
      onEnter()
    }
  }

  const handleFocus = () => {
    if (!disabled) {
      setIsFocused(true)
    }
  }

  const handleBlur = () => {
    setIsFocused(false)
  }

  const inputState = getInputState()

  return (
    <div className={styles.inputContainer}>
      <input
        type={type}
        placeholder={placeholder}
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        onFocus={handleFocus}
        onBlur={handleBlur}
        disabled={disabled}
        className={`
          ${styles.input}
          ${styles[size]}
          ${styles[inputState]}
        `}
      />
      {error && (
        <div className={styles.errorMessage}>
          {error}
        </div>
      )}
    </div>
  )
}