import { t } from "i18next";
import type { AIModel, Conversation, Participant } from './types';
import { time } from '@/lib/utils';
import styles from '../AI.module.css';
interface Props {
  conversations: Conversation[];
  loadingConversations: boolean;
  activeChat: number | null;
  models: AIModel[];
  participants: Map<number, Participant[]>;
  onSelect: (id: number) => void;
  onDelete: (id: number) => void;
  onCreate: () => void;
}
export default function GroupChatList({
  conversations,
  loadingConversations,
  activeChat,
  models,
  participants,
  onSelect,
  onDelete,
  onCreate
}: Props) {
  const groupConvs = conversations.filter(c => c.type === 'group');
  return <div className={styles.sidebarList}>
      <button className={styles.addBtn} onClick={onCreate}>{t("ai.GroupChatList.k1")}</button>
      {loadingConversations ? <div className={styles.loadingHint}>{t("common.loading")}</div> : groupConvs.length === 0 ? <div className={styles.emptyHint}>{t("ai.GroupChatList.k2")}</div> : groupConvs.map(conv => {
      const parts = participants.get(conv.id) || [];
      const modelNames = parts.filter(p => p.model_id).map(p => {
        const m = models.find(mo => mo.id === p.model_id);
        return m?.name || t("ai.AgentManager.k9", {
          model_id: p.model_id
        });
      });
      return <div key={conv.id} className={`${styles.sidebarItem} ${activeChat === conv.id ? styles.sidebarItemActive : ''}`} onClick={() => onSelect(conv.id)}>
              <div className={styles.sidebarItemHeader}>
                <span className={styles.sidebarItemName}>{conv.title || t("ai.GroupChatList.k3", {
              id: conv.id
            })}</span>
                <button className={styles.deleteBtnSmall} onClick={e => {
            e.stopPropagation();
            onDelete(conv.id);
          }}>✕</button>
              </div>
              <div className={styles.sidebarItemMeta}>
                <span>{t("ai.GroupChatList.k4")}</span>
                <span>{time.formatUtcToLocal(conv.updated_at)}</span>
              </div>
              {modelNames.length > 0 && <div className={styles.groupMemberRow}>
                  {modelNames.map((name, i) => <span key={i} className={styles.groupMemberChip}>{name}</span>)}
                </div>}
            </div>;
    })}
    </div>;
}