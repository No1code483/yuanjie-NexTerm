/**
 * D3.6 实时对话会话 Hook
 *
 * 职责：
 *   1. 启动/停止实时会话（调用后端 xin_realtime_start/stop）
 *   2. 通过 Web Audio API 采集麦克风 PCM 16kHz 单声道
 *   3. 每 ~32ms 自动推送 PCM 块到后端（xin_realtime_push_chunk）
 *   4. 暴露当前会话状态给 UI
 *
 * 架构说明：
 *   - 采样率：强制 AudioContext sampleRate=16000（Whisper 推荐）
 *   - 缓冲：ScriptProcessorNode bufferSize=512（32ms @ 16kHz）
 *   - 格式：Float32 → Int16 转换后推送
 *   - 注：ScriptProcessorNode 已废弃但兼容性好；后续优化可换 AudioWorklet
 *     （需单独 worklet .js 文件 + Vite 打包配置）
 *
 * 规范：功能展望/模块深化/03_小欣_多模态融合_深度.md §2.5
 */

import { useState, useRef, useCallback, useEffect } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { realtime, type RealtimeConfig, type RealtimeState } from '@/lib/ipc';

/**
 * 后端推送的实时事件类型（对齐 xin_realtime_service.rs RealtimeEvent）
 */
type RealtimeEvent =
  | { type: 'state_changed'; state: RealtimeState }
  | { type: 'stt_increment'; delta: string; full_text: string; is_final: boolean }
  | { type: 'llm_chunk'; delta: string; full_text: string; done: boolean }
  | { type: 'tts_chunk'; audio_b64: string; format: string; sample_rate: number; is_final: boolean }
  | { type: 'error'; message: string; code: string }
  | { type: 'session_ended' };

interface UseRealtimeSessionReturn {
  /** 当前会话状态 */
  state: RealtimeState;
  /** 是否正在录音（非 idle 状态） */
  isActive: boolean;
  /** 累积 STT 转写文本 */
  sttText: string;
  /** 累积 LLM 回复文本 */
  llmText: string;
  /** 启动实时会话 */
  start: (config: RealtimeConfig) => Promise<void>;
  /** 停止实时会话 */
  stop: () => Promise<void>;
  /** 错误信息（启动失败等） */
  error: string | null;
}

