import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import zh from './zh.json'
import en from './en.json'
import ja from './ja.json'
import ko from './ko.json'
import ru from './ru.json'
import ar from './ar.json'
import zhTW from './zh-TW.json'
import fr from './fr.json'
import es from './es.json'

const LANG_KEY = 'nexterm_lang'

/** 支持的语言列表（用于 LanguageSelector 渲染）
 *  M5.①③：语言卡显示名走 i18n t(`language.name.${code}`)，label/nativeLabel 仅作兜底元数据。
 *  M5.③：新增繁体中文 / 法语 / 西班牙语（ru/ar/zh-TW 为完整基底复制；fr/es 为 en 基底 + 核心组真翻译，故 experimental）。 */
export const SUPPORTED_LANGUAGES = [
  { code: 'zh', label: '中文', labelEn: 'Chinese', nativeLabel: '简体中文' },
  { code: 'zh-TW', label: '繁體中文', labelEn: 'Traditional Chinese', nativeLabel: '繁體中文' },
  { code: 'en', label: 'English', labelEn: 'English', nativeLabel: 'English' },
  { code: 'ja', label: '日本語', labelEn: 'Japanese', nativeLabel: '日本語' },
  { code: 'ko', label: '한국어', labelEn: 'Korean', nativeLabel: '한국어' },
  { code: 'ru', label: 'Русский', labelEn: 'Russian', nativeLabel: 'Русский' },
  // C2.8：阿拉伯语（RTL）实验性支持，ar.json 当前为 en.json 副本，逐步翻译中
  { code: 'ar', label: 'العربية', labelEn: 'Arabic (Beta)', nativeLabel: 'العربية' },
  // M5.③：法语 / 西班牙语，en.json 基底复制 + 核心组真翻译，逐步完善（experimental 标记部分翻译）
  { code: 'fr', label: 'Français', labelEn: 'French', nativeLabel: 'Français' },
  { code: 'es', label: 'Español', labelEn: 'Spanish', nativeLabel: 'Español' }
] as const

export type SupportedLangCode = typeof SUPPORTED_LANGUAGES[number]['code']

function detectLanguage(): string {
  const stored = localStorage.getItem(LANG_KEY)
  if (stored && SUPPORTED_LANGUAGES.some(l => l.code === stored)) return stored

  const navLang = navigator.language?.toLowerCase() || ''
  // M5.③：繁中需在简中之前判定（zh-TW / zh-Hant）
  if (navLang.startsWith('zh-tw') || navLang.startsWith('zh-hant')) return 'zh-TW'
  if (navLang.startsWith('zh')) return 'zh'
  if (navLang.startsWith('ja')) return 'ja'
  if (navLang.startsWith('ko')) return 'ko'
  if (navLang.startsWith('ru')) return 'ru'
  if (navLang.startsWith('ar')) return 'ar'
  if (navLang.startsWith('fr')) return 'fr'
  if (navLang.startsWith('es')) return 'es'
  if (navLang.startsWith('en')) return 'en'
  return 'zh'
}

i18n.use(initReactI18next).init({
  resources: {
    zh: { translation: zh },
    'zh-TW': { translation: zhTW },
    en: { translation: en },
    ja: { translation: ja },
    ko: { translation: ko },
    ru: { translation: ru },
    ar: { translation: ar },
    fr: { translation: fr },
    es: { translation: es },
  },
  lng: detectLanguage(),
  // C2.1：ja/ko 仅翻译核心 key，其余 fallback 到 en，最终 fallback 到 zh
  // C2.7：ru/ar 从 en.json 复制作为基础，后续逐步翻译
  // M5.③：zh-TW 回退到 zh；fr/es 回退到 en 再 zh
  fallbackLng: {
    'zh-TW': ['zh'],
    ja: ['en', 'zh'],
    ko: ['en', 'zh'],
    ru: ['en', 'zh'],
    ar: ['en', 'zh'],
    fr: ['en', 'zh'],
    es: ['en', 'zh'],
    en: ['zh'],
    zh: ['en'],
    default: ['zh'],
  },
  interpolation: {
    escapeValue: false,
  },
})

export function toggleLanguage(): string {
  const current = i18n.language
  const next = current === 'zh' ? 'en' : 'zh'
  i18n.changeLanguage(next)
  localStorage.setItem(LANG_KEY, next)
  return next
}

export function getCurrentLanguage(): string {
  return i18n.language ?? 'zh'
}

/** C2.1：切换到指定语言（支持所有 SUPPORTED_LANGUAGES） */
export function changeLanguage(code: string): void {
  if (!SUPPORTED_LANGUAGES.some(l => l.code === code)) return
  i18n.changeLanguage(code)
  localStorage.setItem(LANG_KEY, code)
  // C2.3：同步 <html lang> 属性（无障碍 + 屏幕阅读器）
  const langMap: Record<string, string> = {
    zh: 'zh-CN',
    'zh-TW': 'zh-TW',
    en: 'en-US',
    ja: 'ja-JP',
    ko: 'ko-KR',
    ru: 'ru-RU',
    ar: 'ar-SA',
    fr: 'fr-FR',
    es: 'es-ES',
  };
  document.documentElement.setAttribute('lang', langMap[code] || 'zh-CN')
  // C2.8：RTL 语言同步 dir 属性（ar 已启用，he/fa 预留）
  const rtlLangs = ['ar', 'he', 'fa']
  const baseLang = code.split('-')[0]
  if (rtlLangs.includes(baseLang)) {
    document.documentElement.setAttribute('dir', 'rtl')
  } else {
    document.documentElement.removeAttribute('dir')
  }
}

export default i18n
