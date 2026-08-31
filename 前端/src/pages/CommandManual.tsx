import { t } from "i18next";
import { useNavigate, useLocation } from 'react-router-dom';
import styles from './CommandManual.module.css';
const terminalCommands = [{
  name: 'help',
  syntax: 'help',
  desc: t("CommandManual.k1"),
  category: t("CommandManual.k2")
}, {
  name: 'ls',
  syntax: t("CommandManual.k3"),
  desc: t("CommandManual.k4"),
  category: t("CommandManual.k5")
}, {
  name: 'pwd',
  syntax: 'pwd',
  desc: t("CommandManual.k6"),
  category: t("CommandManual.k5")
}, {
  name: 'cd',
  syntax: t("CommandManual.k7"),
  desc: t("CommandManual.k8"),
  category: t("CommandManual.k5")
}, {
  name: 'mkdir',
  syntax: t("CommandManual.k9"),
  desc: t("CommandManual.k10"),
  category: t("CommandManual.k5")
}, {
  name: 'cat',
  syntax: t("CommandManual.k11"),
  desc: t("CommandManual.k12"),
  category: t("CommandManual.k5")
}, {
  name: 'tree',
  syntax: t("CommandManual.k13"),
  desc: t("CommandManual.k14"),
  category: t("CommandManual.k5")
}, {
  name: 'cp',
  syntax: t("CommandManual.k15"),
  desc: t("CommandManual.k16"),
  category: t("CommandManual.k5")
}, {
  name: 'mv',
  syntax: t("CommandManual.k17"),
  desc: t("CommandManual.k18"),
  category: t("CommandManual.k5")
}, {
  name: 'rm',
  syntax: t("CommandManual.k19"),
  desc: t("CommandManual.k20"),
  category: t("CommandManual.k5")
}, {
  name: 'touch',
  syntax: t("CommandManual.k21"),
  desc: t("CommandManual.k22"),
  category: t("CommandManual.k5")
}, {
  name: 'grep',
  syntax: t("CommandManual.k23"),
  desc: t("CommandManual.k24"),
  category: t("CommandManual.k5")
}, {
  name: 'find',
  syntax: t("CommandManual.k25"),
  desc: t("CommandManual.k26"),
  category: t("CommandManual.k5")
}, {
  name: 'wc',
  syntax: t("CommandManual.k27"),
  desc: t("CommandManual.k28"),
  category: t("CommandManual.k5")
}, {
  name: 'head',
  syntax: t("CommandManual.k29"),
  desc: t("CommandManual.k30"),
  category: t("CommandManual.k5")
}, {
  name: 'tail',
  syntax: t("CommandManual.k31"),
  desc: t("CommandManual.k32"),
  category: t("CommandManual.k5")
}, {
  name: 'echo',
  syntax: t("CommandManual.k33"),
  desc: t("CommandManual.k34"),
  category: t("CommandManual.k2")
}, {
  name: 'date',
  syntax: 'date',
  desc: t("CommandManual.k35"),
  category: t("CommandManual.k36")
}, {
  name: 'whoami',
  syntax: 'whoami',
  desc: t("CommandManual.k37"),
  category: t("CommandManual.k36")
}, {
  name: 'env',
  syntax: 'env',
  desc: t("CommandManual.k38"),
  category: t("CommandManual.k36")
}, {
  name: 'sysinfo',
  syntax: 'sysinfo',
  desc: t("CommandManual.k39"),
  category: t("CommandManual.k36")
}, {
  name: 'version',
  syntax: 'version',
  desc: t("CommandManual.k40"),
  category: t("CommandManual.k36")
}, {
  name: 'clear',
  syntax: 'clear',
  desc: t("CommandManual.k41"),
  category: t("CommandManual.k2")
}, {
  name: 'clearscrollback',
  syntax: 'clearscrollback',
  desc: t("CommandManual.k42"),
  category: t("CommandManual.k2")
}, {
  name: 'history',
  syntax: 'history',
  desc: t("CommandManual.k43"),
  category: t("CommandManual.k2")
}, {
  name: 'reset',
  syntax: 'reset',
  desc: t("CommandManual.k44"),
  category: t("CommandManual.k2")
}, {
  name: 'fontsize',
  syntax: 'fontsize <+/up/increase | -/down/decrease | 0/reset>',
  desc: t("CommandManual.k45"),
  category: t("CommandManual.k2")
}, {
  name: 'fullscreen',
  syntax: 'fullscreen',
  desc: t("CommandManual.k46"),
  category: t("CommandManual.k2")
}, {
  name: 'reload',
  syntax: 'reload',
  desc: t("CommandManual.k47"),
  category: t("CommandManual.k2")
}, {
  name: 'scroll',
  syntax: 'scroll <top | bottom | up [N] | down [N]>',
  desc: t("CommandManual.k48"),
  category: t("CommandManual.k2")
}, {
  name: 'search',
  syntax: t("CommandManual.k49"),
  desc: t("CommandManual.k50"),
  category: t("CommandManual.k2")
}, {
  name: 'hide',
  syntax: 'hide',
  desc: t("CommandManual.k51"),
  category: t("CommandManual.k2")
}, {
  name: 'quit',
  syntax: 'quit',
  desc: t("CommandManual.k52"),
  category: t("CommandManual.k2")
}, {
  name: 'alwaysontop',
  syntax: 'alwaysontop',
  desc: t("CommandManual.k53"),
  category: t("CommandManual.k2")
}, {
  name: 'cmd',
  syntax: 'cmd',
  desc: t("CommandManual.k54"),
  category: t("CommandManual.k55")
}, {
  name: 'powershell',
  syntax: 'powershell',
  desc: t("CommandManual.k56"),
  category: t("CommandManual.k55")
}, {
  name: 'exit',
  syntax: 'exit',
  desc: t("CommandManual.k57"),
  category: t("CommandManual.k55")
}];
const terminalAliases = [{
  name: 'll / la',
  syntax: '→ ls',
  desc: t("CommandManual.k58"),
  category: t("CommandManual.k59")
}, {
  name: 'cls',
  syntax: '→ clear',
  desc: t("CommandManual.k60"),
  category: t("CommandManual.k59")
}, {
  name: 'dir',
  syntax: '→ ls',
  desc: t("CommandManual.k61"),
  category: t("CommandManual.k59")
}, {
  name: 'type',
  syntax: '→ cat',
  desc: t("CommandManual.k62"),
  category: t("CommandManual.k59")
}, {
  name: 'copy',
  syntax: '→ cp',
  desc: t("CommandManual.k63"),
  category: t("CommandManual.k59")
}, {
  name: 'move / ren',
  syntax: '→ mv',
  desc: t("CommandManual.k64"),
  category: t("CommandManual.k59")
}, {
  name: 'del / erase / rd',
  syntax: '→ rm',
  desc: t("CommandManual.k65"),
  category: t("CommandManual.k59")
}, {
  name: 'md',
  syntax: '→ mkdir',
  desc: t("CommandManual.k66"),
  category: t("CommandManual.k59")
}, {
  name: 'minimize',
  syntax: '→ hide',
  desc: t("CommandManual.k67"),
  category: t("CommandManual.k59")
}, {
  name: 'top',
  syntax: '→ alwaysontop',
  desc: t("CommandManual.k68"),
  category: t("CommandManual.k59")
}];
const yuancodeCommands = [{
  name: 'Ctrl + S',
  syntax: 'Ctrl + S',
  desc: t("CommandManual.k69"),
  category: t("CommandManual.k70")
}, {
  name: 'Ctrl + Enter',
  syntax: 'Ctrl + Enter',
  desc: t("CommandManual.k71"),
  category: t("CommandManual.k72")
}, {
  name: 'Ctrl + F',
  syntax: 'Ctrl + F',
  desc: t("CommandManual.k73"),
  category: t("CommandManual.k74")
}, {
  name: 'Ctrl + H',
  syntax: 'Ctrl + H',
  desc: t("CommandManual.k75"),
  category: t("CommandManual.k74")
}, {
  name: 'Enter',
  syntax: 'Enter',
  desc: t("CommandManual.k76"),
  category: t("CommandManual.k74")
}, {
  name: 'Shift + Enter',
  syntax: 'Shift + Enter',
  desc: t("CommandManual.k77"),
  category: t("CommandManual.k74")
}, {
  name: 'Esc',
  syntax: 'Esc',
  desc: t("CommandManual.k78"),
  category: t("CommandManual.k70")
}, {
  name: 'Alt + Click',
  syntax: 'Alt + Click',
  desc: t("CommandManual.k79"),
  category: t("CommandManual.k80")
}, {
  name: 'Ctrl + D',
  syntax: 'Ctrl + D',
  desc: t("CommandManual.k81"),
  category: t("CommandManual.k80")
}, {
  name: 'Ctrl + Shift + L',
  syntax: 'Ctrl + Shift + L',
  desc: t("CommandManual.k82"),
  category: t("CommandManual.k80")
}, {
  name: 'Ctrl + Alt + ↑',
  syntax: 'Ctrl + Alt + ↑',
  desc: t("CommandManual.k83"),
  category: t("CommandManual.k80")
}, {
  name: 'Ctrl + Alt + ↓',
  syntax: 'Ctrl + Alt + ↓',
  desc: t("CommandManual.k84"),
  category: t("CommandManual.k80")
}, {
  name: 'Ctrl + U',
  syntax: 'Ctrl + U',
  desc: t("CommandManual.k85"),
  category: t("CommandManual.k80")
}, {
  name: t("CommandManual.k86"),
  syntax: t("CommandManual.k86"),
  desc: t("CommandManual.k87"),
  category: t("CommandManual.k80")
}, {
  name: 'Shift + Alt + I',
  syntax: 'Shift + Alt + I',
  desc: t("CommandManual.k88"),
  category: t("CommandManual.k80")
}, {
  name: 'Tab',
  syntax: 'Tab',
  desc: t("CommandManual.k89"),
  category: t("CommandManual.k70")
}, {
  name: '/gen',
  syntax: t("CommandManual.k90"),
  desc: t("CommandManual.k91"),
  category: t("CommandManual.k92")
}, {
  name: '/complete',
  syntax: '/complete [prompt]',
  desc: t("CommandManual.k93"),
  category: t("CommandManual.k92")
}, {
  name: '/explain',
  syntax: t("CommandManual.k94"),
  desc: t("CommandManual.k95"),
  category: t("CommandManual.k92")
}, {
  name: '/run',
  syntax: '/run [lang]',
  desc: t("CommandManual.k96"),
  category: t("CommandManual.k72")
}, {
  name: '/stdin',
  syntax: t("CommandManual.k97"),
  desc: t("CommandManual.k98"),
  category: t("CommandManual.k99")
}, {
  name: '/cancel',
  syntax: t("CommandManual.k100"),
  desc: t("CommandManual.k101"),
  category: t("CommandManual.k99")
}, {
  name: '/kill',
  syntax: t("CommandManual.k102"),
  desc: t("CommandManual.k103"),
  category: t("CommandManual.k99")
}, {
  name: '/search',
  syntax: '/search <regex>',
  desc: t("CommandManual.k104"),
  category: t("CommandManual.k105")
}, {
  name: '/diff',
  syntax: '/diff',
  desc: t("CommandManual.k106"),
  category: 'Diff ✅'
}, {
  name: '/snippet',
  syntax: '/snippet <name>',
  desc: t("CommandManual.k107"),
  category: t("CommandManual.k108")
}, {
  name: t("CommandManual.k109"),
  syntax: 'Tab',
  desc: t("CommandManual.k110"),
  category: t("CommandManual.k108")
}, {
  name: t("CommandManual.k111"),
  syntax: t("CommandManual.k112"),
  desc: t("CommandManual.k113"),
  category: t("CommandManual.k108")
}, {
  name: 'Shift+Alt+F',
  syntax: 'Shift + Alt + F',
  desc: t("CommandManual.k114"),
  category: t("CommandManual.k115")
}, {
  name: t("CommandManual.k116"),
  syntax: t("CommandManual.k117"),
  desc: t("CommandManual.k118"),
  category: t("CommandManual.k115")
}, {
  name: 'Shift+Alt+→',
  syntax: 'Shift + Alt + →',
  desc: t("CommandManual.k119"),
  category: t("CommandManual.k120")
}, {
  name: 'Shift+Alt+←',
  syntax: 'Shift + Alt + ←',
  desc: t("CommandManual.k121"),
  category: t("CommandManual.k120")
}, {
  name: '/ws save',
  syntax: '/ws save',
  desc: t("CommandManual.k122"),
  category: t("CommandManual.k123")
}, {
  name: '/ws load',
  syntax: '/ws load',
  desc: t("CommandManual.k124"),
  category: t("CommandManual.k123")
}, {
  name: '/agent deploy',
  syntax: t("CommandManual.k125"),
  desc: t("CommandManual.k126"),
  category: t("CommandManual.k127")
}, {
  name: '/agent spawn',
  syntax: t("CommandManual.k128"),
  desc: t("CommandManual.k129"),
  category: t("CommandManual.k127")
}, {
  name: '/agent stop',
  syntax: t("CommandManual.k130"),
  desc: t("CommandManual.k131"),
  category: t("CommandManual.k127")
}, {
  name: 'yuan_agent_list_templates',
  syntax: t("CommandManual.k132"),
  desc: t("CommandManual.k133"),
  category: t("CommandManual.k127")
}, {
  name: 'MultiAgentOrchestrator',
  syntax: t("CommandManual.k134"),
  desc: t("CommandManual.k135"),
  category: t("CommandManual.k127")
}, {
  name: '/sandbox config',
  syntax: t("CommandManual.k136"),
  desc: t("CommandManual.k137"),
  category: t("CommandManual.k138")
}, {
  name: '/skill manage',
  syntax: t("CommandManual.k139"),
  desc: t("CommandManual.k140"),
  category: t("CommandManual.k141")
}, {
  name: '/settings',
  syntax: t("CommandManual.k142"),
  desc: t("CommandManual.k143"),
  category: t("CommandManual.k144")
},
// Phase 2: 核心引擎层
{
  name: 'engine_create_session',
  syntax: t("CommandManual.k145"),
  desc: t("CommandManual.k146"),
  category: t("CommandManual.k147")
}, {
  name: 'engine_start_turn',
  syntax: t("CommandManual.k148"),
  desc: t("CommandManual.k149"),
  category: t("CommandManual.k147")
}, {
  name: 'engine_complete_turn',
  syntax: t("CommandManual.k150"),
  desc: t("CommandManual.k151"),
  category: t("CommandManual.k147")
}, {
  name: 'engine_subscribe_events',
  syntax: t("CommandManual.k152"),
  desc: t("CommandManual.k153"),
  category: t("CommandManual.k147")
}, {
  name: 'engine_list_sessions',
  syntax: t("CommandManual.k154"),
  desc: t("CommandManual.k155"),
  category: t("CommandManual.k147")
}, {
  name: 'engine_pause/resume',
  syntax: t("CommandManual.k156"),
  desc: t("CommandManual.k157"),
  category: t("CommandManual.k147")
}, {
  name: 'engine_destroy_session',
  syntax: t("CommandManual.k158"),
  desc: t("CommandManual.k159"),
  category: t("CommandManual.k147")
}, {
  name: t("CommandManual.k160"),
  syntax: t("CommandManual.k161"),
  desc: t("CommandManual.k162"),
  category: t("CommandManual.k147")
}, {
  name: t("CommandManual.k163"),
  syntax: t("CommandManual.k161"),
  desc: t("CommandManual.k164"),
  category: t("CommandManual.k147")
}, {
  name: t("CommandManual.k165"),
  syntax: t("CommandManual.k161"),
  desc: t("CommandManual.k166"),
  category: t("CommandManual.k147")
}];
const linuxCommands = [{
  name: 'list_versions',
  syntax: 'linux_list_versions',
  desc: t("CommandManual.k167"),
  category: t("CommandManual.k168")
}, {
  name: 'download',
  syntax: 'linux_download_kernel <version>',
  desc: t("CommandManual.k169"),
  category: t("CommandManual.k168")
}, {
  name: 'kernel_info',
  syntax: 'linux_get_kernel_info <version>',
  desc: t("CommandManual.k170"),
  category: t("CommandManual.k168")
}, {
  name: 'set_active',
  syntax: 'linux_set_active <version>',
  desc: t("CommandManual.k171"),
  category: t("CommandManual.k168")
}, {
  name: 'remove',
  syntax: 'linux_remove_kernel <version>',
  desc: t("CommandManual.k172"),
  category: t("CommandManual.k168")
}, {
  name: 'ls_dir',
  syntax: t("CommandManual.k173"),
  desc: t("CommandManual.k174"),
  category: t("components.Linux.k16")
}, {
  name: 'view_file',
  syntax: t("CommandManual.k175"),
  desc: t("CommandManual.k176"),
  category: t("components.Linux.k16")
}, {
  name: 'search',
  syntax: t("CommandManual.k177"),
  desc: t("CommandManual.k178"),
  category: t("components.Linux.k16")
}, {
  name: 'env',
  syntax: 'linux_get_environment',
  desc: t("CommandManual.k179"),
  category: t("CommandManual.k180")
}, {
  name: 'build',
  syntax: 'linux_v2_build <config>',
  desc: t("CommandManual.k181"),
  category: t("CommandManual.k182")
}, {
  name: 'patch_inspect',
  syntax: 'linux_v2_patch_inspect <patch>',
  desc: t("CommandManual.k183"),
  category: t("CommandManual.k184")
}, {
  name: 'patch_apply',
  syntax: 'linux_v2_patch_apply <patch>',
  desc: t("CommandManual.k185"),
  category: t("CommandManual.k184")
}, {
  name: 'read_config',
  syntax: 'linux_v2_read_config',
  desc: t("CommandManual.k186"),
  category: t("CommandManual.k187")
}, {
  name: 'list_modules',
  syntax: 'linux_v2_list_modules',
  desc: t("CommandManual.k188"),
  category: t("CommandManual.k189")
}, {
  name: 'get_module',
  syntax: t("CommandManual.k190"),
  desc: t("CommandManual.k191"),
  category: t("CommandManual.k189")
}, {
  name: 'analyze_dmesg',
  syntax: 'linux_v2_analyze_dmesg',
  desc: t("CommandManual.k192"),
  category: t("CommandManual.k193")
}, {
  name: 'perf_profile',
  syntax: 'linux_v2_perf_profile',
  desc: t("CommandManual.k194"),
  category: t("CommandManual.k193")
}];
const shortcuts = [{
  key: 'Enter',
  action: t("CommandManual.k195"),
  scope: t("CommandManual.k196")
}, {
  key: 'Tab',
  action: t("CommandManual.k197"),
  scope: t("CommandManual.k196")
}, {
  key: '↑ / ↓',
  action: t("CommandManual.k198"),
  scope: t("CommandManual.k196")
}, {
  key: 'Ctrl + L',
  action: t("CommandManual.k199"),
  scope: t("CommandManual.k196")
}, {
  key: 'Ctrl + Shift + K',
  action: t("CommandManual.k200"),
  scope: t("CommandManual.k196")
}, {
  key: 'Ctrl + C',
  action: t("CommandManual.k201"),
  scope: t("CommandManual.k196")
}, {
  key: 'Ctrl + A',
  action: t("CommandManual.k202"),
  scope: t("CommandManual.k196")
}, {
  key: 'Ctrl + U',
  action: t("CommandManual.k203"),
  scope: t("CommandManual.k196")
}, {
  key: 'Ctrl + =',
  action: t("CommandManual.k204"),
  scope: t("CommandManual.k196")
}, {
  key: 'Ctrl + -',
  action: t("CommandManual.k205"),
  scope: t("CommandManual.k196")
}, {
  key: 'Ctrl + 0',
  action: t("CommandManual.k206"),
  scope: t("CommandManual.k196")
}, {
  key: 'Alt + Enter',
  action: t("CommandManual.k207"),
  scope: t("CommandManual.k196")
}, {
  key: 'Shift + PageUp',
  action: t("CommandManual.k208"),
  scope: t("CommandManual.k196")
}, {
  key: 'Shift + PageDown',
  action: t("CommandManual.k209"),
  scope: t("CommandManual.k196")
}];
const manualList = [{
  id: 'terminal',
  title: t("CommandManual.k210"),
  route: '/terminal/manual/terminal',
  status: 'available',
  color: '#00F0FF',
  count: terminalCommands.length
}, {
  id: 'yuancode',
  title: t("CommandManual.k211"),
  route: '/terminal/manual/yuancode',
  status: 'available',
  color: '#B026FF',
  count: yuancodeCommands.length
}, {
  id: 'linux',
  title: t("CommandManual.k212"),
  route: '/terminal/manual/linux',
  status: 'available',
  color: '#FFD700',
  count: linuxCommands.length
}, {
  id: 'shortcuts',
  title: t("CommandManual.k213"),
  route: '/terminal/manual/shortcuts',
  status: 'available',
  color: '#00F0FF',
  count: shortcuts.length
}];
function getStatusBadge(status: string) {
  const map: Record<string, {
    text: string;
    cls: string;
  }> = {
    available: {
      text: t("CommandManual.k214"),
      cls: styles.statusAvailable
    },
    developing: {
      text: t("CommandManual.k215"),
      cls: styles.statusDeveloping
    },
    planned: {
      text: t("CommandManual.k216"),
      cls: styles.statusPlanned
    }
  };
  const s = map[status] || map.planned;
  return <span className={s.cls}>{s.text}</span>;
}
function renderMainPage(navigate: ReturnType<typeof useNavigate>) {
  return <div className={styles.container}>
      <div className={styles.header}>
        <div className={styles.headerIcon}>📖</div>
        <h1 className={styles.headerTitle}>{t("components.PermissionRestricted.k9")}</h1>
        <p className={styles.headerSub}>Command Reference</p>
      </div>

      <div className={styles.list}>
        {manualList.map(item => <div key={item.id} className={styles.listItem} onClick={() => navigate(item.route)} style={{
        '--accent': item.color
      } as React.CSSProperties}>
            <div className={styles.listLeft}>
              <span className={styles.listDot} style={{
            background: item.color
          }} />
              <span className={styles.listTitle}>{item.title}</span>
              <span className={styles.listCount}>{item.count} {t("ai.ChatPanel.k19")}</span>
            </div>
            <div className={styles.listRight}>
              {getStatusBadge(item.status)}
              <span className={styles.listArrow}>→</span>
            </div>
          </div>)}
      </div>
    </div>;
}
function renderShortcutsPage(navigate: ReturnType<typeof useNavigate>) {
  return <div className={styles.container}>
      <div className={styles.detailHeader} style={{
      '--accent': '#00F0FF'
    } as React.CSSProperties}>
        <button className={styles.backBtn} onClick={() => navigate('/terminal/manual')}>{t("CommandManual.k217")}</button>
        <div className={styles.detailHeaderCenter}>
          <h1 className={styles.detailTitle} style={{
          color: '#00F0FF'
        }}>{t("CommandManual.k213")}</h1>
          <span className={styles.detailSub}>Keyboard Shortcuts</span>
          {getStatusBadge('available')}
        </div>
        <div />
      </div>

      <div className={styles.detailBody}>
        <table className={styles.table}>
          <thead>
            <tr>
              <th>{t("CommandManual.k218")}</th>
              <th>{t("AutoSaveDemo.k1")}</th>
              <th>{t("CommandManual.k219")}</th>
            </tr>
          </thead>
          <tbody>
            {shortcuts.map((s, i) => <tr key={i}>
                        <td><code style={{
                color: '#00F0FF'
              }}>{s.key}</code></td>
                        <td>{s.action}</td>
                        <td><span className={styles.tag}>{s.scope}</span></td>
                      </tr>)}
          </tbody>
        </table>
      </div>
    </div>;
}
function renderDetailPage(type: string, navigate: ReturnType<typeof useNavigate>) {
  const config: Record<string, {
    title: string;
    sub: string;
    data: typeof terminalCommands;
    color: string;
    status: string;
  }> = {
    terminal: {
      title: t("CommandManual.k210"),
      sub: 'Terminal Commands',
      data: terminalCommands,
      color: '#00F0FF',
      status: 'available'
    },
    yuancode: {
      title: t("CommandManual.k211"),
      sub: 'Yuan Code Commands',
      data: yuancodeCommands,
      color: '#B026FF',
      status: 'available'
    },
    linux: {
      title: t("CommandManual.k212"),
      sub: 'Linux Commands',
      data: linuxCommands,
      color: '#FFD700',
      status: 'available'
    }
  };
  const c = config[type] || config.terminal;
  const showAliases = type === 'terminal';
  return <div className={styles.container}>
      <div className={styles.detailHeader} style={{
      '--accent': c.color
    } as React.CSSProperties}>
        <button className={styles.backBtn} onClick={() => navigate('/terminal/manual')}>{t("CommandManual.k217")}</button>
        <div className={styles.detailHeaderCenter}>
          <h1 className={styles.detailTitle} style={{
          color: c.color
        }}>{c.title}</h1>
          <span className={styles.detailSub}>{c.sub}</span>
          {getStatusBadge(c.status)}
        </div>
        <div />
      </div>

      <div className={styles.detailBody}>
        <table className={styles.table}>
          <thead>
            <tr style={{
            borderBottomColor: c.color
          }}>
              <th>{t("CommandManual.k220")}</th>
              <th>{t("CommandManual.k221")}</th>
              <th>{t("CommandManual.k222")}</th>
              <th>{t("CommandManual.k223")}</th>
            </tr>
          </thead>
          <tbody>
            {c.data.map((row, i) => <tr key={i}>
                <td><code style={{
                color: c.color
              }}>{row.name}</code></td>
                <td><code>{row.syntax}</code></td>
                <td>{row.desc}</td>
                <td><span className={styles.tag}>{row.category}</span></td>
              </tr>)}
          </tbody>
        </table>

        {showAliases && <>
            <h2 className={styles.sectionTitle} style={{
          color: '#FFD700',
          marginTop: '32px'
        }}>{t("CommandManual.k224")}</h2>
            <table className={styles.table}>
              <thead>
                <tr style={{
              borderBottomColor: '#FFD700'
            }}>
                  <th>{t("CommandManual.k59")}</th>
                  <th>{t("CommandManual.k225")}</th>
                  <th>{t("CommandManual.k222")}</th>
                  <th>{t("CommandManual.k223")}</th>
                </tr>
              </thead>
              <tbody>
                {terminalAliases.map((row, i) => <tr key={i}>
                    <td><code style={{
                  color: '#FFD700'
                }}>{row.name}</code></td>
                    <td><code>{row.syntax}</code></td>
                    <td>{row.desc}</td>
                    <td><span className={styles.tag}>{row.category}</span></td>
                  </tr>)}
              </tbody>
            </table>
          </>}
      </div>
    </div>;
}
export default function CommandManual() {
  const navigate = useNavigate();
  const path = useLocation().pathname;
  if (path.endsWith('/terminal')) return renderDetailPage('terminal', navigate);
  if (path.endsWith('/yuancode')) return renderDetailPage('yuancode', navigate);
  if (path.endsWith('/linux')) return renderDetailPage('linux', navigate);
  if (path.endsWith('/shortcuts')) return renderShortcutsPage(navigate);
  return renderMainPage(navigate);
}