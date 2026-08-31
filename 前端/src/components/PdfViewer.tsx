import { t } from "i18next";
import { useEffect, useRef, useState, useCallback } from 'react';
import * as pdfjsLib from 'pdfjs-dist';
pdfjsLib.GlobalWorkerOptions.workerSrc = new URL('pdfjs-dist/build/pdf.worker.min.mjs', import.meta.url).toString();
interface PdfViewerProps {
  base64Data: string;
  fileName: string;
  className?: string;
}
export default function PdfViewer({
  base64Data,
  fileName,
  className
}: PdfViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [pdf, setPdf] = useState<pdfjsLib.PDFDocumentProxy | null>(null);
  const [currentPage, setCurrentPage] = useState(1);
  const [totalPages, setTotalPages] = useState(0);
  const [scale, setScale] = useState(1.0);
  const [loading, setLoading] = useState(true);
  const renderPage = useCallback(async (pageNum: number) => {
    if (!pdf || !canvasRef.current) return;
    const page = await pdf.getPage(pageNum);
    const viewport = page.getViewport({
      scale
    });
    const canvas = canvasRef.current;
    canvas.width = viewport.width;
    canvas.height = viewport.height;
    await page.render({
      canvas,
      viewport
    }).promise;
  }, [pdf, scale]);
  useEffect(() => {
    const loadPdf = async () => {
      setLoading(true);
      try {
        const binaryString = atob(base64Data.split(',')[1] || base64Data);
        const bytes = new Uint8Array(binaryString.length);
        for (let i = 0; i < binaryString.length; i++) {
          bytes[i] = binaryString.charCodeAt(i);
        }
        const loadingTask = pdfjsLib.getDocument({
          data: bytes
        });
        const loadedPdf = await loadingTask.promise;
        setPdf(loadedPdf);
        setTotalPages(loadedPdf.numPages);
        setCurrentPage(1);
      } catch (err) {
        console.error('PDF load error:', err);
      } finally {
        setLoading(false);
      }
    };
    if (base64Data) {
      loadPdf();
    }
    return () => {
      if (pdf) {
        pdf.destroy();
      }
    };
  }, [base64Data]);
  useEffect(() => {
    if (pdf) {
      renderPage(currentPage);
    }
  }, [pdf, currentPage, scale, renderPage]);
  const goPrev = () => {
    if (currentPage > 1) {
      setCurrentPage(p => p - 1);
    }
  };
  const goNext = () => {
    if (currentPage < totalPages) {
      setCurrentPage(p => p + 1);
    }
  };
  const zoomIn = () => setScale(s => Math.min(s + 0.25, 3.0));
  const zoomOut = () => setScale(s => Math.max(s - 0.25, 0.5));
  const resetZoom = () => setScale(1.0);
  return <div className={className} style={{
    display: 'flex',
    flexDirection: 'column',
    height: '100%'
  }}>
      <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '8px 12px',
      background: 'rgba(30, 30, 40, 0.8)',
      borderBottom: '1px solid rgba(255, 255, 255, 0.08)'
    }}>
        <span style={{
        color: '#fff',
        fontSize: 13,
        fontWeight: 500
      }}>📄 {fileName}</span>
        <div style={{
        flex: 1
      }} />
        <button onClick={zoomOut} disabled={scale <= 0.5} style={{
        background: 'rgba(255, 255, 255, 0.1)',
        border: '1px solid rgba(255, 255, 255, 0.2)',
        color: scale <= 0.5 ? 'rgba(255,255,255,0.3)' : '#fff',
        cursor: scale <= 0.5 ? 'default' : 'pointer',
        padding: '4px 10px',
        borderRadius: 4,
        fontSize: 12
      }}>
          ➖
        </button>
        <button onClick={resetZoom} style={{
        background: 'rgba(0, 240, 255, 0.15)',
        border: '1px solid rgba(0, 240, 255, 0.3)',
        color: '#00F0FF',
        cursor: 'pointer',
        padding: '4px 8px',
        borderRadius: 4,
        fontSize: 11
      }}>
          {Math.round(scale * 100)}%
        </button>
        <button onClick={zoomIn} disabled={scale >= 3.0} style={{
        background: 'rgba(255, 255, 255, 0.1)',
        border: '1px solid rgba(255, 255, 255, 0.2)',
        color: scale >= 3.0 ? 'rgba(255,255,255,0.3)' : '#fff',
        cursor: scale >= 3.0 ? 'default' : 'pointer',
        padding: '4px 10px',
        borderRadius: 4,
        fontSize: 12
      }}>
          ➕
        </button>
        <div style={{
        width: 1,
        height: 16,
        background: 'rgba(255,255,255,0.15)'
      }} />
        <button onClick={goPrev} disabled={currentPage <= 1} style={{
        background: 'rgba(255, 255, 255, 0.1)',
        border: '1px solid rgba(255, 255, 255, 0.2)',
        color: currentPage <= 1 ? 'rgba(255,255,255,0.3)' : '#fff',
        cursor: currentPage <= 1 ? 'default' : 'pointer',
        padding: '4px 10px',
        borderRadius: 4,
        fontSize: 12
      }}>
          {t("components.PdfViewer.k1")}
        </button>
        <span style={{
        color: '#fff',
        fontSize: 12
      }}>
          {currentPage} / {totalPages}
        </span>
        <button onClick={goNext} disabled={currentPage >= totalPages} style={{
        background: 'rgba(255, 255, 255, 0.1)',
        border: '1px solid rgba(255, 255, 255, 0.2)',
        color: currentPage >= totalPages ? 'rgba(255,255,255,0.3)' : '#fff',
        cursor: currentPage >= totalPages ? 'default' : 'pointer',
        padding: '4px 10px',
        borderRadius: 4,
        fontSize: 12
      }}>
          {t("components.PdfViewer.k2")}
        </button>
      </div>

      <div ref={containerRef} style={{
      flex: 1,
      overflow: 'auto',
      background: '#2a2a3a',
      padding: 16
    }}>
        {loading ? <div style={{
        color: '#fff',
        textAlign: 'center',
        padding: 40
      }}>{t("components.PdfViewer.k3")}</div> : <div style={{
        display: 'flex',
        justifyContent: 'center'
      }}>
            <canvas ref={canvasRef} style={{
          boxShadow: '0 4px 20px rgba(0,0,0,0.5)'
        }} />
          </div>}
      </div>
    </div>;
}