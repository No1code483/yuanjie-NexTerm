import { t } from "i18next";
import { useState, useRef, useEffect, useCallback } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
interface ImageEditorProps {
  filePath: string;
  fileName: string;
  onClose: () => void;
  showStatus: (type: 'success' | 'error', text: string) => void;
}
type Tool = 'select' | 'crop' | 'pen' | 'text';
export default function ImageEditor({
  filePath,
  fileName,
  onClose,
  showStatus
}: ImageEditorProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [img, setImg] = useState<HTMLImageElement | null>(null);
  const [loading, setLoading] = useState(true);
  const [unsaved, setUnsaved] = useState(false);
  const [saving, setSaving] = useState(false);
  const [tool, setTool] = useState<Tool>('select');
  const [penColor, setPenColor] = useState('#FF6B6B');
  const [penSize, setPenSize] = useState(3);
  const [cropRect, setCropRect] = useState<{
    x: number;
    y: number;
    w: number;
    h: number;
  } | null>(null);
  const [dragStart, setDragStart] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const [drawing, setDrawing] = useState(false);
  const lastPointRef = useRef<{
    x: number;
    y: number;
  } | null>(null);
  const fileUrl = convertFileSrc(filePath);
  useEffect(() => {
    setLoading(true);
    const image = new Image();
    image.onload = () => {
      setImg(image);
      setLoading(false);
    };
    image.onerror = () => {
      showStatus('error', t("components.ImageEditor.k1"));
      setLoading(false);
    };
    image.src = fileUrl;
  }, [fileUrl, showStatus]);
  const fitCanvas = useCallback(() => {
    if (!img || !canvasRef.current || !containerRef.current) return;
    const container = containerRef.current;
    const maxW = container.clientWidth - 40;
    const maxH = container.clientHeight - 40;
    const scale = Math.min(maxW / img.width, maxH / img.height, 1);
    const canvas = canvasRef.current;
    canvas.width = img.width * scale;
    canvas.height = img.height * scale;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
    if (cropRect) {
      ctx.strokeStyle = '#339AF0';
      ctx.lineWidth = 2;
      ctx.setLineDash([6, 3]);
      ctx.strokeRect(cropRect.x, cropRect.y, cropRect.w, cropRect.h);
    }
  }, [img, cropRect]);
  useEffect(() => {
    fitCanvas();
  }, [fitCanvas]);
  useEffect(() => {
    window.addEventListener('resize', fitCanvas);
    return () => window.removeEventListener('resize', fitCanvas);
  }, [fitCanvas]);
  const getCanvasCoords = (e: React.MouseEvent) => {
    if (!canvasRef.current) return {
      x: 0,
      y: 0
    };
    const rect = canvasRef.current.getBoundingClientRect();
    return {
      x: (e.clientX - rect.left) * (canvasRef.current.width / rect.width),
      y: (e.clientY - rect.top) * (canvasRef.current.height / rect.height)
    };
  };
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    const {
      x,
      y
    } = getCanvasCoords(e);
    if (tool === 'crop') {
      setDragStart({
        x,
        y
      });
      setCropRect({
        x,
        y,
        w: 0,
        h: 0
      });
      return;
    }
    if (tool === 'pen') {
      setDrawing(true);
      lastPointRef.current = {
        x,
        y
      };
    }
  }, [tool]);
  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    const {
      x,
      y
    } = getCanvasCoords(e);
    if (tool === 'crop' && dragStart) {
      setCropRect({
        x: Math.min(dragStart.x, x),
        y: Math.min(dragStart.y, y),
        w: Math.abs(x - dragStart.x),
        h: Math.abs(y - dragStart.y)
      });
      return;
    }
    if (tool === 'pen' && drawing && canvasRef.current && lastPointRef.current) {
      const ctx = canvasRef.current.getContext('2d');
      if (!ctx) return;
      ctx.strokeStyle = penColor;
      ctx.lineWidth = penSize;
      ctx.lineCap = 'round';
      ctx.lineJoin = 'round';
      ctx.beginPath();
      ctx.moveTo(lastPointRef.current.x, lastPointRef.current.y);
      ctx.lineTo(x, y);
      ctx.stroke();
      lastPointRef.current = {
        x,
        y
      };
    }
  }, [tool, dragStart, drawing, penColor, penSize]);
  const handleMouseUp = useCallback(() => {
    if (tool === 'pen' && drawing) {
      setUnsaved(true);
      setDrawing(false);
      lastPointRef.current = null;
    }
    if (tool === 'crop' && dragStart) {
      setDragStart(null);
    }
  }, [tool, drawing, dragStart]);
  const handleCropApply = useCallback(() => {
    if (!cropRect || !canvasRef.current || !img) return;
    if (cropRect.w < 10 || cropRect.h < 10) return;
    const canvas = canvasRef.current;
    const srcCanvas = document.createElement('canvas');
    srcCanvas.width = canvas.width;
    srcCanvas.height = canvas.height;
    const srcCtx = srcCanvas.getContext('2d');
    if (!srcCtx) return;
    srcCtx.drawImage(canvas, 0, 0);
    const croppedCanvas = document.createElement('canvas');
    croppedCanvas.width = cropRect.w;
    croppedCanvas.height = cropRect.h;
    const cropCtx = croppedCanvas.getContext('2d');
    if (!cropCtx) return;
    cropCtx.drawImage(srcCanvas, cropRect.x, cropRect.y, cropRect.w, cropRect.h, 0, 0, cropRect.w, cropRect.h);
    const newImage = new Image();
    newImage.onload = () => {
      setImg(newImage);
      setCropRect(null);
      setUnsaved(true);
    };
    newImage.src = croppedCanvas.toDataURL();
    canvas.width = cropRect.w;
    canvas.height = cropRect.h;
    const ctx = canvas.getContext('2d');
    if (ctx) ctx.drawImage(croppedCanvas, 0, 0);
  }, [cropRect, img]);
  const handleRotate = useCallback((deg: number) => {
    if (!img || !canvasRef.current) return;
    const canvas = canvasRef.current;
    const rotatedCanvas = document.createElement('canvas');
    if (deg === 90 || deg === 270) {
      rotatedCanvas.width = canvas.height;
      rotatedCanvas.height = canvas.width;
    } else {
      rotatedCanvas.width = canvas.width;
      rotatedCanvas.height = canvas.height;
    }
    const rCtx = rotatedCanvas.getContext('2d');
    if (!rCtx) return;
    rCtx.translate(rotatedCanvas.width / 2, rotatedCanvas.height / 2);
    rCtx.rotate(deg * Math.PI / 180);
    rCtx.drawImage(canvas, -canvas.width / 2, -canvas.height / 2, canvas.width, canvas.height);
    const newImage = new Image();
    newImage.onload = () => {
      setImg(newImage);
      setUnsaved(true);
    };
    newImage.src = rotatedCanvas.toDataURL();
  }, [img]);
  const handleFilter = useCallback((filterType: string) => {
    if (!img || !canvasRef.current) return;
    const canvas = canvasRef.current;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const data = imageData.data;
    for (let i = 0; i < data.length; i += 4) {
      const r = data[i];
      const g = data[i + 1];
      const b = data[i + 2];
      if (filterType === 'grayscale') {
        const gray = 0.299 * r + 0.587 * g + 0.114 * b;
        data[i] = gray;
        data[i + 1] = gray;
        data[i + 2] = gray;
      } else if (filterType === 'sepia') {
        data[i] = Math.min(255, 0.393 * r + 0.769 * g + 0.189 * b);
        data[i + 1] = Math.min(255, 0.349 * r + 0.686 * g + 0.168 * b);
        data[i + 2] = Math.min(255, 0.272 * r + 0.534 * g + 0.131 * b);
      } else if (filterType === 'invert') {
        data[i] = 255 - r;
        data[i + 1] = 255 - g;
        data[i + 2] = 255 - b;
      }
    }
    ctx.putImageData(imageData, 0, 0);
    setUnsaved(true);
  }, [img]);
  const handleSave = useCallback(async () => {
    if (!canvasRef.current) return;
    setSaving(true);
    try {
      const dataUrl = canvasRef.current.toDataURL('image/png');
      const base64 = dataUrl.split(',')[1];
      const {
        invoke
      } = await import('@tauri-apps/api/core');
      await invoke('imageedit_save', {
        path: filePath,
        base64_data: base64
      });
      setUnsaved(false);
      showStatus('success', t("components.ImageEditor.k2"));
    } catch (e: any) {
      showStatus('error', e?.toString() || t("errors.saveFailed"));
    } finally {
      setSaving(false);
    }
  }, [filePath, showStatus]);
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
  const colors = ['#FF6B6B', '#FFD700', '#51CF66', '#339AF0', '#B026FF', '#FFFFFF', '#FF922B'];
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
          color: '#FF6B6B',
          fontWeight: 600
        }}>{t("components.ImageEditor.k3")}</span>
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
          <button onClick={handleSave} disabled={saving || !unsaved} style={{
          background: unsaved ? '#FF6B6B' : '#333',
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
          fontSize: 18
        }}>
            ✕
          </button>
        </div>
      </div>

      <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      padding: '6px 16px',
      borderBottom: '1px solid #222',
      flexShrink: 0,
      flexWrap: 'wrap'
    }}>
        <span style={{
        fontSize: 12,
        color: '#888'
      }}>{t("components.ImageEditor.k4")}</span>
        {([['select', t("components.ImageEditor.k5")], ['crop', t("components.ImageEditor.k6")], ['pen', t("components.ImageEditor.k7")]] as const).map(([t, label]) => <button key={t} onClick={() => setTool(t)} style={{
        background: tool === t ? '#FF6B6B' : '#222',
        border: 'none',
        color: tool === t ? '#fff' : '#888',
        padding: '4px 10px',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12
      }}>
            {label}
          </button>)}

        {tool === 'pen' && <>
            <span style={{
          fontSize: 12,
          color: '#888',
          marginLeft: 4
        }}>{t("components.ImageEditor.k8")}</span>
            <div style={{
          display: 'flex',
          gap: 3
        }}>
              {colors.map(c => <button key={c} onClick={() => setPenColor(c)} style={{
            width: 18,
            height: 18,
            borderRadius: '50%',
            background: c,
            border: penColor === c ? '2px solid #fff' : '2px solid transparent',
            cursor: 'pointer'
          }} />)}
            </div>
            <span style={{
          fontSize: 12,
          color: '#888',
          marginLeft: 4
        }}>{t("components.ImageEditor.k9")}</span>
            <select value={penSize} onChange={e => setPenSize(Number(e.target.value))} style={{
          background: '#222',
          color: '#ccc',
          border: '1px solid #333',
          padding: '3px 6px',
          borderRadius: 4,
          fontSize: 12
        }}>
              <option value={1}>1px</option>
              <option value={3}>3px</option>
              <option value={5}>5px</option>
              <option value={8}>8px</option>
            </select>
          </>}

        <span style={{
        color: '#444',
        margin: '0 4px'
      }}>|</span>
        <span style={{
        fontSize: 12,
        color: '#888'
      }}>{t("components.ImageEditor.k10")}</span>
        <button onClick={() => handleRotate(-90)} style={{
        background: '#222',
        border: 'none',
        color: '#aaa',
        padding: '4px 8px',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12
      }}>{t("components.ImageEditor.k11")}</button>
        <button onClick={() => handleRotate(90)} style={{
        background: '#222',
        border: 'none',
        color: '#aaa',
        padding: '4px 8px',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12
      }}>{t("components.ImageEditor.k12")}</button>

        <span style={{
        color: '#444',
        margin: '0 4px'
      }}>|</span>
        <span style={{
        fontSize: 12,
        color: '#888'
      }}>{t("components.ImageEditor.k13")}</span>
        <button onClick={() => handleFilter('grayscale')} style={{
        background: '#222',
        border: 'none',
        color: '#aaa',
        padding: '4px 8px',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12
      }}>{t("components.ImageEditor.k14")}</button>
        <button onClick={() => handleFilter('sepia')} style={{
        background: '#222',
        border: 'none',
        color: '#aaa',
        padding: '4px 8px',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12
      }}>{t("components.ImageEditor.k15")}</button>
        <button onClick={() => handleFilter('invert')} style={{
        background: '#222',
        border: 'none',
        color: '#aaa',
        padding: '4px 8px',
        borderRadius: 4,
        cursor: 'pointer',
        fontSize: 12
      }}>{t("components.ImageEditor.k16")}</button>

        {cropRect && cropRect.w > 10 && <>
            <span style={{
          color: '#444',
          margin: '0 4px'
        }}>|</span>
            <button onClick={handleCropApply} style={{
          background: '#339AF0',
          border: 'none',
          color: '#fff',
          padding: '4px 10px',
          borderRadius: 4,
          cursor: 'pointer',
          fontSize: 12
        }}>
              {t("components.ImageEditor.k17")}
            </button>
            <button onClick={() => setCropRect(null)} style={{
          background: '#333',
          border: 'none',
          color: '#aaa',
          padding: '4px 10px',
          borderRadius: 4,
          cursor: 'pointer',
          fontSize: 12
        }}>
              {t("common.cancel")}
            </button>
          </>}
      </div>

      {loading ? <div style={{
      flex: 1,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      color: '#555'
    }}>
          {t("components.ImageEditor.k18")}
        </div> : <div ref={containerRef} style={{
      flex: 1,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: '#0d0d14',
      overflow: 'hidden'
    }}>
          <canvas ref={canvasRef} onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} onMouseLeave={() => {
        setDrawing(false);
        lastPointRef.current = null;
      }} style={{
        cursor: tool === 'crop' ? 'crosshair' : tool === 'pen' ? 'crosshair' : 'default',
        maxWidth: '100%',
        maxHeight: '100%',
        objectFit: 'contain'
      }} />
        </div>}
    </div>;
}