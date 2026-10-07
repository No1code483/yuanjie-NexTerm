import { t } from "i18next";
import styles from '../Knowledge.module.css';
interface ContextMenuState {
  x: number;
  y: number;
  node: {
    type: 'category' | 'entry';
    id: number;
    name: string;
  };
}
interface Props {
  menu: ContextMenuState;
  onClose: () => void;
  onRename: (type: 'category' | 'entry', id: number, name: string) => void;
  onMove: (type: 'category' | 'entry', id: number) => void;
  onDelete: (type: 'category' | 'entry', id: number) => void;
  onAiClassify?: (id: number, name: string) => void;
}
export default function ContextMenu({
  menu,
  onClose,
  onRename,
  onMove,
  onDelete,
  onAiClassify
}: Props) {
  return <>
      <div className={styles.contextOverlay} onClick={onClose} />
      <div className={styles.contextMenu} style={{
      left: menu.x,
      top: menu.y
    }}>
        {menu.node.type === 'category' && <>
            <button className={styles.contextItem} onClick={() => onRename('category', menu.node.id, menu.node.name)}>
              {t("knowledge.ContextMenu.k1")}
            </button>
            <button className={styles.contextItem} onClick={() => onMove('category', menu.node.id)}>
              {t("knowledge.ContextMenu.k2")}
            </button>
          </>}
        {menu.node.type === 'entry' && <>
            <button className={styles.contextItem} onClick={() => onRename('entry', menu.node.id, menu.node.name)}>
              {t("knowledge.ContextMenu.k1")}
            </button>
            <button className={styles.contextItem} onClick={() => onMove('entry', menu.node.id)}>
              {t("knowledge.ContextMenu.k2")}
            </button>
            {onAiClassify && <button className={styles.contextItem} onClick={() => {
          onClose();
          onAiClassify(menu.node.id, menu.node.name);
        }} style={{
          color: '#00FF00',
          borderColor: 'rgba(0,255,0,0.3)'
        }}>
                {t("knowledge.ContextMenu.k3")}
              </button>}
          </>}
        <div className={styles.contextDivider} />
        <button className={styles.contextItemDanger} onClick={() => {
        onClose();
        onDelete(menu.node.type, menu.node.id);
      }}>
          {t("knowledge.ContextMenu.k4")}
        </button>
      </div>
    </>;
}