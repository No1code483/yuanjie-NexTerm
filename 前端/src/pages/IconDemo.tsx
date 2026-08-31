import { t } from "i18next";
import { useState } from 'react';
import { Icon, IconName } from '../components/icons/Icon';

/**
 * 图标演示页面
 */
export default function IconDemoPage() {
  const [selectedIcon, setSelectedIcon] = useState<IconName>('home');
  const [iconSize, setIconSize] = useState<number>(24);
  const [iconColor, setIconColor] = useState<string>('#00FF00');
  const [showAnimation, setShowAnimation] = useState<boolean>(false);
  const iconCategories = {
    '导航': ['home', 'back'],
    '用户': ['profile', 'logout'],
    '系统': ['settings', 'trash'],
    '功能': ['knowledge', 'ai', 'game', 'terminal', 'xin'],
    '操作': ['add', 'delete'],
    '内容': ['news', 'todo', 'log', 'timer']
  };
  return <div style={{
    padding: '20px'
  }}>
      <h2 className="nt-title">{t("IconDemo.k1")}</h2>
      
      {/* 图标预览区 */}
      <div style={{
      padding: '40px',
      border: '1.5px solid var(--nt-green)',
      marginBottom: '30px',
      textAlign: 'center',
      backgroundColor: 'rgba(0, 255, 0, 0.05)'
    }}>
        <h3 style={{
        color: 'var(--nt-green)',
        marginBottom: '20px'
      }}>
          {t("IconDemo.k2")}
        </h3>
        
        <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '20px',
        marginBottom: '20px'
      }}>
          <Icon name={selectedIcon} size={iconSize} color={iconColor} className={showAnimation ? 'nt-icon-bounce' : ''} />
          
          <div style={{
          textAlign: 'left'
        }}>
            <div style={{
            color: 'var(--nt-green)',
            fontSize: '18px',
            fontWeight: 'bold'
          }}>
              {selectedIcon}
            </div>
            <div style={{
            color: 'var(--nt-gray-dark)',
            fontSize: '14px'
          }}>
              {t("IconDemo.k3")} {iconSize}{t("IconDemo.k4")} {iconColor}
            </div>
          </div>
        </div>
        
        {/* 控制面板 */}
        <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(3, 1fr)',
        gap: '20px',
        maxWidth: '400px',
        margin: '0 auto'
      }}>
          <div>
            <label style={{
            color: 'var(--nt-green)',
            fontSize: '12px'
          }}>
              {t("IconDemo.k5")}
            </label>
            <input type="range" min="16" max="64" value={iconSize} onChange={e => setIconSize(parseInt(e.target.value))} style={{
            width: '100%'
          }} />
          </div>
          
          <div>
            <label style={{
            color: 'var(--nt-green)',
            fontSize: '12px'
          }}>
              {t("IconDemo.k6")}
            </label>
            <input type="color" value={iconColor} onChange={e => setIconColor(e.target.value)} style={{
            width: '100%',
            height: '30px'
          }} />
          </div>
          
          <div>
            <label style={{
            color: 'var(--nt-green)',
            fontSize: '12px'
          }}>
              {t("lib.xinChatEngine.k28")}
            </label>
            <button onClick={() => setShowAnimation(!showAnimation)} style={{
            width: '100%',
            padding: '5px',
            backgroundColor: showAnimation ? 'var(--nt-green)' : 'transparent',
            color: showAnimation ? 'var(--nt-black)' : 'var(--nt-green)',
            border: '1px solid var(--nt-green)'
          }}>
              {showAnimation ? t("common.close") : t("IconDemo.k7")}
            </button>
          </div>
        </div>
      </div>
      
      {/* 图标库展示 */}
      <div style={{
      marginBottom: '40px'
    }}>
        <h3 className="nt-subtitle">{t("IconDemo.k8")}</h3>
        
        {Object.entries(iconCategories).map(([category, icons]) => <div key={category} style={{
        marginBottom: '30px'
      }}>
            <h4 style={{
          color: 'var(--nt-cyan)',
          marginBottom: '15px',
          borderBottom: '1px solid var(--nt-gray-dark)',
          paddingBottom: '5px'
        }}>
              {category}
            </h4>
            
            <div style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(100px, 1fr))',
          gap: '15px'
        }}>
              {icons.map(iconName => <div key={iconName} onClick={() => setSelectedIcon(iconName)} style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            padding: '15px',
            border: selectedIcon === iconName ? '2px solid var(--nt-green)' : '1px solid var(--nt-gray-dark)',
            borderRadius: '8px',
            cursor: 'pointer',
            transition: 'all 0.3s ease',
            backgroundColor: selectedIcon === iconName ? 'rgba(0, 255, 0, 0.1)' : 'transparent'
          }}>
                  <Icon name={iconName} size={32} />
                  <div style={{
              marginTop: '8px',
              fontSize: '12px',
              color: 'var(--nt-gray-dark)',
              textAlign: 'center'
            }}>
                    {iconName}
                  </div>
                </div>)}
            </div>
          </div>)}
      </div>
      
      {/* 交互演示 */}
      <div style={{
      marginBottom: '40px'
    }}>
        <h3 className="nt-subtitle">{t("IconDemo.k9")}</h3>
        
        <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))',
        gap: '20px'
      }}>
          {/* 图标按钮 */}
          <div style={{
          padding: '20px',
          border: '1px solid var(--nt-gray-dark)',
          borderRadius: '8px'
        }}>
            <h5 style={{
            color: 'var(--nt-green)',
            marginBottom: '15px'
          }}>{t("IconDemo.k10")}</h5>
            <div style={{
            display: 'flex',
            gap: '10px',
            flexWrap: 'wrap'
          }}>
              <button className="nt-icon-button">
                <Icon name="add" size={20} />
              </button>
              <button className="nt-icon-button">
                <Icon name="delete" size={20} />
              </button>
              <button className="nt-icon-button">
                <Icon name="settings" size={20} />
              </button>
              <button className="nt-icon-button">
                <Icon name="logout" size={20} />
              </button>
            </div>
          </div>
          
          {/* 图标标签 */}
          <div style={{
          padding: '20px',
          border: '1px solid var(--nt-gray-dark)',
          borderRadius: '8px'
        }}>
            <h5 style={{
            color: 'var(--nt-green)',
            marginBottom: '15px'
          }}>{t("IconDemo.k11")}</h5>
            <div style={{
            display: 'flex',
            flexDirection: 'column',
            gap: '15px'
          }}>
              <div className="nt-icon-label">
                <Icon name="home" size={20} />
                <span className="label-text">{t("components.intelligence.DashboardPanel.k106")}</span>
              </div>
              <div className="nt-icon-label">
                <Icon name="profile" size={20} />
                <span className="label-text">{t("components.intelligence.DashboardPanel.k105")}</span>
              </div>
              <div className="nt-icon-label">
                <Icon name="knowledge" size={20} />
                <span className="label-text">{t("components.intelligence.ActivityPanel.k1")}</span>
              </div>
            </div>
          </div>
          
          {/* 图标组 */}
          <div style={{
          padding: '20px',
          border: '1px solid var(--nt-gray-dark)',
          borderRadius: '8px'
        }}>
            <h5 style={{
            color: 'var(--nt-green)',
            marginBottom: '15px'
          }}>{t("IconDemo.k12")}</h5>
            <div className="nt-icon-group">
              <Icon name="add" size={20} />
              <Icon name="delete" size={20} />
              <Icon name="settings" size={20} />
              <Icon name="logout" size={20} />
            </div>
            <div className="nt-icon-group vertical" style={{
            marginTop: '15px'
          }}>
              <Icon name="home" size={20} />
              <Icon name="profile" size={20} />
              <Icon name="knowledge" size={20} />
            </div>
          </div>
          
          {/* 图标徽章 */}
          <div style={{
          padding: '20px',
          border: '1px solid var(--nt-gray-dark)',
          borderRadius: '8px'
        }}>
            <h5 style={{
            color: 'var(--nt-green)',
            marginBottom: '15px'
          }}>{t("IconDemo.k13")}</h5>
            <div style={{
            display: 'flex',
            gap: '20px'
          }}>
              <div className="nt-icon-badge">
                <Icon name="news" size={24} />
                <div className="badge">3</div>
              </div>
              <div className="nt-icon-badge">
                <Icon name="todo" size={24} />
                <div className="badge">5</div>
              </div>
              <div className="nt-icon-badge">
                <Icon name="log" size={24} />
                <div className="badge">12</div>
              </div>
            </div>
          </div>
        </div>
      </div>
      
      {/* 使用说明 */}
      <div style={{
      padding: '20px',
      border: '1.5px solid var(--nt-gray-dark)',
      fontSize: '14px',
      color: 'var(--nt-gray-dark)'
    }}>
        <h4 style={{
        color: 'var(--nt-green)',
        marginBottom: '10px'
      }}>{t("IconDemo.k14")}</h4>
        <ul style={{
        marginLeft: '20px',
        lineHeight: '1.6'
      }}>
          <li><strong>{t("IconDemo.k15")}</strong>: <code>{'<Icon name="home" size={24} color="#00FF00" />'}</code></li>
          <li><strong>{t("IconDemo.k10")}</strong>{t("IconDemo.k16")} <code>nt-icon-button</code> {t("IconDemo.k17")}</li>
          <li><strong>{t("IconDemo.k11")}</strong>{t("IconDemo.k16")} <code>nt-icon-label</code> {t("IconDemo.k17")}</li>
          <li><strong>{t("IconDemo.k12")}</strong>{t("IconDemo.k16")} <code>nt-icon-group</code> {t("IconDemo.k17")}</li>
          <li><strong>{t("IconDemo.k13")}</strong>{t("IconDemo.k16")} <code>nt-icon-badge</code> {t("IconDemo.k17")}</li>
          <li><strong>{t("IconDemo.k18")}</strong>{t("IconDemo.k19")} <code>nt-icon-spin</code> {t("IconDemo.k20")} <code>nt-icon-bounce</code> {t("IconDemo.k17")}</li>
        </ul>
      </div>
    </div>;
}