import { t } from "i18next";
import { useState, useRef, useCallback, useEffect } from 'react';
interface ImageViewerProps {
  src: string;
  alt: string;
  className?: string;
}
export default function ImageViewer({
  src,
  alt,
  className
}: ImageViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);
  const [scale, setScale] = useState(1);
  const [rotation, setRotation] = useState(0);
  const [position, setPosition] = useState({
    x: 0,
    y: 0
  });
  const [dragging, setDragging] = useState(false);
  const [dragStart, setDragStart] = useState({
    x: 0,
    y: 0
  });
  const [posStart, setPosStart] = useState({
    x: 0,
    y: 0
  });
  const zoomIn = useCallback(() => setScale(s => Math.min(s + 0.25, 5)), []);
  const zoomOut = useCallback(() => setScale(s => Math.max(s - 0.25, 0.25)), []);
  const rotateCW = useCallback(() => setRotation(r => (r + 90) % 360), []);
  const rotateCCW = useCallback(() => setRotation(r => (r - 90 + 360) % 360), []);
  const reset = useCallback(() => {
    setScale(1);
    setRotation(0);
    setPosition({
      x: 0,
      y: 0
    });
  }, []);
  const fitToScreen = useCallback(() => {
    if (!containerRef.current || !imgRef.current) return;
    const container = containerRef.current;
    const img = imgRef.current;
    const cw = container.clientWidth - 40;
    const ch = container.clientHeight - 40;
    const iw = img.naturalWidth;
    const ih = img.naturalHeight;
    if (iw === 0 || ih === 0) return;
    const fitScale = Math.min(cw / iw, ch / ih, 1);
    setScale(Math.round(fitScale * 100) / 100);
    setPosition({
      x: 0,
      y: 0
    });
    setRotation(0);
  }, []);
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const handleWheel = (e: WheelEvent) => {
      e.preventDefault();
      const delta = e.deltaY > 0 ? -0.1 : 0.1;
      setScale(s => Math.max(0.25, Math.min(5, s + delta)));
    };
    container.addEventListener('wheel', handleWheel, {
      passive: false
    });
    return () => container.removeEventListener('wheel', handleWheel);
  }, []);
  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    if (e.button !== 0) return;
    setDragging(true);
    setDragStart({
      x: e.clientX,
      y: e.clientY
    });
    setPosStart({
      x: position.x,
      y: position.y
    });
  }, [position]);
  const handleMouseMove = useCallback((e: React.MouseEvent) => {
    if (!dragging) return;
    const dx = e.clientX - dragStart.x;
    const dy = e.clientY - dragStart.y;
    setPosition({
      x: posStart.x + dx,
      y: posStart.y + dy
    });
  }, [dragging, dragStart, posStart]);
  const handleMouseUp = useCallback(() => {
    setDragging(false);
  }, []);
  const cursor = dragging ? 'grabbing' : scale > 1 ? 'grab' : 'default';
  return <div className={className} style={{
    display: 'flex',
    flexDirection: 'column',
    height: '100%'
  }}>
      <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      padding: '6px 12px',
      background: 'rgba(30, 30, 40, 0.8)',
      borderBottom: '1px solid rgba(255, 255, 255, 0.08)'
    }}>
        <span style={{
        color: '#fff',
        fontSize: 12,
        fontWeight: 500
      }}>🖼️ {alt}</span>
        <div style={{
        flex: 1
      }} />
        <button onClick={zoomOut} disabled={scale <= 0.25} style={toolbarBtnStyle(scale <= 0.25)}>➖</button>
        <button onClick={reset} style={{
        ...toolbarBtnStyle(false),
        background: 'rgba(0,240,255,0.15)',
        border: '1px solid rgba(0,240,255,0.3)',
        color: '#00F0FF',
        fontSize: 11
      }}>
          {Math.round(scale * 100)}%
        </button>
        <button onClick={zoomIn} disabled={scale >= 5} style={toolbarBtnStyle(scale >= 5)}>➕</button>
        <div style={{
        width: 1,
        height: 14,
        background: 'rgba(255,255,255,0.15)'
      }} />
        <button onClick={fitToScreen} style={toolbarBtnStyle(false)} title={t("components.ImageViewer.k1")}>⊡</button>
        <button onClick={rotateCCW} style={toolbarBtnStyle(false)} title={t("components.ImageViewer.k2")}>↺</button>
        <button onClick={rotateCW} style={toolbarBtnStyle(false)} title={t("components.ImageViewer.k3")}>↻</button>
        <div style={{
        width: 1,
        height: 14,
        background: 'rgba(255,255,255,0.15)'
      }} />
        <button onClick={reset} style={toolbarBtnStyle(false)} title={t("common.reset")}>⟲</button>
      </div>

      <div ref={containerRef} onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} onMouseLeave={handleMouseUp} style={{
      flex: 1,
      overflow: 'hidden',
      background: '#1a1a2a',
      display: 'flex',
      justifyContent: 'center',
      alignItems: 'center',
      cursor
    }}>
        <img ref={imgRef} src={src} alt={alt} draggable={false} style={{
        transform: `translate(${position.x}px, ${position.y}px) rotate(${rotation}deg) scale(${scale})`,
        transformOrigin: 'center center',
        maxWidth: '100%',
        maxHeight: '100%',
        objectFit: 'contain',
        transition: dragging ? 'none' : 'transform 0.1s ease',
        userSelect: 'none'
      }} />
      </div>
    </div>;
}
function toolbarBtnStyle(disabled: boolean): React.CSSProperties {
  return {
    background: 'rgba(255,255,255,0.08)',
    border: '1px solid rgba(255,255,255,0.15)',
    color: disabled ? 'rgba(255,255,255,0.25)' : '#ccc',
    cursor: disabled ? 'default' : 'pointer',
    padding: '3px 8px',
    borderRadius: 4,
    fontSize: 12,
    lineHeight: 1
  };
}