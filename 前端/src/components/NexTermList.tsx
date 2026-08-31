import { ReactNode, useState } from 'react'

interface NexTermListProps {
  items: Array<{
    id: string
    content: ReactNode
    selected?: boolean
    disabled?: boolean
    badge?: string
  }>
  onSelect?: (id: string) => void
  onDoubleClick?: (id: string) => void
  selectionMode?: 'single' | 'multiple'
  virtualScroll?: boolean
}

export default function NexTermList({
  items,
  onSelect,
  onDoubleClick,
  selectionMode = 'single',
  virtualScroll = false
}: NexTermListProps) {
  const [selectedItems, setSelectedItems] = useState<string[]>([])

  const handleItemClick = (id: string) => {
    if (selectionMode === 'single') {
      setSelectedItems([id])
    } else {
      setSelectedItems(prev =>
        prev.includes(id) ? prev.filter(itemId => itemId !== id) : [...prev, id]
      )
    }
    if (onSelect) onSelect(id)
  }

  const handleItemDoubleClick = (id: string) => {
    if (onDoubleClick) onDoubleClick(id)
  }

  const getItemStyle = (item: any) => {
    const baseStyle = {
      padding: '10px 15px',
      border: '1px solid transparent',
      backgroundColor: '#000000',
      color: '#00FF00',
      fontFamily: 'Consolas, monospace',
      cursor: item.disabled ? 'not-allowed' : 'pointer',
      transition: 'all 0.3s',
      opacity: item.disabled ? 0.5 : 1,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between'
    }

    if (item.disabled) {
      return baseStyle
    }

    if (selectedItems.includes(item.id) || item.selected) {
      return {
        ...baseStyle,
        borderColor: '#00FF00',
        backgroundColor: '#001100'
      }
    }

    return baseStyle
  }

  const getBadgeStyle = () => ({
    backgroundColor: '#FF0000',
    color: '#FFFFFF',
    borderRadius: '10px',
    padding: '2px 8px',
    fontSize: '10px',
    fontFamily: 'Consolas, monospace'
  })

  return (
    <div style={{
      border: '1.5px solid #00FF00',
      backgroundColor: '#000000',
      maxHeight: virtualScroll ? '400px' : 'none',
      overflow: virtualScroll ? 'auto' : 'visible'
    }}>
      {items.map(item => (
        <div
          key={item.id}
          style={getItemStyle(item)}
          onClick={() => !item.disabled && handleItemClick(item.id)}
          onDoubleClick={() => !item.disabled && handleItemDoubleClick(item.id)}
          onMouseEnter={(e) => {
            if (!item.disabled && !selectedItems.includes(item.id) && !item.selected) {
              e.currentTarget.style.backgroundColor = '#001100'
              e.currentTarget.style.borderColor = '#00E0E0'
            }
          }}
          onMouseLeave={(e) => {
            if (!item.disabled && !selectedItems.includes(item.id) && !item.selected) {
              e.currentTarget.style.backgroundColor = '#000000'
              e.currentTarget.style.borderColor = 'transparent'
            }
          }}
        >
          <div>{item.content}</div>
          {item.badge && (
            <span style={getBadgeStyle()}>
              {item.badge}
            </span>
          )}
        </div>
      ))}
    </div>
  )
}