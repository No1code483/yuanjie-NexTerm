import { t } from "i18next";
import { useState, useRef, useEffect, useCallback } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import * as pdfjsLib from 'pdfjs-dist';
import { random } from '@/lib/utils';
pdfjsLib.GlobalWorkerOptions.workerSrc =
  new URL('pdfjs-dist/build/pdf.worker.min.mjs', import.meta.url).toString();
interface Annotation {
  id: string;
  page: number;
  type: 'highlight' | 'underline' | 'note';
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  text: string;
}
interface PdfEditorProps {
  filePath: string;
  fileName: string;
  onClose: () => void;
  showStatus: (type: 'success' | 'error', text: string) => void;
}
export default function PdfEditor({
  filePath,
  fileName,
  onClose,
  showStatus
}: PdfEditorProps) {
  const [pageCount, setPageCount] = useState(0);
  const [currentPage, setCurrentPage] = useState(1);
  const [annotations, setAnnotations] = useState<Annotation[]>([]);
  const [tool, setTool] = useState<'highlight' | 'underline' | 'note'>('highlight');
  const [toolColor, setToolColor] = useState('#FFD700');
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [unsaved, setUnsaved] = useState(false);
  const [pdfDoc, setPdfDoc] = useState<pdfjsLib.PDFDocumentProxy | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1.5);
  const [dragStart, setDragStart] = useState<{
    x: number;
    y: number;
    page: number;
  } | null>(null);
  const [dragRect, setDragRect] = useState<{
    x: number;
    y: number;
    w: number;
    h: number;
  } | null>(null);
  const pageCanvasesRef = useRef<Map<number, HTMLCanvasElement>>(new Map());
  const [noteInput, setNoteInput] = useState<{
    x: number;
    y: number;
    page: number;
  } | null>(null);
  const [noteText, setNoteText] = useState('');
  const noteInputRef = useRef<HTMLTextAreaElement>(null);
  const annotId = useCallback(() => random.uid('a_'), []);
  const fileUrl = convertFileSrc(filePath);
  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    pdfjsLib.getDocument({
      url: fileUrl
    }).promise.then(doc => {
      if (cancelled) return;
      setPdfDoc(doc);
      setPageCount(doc.numPages);
      setLoading(false);
    }).catch(e => {
      if (cancelled) return;
      showStatus('error', t("components.PdfEditor.k1", {
        e: e
      }));
      setLoading(false);
    });
    return () => {
      cancelled = true;
    };
  }, [fileUrl, showStatus]);
  const renderPage = useCallback(async (pageNum: number) => {
    if (!pdfDoc) return;
    const canvas = pageCanvasesRef.current.get(pageNum);
    if (!canvas) return;
    try {
      const page = await pdfDoc.getPage(pageNum);
      const viewport = page.getViewport({
        scale
      });
      canvas.width = viewport.width;
      canvas.height = viewport.height;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      await page.render({
        canvas,
        canvasContext: ctx,
        viewport
      }).promise;
    } catch (_) {}
  }, [pdfDoc, scale]);
  useEffect(() => {
    if (!pdfDoc) return;
    pageCanvasesRef.current.clear();
    for (let i = 1; i <= pageCount; i++) {
      setTimeout(() => renderPage(i), 10);
    }
  }, [pdfDoc, pageCount, renderPage]);
  useEffect(() => {
    if (!pdfDoc) return;
    for (let i = 1; i <= pageCount; i++) {
      renderPage(i);
    }
  }, [scale, pdfDoc, pageCount, renderPage]);
  const pageAnnotations = annotations.filter(a => a.page === currentPage);
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (!overlayRef.current) return;
    const rect = overlayRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left + overlayRef.current.scrollLeft;
    const y = e.clientY - rect.top + overlayRef.current.scrollTop;
    const pageHeight = pageCanvasesRef.current.get(currentPage)?.height || 792 * scale;
    const pageY = y % pageHeight;
    const pageIdx = Math.floor(y / pageHeight);
    const page = pageIdx + 1;
    if (tool === 'note') {
      setNoteInput({
        x,
        y: pageY,
        page
      });
      setNoteText('');
      setTimeout(() => noteInputRef.current?.focus(), 50);
      return;
    }
    if (page < 1 || page > pageCount) return;
    setDragStart({
      x,
      y: pageY,
      page
    });
  }, [tool, currentPage, pageCount, scale]);
  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragStart || !overlayRef.current) return;
    const rect = overlayRef.current.getBoundingClientRect();
    const x = e.clientX - rect.left + overlayRef.current.scrollLeft;
    const y = e.clientY - rect.top + overlayRef.current.scrollTop;
    const pageHeight = pageCanvasesRef.current.get(currentPage)?.height || 792 * scale;
    const pageY = y % pageHeight;
    const pageIdx = Math.floor(y / pageHeight);
    if (pageIdx + 1 !== dragStart.page) return;
    setDragRect({
      x: Math.min(dragStart.x, x),
      y: Math.min(dragStart.y, pageY),
      w: Math.abs(x - dragStart.x),
      h: Math.abs(pageY - dragStart.y)
    });
  }, [dragStart, currentPage, scale]);
  const handleMouseUp = useCallback(() => {
    if (!dragStart || !dragRect || dragRect.w < 5 || dragRect.h < 5) {
      setDragStart(null);
      setDragRect(null);
      return;
    }
    setAnnotations(prev => [...prev, {
      id: annotId(),
      page: dragStart.page,
      type: tool,
      x: dragRect.x,
      y: dragRect.y,
      w: dragRect.w,
      h: dragRect.h,
      color: toolColor,
      text: ''
    }]);
    setUnsaved(true);
    setDragStart(null);
    setDragRect(null);
  }, [dragStart, dragRect, tool, toolColor, annotId]);
  const handleNoteSubmit = useCallback(() => {
    if (!noteInput || !noteText.trim()) return;
    setAnnotations(prev => [...prev, {
      id: annotId(),
      page: noteInput.page,
      type: 'note',
      x: noteInput.x,
      y: noteInput.y,
      w: 24,
      h: 24,
      color: toolColor,
      text: noteText.trim()
    }]);
    setUnsaved(true);
    setNoteInput(null);
    setNoteText('');
  }, [noteInput, noteText, toolColor, annotId]);
  const handleDeleteAnnotation = useCallback((id: string) => {
    setAnnotations(prev => prev.filter(a => a.id !== id));
    setUnsaved(true);
  }, []);
  const handleSave = useCallback(async () => {
    if (annotations.length === 0) {
      showStatus('success', t("components.PdfEditor.k2"));
      return;
    }
    setSaving(true);
    try {
      const {
        PDFDocument,
        rgb
      } = await import('pdf-lib');
      const originalBytes = await fetch(fileUrl).then(r => r.arrayBuffer());
      const pdfDoc = await PDFDocument.load(originalBytes);
      const pages = pdfDoc.getPages();
      for (const annot of annotations) {
        const pageIdx = annot.page - 1;
        if (pageIdx >= pages.length) continue;
        const page = pages[pageIdx];
        const {
          height: ph
        } = page.getSize();
        const color = annot.color.replace('#', '');
        const r = parseInt(color.substring(0, 2), 16) / 255;
        const g = parseInt(color.substring(2, 4), 16) / 255;
        const b = parseInt(color.substring(4, 6), 16) / 255;
        const x = annot.x / scale;
        const y = ph - annot.y / scale - annot.h / scale;
        const w = annot.w / scale;
        const h = annot.h / scale;
        if (annot.type === 'note') {
          page.drawText(annot.text, {
            x: x + 5,
            y,
            size: 10,
            color: rgb(r, g, b)
          });
        } else if (annot.type === 'highlight' || annot.type === 'underline') {
          page.drawRectangle({
            x,
            y: annot.type === 'underline' ? y - 2 : y,
            width: w,
            height: annot.type === 'underline' ? 2 : h,
            color: rgb(r, g, b),
            opacity: annot.type === 'highlight' ? 0.35 : 1
          });
        }
      }
      const pdfBytes = await pdfDoc.save();
      const base64 = btoa(String.fromCharCode(...new Uint8Array(pdfBytes)));
      const {
        invoke
      } = await import('@tauri-apps/api/core');
      await invoke('pdfedit_save', {
        path: filePath,
        base64_data: base64
      });
      setUnsaved(false);
      showStatus('success', t("components.PdfEditor.k3", {
        length: annotations.length
      }));
    } catch (e: any) {
      showStatus('error', e?.toString() || t("errors.saveFailed"));
    } finally {
      setSaving(false);
    }
  }, [annotations, fileUrl, filePath, scale, showStatus]);
  const colors = ['#FFD700', '#FF6B6B', '#51CF66', '#339AF0', '#B026FF', '#FF922B'];
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
          color: '#339AF0',
          fontWeight: 600
        }}>{t("components.PdfEditor.k4")}</span>
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
        gap: 8,
        alignItems: 'center'
      }}>
          <select value={scale} onChange={e => setScale(Number(e.target.value))} style={{
          background: '#222',
          color: '#ccc',
          border: '1px solid #333',
          padding: '4px 8px',
          borderRadius: 4,
          fontSize: 12
        }}>
            <option value={1}>100%</option>
            <option value={1.25}>125%</option>
            <option value={1.5}>150%</option>
            <option value={2}>200%</option>
          </select>
          <button onClick={handleSave} disabled={saving || !unsaved} style={{
          background: unsaved ? '#339AF0' : '#333',
          border: 'none',
          color: unsaved ? '#fff' : '#666',
          padding: '5px 12px',
          borderRadius: 4,
          cursor: unsaved ? 'pointer' : 'not-allowed',
          fontSize: 13
        }}>
            {saving ? t("components.AudioEditor.k6") : t("components.PdfEditor.k5")}
          </button>
          <button onClick={onClose} style={{
          background: 'transparent',
          border: 'none',
          color: '#fff',
          cursor: 'pointer',
          fontSize: 18
        }}>
            ✕
          </button>
        </div>
      </div>

      <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '6px 16px',
      borderBottom: '1px solid #222',
      flexShrink: 0
    }}>
        <span style={{
        fontSize: 12,
        color: '#888'
      }}>{t("components.ImageEditor.k4")}</span>
        {([['highlight', t("components.PdfEditor.k6")], ['underline', t("components.PdfEditor.k7")], ['note', t("components.PdfEditor.k8")]] as const).map(([t, label]) => <button key={t} onClick={() => setTool(t)} style={{
        background: tool === t ? '#339AF0' : '#222',
        border: 'none',
        color: tool === t ? '#fff' : '#888',
        padding: '4px 10px',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12
      }}>
            {label}
          </button>)}
        <span style={{
        fontSize: 12,
        color: '#888',
        marginLeft: 8
      }}>{t("components.ImageEditor.k8")}</span>
        <div style={{
        display: 'flex',
        gap: 4
      }}>
          {colors.map(c => <button key={c} onClick={() => setToolColor(c)} style={{
          width: 20,
          height: 20,
          borderRadius: '50%',
          background: c,
          border: toolColor === c ? '2px solid #fff' : '2px solid transparent',
          cursor: 'pointer'
        }} />)}
        </div>
        {tool === 'highlight' && <span style={{
        fontSize: 11,
        color: '#666'
      }}>{t("components.PdfEditor.k9")}</span>}
        {tool === 'underline' && <span style={{
        fontSize: 11,
        color: '#666'
      }}>{t("components.PdfEditor.k10")}</span>}
        {tool === 'note' && <span style={{
        fontSize: 11,
        color: '#666'
      }}>{t("components.PdfEditor.k11")}</span>}

        {pageAnnotations.length > 0 && <button onClick={() => {
        setAnnotations(annotations.filter(a => a.page !== currentPage));
        setUnsaved(true);
      }} style={{
        background: '#333',
        border: 'none',
        color: '#ff6b6b',
        padding: '4px 10px',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12,
        marginLeft: 'auto'
      }}>
            {t("components.PdfEditor.k12")}{pageAnnotations.length})
          </button>}
      </div>

      {loading ? <div style={{
      flex: 1,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      color: '#555'
    }}>
          {t("components.PdfEditor.k13")}
        </div> : <div style={{
      flex: 1,
      position: 'relative',
      overflow: 'hidden'
    }}>
          <div ref={containerRef} style={{
        height: '100%',
        overflowY: 'auto',
        overflowX: 'auto',
        background: '#0d0d14',
        display: 'flex',
        justifyContent: 'center'
      }}>
            <div style={{
          position: 'relative',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          padding: '20px 0'
        }}>
              {Array.from({
            length: pageCount
          }, (_, i) => i + 1).map(pageNum => <div key={pageNum} style={{
            position: 'relative',
            marginBottom: 16,
            boxShadow: '0 4px 20px rgba(0,0,0,0.5)'
          }}>
                  <canvas ref={el => {
              if (el) pageCanvasesRef.current.set(pageNum, el);
            }} style={{
              display: 'block'
            }} />
                </div>)}
            </div>
          </div>

          <div ref={overlayRef} onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} onMouseLeave={() => {
        setDragStart(null);
        setDragRect(null);
      }} style={{
        position: 'absolute',
        inset: 0,
        cursor: tool === 'note' ? 'crosshair' : 'crosshair',
        zIndex: 10
      }} />

          <div style={{
        position: 'absolute',
        bottom: 16,
        left: '50%',
        transform: 'translateX(-50%)',
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        background: 'rgba(0,0,0,0.8)',
        borderRadius: 8,
        padding: '4px 12px'
      }}>
            <button onClick={() => setCurrentPage(p => Math.max(1, p - 1))} disabled={currentPage === 1} style={{
          background: 'transparent',
          border: 'none',
          color: currentPage === 1 ? '#444' : '#fff',
          cursor: 'pointer',
          fontSize: 16
        }}>◀</button>
            <span style={{
          color: '#ccc',
          fontSize: 13
        }}>
              {currentPage} / {pageCount}
            </span>
            <button onClick={() => setCurrentPage(p => Math.min(pageCount, p + 1))} disabled={currentPage === pageCount} style={{
          background: 'transparent',
          border: 'none',
          color: currentPage === pageCount ? '#444' : '#fff',
          cursor: 'pointer',
          fontSize: 16
        }}>▶</button>
          </div>

          {noteInput && <div style={{
        position: 'absolute',
        left: noteInput.x,
        top: noteInput.y,
        background: '#222',
        border: '1px solid #339AF0',
        borderRadius: 6,
        padding: 8,
        zIndex: 20,
        display: 'flex',
        flexDirection: 'column',
        gap: 6
      }}>
              <textarea ref={noteInputRef} value={noteText} onChange={e => setNoteText(e.target.value)} placeholder={t("components.PdfEditor.k14")} rows={3} style={{
          width: 200,
          background: '#111',
          color: '#ddd',
          border: '1px solid #444',
          borderRadius: 4,
          padding: 6,
          fontSize: 13,
          resize: 'none',
          outline: 'none'
        }} onKeyDown={e => {
          if (e.key === 'Escape') setNoteInput(null);
        }} />
              <div style={{
          display: 'flex',
          gap: 6,
          justifyContent: 'flex-end'
        }}>
                <button onClick={() => setNoteInput(null)} style={{
            background: '#333',
            border: 'none',
            color: '#aaa',
            padding: '3px 10px',
            borderRadius: 4,
            cursor: 'pointer',
            fontSize: 12
          }}>
                  {t("common.cancel")}
                </button>
                <button onClick={handleNoteSubmit} style={{
            background: '#339AF0',
            border: 'none',
            color: '#fff',
            padding: '3px 10px',
            borderRadius: 4,
            cursor: 'pointer',
            fontSize: 12
          }}>
                  {t("common.add")}
                </button>
              </div>
            </div>}

          {pageAnnotations.map(annot => {
        const isNote = annot.type === 'note';
        return <div key={annot.id} onClick={() => handleDeleteAnnotation(annot.id)} title={t("components.PdfEditor.k15")} style={{
          position: 'absolute',
          left: annot.x,
          top: annot.y,
          width: annot.w,
          height: annot.h,
          background: isNote ? annot.color : undefined,
          borderBottom: annot.type === 'underline' ? `2px solid ${annot.color}` : undefined,
          backgroundColor: annot.type === 'highlight' ? `${annot.color}44` : undefined,
          borderRadius: isNote ? '50%' : 2,
          cursor: 'pointer',
          zIndex: 15,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: '#fff',
          fontSize: 10,
          fontWeight: 600
        }}>
                {isNote && 'N'}
              </div>;
      })}

          {dragRect && <div style={{
        position: 'absolute',
        left: dragRect.x,
        top: dragRect.y,
        width: dragRect.w,
        height: dragRect.h,
        background: tool === 'highlight' ? `${toolColor}44` : undefined,
        borderBottom: tool === 'underline' ? `3px solid ${toolColor}` : undefined,
        border: tool === 'highlight' ? `2px dashed ${toolColor}` : undefined,
        pointerEvents: 'none',
        zIndex: 15
      }} />}
        </div>}
    </div>;
}