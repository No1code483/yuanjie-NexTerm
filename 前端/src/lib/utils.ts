import { t } from "i18next";
// 简单的Base64加密解密，避免外部依赖
const encodeBase64 = (str: string): string => {
  if (typeof btoa === 'function') {
    return btoa(unescape(encodeURIComponent(str)));
  }
  // 浏览器环境下的Base64编码
  const encoder = new TextEncoder();
  const data = encoder.encode(str);
  let binary = '';
  for (let i = 0; i < data.length; i++) {
    binary += String.fromCharCode(data[i]);
  }
  return btoa ? btoa(binary) : '';
};
const decodeBase64 = (str: string): string => {
  if (typeof atob === 'function') {
    return decodeURIComponent(escape(atob(str)));
  }
  // 浏览器环境下的Base64解码
  const binary = atob ? atob(str) : '';
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  const decoder = new TextDecoder();
  return decoder.decode(bytes);
};
export const encryption = {
  encrypt: (data: string, key: string): string => {
    // 简单的XOR加密
    let result = '';
    for (let i = 0; i < data.length; i++) {
      const charCode = data.charCodeAt(i) ^ key.charCodeAt(i % key.length);
      result += String.fromCharCode(charCode);
    }
    return encodeBase64(result);
  },
  decrypt: (encrypted: string, key: string): string => {
    try {
      const decoded = decodeBase64(encrypted);
      let result = '';
      for (let i = 0; i < decoded.length; i++) {
        const charCode = decoded.charCodeAt(i) ^ key.charCodeAt(i % key.length);
        result += String.fromCharCode(charCode);
      }
      return result;
    } catch {
      return '';
    }
  }
};
export const storage = {
  set: (key: string, value: string): void => {
    localStorage.setItem(key, value);
  },
  get: (key: string): string | null => {
    return localStorage.getItem(key);
  },
  remove: (key: string): void => {
    localStorage.removeItem(key);
  },
  clear: (): void => {
    localStorage.clear();
  }
};
export const time = {
  /** 将数据库 UTC 时间戳转为本地时间显示（统一时间源）
   *  支持格式：
   *    "2026-06-13T09:41:52Z"     标准 ISO（toISOString 原始输出）
   *    "2026-06-13 09:41:52"      useActivityTracker 存储格式（去 T 无 Z）
   *    "2026-06-13T09:41:52"      其他 ISO 变体
   *    1749800000000               数字时间戳（毫秒）
   */
  formatUtcToLocal(iso: string | number): string {
    try {
      const isoStr = typeof iso === 'number' ? new Date(iso).toISOString() : String(iso);
      const normalized = isoStr.replace(' ', 'T');
      const d = normalized.includes('Z') || normalized.includes('+') ? new Date(normalized) : new Date(normalized + 'Z');
      return d.toLocaleString('zh-CN', {
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit'
      });
    } catch {
      return String(iso);
    }
  },
  /** 相对时间："刚刚" / "X分钟前" / "X小时前" / "X天前" */
  timeAgo(iso: string | number): string {
    try {
      const isoStr = typeof iso === 'number' ? new Date(iso).toISOString() : String(iso);
      const normalized = isoStr.replace(' ', 'T');
      const d = normalized.includes('Z') || normalized.includes('+') ? new Date(normalized) : new Date(normalized + 'Z');
      const diff = Date.now() - d.getTime();
      if (diff < 60000) return t("lib.utils.k1");
      if (diff < 3600000) return t("lib.utils.k2", {
        arg0: Math.floor(diff / 60000)
      });
      if (diff < 86400000) return t("lib.utils.k3", {
        arg0: Math.floor(diff / 3600000)
      });
      return t("lib.utils.k4", {
        arg0: Math.floor(diff / 86400000)
      });
    } catch {
      return String(iso);
    }
  },
  /** 紧凑格式：MM-DD HH:mm（用于列表等空间受限场景） */
  formatCompact(iso: string | number): string {
    try {
      const isoStr = typeof iso === 'number' ? new Date(iso).toISOString() : String(iso);
      const normalized = isoStr.replace(' ', 'T');
      const d = normalized.includes('Z') || normalized.includes('+') ? new Date(normalized) : new Date(normalized + 'Z');
      const pad = (n: number) => String(n).padStart(2, '0');
      return `${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
    } catch {
      return String(iso);
    }
  },
  /** 秒数 → 时长显示：≥1h 显示 "X.Xh"，否则 "Xmin" */
  formatDurationSecs(secs: number): string {
    if (!secs || secs <= 0) return '0min';
    const h = secs / 3600;
    return h >= 1 ? `${h.toFixed(1)}h` : `${Math.round(secs / 60)}min`;
  },
  /** 统一生成 UTC 时间戳（ISO 格式，带 Z 后缀） */
  nowUtc(): string {
    return new Date().toISOString();
  },
  /** 统一生成 UTC 时间戳（兼容旧存储格式，去 T 无 Z） */
  nowUtcCompact(): string {
    return new Date().toISOString().replace('T', ' ').slice(0, 19);
  },
  formatTime: (date: Date, format: string = 'YYYY-MM-DD HH:mm:ss'): string => {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    const seconds = String(date.getSeconds()).padStart(2, '0');
    return format.replace('YYYY', String(year)).replace('MM', month).replace('DD', day).replace('HH', hours).replace('mm', minutes).replace('ss', seconds);
  },
  parseTime: (timeString: string, _format: string = 'YYYY-MM-DD HH:mm:ss'): Date => {
    const regex = /(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})/;
    const match = timeString.match(regex);
    if (match) {
      return new Date(parseInt(match[1]), parseInt(match[2]) - 1, parseInt(match[3]), parseInt(match[4]), parseInt(match[5]), parseInt(match[6]));
    }
    return new Date();
  },
  getDuration: (start: Date, end: Date): number => {
    return end.getTime() - start.getTime();
  },
  formatDuration: (ms: number): string => {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    const remainingSeconds = seconds % 60;
    const remainingMinutes = minutes % 60;
    const remainingHours = hours % 24;
    if (days > 0) {
      return t("lib.utils.k5", {
        days: days,
        remainingHours: remainingHours,
        remainingMinutes: remainingMinutes,
        remainingSeconds: remainingSeconds
      });
    }
    if (hours > 0) {
      return t("lib.utils.k6", {
        remainingHours: remainingHours,
        remainingMinutes: remainingMinutes,
        remainingSeconds: remainingSeconds
      });
    }
    if (minutes > 0) {
      return t("lib.utils.k7", {
        remainingMinutes: remainingMinutes,
        remainingSeconds: remainingSeconds
      });
    }
    return t("lib.utils.k8", {
      remainingSeconds: remainingSeconds
    });
  }
};
export const file = {
  /** 文件大小格式化（B → KB → MB → GB），统一入口 */
  formatSize: (bytes: number | null | undefined): string => {
    if (!bytes) return '-';
    if (bytes < 1024) return `${bytes}B`;
    if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)}KB`;
    if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)}MB`;
    return `${(bytes / 1073741824).toFixed(1)}GB`;
  },
  validateFileSize: (bytes: number, maxSize: number = 2 * 1024 * 1024 * 1024): boolean => {
    return bytes <= maxSize;
  }
};
export const validation = {
  isUsername: (username: string): boolean => {
    return /^[a-zA-Z0-9_\u4e00-\u9fa5]{3,20}$/.test(username);
  },
  isPassword: (password: string): boolean => {
    return password.length >= 6;
  },
  isEmail: (email: string): boolean => {
    return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email);
  }
};
export const random = {
  uuid: (): string => {
    return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
      const r = Math.random() * 16 | 0;
      const v = c === 'x' ? r : r & 0x3 | 0x8;
      return v.toString(16);
    });
  },
  /** 短唯一 ID（时间戳 + 随机字符），统一入口替代各处分散的 Date.now+Math.random */
  uid: (prefix: string = ''): string => {
    return `${prefix}${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
  },
  string: (length: number = 8): string => {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    for (let i = 0; i < length; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }
};

/** 剪贴板复制（统一入口，含错误处理），返回是否成功 */
export async function copy(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // 降级：尝试 execCommand（兼容非 HTTPS 环境）
    try {
      const ta = document.createElement('textarea');
      ta.value = text;
      ta.style.position = 'fixed';
      ta.style.left = '-9999px';
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      document.body.removeChild(ta);
      return true;
    } catch {
      return false;
    }
  }
}