import { t } from "i18next";
import { useState, useRef, useEffect, useCallback } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
interface AudioEditorProps {
  filePath: string;
  fileName: string;
  onClose: () => void;
  showStatus: (type: 'success' | 'error', text: string) => void;
}
function encodeWav(audioBuffer: AudioBuffer): ArrayBuffer {
  const numChannels = audioBuffer.numberOfChannels;
  const sampleRate = audioBuffer.sampleRate;
  const length = audioBuffer.length;
  const bitsPerSample = 16;
  const bytesPerSample = bitsPerSample / 8;
  const blockAlign = numChannels * bytesPerSample;
  const dataSize = length * blockAlign;
  const buffer = new ArrayBuffer(44 + dataSize);
  const view = new DataView(buffer);
  function writeString(offset: number, str: string) {
    for (let i = 0; i < str.length; i++) view.setUint8(offset + i, str.charCodeAt(i));
  }
  writeString(0, 'RIFF');
  view.setUint32(4, 36 + dataSize, true);
  writeString(8, 'WAVE');
  writeString(12, 'fmt ');
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, numChannels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, sampleRate * blockAlign, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);
  writeString(36, 'data');
  view.setUint32(40, dataSize, true);
  const channels: Float32Array[] = [];
  for (let c = 0; c < numChannels; c++) channels.push(audioBuffer.getChannelData(c));
  let offset = 44;
  for (let i = 0; i < length; i++) {
    for (let c = 0; c < numChannels; c++) {
      const sample = Math.max(-1, Math.min(1, channels[c][i]));
      const int16 = sample < 0 ? sample * 0x8000 : sample * 0x7FFF;
      view.setInt16(offset, int16, true);
      offset += 2;
    }
  }
  return buffer;
}
export default function AudioEditor({
  filePath,
  fileName,
  onClose,
  showStatus
}: AudioEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const wavesurferRef = useRef<any>(null);
  const regionsRef = useRef<any>(null);
  const [loading, setLoading] = useState(true);
  const [playing, setPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [hasRegion, setHasRegion] = useState(false);
  const [regionStart, setRegionStart] = useState(0);
  const [regionEnd, setRegionEnd] = useState(0);
  const [saving, setSaving] = useState(false);
  const [unsaved, setUnsaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fileUrl = convertFileSrc(filePath);
  const formatTime = (s: number) => {
    const m = Math.floor(s / 60);
    const sec = Math.floor(s % 60);
    return `${m}:${sec.toString().padStart(2, '0')}`;
  };
  useEffect(() => {
    let ws: any = null;
    async function init() {
      setLoading(true);
      setError(null);
      try {
        const WaveSurfer = (await import('wavesurfer.js')).default;
        const RegionsPlugin = (await import('wavesurfer.js/dist/plugins/regions.esm.js')).default;
        if (!containerRef.current) return;
        const regions = RegionsPlugin.create();
        regionsRef.current = regions;
        ws = WaveSurfer.create({
          container: containerRef.current,
          url: fileUrl,
          waveColor: '#2a2a3a',
          progressColor: '#FF6B6B',
          cursorColor: '#FFD700',
          cursorWidth: 1,
          height: 160,
          barWidth: 2,
          barGap: 1,
          barRadius: 2,
          barAlign: 'bottom'
        });
        ws.registerPlugin(regions);
        wavesurferRef.current = ws;
        ws.on('ready', () => {
          setLoading(false);
          setDuration(ws.getDuration());
          regions.addRegion({
            start: 0,
            end: ws.getDuration(),
            color: 'rgba(255, 107, 107, 0.2)',
            drag: true,
            resize: true
          });
          setHasRegion(true);
          setRegionStart(0);
          setRegionEnd(ws.getDuration());
        });
        ws.on('play', () => setPlaying(true));
        ws.on('pause', () => setPlaying(false));
        ws.on('timeupdate', (t: number) => setCurrentTime(t));
        ws.on('finish', () => setPlaying(false));
        ws.on('error', (err: any) => {
          setError(err?.message || t("components.AudioEditor.k1"));
          setLoading(false);
        });
        regions.on('region-updated', (region: any) => {
          setRegionStart(region.start);
          setRegionEnd(region.end);
          setUnsaved(true);
        });
      } catch (err: any) {
        setError(err?.message || t("components.AudioEditor.k2"));
        setLoading(false);
      }
    }
    init();
    return () => {
      if (ws) {
        try {
          ws.destroy();
        } catch (_) {}
      }
      wavesurferRef.current = null;
      regionsRef.current = null;
    };
  }, [fileUrl]);
  const handlePlayPause = useCallback(() => {
    if (!wavesurferRef.current) return;
    wavesurferRef.current.playPause();
  }, []);
  const handleSave = useCallback(async () => {
    const ws = wavesurferRef.current;
    if (!ws) return;
    setSaving(true);
    try {
      const response = await fetch(fileUrl);
      const arrayBuffer = await response.arrayBuffer();
      const audioCtx = new AudioContext();
      const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer);
      const sampleRate = audioBuffer.sampleRate;
      const startSample = Math.floor(regionStart * sampleRate);
      const endSample = Math.floor(regionEnd * sampleRate);
      const newLength = endSample - startSample;
      const numChannels = audioBuffer.numberOfChannels;
      const trimmedBuffer = audioCtx.createBuffer(numChannels, newLength, sampleRate);
      for (let c = 0; c < numChannels; c++) {
        const srcData = audioBuffer.getChannelData(c);
        const dstData = trimmedBuffer.getChannelData(c);
        dstData.set(srcData.subarray(startSample, endSample));
      }
      await audioCtx.close();
      const wavBuffer = encodeWav(trimmedBuffer);
      const bytes = new Uint8Array(wavBuffer);
      let base64 = '';
      for (let i = 0; i < bytes.length; i++) {
        base64 += String.fromCharCode(bytes[i]);
      }
      base64 = btoa(base64);
      const {
        invoke
      } = await import('@tauri-apps/api/core');
      await invoke('audioedit_save', {
        path: filePath,
        base64_data: base64
      });
      setUnsaved(false);
      showStatus('success', t("components.AudioEditor.k3"));
    } catch (e: any) {
      showStatus('error', e?.toString() || t("errors.saveFailed"));
    } finally {
      setSaving(false);
    }
  }, [fileUrl, filePath, regionStart, regionEnd, showStatus]);
  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key === 's') {
        e.preventDefault();
        handleSave();
      } else if (e.key === ' ') {
        e.preventDefault();
        handlePlayPause();
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, [handleSave, handlePlayPause]);
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
        }}>{t("components.AudioEditor.k4")}</span>
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
      flex: 1,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      padding: '20px 40px',
      gap: 16
    }}>
        {error ? <div style={{
        color: '#FF6B6B',
        textAlign: 'center'
      }}>
            <p>{t("errors.loadFailed")}</p>
            <p style={{
          fontSize: 12,
          color: '#888'
        }}>{error}</p>
            <p style={{
          fontSize: 11,
          color: '#555'
        }}>{t("components.AudioEditor.k8")}</p>
          </div> : <>
            <div style={{
          display: 'flex',
          alignItems: 'center',
          gap: 12
        }}>
              <button onClick={handlePlayPause} disabled={loading} style={{
            width: 44,
            height: 44,
            borderRadius: '50%',
            border: 'none',
            background: loading ? '#222' : '#FF6B6B',
            color: '#fff',
            cursor: loading ? 'not-allowed' : 'pointer',
            fontSize: 18,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center'
          }}>
                {playing ? '⏸' : '▶'}
              </button>
              <div style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 2
          }}>
                <span style={{
              color: '#aaa',
              fontSize: 14,
              fontFamily: 'monospace'
            }}>
                  {formatTime(currentTime)} / {formatTime(duration)}
                </span>
                {hasRegion && <span style={{
              color: '#FF6B6B',
              fontSize: 11,
              fontFamily: 'monospace'
            }}>
                    {t("components.AudioEditor.k9")} {formatTime(regionStart)} ~ {formatTime(regionEnd)}
                    <span style={{
                color: '#888'
              }}> ({formatTime(regionEnd - regionStart)})</span>
                  </span>}
              </div>
            </div>

            <div ref={containerRef} style={{
          width: '100%',
          maxWidth: 800,
          height: 160,
          background: '#0d0d14',
          borderRadius: 8,
          border: '1px solid #222',
          opacity: loading ? 0.3 : 1,
          transition: 'opacity 0.3s'
        }} />

            <div style={{
          color: '#555',
          fontSize: 11,
          textAlign: 'center',
          maxWidth: 500,
          lineHeight: 1.5
        }}>
              {t("components.AudioEditor.k10")}
            </div>
          </>}
      </div>
    </div>;
}