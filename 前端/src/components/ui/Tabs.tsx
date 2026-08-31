import { memo } from 'react'
import styles from './Tabs.module.css'

interface Tab {
  key: string
  label: string
}

interface TabsProps {
  tabs: Tab[]
  activeKey: string
  onChange: (key: string) => void
  className?: string
}

const Tabs = memo(function Tabs({ tabs, activeKey, onChange, className = '' }: TabsProps) {
  return (
    <div className={`${styles.tabs} ${className}`} role="tablist">
      {tabs.map(tab => (
        <button
          key={tab.key}
          role="tab"
          aria-selected={activeKey === tab.key}
          className={`${styles.tab} ${activeKey === tab.key ? styles.active : ''}`}
          onClick={() => onChange(tab.key)}
        >
          {tab.label}
        </button>
      ))}
    </div>
  )
})

export default Tabs