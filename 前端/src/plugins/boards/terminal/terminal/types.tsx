import { t } from "i18next";
export interface LauncherEntry {
  command: string;
  description: string;
  category: string;
}
export interface QuickSelectMatch {
  label: string;
  text: string;
  lineIndex: number;
  startIndex: number;
  patternName: string;
}
export const QUICK_SELECT_PATTERNS: {
  name: string;
  regex: RegExp;
}[] = [{
  name: 'URL',
  regex: /(?:https?:\/\/|git@|git:\/\/|ssh:\/\/|ftp:\/\/|file:\/\/)\S+/gi
}, {
  name: t("Linux.k91"),
  regex: /(?:[.\w\-@~]+)?(?:\/[.\w\-@~]+)+/g
}, {
  name: 'IPv4',
  regex: /\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}/g
}, {
  name: 'IPv6',
  regex: /[A-f0-9:]+:+[A-f0-9:]+[%\w\d]+/gi
}, {
  name: 'SHA',
  regex: /[0-9a-f]{7,40}/gi
}, {
  name: 'UUID',
  regex: /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/gi
}, {
  name: 'HexAddr',
  regex: /0x[0-9a-fA-F]+/g
}, {
  name: t("terminal.types.k1"),
  regex: /#[0-9a-fA-F]{6}\b/g
}, {
  name: t("terminal.types.k2"),
  regex: /\b[0-9]{4,}\b/g
}];
export const QUICK_SELECT_ALPHABET = 'asdfghjklqwertyuiopzxcvbnm';
export function computeQuickSelectLabels(count: number): string[] {
  const labels: string[] = [];
  const chars = QUICK_SELECT_ALPHABET.split('');
  for (let i = 0; i < Math.min(count, chars.length); i++) {
    labels.push(chars[i]);
  }
  for (let i = 0; i < Math.min(Math.max(0, count - chars.length), chars.length * chars.length); i++) {
    const first = chars[Math.floor(i / chars.length)];
    const second = chars[i % chars.length];
    labels.push(first + second);
  }
  return labels;
}
export const LAUNCHER_ENTRIES: LauncherEntry[] = [{
  command: 'ls',
  description: t("CommandManual.k58"),
  category: t("CommandManual.k5")
}, {
  command: 'pwd',
  description: t("terminal.types.k3"),
  category: t("CommandManual.k5")
}, {
  command: 'cd',
  description: t("terminal.types.k4"),
  category: t("CommandManual.k5")
}, {
  command: 'mkdir',
  description: t("terminal.types.k5"),
  category: t("CommandManual.k5")
}, {
  command: 'cat',
  description: t("terminal.types.k6"),
  category: t("CommandManual.k5")
}, {
  command: 'tree',
  description: t("terminal.types.k7"),
  category: t("CommandManual.k5")
}, {
  command: 'cp',
  description: t("terminal.types.k8"),
  category: t("CommandManual.k5")
}, {
  command: 'mv',
  description: t("terminal.types.k9"),
  category: t("CommandManual.k5")
}, {
  command: 'rm',
  description: t("terminal.types.k10"),
  category: t("CommandManual.k5")
}, {
  command: 'touch',
  description: t("terminal.types.k11"),
  category: t("CommandManual.k5")
}, {
  command: 'grep',
  description: t("terminal.types.k12"),
  category: t("CommandManual.k5")
}, {
  command: 'find',
  description: t("terminal.types.k13"),
  category: t("CommandManual.k5")
}, {
  command: 'wc',
  description: t("terminal.types.k14"),
  category: t("CommandManual.k5")
}, {
  command: 'head',
  description: t("terminal.types.k15"),
  category: t("CommandManual.k5")
}, {
  command: 'tail',
  description: t("terminal.types.k16"),
  category: t("CommandManual.k5")
}, {
  command: 'date',
  description: t("terminal.types.k17"),
  category: t("CommandManual.k36")
}, {
  command: 'whoami',
  description: t("terminal.types.k18"),
  category: t("CommandManual.k36")
}, {
  command: 'env',
  description: t("terminal.types.k19"),
  category: t("CommandManual.k36")
}, {
  command: 'sysinfo',
  description: t("terminal.types.k20"),
  category: t("CommandManual.k36")
}, {
  command: 'version',
  description: t("terminal.types.k21"),
  category: t("CommandManual.k36")
}, {
  command: 'echo',
  description: t("terminal.types.k22"),
  category: t("CommandManual.k2")
}, {
  command: 'clear',
  description: t("terminal.types.k23"),
  category: t("CommandManual.k2")
}, {
  command: 'clearscrollback',
  description: t("terminal.types.k24"),
  category: t("CommandManual.k2")
}, {
  command: 'history',
  description: t("terminal.types.k25"),
  category: t("CommandManual.k2")
}, {
  command: 'help',
  description: t("terminal.types.k26"),
  category: t("CommandManual.k2")
}, {
  command: 'reset',
  description: t("terminal.types.k27"),
  category: t("CommandManual.k2")
}, {
  command: 'fontsize',
  description: t("terminal.types.k28"),
  category: t("CommandManual.k2")
}, {
  command: 'fullscreen',
  description: t("terminal.types.k29"),
  category: t("CommandManual.k2")
}, {
  command: 'reload',
  description: t("CommandManual.k47"),
  category: t("CommandManual.k2")
}, {
  command: 'scroll',
  description: t("terminal.types.k30"),
  category: t("CommandManual.k2")
}, {
  command: 'search',
  description: t("terminal.types.k31"),
  category: t("CommandManual.k2")
}, {
  command: 'hide',
  description: t("terminal.types.k32"),
  category: t("CommandManual.k2")
}, {
  command: 'quit',
  description: t("terminal.types.k33"),
  category: t("CommandManual.k2")
}, {
  command: 'alwaysontop',
  description: t("terminal.types.k34"),
  category: t("CommandManual.k2")
}, {
  command: 'cmd',
  description: t("terminal.types.k35"),
  category: t("CommandManual.k55")
}, {
  command: 'powershell',
  description: t("terminal.types.k36"),
  category: t("CommandManual.k55")
}, {
  command: 'exit',
  description: t("terminal.types.k37"),
  category: t("CommandManual.k55")
}];
export interface AnsiSegment {
  text: string;
  fgColor?: string;
  bold?: boolean;
  italic?: boolean;
  underline?: boolean;
}
export const LINK_PATTERN = /(https?:\/\/[^\s<>"'\]]+|file:\/\/\/[^\s<>"'\]]+)/gi;
export function splitByLinks(text: string): Array<{
  text: string;
  isLink: boolean;
}> {
  const parts: Array<{
    text: string;
    isLink: boolean;
  }> = [];
  let lastIndex = 0;
  const regex = new RegExp(LINK_PATTERN.source, LINK_PATTERN.flags);
  let match: RegExpExecArray | null;
  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push({
        text: text.slice(lastIndex, match.index),
        isLink: false
      });
    }
    parts.push({
      text: match[0],
      isLink: true
    });
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length) {
    parts.push({
      text: text.slice(lastIndex),
      isLink: false
    });
  }
  return parts;
}
export function parseAnsi(text: string): AnsiSegment[] {
  const segments: AnsiSegment[] = [];
  const ansiRegex = /\x1b\[([0-9;]*)m/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  const currentStyle: Omit<AnsiSegment, 'text'> = {};
  while ((match = ansiRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      segments.push({
        text: text.slice(lastIndex, match.index),
        ...currentStyle
      });
    }
    const codes = match[1].split(';').map(Number);
    for (const code of codes) {
      switch (code) {
        case 0:
          delete currentStyle.fgColor;
          delete currentStyle.bold;
          delete currentStyle.italic;
          delete currentStyle.underline;
          break;
        case 1:
          currentStyle.bold = true;
          break;
        case 3:
          currentStyle.italic = true;
          break;
        case 4:
          currentStyle.underline = true;
          break;
        case 30:
          currentStyle.fgColor = '#000000';
          break;
        case 31:
          currentStyle.fgColor = '#FF5555';
          break;
        case 32:
          currentStyle.fgColor = '#50FA7B';
          break;
        case 33:
          currentStyle.fgColor = '#F1FA8C';
          break;
        case 34:
          currentStyle.fgColor = '#6272A4';
          break;
        case 35:
          currentStyle.fgColor = '#BD93F9';
          break;
        case 36:
          currentStyle.fgColor = '#00F0FF';
          break;
        case 37:
          currentStyle.fgColor = '#E0E0E0';
          break;
        case 90:
          currentStyle.fgColor = '#888888';
          break;
        case 91:
          currentStyle.fgColor = '#FF6E6E';
          break;
        case 92:
          currentStyle.fgColor = '#69FF94';
          break;
        case 93:
          currentStyle.fgColor = '#FFFFA5';
          break;
        case 94:
          currentStyle.fgColor = '#8BE9FD';
          break;
        case 95:
          currentStyle.fgColor = '#FF79C6';
          break;
        case 96:
          currentStyle.fgColor = '#8BE9FD';
          break;
        case 97:
          currentStyle.fgColor = '#FFFFFF';
          break;
      }
    }
    lastIndex = match.index + match[0].length;
  }
  if (lastIndex < text.length) {
    segments.push({
      text: text.slice(lastIndex),
      ...currentStyle
    });
  }
  return segments;
}
export function renderAnsiText(text: string): React.ReactNode {
  const segments = parseAnsi(text);
  if (segments.length === 0) return text;
  return segments.map((seg, i) => {
    if (!seg.fgColor && !seg.bold && !seg.italic && !seg.underline) {
      return <span key={i}>{seg.text}</span>;
    }
    const style: React.CSSProperties = {};
    if (seg.fgColor) style.color = seg.fgColor;
    if (seg.bold) style.fontWeight = 'bold';
    if (seg.italic) style.fontStyle = 'italic';
    if (seg.underline) style.textDecoration = 'underline';
    return <span key={i} style={style}>{seg.text}</span>;
  });
}