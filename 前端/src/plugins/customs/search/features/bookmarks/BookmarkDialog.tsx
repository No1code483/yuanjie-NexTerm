import { t } from "i18next";
import { useState } from 'react';
import styles from '../../routes/Search.module.css';
import { addBookmark, type Bookmark } from './bookmarks';
interface Props {
  url: string;
  onClose: () => void;
}
/** 收藏站点：新增弹窗（标题输入 → 派发 bookmark-add 事件） */
export default function BookmarkDialog({
  url,
  onClose
}: Props) {
  const [name, setName] = useState<string>(() => {
    try {
      return new URL(url).hostname;
    } catch {
      return url;
    }
  });
  const handleConfirm = () => {
    if (!name.trim()) return;
    const hostname = (() => {
      try {
        return new URL(url).hostname;
      } catch {
        return url;
      }
    })();
    const bookmark: Bookmark = {
      id: `bkm-${Date.now()}`,
      label: name.trim(),
      icon: hostname.charAt(0).toUpperCase(),
      iconUrl: '',
      url
    };
    addBookmark(bookmark);
    onClose();
  };
  return <div className={styles.bookmarkOverlay} onClick={onClose}>
      <div className={styles.bookmarkDialog} onClick={e => e.stopPropagation()}>
        <div className={styles.bookmarkDialogTitle}>{t("Search.k25")}</div>
        <div className={styles.bookmarkUrlHint}>{url}</div>
        <input className={styles.bookmarkInput} value={name} onChange={e => setName(e.target.value)} onKeyDown={e => {
        if (e.key === 'Enter') handleConfirm();
      }} placeholder={t("Search.k26")} autoFocus />
        <div className={styles.bookmarkActions}>
          <button className={styles.bookmarkCancelBtn} onClick={onClose}>{t("common.cancel")}</button>
          <button className={styles.bookmarkConfirmBtn} onClick={handleConfirm} disabled={!name.trim()}>{t("components.FloatingBall.k57")}</button>
        </div>
      </div>
    </div>;
}
