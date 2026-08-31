import { t } from "i18next";
import { PermissionRestricted, PermissionButton, PermissionInput } from '../components/PermissionRestricted';
import { usePermission } from '../utils/permission';

/**
 * 权限演示页面
 * 用于展示临时账号权限限制效果
 */
export default function PermissionDemo() {
  const {
    isTempAccount
  } = usePermission('read', 'home');
  return <div style={{
    padding: '20px'
  }}>
      <h2 className="nt-title">{t("PermissionDemo.k1")}</h2>
      
      <div style={{
      marginBottom: '30px'
    }}>
        <div style={{
        padding: '15px',
        border: '1.5px solid var(--nt-green)',
        marginBottom: '20px'
      }}>
          <h3 style={{
          color: 'var(--nt-green)',
          marginBottom: '10px'
        }}>
            {t("PermissionDemo.k2")}
          </h3>
          <p style={{
          color: isTempAccount ? 'var(--nt-red)' : 'var(--nt-green)'
        }}>
            {isTempAccount ? t("PermissionDemo.k3") : t("PermissionDemo.k4")}
          </p>
        </div>
      </div>
      
      {/* 首页权限演示 */}
      <div style={{
      marginBottom: '30px'
    }}>
        <h3 className="nt-subtitle">{t("PermissionDemo.k5")}</h3>
        <div style={{
        display: 'grid',
        gridTemplateColumns: '1fr 1fr',
        gap: '20px'
      }}>
          <div>
            <h4>{t("PermissionDemo.k6")}</h4>
            <PermissionRestricted action="read" resource="home">
              <div style={{
              padding: '15px',
              border: '1.5px solid var(--nt-green)',
              color: 'var(--nt-green)'
            }}>
                {t("PermissionDemo.k7")}
              </div>
            </PermissionRestricted>
          </div>
          
          <div>
            <h4>{t("PermissionDemo.k8")}</h4>
            <PermissionRestricted action="modify" resource="home">
              <div style={{
              padding: '15px',
              border: '1.5px solid var(--nt-green)',
              color: 'var(--nt-green)'
            }}>
                {t("PermissionDemo.k9")}
              </div>
            </PermissionRestricted>
          </div>
        </div>
      </div>
      
      {/* 按钮权限演示 */}
      <div style={{
      marginBottom: '30px'
    }}>
        <h3 className="nt-subtitle">{t("PermissionDemo.k10")}</h3>
        <div style={{
        display: 'flex',
        gap: '10px',
        flexWrap: 'wrap'
      }}>
          <PermissionButton action="read" resource="home">
            {t("PermissionDemo.k11")}
          </PermissionButton>
          
          <PermissionButton action="write" resource="home">
            {t("PermissionDemo.k12")}
          </PermissionButton>
          
          <PermissionButton action="delete" resource="terminal">
            {t("PermissionDemo.k13")}
          </PermissionButton>
          
          <PermissionButton action="modify" resource="profile_settings">
            {t("PermissionDemo.k14")}
          </PermissionButton>
        </div>
      </div>
      
      {/* 输入框权限演示 */}
      <div style={{
      marginBottom: '30px'
    }}>
        <h3 className="nt-subtitle">{t("PermissionDemo.k15")}</h3>
        <div style={{
        display: 'grid',
        gap: '15px',
        maxWidth: '400px'
      }}>
          <div>
            <label style={{
            display: 'block',
            marginBottom: '5px',
            color: 'var(--nt-green)'
          }}>
              {t("PermissionDemo.k16")}
            </label>
            <PermissionInput action="read" resource="home" value={t("PermissionDemo.k17")} placeholder={t("PermissionDemo.k18")} />
          </div>
          
          <div>
            <label style={{
            display: 'block',
            marginBottom: '5px',
            color: 'var(--nt-green)'
          }}>
              {t("PermissionDemo.k19")}
            </label>
            <PermissionInput action="write" resource="terminal" value={t("PermissionDemo.k20")} placeholder={t("PermissionDemo.k21")} />
          </div>
          
          <div>
            <label style={{
            display: 'block',
            marginBottom: '5px',
            color: 'var(--nt-green)'
          }}>
              {t("PermissionDemo.k22")}
            </label>
            <PermissionInput action="modify" resource="profile_settings" value={t("PermissionDemo.k20")} placeholder={t("PermissionDemo.k23")} />
          </div>
        </div>
      </div>
      
      {/* 模块权限说明 */}
      <div style={{
      marginBottom: '30px'
    }}>
        <h3 className="nt-subtitle">{t("PermissionDemo.k24")}</h3>
        <div style={{
        padding: '15px',
        border: '1.5px solid var(--nt-gray-dark)',
        fontSize: '14px',
        lineHeight: '1.6'
      }}>
          <h4 style={{
          color: 'var(--nt-green)',
          marginBottom: '10px'
        }}>{t("PermissionDemo.k25")}</h4>
          <ul style={{
          marginLeft: '20px',
          marginBottom: '15px'
        }}>
            <li>{t("PermissionDemo.k26")}</li>
            <li>{t("PermissionDemo.k27")}</li>
            <li>{t("PermissionDemo.k28")}</li>
            <li>{t("PermissionDemo.k29")}</li>
            <li>{t("PermissionDemo.k30")}</li>
          </ul>
          
          <h4 style={{
          color: 'var(--nt-red)',
          marginBottom: '10px'
        }}>{t("PermissionDemo.k31")}</h4>
          <ul style={{
          marginLeft: '20px'
        }}>
            <li>{t("PermissionDemo.k32")}</li>
            <li>{t("PermissionDemo.k33")}</li>
            <li>{t("PermissionDemo.k34")}</li>
            <li>{t("PermissionDemo.k35")}</li>
          </ul>
        </div>
      </div>
    </div>;
}