export function useRealtimeSession(): UseRealtimeSessionReturn {
  const [state, setState] = useState<RealtimeState>('idle');
  const [error, setError] = useState<string | null>(null);
  const [sttText, setSttText] = useState<string>('');
  const [llmText, setLlmText] = useState<string>('');

  const audioContextRef = useRef<AudioContext | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const sourceNodeRef = useRef<MediaStreamAudioSourceNode | null>(null);
  const processorNodeRef = useRef<ScriptProcessorNode | null>(null);
  const timestampStartRef = useRef<number>(0);
  const sampleCounterRef = useRef<number>(0);
  const activeRef = useRef<boolean>(false);
  const unlistenRef = useRef<UnlistenFn | null>(null);
  // D3.6.5 TTS 音频播放队列（顺序播放分句音频块，避免重叠）
  const audioQueueRef = useRef<string[]>([]);
  const isPlayingAudioRef = useRef<boolean>(false);

  /**
   * 将 Float32 样本转换为 Int16
   * Web Audio 原生 Float32 [-1.0, 1.0]，PCM 16-bit 范围 [-32768, 32767]
   */
  const float32ToInt16 = (float32: Float32Array): Int16Array => {
    const int16 = new Int16Array(float32.length);
    for (let i = 0; i < float32.length; i++) {
      const clamped = Math.max(-1, Math.min(1, float32[i]));
      int16[i] = clamped < 0 ? clamped * 0x8000 : clamped * 0x7fff;
    }
    return int16;
  };

  /**
   * 推送 PCM 块到后端
   * 错误静默处理（避免单次推送失败中断整个会话）
   */
  const pushChunk = async (samples: Int16Array) => {
    if (!activeRef.current) return;
    const timestampMs = Date.now() - timestampStartRef.current;
    // Int16Array → number[]（IPC 序列化需要普通数组）
    const samplesArr = Array.from(samples);
    try {
      const result = await realtime.pushChunk(samplesArr, timestampMs);
      if (result?.data) {
        setState(result.data);
      }
    } catch (e) {
      // 单次推送失败不中断，仅日志
      console.warn('[realtime] push_chunk 失败:', e);
    }
  };

  /**
   * D3.6.5 顺序播放音频队列
   *
   * 设计：后端按句子分块调用云端 TTS，每句一个 TtsChunk 事件。
   * 前端用队列保证句子顺序播放，避免后句覆盖前句。
   * onended 触发下一句；onerror 跳过当前句继续播放下一句。
   */
  const playAudioQueue = () => {
    if (audioQueueRef.current.length === 0) {
      isPlayingAudioRef.current = false;
      return;
    }
    isPlayingAudioRef.current = true;
    const url = audioQueueRef.current.shift()!;
    const audio = new Audio(url);
    audio.onended = () => {
      URL.revokeObjectURL(url);
      playAudioQueue();
    };
    audio.onerror = () => {
      URL.revokeObjectURL(url);
      console.warn('[realtime] TTS 音频块播放失败，跳过');
      playAudioQueue();
    };
    void audio.play().catch((e) => {
      URL.revokeObjectURL(url);
      console.warn('[realtime] TTS 播放启动失败:', e);
      playAudioQueue();
    });
  };

  /**
   * D3.6.5 将 base64 音频块入队并触发播放
   *
   * 流程：base64 → Uint8Array → Blob → ObjectURL → 入队
   * 若当前未在播放，则立即启动队列播放。
   */
  const enqueueAudioChunk = (audioB64: string, format: string) => {
    try {
      const binaryStr = atob(audioB64);
      const bytes = new Uint8Array(binaryStr.length);
      for (let i = 0; i < binaryStr.length; i++) {
        bytes[i] = binaryStr.charCodeAt(i);
      }
      const mimeType = format === 'wav' ? 'audio/wav' : 'audio/mpeg';
      const blob = new Blob([bytes], { type: mimeType });
      const url = URL.createObjectURL(blob);
      audioQueueRef.current.push(url);
      if (!isPlayingAudioRef.current) {
        playAudioQueue();
      }
    } catch (e) {
      console.warn('[realtime] TTS 音频块解码失败:', e);
    }
  };

  /**
   * 启动实时会话
   * 流程：调用后端 start → 获取麦克风 → 创建 AudioContext(16kHz) → 接 ScriptProcessor → 开始推送
   */
  const start = useCallback(async (config: RealtimeConfig) => {
    if (activeRef.current) {
      console.warn('[realtime] 会话已在进行中');
      return;
    }
    setError(null);
    try {
      // 1. 启动后端会话
      const startResult = await realtime.start(config);
      if (!startResult?.data) {
        throw new Error(startResult?.message || '后端启动会话失败');
      }
      setState(startResult.data);
      activeRef.current = true;
      timestampStartRef.current = Date.now();
      sampleCounterRef.current = 0;

      // 1.5 监听后端 realtime_event 事件（状态变更 / STT 增量 / LLM 流式 / TTS 音频块）
      unlistenRef.current = await listen<RealtimeEvent>('realtime_event', (event) => {
        const payload = event.payload;
        if (!payload) return;
        switch (payload.type) {
          case 'state_changed':
            setState(payload.state);
            break;
          case 'stt_increment':
            // full_text 是后端累积的全部转写文本
            setSttText(payload.full_text);
            break;
          case 'llm_chunk':
            setLlmText(payload.full_text);
            break;
          case 'error':
            setError(payload.message);
            console.error('[realtime] 后端错误:', payload.code, payload.message);
            break;
          case 'session_ended':
            setState('idle');
            break;
          case 'tts_chunk':
            // D3.6.5: TTS 音频块入队播放
            if (payload.is_final) {
              console.info('[realtime] TTS 播放完成（is_final）');
            } else if (payload.audio_b64) {
              enqueueAudioChunk(payload.audio_b64, payload.format || 'mp3');
            }
            break;
        }
      });

      // 2. 获取麦克风
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          channelCount: 1,
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true
        }
      });
      mediaStreamRef.current = stream;

      // 3. 创建 AudioContext（强制 16kHz）
      const AudioContextCtor = window.AudioContext || (window as any).webkitAudioContext;
      const audioContext = new AudioContextCtor({ sampleRate: 16000 });
      audioContextRef.current = audioContext;

      // 4. 创建处理节点
      const source = audioContext.createMediaStreamSource(stream);
      sourceNodeRef.current = source;
      // bufferSize=512 → 32ms @ 16kHz；ScriptProcessorNode 已废弃但兼容性好
      const processor = audioContext.createScriptProcessor(512, 1, 1);
      processorNodeRef.current = processor;

      processor.onaudioprocess = (event: AudioProcessingEvent) => {
        if (!activeRef.current) return;
        const inputBuffer = event.inputBuffer;
        const float32 = inputBuffer.getChannelData(0);
        // 复制一份避免引用问题
        const copy = new Float32Array(float32.length);
        copy.set(float32);
        const int16 = float32ToInt16(copy);
        void pushChunk(int16);
        sampleCounterRef.current += int16.length;
      };

      source.connect(processor);
      // ScriptProcessorNode 需要连接到 destination 才能触发 onaudioprocess
      // 但我们不希望听到自己的声音，所以用 gain=0 的中间节点
      const silentGain = audioContext.createGain();
      silentGain.gain.value = 0;
      processor.connect(silentGain);
      silentGain.connect(audioContext.destination);

      console.info('[realtime] 实时会话已启动: 16kHz 单声道, 32ms/块');
    } catch (e: any) {
      console.error('[realtime] 启动失败:', e);
      setError(e?.message || String(e));
      // 清理已创建的资源
      await stopInternal();
    }
  }, []);

  /**
   * 内部停止逻辑（不抛错，幂等）
   */
  const stopInternal = async () => {
    activeRef.current = false;
    // D3.6.5 清理 TTS 音频队列：释放待播放的 ObjectURL，避免内存泄漏
    isPlayingAudioRef.current = false;
    while (audioQueueRef.current.length > 0) {
      const url = audioQueueRef.current.shift();
      if (url) URL.revokeObjectURL(url);
    }
    // 取消后端事件监听
    if (unlistenRef.current) {
      unlistenRef.current();
      unlistenRef.current = null;
    }
    if (processorNodeRef.current) {
      processorNodeRef.current.disconnect();
      processorNodeRef.current.onaudioprocess = null;
      processorNodeRef.current = null;
    }
    if (sourceNodeRef.current) {
      sourceNodeRef.current.disconnect();
      sourceNodeRef.current = null;
    }
    if (mediaStreamRef.current) {
      mediaStreamRef.current.getTracks().forEach(t => t.stop());
      mediaStreamRef.current = null;
    }
    if (audioContextRef.current && audioContextRef.current.state !== 'closed') {
      try {
        await audioContextRef.current.close();
      } catch {/* silent */}
    }
    audioContextRef.current = null;
  };

  /**
   * 停止实时会话
   */
  const stop = useCallback(async () => {
    await stopInternal();
    try {
      await realtime.stop();
    } catch (e) {
      console.warn('[realtime] 后端停止失败:', e);
    }
    setState('idle');
    setSttText('');
    setLlmText('');
    setError(null);
  }, []);

  // 组件卸载时清理
  useEffect(() => {
    return () => {
      void stopInternal();
    };
  }, []);

  return {
    state,
    isActive: state !== 'idle',
    sttText,
    llmText,
    start,
    stop,
    error
  };
}
