import type { ApiResponse } from '@/lib/ipc/core';
import { defineIpcNamespace } from '@/kernel/ipc/namespace';
import type { IpcMethodSpec } from '@/kernel/registry/definePlugin';

export const PROFILE_IPC_METHODS = {
  getProfile: { cmd: 'get_profile' },
  setProfile: { cmd: 'set_profile' },
  getResumes: { cmd: 'get_resumes' },
  addResume: { cmd: 'add_resume' },
  updateResume: { cmd: 'update_resume' },
  deleteResume: { cmd: 'delete_resume' },
  getRandomQuote: { cmd: 'get_random_quote' },
  addQuote: { cmd: 'add_quote' },
  checkQuoteDuplicate: { cmd: 'check_quote_duplicate' },
  getAllQuotes: { cmd: 'get_all_quotes' },
  batchAddQuotes: { cmd: 'batch_add_quotes' },
  seedDefaultQuotes: { cmd: 'seed_default_quotes' },
  deleteQuote: { cmd: 'delete_quote' },
  savePersonalInfo: { cmd: 'save_personal_info' },
  getPersonalInfo: { cmd: 'get_personal_info' },
  aiPolishResume: { cmd: 'profile_ai_polish_resume' },
  aiSpellCheckResume: { cmd: 'profile_ai_spell_check_resume' },
  aiGenerateResume: { cmd: 'profile_ai_generate_resume' },
  aiQuoteCheck: { cmd: 'profile_ai_quote_check' },
  aiQuoteComplete: { cmd: 'profile_ai_quote_complete' },
  resumePolish: { cmd: 'resume_polish' },
  exportUserData: { cmd: 'export_user_data' }
} satisfies Record<string, IpcMethodSpec>;

const dispatcherProfile = defineIpcNamespace('pf', PROFILE_IPC_METHODS);
type ProfileMethod = keyof typeof PROFILE_IPC_METHODS;

function invokeProfile<T>(method: ProfileMethod, args?: Record<string, unknown>): Promise<ApiResponse<T>> {
  return dispatcherProfile[method](args) as Promise<ApiResponse<T>>;
}

export const profile = {
  getProfile: (key: string) => invokeProfile<any>('getProfile', { key }),
  setProfile: (key: string, value: string) => invokeProfile('setProfile', { key, value }),
  getResumes: () => invokeProfile<any[]>('getResumes'),
  addResume: (title: string, content: string) => invokeProfile('addResume', { title, content }),
  updateResume: (id: number, title: string, content: string) => invokeProfile('updateResume', { id, title, content }),
  deleteResume: (id: number) => invokeProfile('deleteResume', { id }),
  getRandomQuote: () => invokeProfile<any>('getRandomQuote'),
  addQuote: (content: string, source?: string, quoteType?: string) => invokeProfile('addQuote', {
    content,
    source: source || null,
    quote_type: quoteType || 'daily'
  }),
  checkQuoteDuplicate: (content: string) => invokeProfile<any[]>('checkQuoteDuplicate', { content }),
  getAllQuotes: () => invokeProfile<any>('getAllQuotes'),
  batchAddQuotes: (quotes: Array<[string, string | null, string]>) => invokeProfile('batchAddQuotes', { quotes }),
  seedDefaultQuotes: () => invokeProfile('seedDefaultQuotes'),
  deleteQuote: (id: number) => invokeProfile('deleteQuote', { id }),
  savePersonalInfo: (json: string) => invokeProfile('savePersonalInfo', { json }),
  getPersonalInfo: () => invokeProfile<string | null>('getPersonalInfo'),
  aiPolishResume: (resumeId: number, content: string) => invokeProfile<{ content: string }>('aiPolishResume', {
    resume_id: resumeId,
    content
  }),
  aiSpellCheckResume: (resumeId: number, content: string) => invokeProfile<{ content: string }>('aiSpellCheckResume', {
    resume_id: resumeId,
    content
  }),
  aiGenerateResume: (resumeId: number, content: string) => invokeProfile<{ content: string }>('aiGenerateResume', {
    resume_id: resumeId,
    content
  }),
  aiQuoteCheck: (quoteId: number) => invokeProfile<{ result: string }>('aiQuoteCheck', { quote_id: quoteId }),
  aiQuoteComplete: (content: string) => invokeProfile<{ completion: string }>('aiQuoteComplete', { content }),
  resumePolish: (text: string, section?: string) => invokeProfile<string>('resumePolish', { text, section }),
  exportUserData: () => invokeProfile<any>('exportUserData')
};
