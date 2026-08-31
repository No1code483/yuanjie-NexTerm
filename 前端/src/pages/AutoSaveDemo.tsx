import { t } from "i18next";
import { AutoSaveInputWithIndicator, AutoSaveTextareaWithIndicator, AutoSaveDemo } from '../components/AutoSaveIndicator';

/**
 * 自动保存演示页面
 */
export default function AutoSaveDemoPage() {
  return <div style={{
    padding: '20px'
  }}>
      <h2 className="nt-title">{t("components.AutoSaveIndicator.k2")}</h2>
      
      <div style={{
      padding: '20px',
      border: '1.5px solid var(--nt-green)',
      marginBottom: '30px'
    }}>
        <h3 style={{
        color: 'var(--nt-green)',
        marginBottom: '15px'
      }}>
          {t("AutoSaveDemo.k1")}
        </h3>
        <ul style={{
        color: 'var(--nt-gray-dark)',
        lineHeight: '1.6',
        marginLeft: '20px'
      }}>
          <li><strong>{t("AutoSaveDemo.k2")}</strong> {t("AutoSaveDemo.k3")}</li>
          <li><strong>{t("AutoSaveDemo.k4")}</strong> {t("AutoSaveDemo.k5")}</li>
          <li><strong>{t("AutoSaveDemo.k6")}</strong> {t("AutoSaveDemo.k7")}</li>
          <li><strong>{t("AutoSaveDemo.k8")}</strong> {t("AutoSaveDemo.k9")}</li>
          <li><strong>{t("AutoSaveDemo.k10")}</strong> {t("AutoSaveDemo.k11")}</li>
        </ul>
      </div>
      
      {/* 实际应用示例 */}
      <div style={{
      marginBottom: '40px'
    }}>
        <h3 className="nt-subtitle">{t("AutoSaveDemo.k12")}</h3>
        
        <div style={{
        display: 'grid',
        gridTemplateColumns: '1fr 1fr',
        gap: '30px'
      }}>
          {/* 左侧：待办事项 */}
          <div>
            <h4 style={{
            color: 'var(--nt-cyan)',
            marginBottom: '15px'
          }}>{t("AutoSaveDemo.k13")}</h4>
            <AutoSaveTextareaWithIndicator storageKey="todo_daily_tasks" defaultValue="" placeholder={t("AutoSaveDemo.k14")} rows={8} label={t("AutoSaveDemo.k15")} />
            
            <AutoSaveTextareaWithIndicator storageKey="todo_weekly_goals" defaultValue="" placeholder={t("AutoSaveDemo.k16")} rows={4} label={t("AutoSaveDemo.k17")} />
          </div>
          
          {/* 右侧：学习笔记 */}
          <div>
            <h4 style={{
            color: 'var(--nt-purple)',
            marginBottom: '15px'
          }}>{t("AutoSaveDemo.k18")}</h4>
            <AutoSaveInputWithIndicator storageKey="notes_topic" defaultValue="" placeholder={t("AutoSaveDemo.k19")} label={t("AutoSaveDemo.k20")} />
            
            <AutoSaveTextareaWithIndicator storageKey="notes_content" defaultValue="" placeholder={t("AutoSaveDemo.k21")} rows={10} label={t("AutoSaveDemo.k22")} />
          </div>
        </div>
      </div>
      
      {/* 配置设置示例 */}
      <div style={{
      marginBottom: '40px'
    }}>
        <h3 className="nt-subtitle">{t("AutoSaveDemo.k23")}</h3>
        
        <div style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(2, 1fr)',
        gap: '20px'
      }}>
          <AutoSaveInputWithIndicator storageKey="config_username" defaultValue="" placeholder={t("common.username")} label={t("common.username")} />
          
          <AutoSaveInputWithIndicator storageKey="config_email" defaultValue="" placeholder={t("AutoSaveDemo.k24")} label={t("AutoSaveDemo.k24")} />
          
          <AutoSaveTextareaWithIndicator storageKey="config_bio" defaultValue="" placeholder={t("AutoSaveDemo.k25")} rows={3} label={t("AutoSaveDemo.k26")} />
          
          <AutoSaveTextareaWithIndicator storageKey="config_preferences" defaultValue="" placeholder={t("AutoSaveDemo.k27")} rows={3} label={t("AutoSaveDemo.k28")} />
        </div>
      </div>
      
      {/* 功能演示组件 */}
      <AutoSaveDemo />
      
      {/* 使用说明 */}
      <div style={{
      padding: '20px',
      border: '1.5px solid var(--nt-gray-dark)',
      marginTop: '40px',
      fontSize: '14px',
      color: 'var(--nt-gray-dark)'
    }}>
        <h4 style={{
        color: 'var(--nt-green)',
        marginBottom: '10px'
      }}>{t("AutoSaveDemo.k29")}</h4>
        <ul style={{
        marginLeft: '20px',
        lineHeight: '1.6'
      }}>
          <li>{t("AutoSaveDemo.k30")}</li>
          <li>{t("AutoSaveDemo.k31")}</li>
          <li>{t("AutoSaveDemo.k32")}</li>
          <li>{t("AutoSaveDemo.k33")}</li>
        </ul>
      </div>
    </div>;
}