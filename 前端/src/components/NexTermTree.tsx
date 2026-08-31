import { ReactNode, useState } from 'react'

interface TreeNode {
  id: string
  label: ReactNode
  children?: TreeNode[]
  expanded?: boolean
  selected?: boolean
  disabled?: boolean
  icon?: ReactNode
}

interface NexTermTreeProps {
  data: TreeNode[]
  onNodeClick?: (node: TreeNode) => void
  onNodeToggle?: (node: TreeNode) => void
  showIcons?: boolean
}

export default function NexTermTree({
  data,
  onNodeClick,
  onNodeToggle,
  showIcons = true
}: NexTermTreeProps) {
  const [expandedNodes, setExpandedNodes] = useState<string[]>([])
  const [selectedNode, setSelectedNode] = useState<string | null>(null)

  const isExpanded = (nodeId: string) => {
    return expandedNodes.includes(nodeId)
  }

  const toggleNode = (node: TreeNode) => {
    if (node.children && node.children.length > 0) {
      setExpandedNodes(prev =>
        isExpanded(node.id) ? prev.filter(id => id !== node.id) : [...prev, node.id]
      )
      if (onNodeToggle) onNodeToggle(node)
    }
  }

  const handleNodeClick = (node: TreeNode) => {
    if (!node.disabled) {
      setSelectedNode(node.id)
      if (onNodeClick) onNodeClick(node)
    }
  }

  const renderTreeNode = (node: TreeNode, level = 0) => {
    const hasChildren = node.children && node.children.length > 0
    const expanded = isExpanded(node.id)
    const selected = selectedNode === node.id || node.selected

    const getNodeStyle = () => ({
      padding: '8px 15px',
      paddingLeft: `${15 + level * 20}px`,
      border: selected ? '1px solid #00FF00' : '1px solid transparent',
      backgroundColor: selected ? '#001100' : '#000000',
      color: node.disabled ? '#888888' : '#00FF00',
      fontFamily: 'Consolas, monospace',
      cursor: node.disabled ? 'not-allowed' : 'pointer',
      transition: 'all 0.3s',
      display: 'flex',
      alignItems: 'center',
      gap: '8px'
    })

    const getExpandIconStyle = () => ({
      width: '16px',
      height: '16px',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      color: '#00FF00',
      transform: expanded ? 'rotate(90deg)' : 'rotate(0deg)',
      transition: 'transform 0.3s'
    })

    return (
      <div key={node.id}>
        <div
          style={getNodeStyle()}
          onClick={() => handleNodeClick(node)}
          onMouseEnter={(e) => {
            if (!node.disabled && !selected) {
              e.currentTarget.style.backgroundColor = '#001100'
              e.currentTarget.style.borderColor = '#00E0E0'
            }
          }}
          onMouseLeave={(e) => {
            if (!node.disabled && !selected) {
              e.currentTarget.style.backgroundColor = '#000000'
              e.currentTarget.style.borderColor = 'transparent'
            }
          }}
        >
          {hasChildren && (
            <div
              style={getExpandIconStyle()}
              onClick={(e) => {
                e.stopPropagation()
                toggleNode(node)
              }}
            >
              ▶
            </div>
          )}
          {!hasChildren && showIcons && (
            <div style={{ width: '16px', height: '16px' }}>
              {node.icon || '•'}
            </div>
          )}
          <div>{node.label}</div>
        </div>
        {hasChildren && expanded && (
          <div>
            {node.children!.map(child => renderTreeNode(child, level + 1))}
          </div>
        )}
      </div>
    )
  }

  return (
    <div style={{
      border: '1.5px solid #00FF00',
      backgroundColor: '#000000',
      maxHeight: '400px',
      overflow: 'auto'
    }}>
      {data.map(node => renderTreeNode(node))}
    </div>
  )
}