import { t } from "i18next";
import { useState, useEffect, useCallback, useRef } from 'react';
interface SlideData {
  index: number;
  name: string;
  text_content: string;
  raw_xml: string;
}
interface PptEditorProps {
  filePath: string;
  initialSlides: SlideData[];
  fileName: string;
  onSave: () => void;
  onClose: () => void;
  showStatus: (type: 'success' | 'error', text: string) => void;
}
export default function PptEditor({
  filePath,
  initialSlides,
  fileName,
  onSave,
  onClose,
  showStatus
}: PptEditorProps) {
  const [slides, setSlides] = useState<SlideData[]>(initialSlides);
  const [activeSlide, setActiveSlide] = useState(0);
  const [unsaved, setUnsaved] = useState(false);
  const [saving, setSaving] = useState(false);
  const [adding, setAdding] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const editRef = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    return () => {
      if (unsaved && editRef.current) {
        const current = slides[activeSlide];
        if (current && editRef.current.value !== current.text_content) {
          onSave();
        }
      }
    };
  });
  const currentSlide = slides[activeSlide];
  const handleTextChange = useCallback((text: string) => {
    setSlides(prev => {
      const next = [...prev];
      next[activeSlide] = {
        ...next[activeSlide],
        text_content: text
      };
      return next;
    });
    setUnsaved(true);
  }, [activeSlide]);
  const handleSave = useCallback(async () => {
    if (!currentSlide) return;
    setSaving(true);
    try {
      const {
        invoke
      } = await import('@tauri-apps/api/core');
      await invoke('pptedit_update_slide', {
        path: filePath,
        slide_index: activeSlide,
        text_content: currentSlide.text_content
      });
      setUnsaved(false);
      showStatus('success', t("components.PptEditor.k1"));
    } catch (e: any) {
      showStatus('error', e?.toString() || t("errors.saveFailed"));
    } finally {
      setSaving(false);
    }
  }, [filePath, activeSlide, currentSlide, showStatus]);
  const handleAddSlide = useCallback(async () => {
    setAdding(true);
    try {
      const {
        invoke
      } = await import('@tauri-apps/api/core');
      const res: any = await invoke('pptedit_add_slide', {
        path: filePath
      });
      const newIndex = res.data ?? slides.length;
      const newSlide: SlideData = {
        index: newIndex,
        name: `ppt/slides/slide${newIndex + 1}.xml`,
        text_content: t("components.PptEditor.k2"),
        raw_xml: ''
      };
      setSlides(prev => {
        const next = [...prev, newSlide];
        return next.map((s, i) => ({
          ...s,
          index: i
        }));
      });
      setActiveSlide(slides.length);
      showStatus('success', t("components.PptEditor.k3"));
    } catch (e: any) {
      showStatus('error', e?.toString() || t("components.PptEditor.k4"));
    } finally {
      setAdding(false);
    }
  }, [filePath, slides.length, showStatus]);
  const handleDeleteSlide = useCallback(async () => {
    if (slides.length <= 1) {
      showStatus('error', t("components.PptEditor.k5"));
      return;
    }
    setDeleting(true);
    try {
      const {
        invoke
      } = await import('@tauri-apps/api/core');
      await invoke('pptedit_delete_slide', {
        path: filePath,
        slideIndex: activeSlide
      });
      setSlides(prev => {
        const next = prev.filter((_, i) => i !== activeSlide);
        return next.map((s, i) => ({
          ...s,
          index: i
        }));
      });
      if (activeSlide >= slides.length - 1) {
        setActiveSlide(Math.max(0, slides.length - 2));
      }
      showStatus('success', t("components.PptEditor.k6"));
    } catch (e: any) {
      showStatus('error', e?.toString() || t("errors.deleteFailed"));
    } finally {
      setDeleting(false);
    }
  }, [filePath, activeSlide, slides.length, showStatus]);
  const handleReorder = useCallback(async (direction: 'up' | 'down') => {
    const targetIndex = direction === 'up' ? activeSlide - 1 : activeSlide + 1;
    if (targetIndex < 0 || targetIndex >= slides.length) return;
    try {
      const {
        invoke
      } = await import('@tauri-apps/api/core');
      await invoke('pptedit_reorder', {
        path: filePath,
        slide_index: activeSlide,
        new_index: targetIndex
      });
      setSlides(prev => {
        const next = [...prev];
        const [moved] = next.splice(activeSlide, 1);
        next.splice(targetIndex, 0, moved);
        return next.map((s, i) => ({
          ...s,
          index: i
        }));
      });
      setActiveSlide(targetIndex);
      showStatus('success', t("components.PptEditor.k7"));
    } catch (e: any) {
      showStatus('error', e?.toString() || t("components.PptEditor.k8"));
    }
  }, [filePath, activeSlide, slides.length, showStatus]);
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 's') {
        e.preventDefault();
        handleSave();
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [handleSave]);
  if (slides.length === 0) {
    return <div style={{
      display: 'flex',
      flexDirection: 'column',
      height: '100%',
      background: '#0a0a10'
    }}>
        <div style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '10px 16px',
        borderBottom: '1px solid #222',
        color: '#aaa'
      }}>
          <span>{t("components.PptEditor.k9")} {fileName}</span>
          <button onClick={onClose} style={{
          background: 'transparent',
          border: 'none',
          color: '#fff',
          cursor: 'pointer',
          fontSize: 18
        }}>✕</button>
        </div>
        <div style={{
        flex: 1,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        color: '#555'
      }}>
          {t("components.PptEditor.k10")}
        </div>
      </div>;
  }
  return <div style={{
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    background: '#0a0a10',
    color: '#ccc'
  }}>
      <div style={{
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      padding: '8px 16px',
      borderBottom: '1px solid #222',
      flexShrink: 0
    }}>
        <div style={{
        display: 'flex',
        alignItems: 'center',
        gap: 12
      }}>
          <span style={{
          color: '#B026FF',
          fontWeight: 600
        }}>{t("components.PptEditor.k11")}</span>
          <span style={{
          color: '#666',
          fontSize: 13
        }}>{fileName}</span>
          {unsaved && <span style={{
          color: '#FFD700',
          fontSize: 12
        }}>{t("components.AudioEditor.k5")}</span>}
        </div>
        <div style={{
        display: 'flex',
        gap: 8
      }}>
          <button onClick={handleAddSlide} disabled={adding} style={{
          background: '#B026FF',
          border: 'none',
          color: '#fff',
          padding: '5px 12px',
          borderRadius: 4,
          cursor: 'pointer',
          fontSize: 13
        }}>
            {adding ? '...' : t("components.PptEditor.k12")}
          </button>
          <button onClick={handleDeleteSlide} disabled={deleting || slides.length <= 1} style={{
          background: '#333',
          border: 'none',
          color: '#ff6b6b',
          padding: '5px 12px',
          borderRadius: 4,
          cursor: 'pointer',
          fontSize: 13
        }}>
            {deleting ? '...' : t("components.PptEditor.k13")}
          </button>
          <button onClick={handleSave} disabled={saving || !unsaved} style={{
          background: unsaved ? '#B026FF' : '#333',
          border: 'none',
          color: unsaved ? '#fff' : '#666',
          padding: '5px 12px',
          borderRadius: 4,
          cursor: unsaved ? 'pointer' : 'not-allowed',
          fontSize: 13
        }}>
            {saving ? t("components.AudioEditor.k6") : t("components.AudioEditor.k7")}
          </button>
          <button onClick={onClose} style={{
          background: 'transparent',
          border: 'none',
          color: '#fff',
          cursor: 'pointer',
          fontSize: 18,
          padding: '0 4px'
        }}>
            ✕
          </button>
        </div>
      </div>

      <div style={{
      display: 'flex',
      flex: 1,
      minHeight: 0
    }}>
        <div style={{
        width: 220,
        borderRight: '1px solid #222',
        overflowY: 'auto',
        padding: 8,
        flexShrink: 0
      }}>
          <div style={{
          fontSize: 11,
          color: '#666',
          marginBottom: 8,
          paddingLeft: 4
        }}>
            {t("components.PptEditor.k14")}{slides.length})
          </div>
          {slides.map((s, i) => <div key={i} onClick={() => setActiveSlide(i)} style={{
          padding: '8px 10px',
          marginBottom: 4,
          borderRadius: 4,
          cursor: 'pointer',
          background: i === activeSlide ? '#B026FF22' : 'transparent',
          border: i === activeSlide ? '1px solid #B026FF' : '1px solid transparent',
          fontSize: 13
        }}>
              <div style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6
          }}>
                <span style={{
              width: 24,
              height: 24,
              borderRadius: 3,
              background: i === activeSlide ? '#B026FF' : '#222',
              color: i === activeSlide ? '#fff' : '#888',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontSize: 11,
              fontWeight: 600,
              flexShrink: 0
            }}>
                  {i + 1}
                </span>
                <span style={{
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
              color: i === activeSlide ? '#fff' : '#aaa'
            }}>
                  {s.text_content.substring(0, 30) || t("components.PptEditor.k15", {
                arg0: i + 1
              })}
                </span>
              </div>
              <div style={{
            display: 'flex',
            gap: 2,
            marginTop: 4,
            marginLeft: 30
          }}>
                <button onClick={ev => {
              ev.stopPropagation();
              handleReorder('up');
            }} disabled={i === 0} title={t("components.PptEditor.k16")} style={{
              background: 'transparent',
              border: 'none',
              color: i === 0 ? '#333' : '#888',
              cursor: i === 0 ? 'default' : 'pointer',
              fontSize: 11,
              padding: '0 4px'
            }}>
                  ↑
                </button>
                <button onClick={ev => {
              ev.stopPropagation();
              handleReorder('down');
            }} disabled={i === slides.length - 1} title={t("components.PptEditor.k17")} style={{
              background: 'transparent',
              border: 'none',
              color: i === slides.length - 1 ? '#333' : '#888',
              cursor: i === slides.length - 1 ? 'default' : 'pointer',
              fontSize: 11,
              padding: '0 4px'
            }}>
                  ↓
                </button>
              </div>
            </div>)}
        </div>

        <div style={{
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        minWidth: 0
      }}>
          <div style={{
          padding: '8px 16px',
          borderBottom: '1px solid #222',
          fontSize: 13,
          color: '#888',
          flexShrink: 0
        }}>
            {t("components.PptEditor.k18")} {activeSlide + 1} / {slides.length}
            {currentSlide && <span style={{
            marginLeft: 12,
            color: '#B026FF',
            fontSize: 11
          }}>
                {currentSlide.name}
              </span>}
          </div>

          <textarea ref={editRef} value={currentSlide?.text_content || ''} onChange={e => handleTextChange(e.target.value)} placeholder={t("components.PptEditor.k19")} style={{
          flex: 1,
          padding: 16,
          background: '#111118',
          color: '#ddd',
          border: 'none',
          outline: 'none',
          resize: 'none',
          fontFamily: '"Segoe UI", "Microsoft YaHei", sans-serif',
          fontSize: 15,
          lineHeight: 1.7
        }} />
        </div>
      </div>
    </div>;
}