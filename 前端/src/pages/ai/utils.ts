import { t } from "i18next";
import type { AIModel, BackendAgent, Message, Participant } from './types';
import { PROVIDER_LABEL_MAP } from './constants';

// 相对时间显示：刚刚 / N分钟前 / HH:mm
export function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  if (diff < 60000) return t("lib.utils.k1");
  if (diff < 3600000) return t("ai.utils.k1", {
    arg0: Math.floor(diff / 60000)
  });
  return new Date(timestamp).toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit'
  });
}
export function renderMarkdown(text: string): string {
  let html = text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

  // 代码块：包裹容器 + 复制按钮
  html = html.replace(/```([\s\S]*?)```/g, (_m, code) => {
    const trimmed = code.trim();
    return t("ai.utils.k2", {
      trimmed: trimmed,
      arg0: encodeURIComponent(trimmed)
    });
  });
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');
  html = html.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
  html = html.replace(/^[\s]*[-*]\s+(.+)$/gm, '<li>$1</li>');
  html = html.replace(/(<li>.*<\/li>\n?)+/g, match => {
    return '<ul>' + match + '</ul>';
  });
  html = html.replace(/\n/g, '<br/>');
  return html;
}
export function getParticipantBaseName(participant: Participant, models: AIModel[], agents: BackendAgent[]): string {
  if (participant.agent_id) {
    const agent = agents.find(a => a.id === participant.agent_id);
    if (agent) return agent.name;
  }
  if (participant.model_id) {
    const model = models.find(m => m.id === participant.model_id);
    if (model) return model.name;
  }
  return participant.role || t("components.GroupChatOrchestrationPanel.k35");
}
function resolveDedupName(baseName: string, participant: Participant, participants: Participant[], models: AIModel[], agents: BackendAgent[]): string {
  const sameNameParts = participants.filter(p => getParticipantBaseName(p, models, agents) === baseName).sort((a, b) => a.id - b.id);
  if (sameNameParts.length <= 1) return baseName;
  const index = sameNameParts.findIndex(p => p.id === participant.id);
  return `${baseName}_${index + 1}`;
}
export function getSenderDisplayName(msg: Message, participants: Participant[], models: AIModel[], agents: BackendAgent[]): string {
  if (msg.sender_type === 'user') return t("components.FloatingBall.k61");
  if (msg.sender_type === 'system') return t("components.GroupChatOrchestrationPanel.k2");
  if (msg.sender_type === 'error') return t("common.error");
  if (msg.sender_type === 'model' && msg.sender_id) {
    const participant = participants.find(p => p.id === msg.sender_id);
    if (participant) {
      const baseName = getParticipantBaseName(participant, models, agents);
      return resolveDedupName(baseName, participant, participants, models, agents);
    }
    const model = models.find(m => m.id === msg.sender_id);
    if (model) return model.name;
  }
  return 'AI';
}
export function getProviderLabel(provider: string): string {
  return PROVIDER_LABEL_MAP[provider] || provider;
}