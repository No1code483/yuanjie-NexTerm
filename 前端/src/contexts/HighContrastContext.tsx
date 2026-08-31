import { createContext, useContext, useState, useEffect, useCallback } from 'react'

const HC_KEY = 'nexterm_high_contrast'

interface HighContrastContextType {
  highContrast: boolean
  toggleHighContrast: () => void
}

const HighContrastContext = createContext<HighContrastContextType>({
  highContrast: false,
  toggleHighContrast: () => {},
})

export function HighContrastProvider({ children }: { children: React.ReactNode }) {
  const [highContrast, setHighContrast] = useState(() => {
    return localStorage.getItem(HC_KEY) === 'true'
  })

  useEffect(() => {
    if (highContrast) {
      document.body.classList.add('high-contrast')
    } else {
      document.body.classList.remove('high-contrast')
    }
    localStorage.setItem(HC_KEY, String(highContrast))
  }, [highContrast])

  const toggleHighContrast = useCallback(() => {
    setHighContrast(prev => !prev)
  }, [])

  return (
    <HighContrastContext.Provider value={{ highContrast, toggleHighContrast }}>
      {children}
    </HighContrastContext.Provider>
  )
}

export function useHighContrast() {
  return useContext(HighContrastContext)
}