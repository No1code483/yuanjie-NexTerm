import { t } from "i18next";
export const API_FORMAT_OPTIONS = [{
  value: 'openai-chat',
  label: t("ai.constants.k1")
}, {
  value: 'anthropic-messages',
  label: t("ai.constants.k2")
}, {
  value: 'ollama',
  label: t("ai.constants.k3")
}];
export const PROVIDER_OPTIONS = [{
  value: 'openai',
  label: 'OpenAI',
  defaultUrl: 'https://api.openai.com/v1',
  defaultModel: 'gpt-4o-mini'
}, {
  value: 'deepseek',
  label: t("ai.constants.k4"),
  defaultUrl: 'https://api.deepseek.com/v1',
  defaultModel: 'deepseek-chat'
}, {
  value: 'moonshot',
  label: t("ai.constants.k5"),
  defaultUrl: 'https://api.moonshot.cn/v1',
  defaultModel: 'moonshot-v1-8k'
}, {
  value: 'zhipu',
  label: t("ai.constants.k6"),
  defaultUrl: 'https://open.bigmodel.cn/api/paas/v4',
  defaultModel: 'glm-4-flash'
}, {
  value: 'qwen',
  label: t("ai.constants.k7"),
  defaultUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
  defaultModel: 'qwen-turbo'
}, {
  value: 'anthropic',
  label: 'Anthropic (Claude)',
  defaultUrl: 'https://api.anthropic.com/v1',
  defaultModel: 'claude-sonnet-4-20250514'
}, {
  value: 'ollama',
  label: t("components.intelligence.SettingsPanel.k28"),
  defaultUrl: 'http://localhost:11434',
  defaultModel: 'llama3.2'
}, {
  value: 'custom',
  label: t("lib.ipcMock.k42"),
  defaultUrl: '',
  defaultModel: ''
}];
export const PROVIDER_LABEL_MAP: Record<string, string> = {
  ollama: t("components.intelligence.SettingsPanel.k28"),
  openai: 'OpenAI',
  deepseek: 'DeepSeek',
  moonshot: 'Moonshot',
  zhipu: t("ai.constants.k8"),
  qwen: t("ai.constants.k9"),
  anthropic: 'Anthropic',
  azure: 'Azure',
  custom: t("lib.ipcMock.k42")
};