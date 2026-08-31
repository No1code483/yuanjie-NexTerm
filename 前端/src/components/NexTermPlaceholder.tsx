import { t } from "i18next";
import { ReactNode } from 'react';
interface NexTermPlaceholderProps {
  title?: string;
  description?: string;
  icon?: ReactNode;
  size?: 'small' | 'medium' | 'large';
}
export default function NexTermPlaceholder({
  title = t("components.NexTermPlaceholder.k1"),
  description,
  icon,
  size = 'medium'
}: NexTermPlaceholderProps) {
  const getContainerStyle = () => {
    const baseStyle = {
      display: 'flex',
      flexDirection: 'column' as const,
      alignItems: 'center',
      justifyContent: 'center',
      backgroundColor: '#000000',
      border: '1.5px solid #00FF00',
      color: '#888888',
      fontFamily: 'Consolas, monospace',
      textAlign: 'center' as const
    };
    const sizeStyle = {
      small: {
        padding: '20px',
        minHeight: '100px'
      },
      medium: {
        padding: '40px',
        minHeight: '200px'
      },
      large: {
        padding: '60px',
        minHeight: '300px'
      }
    };
    return {
      ...baseStyle,
      ...sizeStyle[size]
    };
  };
  const getTitleStyle = () => ({
    fontSize: size === 'small' ? '16px' : size === 'medium' ? '24px' : '32px',
    marginBottom: description ? '10px' : '0',
    color: '#888888'
  });
  const getDescriptionStyle = () => ({
    fontSize: size === 'small' ? '12px' : size === 'medium' ? '14px' : '16px',
    color: '#888888'
  });
  const getIconStyle = () => ({
    fontSize: size === 'small' ? '32px' : size === 'medium' ? '48px' : '64px',
    marginBottom: '20px',
    color: '#888888'
  });
  return <div style={getContainerStyle()}>
      {icon && <div style={getIconStyle()}>
          {icon}
        </div>}
      <div style={getTitleStyle()}>
        {title}
      </div>
      {description && <div style={getDescriptionStyle()}>
          {description}
        </div>}
    </div>;
}