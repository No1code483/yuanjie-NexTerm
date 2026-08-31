// ipc/profile.ts — profile 模块 IPC 封装（从 ipc.ts 拆分，T2.1.6）
import { ipc } from './core';

export const profile = {
  getProfile: (key: string) => ipc.invoke<any>('get_profile', {
    key
  }),
  setProfile: (key: string, value: string) => ipc.invoke('set_profile', {
    key,
    value
  }),
  getResumes: () => ipc.invoke<any[]>('get_resumes'),
  addResume: (title: string, content: string) => ipc.invoke('add_resume', {
    title,
    content
  }),
  updateResume: (id: number, title: string, content: string) => ipc.invoke('update_resume', {
    id,
    title,
    content
  }),
  deleteResume: (id: number) => ipc.invoke('delete_resume', {
    id
  }),
  getRandomQuote: () => ipc.invoke<any>('get_random_quote'),
  addQuote: (content: string, source?: string, quoteType?: string) => ipc.invoke('add_quote', {
    content,
    source: source || null,
    quote_type: quoteType || 'daily'
  }),
  checkQuoteDuplicate: (content: string) => ipc.invoke<any[]>('check_quote_duplicate', {
    content
  }),
  getAllQuotes: () => ipc.invoke<any>('get_all_quotes'),
  batchAddQuotes: (quotes: Array<[string, string | null, string]>) => ipc.invoke('batch_add_quotes', {
    quotes
  }),
  seedDefaultQuotes: () => ipc.invoke('seed_default_quotes'),
  deleteQuote: (id: number) => ipc.invoke('delete_quote', {
    id
  }),
  changePassword: (oldPassword: string, newPassword: string) => ipc.invoke('profile_change_password', {
    request: {
      old_password: oldPassword,
      new_password: newPassword
    }
  }),
  changeUsername: (newUsername: string) => ipc.invoke('profile_change_username', {
    request: {
      new_username: newUsername
    }
  }),
  updateProfile: (data: {
    avatar_url?: string | null;
    bio?: string | null;
    display_name?: string | null;
  }) => ipc.invoke('profile_update_profile', {
    request: data
  }),
  createTempAccount: (username: string, duration: '1h' | '24h' | '7d') => ipc.invoke('create_temp_account', {
    duration_hours: duration === '1h' ? 1 : duration === '24h' ? 24 : 168,
    username
  }),
  savePersonalInfo: (json: string) => ipc.invoke('save_personal_info', {
    json
  }),
  getPersonalInfo: () => ipc.invoke<string | null>('get_personal_info'),
  aiPolishResume: (resumeId: number, content: string) => ipc.invoke<{
    content: string;
  }>('profile_ai_polish_resume', {
    resume_id: resumeId,
    content
  }),
  aiSpellCheckResume: (resumeId: number, content: string) => ipc.invoke<{
    content: string;
  }>('profile_ai_spell_check_resume', {
    resume_id: resumeId,
    content
  }),
  aiGenerateResume: (resumeId: number, content: string) => ipc.invoke<{
    content: string;
  }>('profile_ai_generate_resume', {
    resume_id: resumeId,
    content
  }),
  aiQuoteCheck: (quoteId: number) => ipc.invoke<{
    result: string;
  }>('profile_ai_quote_check', {
    quote_id: quoteId
  }),
  aiQuoteComplete: (content: string) => ipc.invoke<{
    completion: string;
  }>('profile_ai_quote_complete', {
    content
  }),
  // P2: AI 简历润色
  resumePolish: (text: string, section?: string) => ipc.invoke<string>('resume_polish', {
    text,
    section
  }),
  // P2: 数据导出
  exportUserData: () => ipc.invoke<any>('export_user_data')
};
