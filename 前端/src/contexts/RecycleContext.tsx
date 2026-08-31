import { createContext, useContext, useState, useCallback } from 'react'

interface RecycleContextType {
  selectedFiles: string[]
  setSelectedFiles: (files: string[]) => void
  toggleFileSelection: (fileId: string) => void
  selectAll: (allIds: string[]) => void
  clearSelection: () => void
  totalFiles: number
  setTotalFiles: (count: number) => void
}

const RecycleContext = createContext<RecycleContextType | null>(null)

export function RecycleProvider({ children }: { children: React.ReactNode }) {
  const [selectedFiles, setSelectedFiles] = useState<string[]>([])
  const [totalFiles, setTotalFiles] = useState(0)

  const toggleFileSelection = useCallback((fileId: string) => {
    setSelectedFiles(prev =>
      prev.includes(fileId)
        ? prev.filter(id => id !== fileId)
        : [...prev, fileId]
    )
  }, [])

  const selectAll = useCallback((allIds: string[]) => {
    setSelectedFiles(allIds)
  }, [])

  const clearSelection = useCallback(() => {
    setSelectedFiles([])
  }, [])

  return (
    <RecycleContext.Provider value={{
      selectedFiles,
      setSelectedFiles,
      toggleFileSelection,
      selectAll,
      clearSelection,
      totalFiles,
      setTotalFiles,
    }}>
      {children}
    </RecycleContext.Provider>
  )
}

export function useRecycleContext() {
  const ctx = useContext(RecycleContext)
  if (!ctx) throw new Error('useRecycleContext must be used within RecycleProvider')
  return ctx
}