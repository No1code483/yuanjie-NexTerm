import { CSSProperties } from 'react'

export const createFlexStyle = (
  direction: 'row' | 'column' = 'row',
  align: 'flex-start' | 'center' | 'flex-end' = 'center',
  justify: 'flex-start' | 'center' | 'flex-end' | 'space-between' = 'center'
): CSSProperties => ({
  display: 'flex',
  flexDirection: direction,
  alignItems: align,
  justifyContent: justify
})

export const createBorderStyle = (
  width: string = 'var(--nt-border-width)',
  color: string = 'var(--nt-green)',
  style: string = 'solid'
): CSSProperties => ({
  border: `${width} ${style} ${color}`
})

export const createPaddingStyle = (value: string | number = '20px'): CSSProperties => ({
  padding: typeof value === 'number' ? `${value}px` : value
})

export const createColorStyle = (
  color: string = 'var(--nt-green)',
  backgroundColor: string = 'var(--nt-black)'
): CSSProperties => ({
  color,
  backgroundColor
})

export const createTextStyle = (
  fontSize: string = '14px',
  fontFamily: string = 'var(--nt-font-mono)',
  fontWeight: string = 'normal'
): CSSProperties => ({
  fontSize,
  fontFamily,
  fontWeight
})

export const createHoverEffect = (
  baseColor: string = 'var(--nt-green)',
  hoverColor: string = 'var(--nt-cyan)',
  shadowColor: string = 'var(--nt-green)'
): {
  base: CSSProperties
  hover: CSSProperties
} => ({
  base: {
    color: baseColor,
    transition: 'all 0.3s'
  },
  hover: {
    color: hoverColor,
    boxShadow: `0 0 10px ${shadowColor}`
  }
})

export const createButtonStyle = (
  variant: 'default' | 'danger' | 'warning' | 'ghost' = 'default',
  size: 'small' | 'medium' | 'large' = 'medium'
): CSSProperties => {
  const baseStyle: CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    cursor: 'pointer',
    transition: 'all 0.3s',
    fontFamily: 'var(--nt-font-mono)',
    border: 'var(--nt-border-width) solid',
    background: 'var(--nt-black)',
    outline: 'none'
  }

  const variantStyles: Record<string, CSSProperties> = {
    default: {
      color: 'var(--nt-green)',
      borderColor: 'var(--nt-green)'
    },
    danger: {
      color: 'var(--nt-red)',
      borderColor: 'var(--nt-red)'
    },
    warning: {
      color: 'var(--nt-light-red)',
      borderColor: 'var(--nt-light-red)'
    },
    ghost: {
      background: 'transparent',
      borderColor: 'transparent',
      color: 'var(--nt-green)'
    }
  }

  const sizeStyles: Record<string, CSSProperties> = {
    small: {
      padding: '6px 12px',
      fontSize: '12px',
      minWidth: '60px',
      minHeight: '24px'
    },
    medium: {
      padding: '10px 20px',
      fontSize: '14px',
      minWidth: '80px',
      minHeight: '32px'
    },
    large: {
      padding: '14px 28px',
      fontSize: '16px',
      minWidth: '100px',
      minHeight: '40px'
    }
  }

  return {
    ...baseStyle,
    ...variantStyles[variant],
    ...sizeStyles[size]
  }
}

export const createSidebarItemStyle = (
  active: boolean = false,
  isDanger: boolean = false
): CSSProperties => {
  const baseStyle: CSSProperties = {
    padding: '6px 8px',
    border: 'var(--nt-border-width) solid transparent',
    cursor: 'pointer',
    transition: 'all 0.3s',
    display: 'flex',
    alignItems: 'center',
    gap: '6px'
  }

  if (active) {
    return {
      ...baseStyle,
      borderColor: isDanger ? 'var(--nt-red)' : 'var(--nt-green)',
      color: isDanger ? 'var(--nt-red)' : 'var(--nt-green)',
      background: `rgba(${isDanger ? '255, 0, 0' : '0, 255, 0'}, 0.1)`
    }
  }

  return {
    ...baseStyle,
    color: isDanger ? 'var(--nt-light-red)' : 'var(--nt-gray-dark)'
  }
}

export const createHeaderStyle = (
  height: string | number = '50px',
  padding: string | number = '0 20px'
): CSSProperties => ({
  height: typeof height === 'number' ? `${height}px` : height,
  background: 'var(--nt-black)',
  borderBottom: 'var(--nt-border-width) solid var(--nt-green)',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: typeof padding === 'number' ? `${padding}px` : padding
})

export const createSidebarStyle = (
  width: string | number = '120px',
  padding: string | number = '15px 0'
): CSSProperties => ({
  width: typeof width === 'number' ? `${width}px` : width,
  borderRight: 'var(--nt-border-width) solid var(--nt-green)',
  background: 'var(--nt-black)',
  padding: typeof padding === 'number' ? `${padding}px` : padding,
  display: 'flex',
  flexDirection: 'column',
  position: 'relative'
})

export const createModalStyle = (
  variant: 'default' | 'danger' | 'warning' = 'default'
): CSSProperties => {
  const variantStyles = {
    default: {
      borderColor: 'var(--nt-green)',
      boxShadow: '0 0 20px var(--nt-green)'
    },
    danger: {
      borderColor: 'var(--nt-red)',
      boxShadow: '0 0 20px var(--nt-red)'
    },
    warning: {
      borderColor: 'var(--nt-warning)',
      boxShadow: '0 0 20px var(--nt-warning)'
    }
  }

  return {
    background: 'var(--nt-black)',
    border: 'var(--nt-border-width) solid',
    color: 'var(--nt-green)',
    fontFamily: 'var(--nt-font-mono)',
    maxHeight: '80vh',
    overflow: 'auto',
    ...variantStyles[variant]
  }
}

export const createModalVariantStyle = (
  variant: 'default' | 'danger' | 'warning' = 'default'
): CSSProperties => {
  const variantStyles: Record<string, CSSProperties> = {
    default: {
      borderColor: 'var(--nt-green)',
      boxShadow: '0 0 20px var(--nt-green)'
    },
    danger: {
      borderColor: 'var(--nt-red)',
      boxShadow: '0 0 20px var(--nt-red)'
    },
    warning: {
      borderColor: 'var(--nt-light-red)',
      boxShadow: '0 0 20px var(--nt-light-red)'
    }
  }
  return variantStyles[variant]
}

export const createHeaderVariantStyle = (
  variant: 'default' | 'danger' | 'warning' = 'default'
): CSSProperties => {
  const variantStyles: Record<string, CSSProperties> = {
    default: {
      borderColor: 'var(--nt-green)'
    },
    danger: {
      borderColor: 'var(--nt-red)'
    },
    warning: {
      borderColor: 'var(--nt-light-red)'
    }
  }
  return variantStyles[variant]
}

export const createTitleVariantStyle = (
  variant: 'default' | 'danger' | 'warning' = 'default'
): CSSProperties => {
  const variantStyles: Record<string, CSSProperties> = {
    default: {
      color: 'var(--nt-green)'
    },
    danger: {
      color: 'var(--nt-red)'
    },
    warning: {
      color: 'var(--nt-light-red)'
    }
  }
  return variantStyles[variant]
}
