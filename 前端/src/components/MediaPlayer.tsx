import { t } from "i18next";
import { useRef, useState, useCallback, useEffect } from 'react';
interface MediaPlayerProps {
  src: string;
  mimeType: string;
  fileName: string;
  className?: string;
}
const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 2];
export default function MediaPlayer({
  src,
  mimeType,
  fileName,
  className
}: MediaPlayerProps) {
  const mediaRef = useRef<HTMLVideoElement | HTMLAudioElement>(null);
  const isVideo = mimeType.startsWith('video/');
  const [playing, setPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [volume, setVolume] = useState(1);
  const [muted, setMuted] = useState(false);
  const [speed, setSpeed] = useState(1);
  const [showSpeedMenu, setShowSpeedMenu] = useState(false);
  useEffect(() => {
    const el = mediaRef.current;
    if (!el) return;
    const onTimeUpdate = () => setCurrentTime(el.currentTime);
    const onLoadedMeta = () => setDuration(el.duration);
    const onPlay = () => setPlaying(true);
    const onPause = () => setPlaying(false);
    const onEnded = () => setPlaying(false);
    el.addEventListener('timeupdate', onTimeUpdate);
    el.addEventListener('loadedmetadata', onLoadedMeta);
    el.addEventListener('play', onPlay);
    el.addEventListener('pause', onPause);
    el.addEventListener('ended', onEnded);
    return () => {
      el.removeEventListener('timeupdate', onTimeUpdate);
      el.removeEventListener('loadedmetadata', onLoadedMeta);
      el.removeEventListener('play', onPlay);
      el.removeEventListener('pause', onPause);
      el.removeEventListener('ended', onEnded);
    };
  }, [src]);
  const togglePlay = useCallback(() => {
    const el = mediaRef.current;
    if (!el) return;
    if (el.paused) el.play();else el.pause();
  }, []);
  const handleSeek = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const el = mediaRef.current;
    if (!el) return;
    const t = parseFloat(e.target.value);
    el.currentTime = t;
    setCurrentTime(t);
  }, []);
  const handleVolume = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const el = mediaRef.current;
    if (!el) return;
    const v = parseFloat(e.target.value);
    el.volume = v;
    setVolume(v);
    setMuted(v === 0);
  }, []);
  const toggleMute = useCallback(() => {
    const el = mediaRef.current;
    if (!el) return;
    el.muted = !el.muted;
    setMuted(el.muted);
  }, []);
  const setPlaybackSpeed = useCallback((s: number) => {
    const el = mediaRef.current;
    if (!el) return;
    el.playbackRate = s;
    setSpeed(s);
    setShowSpeedMenu(false);
  }, []);
  const requestPiP = useCallback(async () => {
    const el = mediaRef.current as HTMLVideoElement | null;
    if (!el || !isVideo) return;
    try {
      if (document.pictureInPictureElement) {
        await document.exitPictureInPicture();
      } else {
        await el.requestPictureInPicture();
      }
    } catch {/* PiP not supported */}
  }, [isVideo]);
  const formatTime = (t: number) => {
    if (!isFinite(t) || t < 0) return '0:00';
    const m = Math.floor(t / 60);
    const s = Math.floor(t % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  };
  return <div className={className} style={{
    display: 'flex',
    flexDirection: 'column',
    height: '100%',
    background: '#1a1a2a'
  }}>
      {isVideo ? <video ref={mediaRef as React.Ref<HTMLVideoElement>} src={src} autoPlay onClick={togglePlay} style={{
      flex: 1,
      width: '100%',
      objectFit: 'contain',
      background: '#000'
    }} /> : <div style={{
      flex: 1,
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 16
    }}>
          <div style={{
        fontSize: 64
      }}>🎵</div>
          <div style={{
        color: '#ccc',
        fontSize: 16,
        fontWeight: 500
      }}>{fileName}</div>
          <audio ref={mediaRef as React.Ref<HTMLAudioElement>} src={src} autoPlay />
        </div>}

      <div style={{
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '8px 12px',
      background: 'rgba(20, 20, 30, 0.95)',
      borderTop: '1px solid rgba(255,255,255,0.06)'
    }}>
        <button onClick={togglePlay} style={controlBtnStyle()}>
          {playing ? '⏸' : '▶'}
        </button>

        <span style={{
        color: '#aaa',
        fontSize: 11,
        fontVariantNumeric: 'tabular-nums',
        minWidth: 36,
        textAlign: 'right'
      }}>
          {formatTime(currentTime)}
        </span>

        <input type="range" min={0} max={duration || 0} step={0.1} value={currentTime} onChange={handleSeek} style={{
        flex: 1,
        height: 4,
        accentColor: '#00F0FF',
        cursor: 'pointer'
      }} />

        <span style={{
        color: '#aaa',
        fontSize: 11,
        fontVariantNumeric: 'tabular-nums',
        minWidth: 36
      }}>
          {formatTime(duration)}
        </span>

        <button onClick={toggleMute} style={controlBtnStyle()}>
          {muted || volume === 0 ? '🔇' : volume < 0.5 ? '🔉' : '🔊'}
        </button>

        <input type="range" min={0} max={1} step={0.05} value={muted ? 0 : volume} onChange={handleVolume} style={{
        width: 80,
        height: 4,
        accentColor: '#00F0FF',
        cursor: 'pointer'
      }} />

        <div style={{
        position: 'relative'
      }}>
          <button onClick={() => setShowSpeedMenu(v => !v)} style={{
          ...controlBtnStyle,
          background: speed !== 1 ? 'rgba(0,240,255,0.12)' : 'rgba(255,255,255,0.06)',
          border: speed !== 1 ? '1px solid rgba(0,240,255,0.2)' : '1px solid rgba(255,255,255,0.08)',
          color: speed !== 1 ? '#00F0FF' : '#bbb',
          fontSize: 11,
          fontWeight: 500
        }}>
            {speed}x
          </button>
          {showSpeedMenu && <div style={{
          position: 'absolute',
          bottom: '100%',
          right: 0,
          marginBottom: 4,
          background: 'rgba(20,20,30,0.98)',
          border: '1px solid rgba(255,255,255,0.1)',
          borderRadius: 6,
          padding: '4px 0',
          minWidth: 72,
          zIndex: 10
        }}>
              {SPEEDS.map(s => <div key={s} onClick={() => setPlaybackSpeed(s)} style={{
            padding: '4px 14px',
            cursor: 'pointer',
            color: speed === s ? '#00F0FF' : '#ccc',
            fontSize: 12,
            background: speed === s ? 'rgba(0,240,255,0.1)' : 'transparent'
          }}>
                  {s}x
                </div>)}
            </div>}
        </div>

        {isVideo && <button onClick={requestPiP} title={t("components.MediaPlayer.k1")} style={controlBtnStyle()}>
            🖼
          </button>}
      </div>
    </div>;
}
function controlBtnStyle(): React.CSSProperties {
  return {
    background: 'rgba(255,255,255,0.06)',
    border: '1px solid rgba(255,255,255,0.08)',
    color: '#bbb',
    cursor: 'pointer',
    padding: '4px 8px',
    borderRadius: 4,
    fontSize: 13,
    lineHeight: 1,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center'
  };
}