import { ReactNode, useEffect, useState } from 'react'

interface NexTermTipProps {
  type: 'success' | 'error' | 'warning' | 'info'
  message: ReactNode
  duration?: number
  onClose?: () => void
}

export default function NexTermTip({
  type,
  message,
  duration = 3000,
  onClose
}: NexTermTipProps) {
  const [visible, setVisible] = useState(true)

  useEffect(() => {
    if (duration > 0) {
      const timer = setTimeout(() => {
        setVisible(false)
        if (onClose) onClose()
      }, duration)
      return () => clearTimeout(timer)
    }
  }, [duration, onClose])

  if (!visible) return null

  const getTipStyle = () => {
    const baseStyle = {
      position: 'fixed' as const,
      top: '20px',
      right: '20px',
      backgroundColor: '#000000',
      border: '1.5px solid',
      color: '#00FF00',
      fontFamily: 'Consolas, monospace',
      padding: '12px 20px',
      zIndex: 10000,
      animation: 'nt-fade-in 0.3s ease-out'
    }

    const typeStyle = {
      success: {
        borderColor: '#00FF00',
        boxShadow: '0 0 15px #00FF00',
        animation: 'nt-blink-green 2s infinite'
      },
      error: {
        borderColor: '#FF0000',
        color: '#FF0000',
        boxShadow: '0 0 15px #FF0000',
        animation: 'nt-blink-red 1s infinite'
      },
      warning: {
        borderColor: '#FF4444',
        color: '#FF4444',
        boxShadow: '0 0 15px #FF4444',
        animation: 'nt-blink-red 1.5s infinite'
      },
      info: {
        borderColor: '#00E0E0',
        color: '#00E0E0',
        boxShadow: '0 0 15px #00E0E0',
        animation: 'nt-blink-green 2s infinite'
      }
    }

    return {
      ...baseStyle,
      ...typeStyle[type]
    }
  }

  return (
    <div style={getTipStyle()}>
      {message}
    </div>
  )
}