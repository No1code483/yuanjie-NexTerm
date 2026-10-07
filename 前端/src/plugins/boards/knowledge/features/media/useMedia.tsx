// knowledge.media L2 功能域：媒体查看器（内联预览 + 全屏浮层 + Office/PDF/表格/PPTX 解析）。
// 局部 state + handlers + JSX 物理落在本目录；核心共享数据经 KnowledgeCore 注入。
import { t } from "i18next";
import { useEffect, useRef, useState } from 'react';
import { intelligence } from '@/lib/ipc';
import { kb } from '../../ipc';
import { convertFileSrc } from '@tauri-apps/api/core';
import { PptxViewer } from '@aiden0z/pptx-renderer';
import mammoth from 'mammoth';
import MediaPlayer from '@/components/MediaPlayer';
import ImageViewer from '@/components/ImageViewer';
import PdfViewer from '@/components/PdfViewer';
import { systemtools } from '@/plugins/customs/systemtools';
import { sanitizeHtml } from '@/utils/sanitize';
import { excelColName } from '../../knowledge/utils';
import styles from '../../Knowledge.module.css';
import type { KbEntry, KnowledgeCore } from '../../knowledge/types';

interface EditorsApi {
  fileEditMode: boolean;
  editSupportedExts: Set<string>;
  handleStartFileEdit: (entry: KbEntry) => void;
  handleStopFileEdit: () => void;
}

export function useMedia(core: KnowledgeCore) {
  const [mediaViewerEntry, setMediaViewerEntry] = useState<KbEntry | null>(null);
  const [mediaViewerBase64, setMediaViewerBase64] = useState('');
  const [mediaViewerMime, setMediaViewerMime] = useState('');
  const [mediaViewerTextContent, setMediaViewerTextContent] = useState('');
  const [mediaViewerHtmlContent, setMediaViewerHtmlContent] = useState('');
  const [mediaViewerLoading, setMediaViewerLoading] = useState(false);
  const [mediaViewerError, setMediaViewerError] = useState<string | null>(null);
  const [mediaViewerFullscreen, setMediaViewerFullscreen] = useState(false);
  const [mediaViewerTextColor, setMediaViewerTextColor] = useState('#E0E0E0');
  const [mediaViewerFileUrl, setMediaViewerFileUrl] = useState('');
  const [tableData, setTableData] = useState<{
    headers: string[];
    rows: string[][];
    totalCols: number;
  } | null>(null);
  const [pptxSlideCount, setPptxSlideCount] = useState(0);
  const [pptxCurrentSlide, setPptxCurrentSlide] = useState(0);
  const pptxViewerRef = useRef<any>(null);
  const pptxSlideRef = useRef<HTMLDivElement>(null);
  const pptxFullscreenSlideRef = useRef<HTMLDivElement>(null);
  const pptxArrayBufRef = useRef<ArrayBuffer | null>(null);

  const open = async (entry: KbEntry) => {
    kb.recordKbAccess({ entryId: entry.id }).catch(() => {});
    // 记录活动日志：打开知识库文件
    intelligence.logActivity('1', new Date().toISOString().replace('T', ' ').slice(0, 19), 'knowledge', t("components.intelligence.ActivityPanel.k13"), `name:${entry.name},path_url:${entry.path_url},source_path:${entry.source_path || ''}`).catch(() => {});
    setMediaViewerEntry(entry);
    setMediaViewerBase64('');
    setMediaViewerMime('');
    setMediaViewerTextContent('');
    setMediaViewerHtmlContent('');
    setMediaViewerError(null);
    setMediaViewerLoading(true);
    setMediaViewerFullscreen(false);
    setTableData(null);
    let url = entry.path_url || '';
    // A5 Phase 3 Task 4.5: 离线时使用本地缓存路径（仅对已标记为「常用」的附件）
    if (!core.isOnline && core.pinnedEntries.has(entry.id)) {
      try {
        const cachedPathRes = await kb.kbAttachmentGetCachedPath({ entryId: entry.id });
        if (cachedPathRes.code === 0 && cachedPathRes.data) {
          url = cachedPathRes.data;
        }
      } catch {/* 缓存路径获取失败，使用原始路径 */}
    }
    const ext = url.split('.').pop()?.toLowerCase() || '';
    const isInternal = !url || url.startsWith('kb://');
    if (['f4v', 'flv'].includes(ext)) {
      setMediaViewerError(t("Knowledge.k67"));
      setMediaViewerLoading(false);
      return;
    }
    const docxExts = ['docx', 'odt'];
    const epubExts = ['epub'];
    const rtfExts = ['rtf'];
    const legacyDocExts = ['doc'];
    const pptExts = ['pptx', 'ppt'];
    const odpExts = ['odp'];
    const archiveExts = ['zip', 'rar'];
    const designExts = ['psd', 'ai'];
    const tableExts = ['xlsx', 'xls', 'ods'];
    const textExts = [
      // 标记语言
      'md', 'markdown', 'mdown', 'mkd', 'mkdn', 'rst', 'rest', 'restructuredtext', 'asciidoc', 'adoc', 'textile', 'pod', 'org', 'wiki', 'mediawiki', 'creole', 'typ', 'tex', 'latex', 'ltx', 'sty', 'cls', 'bib', 'bibtex', 'nfo', 'diz',
      // 纯文本
      'txt', 'text', 'log', 'csv', 'tsv', 'psv', 'srt', 'vtt', 'ass', 'ssa', 'sub', 'smi', 'lrc',
      // 数据格式
      'json', 'jsonc', 'json5', 'xml', 'xaml', 'xsl', 'xslt', 'xsd', 'yaml', 'yml', 'toml', 'cfg', 'conf', 'ini', 'inf', 'cnf', 'env', 'envrc', 'properties', 'prop', 'lock', 'editorconfig', 'gitignore', 'gitattributes', 'gitmodules', 'dockerignore', 'npmrc', 'yarnrc',
      // 前端
      'html', 'htm', 'xhtml', 'shtml', 'css', 'scss', 'sass', 'less', 'styl', 'js', 'jsx', 'mjs', 'cjs', 'ts', 'tsx', 'vue', 'svelte', 'astro',
      // 后端
      'py', 'pyw', 'pyx', 'rs', 'rlib', 'go', 'java', 'kt', 'kts', 'scala', 'sc', 'groovy', 'gvy', 'gy', 'gradle', 'php', 'phtml', 'php3', 'php4', 'php5', 'phps', 'phpt', 'rb', 'rbw', 'rake', 'gemspec', 'pl', 'pm', 'pod', 't', 'swift', 'dart', 'jl', 'lua', 'r', 'R', 'rprofile', 'rmd', 'rnw',
      // 函数式
      'erl', 'hrl', 'ex', 'exs', 'eex', 'leex', 'heex', 'hs', 'lhs', 'ml', 'mli', 'clj', 'cljs', 'cljc', 'edn', 'elm', 'fs', 'fsx', 'fsi', 'fsscript', 'nim', 'nims', 'zig',
      // 系统/脚本
      'sh', 'bash', 'zsh', 'fish', 'bat', 'cmd', 'ps1', 'psm1', 'psd1', 'makefile', 'mk', 'dockerfile', 'containerfile', 'cmake', 'cmake.in',
      // 数据库/查询
      'sql', 'psql', 'mysql', 'hql', 'prql', 'sparql', 'rq', 'graphql', 'gql', 'ttl', 'nt', 'n3', 'rdf', 'owl', 'cypher', 'cql',
      // 协议/接口
      'proto', 'protobuf', 'thrift', 'avsc', 'avdl', 'wsdl', 'wadl', 'raml', 'oas', 'openapi', 'grpc', 'cue', 'dhall', 'nix', 'tf', 'tfvars', 'hcl', 'nomad', 'sentinel', 'smel', 'smithy',
      // 模板
      'handlebars', 'hbs', 'hbrs', 'mustache', 'ejs', 'ect', 'pug', 'jade', 'twig', 'jinja', 'jinja2', 'j2', 'liquid', 'njk', 'nunjucks', 'dust', 'haml', 'slim', 'erb', 'rhtml', 'volt', 'latte', 'blade', 'blade.php', 'mjml',
      // 其他文本
      'patch', 'diff', 'rej', 'ron', 'eml', 'mbox', 'vcard', 'vcf', 'ics', 'ical', 'ifb', 'icalendar', 'prisma', 'coffee', 'cson', 'iced', 'litcoffee', 'peg', 'pegjs', 'ohm', 'arvo', 'abc', 'ly', 'ily', 'rnc', 'rng', 'schemas', 'dtd', 'sgml', 'sgm', 'ent', 'mod', 'g4', 'ebnf', 'bnf', 'abnf', 'pest', 'pomsky', 'nearley', 'wast', 'wat', 'asm', 's', 'S', 'sage', 'sagews', 'm', 'mat', 'octave', 'matlab', 'mma', 'nb', 'wl', 'wls', 'cirru', 'idr', 'lidr', 'agda', 'lagda', 'v', 'vhdl', 'vhd', 'sv', 'svh', 'bsv', 'a51', 'ispc', 'opencl', 'cl', 'cuh', 'metal', 'wgsl', 'glsl', 'vert', 'frag', 'tesc', 'tese', 'geom', 'comp', 'hlsl', 'fx', 'fxh', 'vsh', 'psh', 'semgrep', 'ftl', 'fxml', 'mxml', 'xib', 'storyboard', 'plist', 'strings', 'dic', 'aff', 'po', 'pot', 'mo', 'lang', 'resx', 'resw', 'resjson', 'xliff', 'xlf', 'arb'
    ];
    const mimeMap: Record<string, string> = {
      pdf: 'application/pdf',
      mp4: 'video/mp4',
      webm: 'video/webm',
      ogg: 'video/ogg',
      f4v: 'video/mp4',
      flv: 'video/x-flv',
      mp3: 'audio/mpeg',
      wav: 'audio/wav',
      oga: 'audio/ogg',
      png: 'image/png',
      jpg: 'image/jpeg',
      jpeg: 'image/jpeg',
      gif: 'image/gif',
      webp: 'image/webp',
      svg: 'image/svg+xml',
      bmp: 'image/bmp',
      ico: 'image/x-icon',
      tiff: 'image/tiff',
      tif: 'image/tiff',
      avif: 'image/avif',
      heic: 'image/heic',
      heif: 'image/heif',
      avi: 'video/x-msvideo',
      mkv: 'video/x-matroska',
      mov: 'video/quicktime',
      wmv: 'video/x-ms-wmv',
      m4v: 'video/mp4',
      '3gp': 'video/3gpp',
      flac: 'audio/flac',
      aac: 'audio/aac',
      wma: 'audio/x-ms-wma',
      m4a: 'audio/mp4',
      opus: 'audio/opus',
      aiff: 'audio/aiff'
    };
    try {
      if (isInternal || entry.entry_type === 'text' || textExts.includes(ext)) {
        let text = entry.content || '';
        if (!text && !isInternal) {
          const readRes = await kb.kbReadExternalFile({ path: url });
          if (readRes.code === 0 && readRes.data) {
            text = readRes.data.content;
          } else {
            setMediaViewerError(readRes.message || t("Knowledge.k60"));
            return;
          }
        }
        if (!text.trim()) {
          setMediaViewerTextContent('');
          setMediaViewerMime('text/plain');
          setMediaViewerLoading(false);
          return;
        }
        setMediaViewerTextContent(text);
        setMediaViewerMime('text/plain');
      } else if (tableExts.includes(ext)) {
        const res = await kb.kbExtractTableData({ path: url });
        if (res.code === 0 && res.data) {
          setTableData({ headers: res.data.headers, rows: res.data.rows, totalCols: res.data.total_cols });
          setMediaViewerMime('table');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k68"));
        }
      } else if (pptExts.includes(ext)) {
        const readRes = await kb.kbReadFileBase64({ path: url });
        if (readRes.code === 0 && readRes.data?.base64) {
          const binaryStr = atob(readRes.data.base64);
          const bytes = new Uint8Array(binaryStr.length);
          for (let i = 0; i < binaryStr.length; i++) {
            bytes[i] = binaryStr.charCodeAt(i);
          }
          const arrayBuffer = bytes.buffer;
          pptxArrayBufRef.current = arrayBuffer;
          if (pptxViewerRef.current) {
            try {
              pptxViewerRef.current.destroy();
            } catch (_) {}
            pptxViewerRef.current = null;
          }
          setPptxSlideCount(0);
          setPptxCurrentSlide(0);
          setMediaViewerMime('pptx');
          setMediaViewerLoading(true);
          requestAnimationFrame(() => {
            const container = mediaViewerFullscreen ? pptxFullscreenSlideRef.current : pptxSlideRef.current;
            if (!container) {
              setMediaViewerError(t("Knowledge.k69"));
              return;
            }
            container.replaceChildren();
            PptxViewer.open(arrayBuffer, container, {
              renderMode: 'slide',
              fitMode: 'contain',
              onSlideChange: (index: number) => setPptxCurrentSlide(index)
            }).then((viewer: any) => {
              pptxViewerRef.current = viewer;
              setPptxSlideCount(viewer.slideCount || 1);
              setPptxCurrentSlide(0);
              setMediaViewerLoading(false);
            }).catch((err: Error) => {
              console.error('[pptx] PptxViewer.open error:', err);
              setMediaViewerError(t("Knowledge.k70", { message: err.message }));
              setMediaViewerLoading(false);
            });
          });
        } else {
          setMediaViewerError(readRes.message || t("Knowledge.k71"));
        }
      } else if (odpExts.includes(ext)) {
        const res = await kb.kbExtractOdpText({ path: url });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k72"));
        }
      } else if (docxExts.includes(ext)) {
        // 使用 mammoth.js 将 .docx 转为 HTML 渲染（保留图片、表格、格式）
        try {
          const res = await kb.kbReadFileBase64({ path: url });
          if (res.code === 0 && res.data && res.data.base64) {
            const binaryStr = atob(res.data.base64);
            const bytes = new Uint8Array(binaryStr.length);
            for (let i = 0; i < binaryStr.length; i++) {
              bytes[i] = binaryStr.charCodeAt(i);
            }
            const result = await mammoth.convertToHtml({ arrayBuffer: bytes.buffer });
            const htmlContent = result.value;
            // 添加基础样式
            const styledHtml = `
              <style>
                .mammoth-doc { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; color: #E0E0E0; line-height: 1.8; padding: 8px; }
                .mammoth-doc h1, .mammoth-doc h2, .mammoth-doc h3 { margin: 16px 0 8px; }
                .mammoth-doc p { margin: 8px 0; }
                .mammoth-doc table { border-collapse: collapse; width: 100%; margin: 12px 0; }
                .mammoth-doc td, .mammoth-doc th { border: 1px solid #555; padding: 6px 10px; text-align: left; }
                .mammoth-doc img { max-width: 100%; height: auto; margin: 8px 0; }
                .mammoth-doc ul, .mammoth-doc ol { padding-left: 24px; }
                .mammoth-doc a { color: #4FC3F7; }
                .mammoth-doc blockquote { border-left: 3px solid #666; padding-left: 12px; margin: 8px 0; color: #AAA; }
              </style>
              <div class="mammoth-doc">${htmlContent}</div>
            `;
            setMediaViewerHtmlContent(styledHtml);
            setMediaViewerMime('text/html');
          } else {
            setMediaViewerError(res.message || t("Knowledge.k73"));
          }
        } catch (e: any) {
          console.error('mammoth conversion error:', e);
          // 降级：使用纯文本提取
          const textRes = await kb.kbExtractDocxText({ path: url });
          if (textRes.code === 0 && textRes.data && textRes.data.content) {
            setMediaViewerTextContent(textRes.data.content);
            setMediaViewerMime('text/plain');
          } else {
            setMediaViewerError(textRes.message || t("Knowledge.k74"));
          }
        }
      } else if (epubExts.includes(ext)) {
        const res = await kb.kbExtractEpubText({ path: url });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k75"));
        }
      } else if (legacyDocExts.includes(ext)) {
        const res = await kb.kbExtractDocText({ path: url });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k76"));
        }
      } else if (rtfExts.includes(ext)) {
        const res = await kb.kbExtractRtfText({ path: url });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k77"));
        }
      } else if (archiveExts.includes(ext)) {
        const res = await kb.kbListZipContents({ path: url });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k78"));
        }
      } else if (designExts.includes(ext)) {
        const res = ext === 'psd'
          ? await kb.kbExtractPsdInfo({ path: url })
          : await kb.kbExtractAiInfo({ path: url });
        if (res.code === 0 && res.data && res.data.content) {
          setMediaViewerTextContent(res.data.content);
          setMediaViewerMime('text/plain');
        } else {
          setMediaViewerError(res.message || t("Knowledge.k79"));
        }
      } else if (ext === 'pdf') {
        const readRes = await kb.kbReadFileBase64({ path: url });
        if (readRes.code === 0 && readRes.data?.base64) {
          setMediaViewerBase64(readRes.data.base64);
          setMediaViewerMime('application/pdf');
        } else {
          setMediaViewerError(readRes.message || t("Knowledge.k80"));
        }
      } else {
        const mime = mimeMap[ext] || 'application/octet-stream';
        if (mime.startsWith('video/') || mime.startsWith('audio/') || mime.startsWith('image/')) {
          const readRes = await kb.kbReadFileBase64({ path: url });
          if (readRes.code === 0 && readRes.data?.base64) {
            setMediaViewerFileUrl(`data:${mime};base64,${readRes.data.base64}`);
            setMediaViewerMime(mime);
          } else {
            setMediaViewerError(readRes.message || t("Knowledge.k81"));
          }
        } else {
          const fileUrl = convertFileSrc(url);
          setMediaViewerFileUrl(fileUrl);
          setMediaViewerMime(mime);
        }
      }
    } catch (err) {
      setMediaViewerError(t("Knowledge.k82", { arg0: err instanceof Error ? err.message : String(err) }));
    } finally {
      setMediaViewerLoading(false);
    }
  };

  const close = () => {
    if (pptxViewerRef.current) {
      try {
        pptxViewerRef.current.destroy();
      } catch (_) {}
      pptxViewerRef.current = null;
    }
    pptxArrayBufRef.current = null;
    setMediaViewerEntry(null);
    setMediaViewerBase64('');
    setMediaViewerMime('');
    setMediaViewerTextContent('');
    setMediaViewerHtmlContent('');
    setMediaViewerFileUrl('');
    setMediaViewerError(null);
    setMediaViewerFullscreen(false);
    setTableData(null);
    setPptxSlideCount(0);
    setPptxCurrentSlide(0);
  };

  // PPTX 幻灯片键盘导航
  useEffect(() => {
    if (!mediaViewerEntry || pptxSlideCount === 0) return;
    const handleSlideKeys = (e: KeyboardEvent) => {
      if (mediaViewerFullscreen && e.key === 'Escape') return;
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
        e.preventDefault();
        setPptxCurrentSlide(prev => Math.max(0, prev - 1));
      } else if (e.key === 'ArrowRight' || e.key === 'ArrowDown' || e.key === ' ') {
        e.preventDefault();
        setPptxCurrentSlide(prev => Math.min(pptxSlideCount - 1, prev + 1));
      }
    };
    window.addEventListener('keydown', handleSlideKeys);
    return () => window.removeEventListener('keydown', handleSlideKeys);
  }, [mediaViewerEntry, pptxSlideCount, mediaViewerFullscreen]);

  // PPTX 切页
  useEffect(() => {
    if (mediaViewerMime === 'pptx' && pptxViewerRef.current) {
      try {
        const v = pptxViewerRef.current as any;
        if (typeof v.goToSlide === 'function') {
          v.goToSlide(pptxCurrentSlide);
        }
      } catch (_) {}
    }
  }, [pptxCurrentSlide, mediaViewerMime, pptxSlideCount, mediaViewerFullscreen]);

  // PPTX 全屏切换后重建查看器
  useEffect(() => {
    if (mediaViewerMime !== 'pptx' || !pptxArrayBufRef.current) return;
    const buf = pptxArrayBufRef.current;
    async function rebuild() {
      if (pptxViewerRef.current) {
        try {
          pptxViewerRef.current.destroy();
        } catch (_) {}
        pptxViewerRef.current = null;
      }
      const container = mediaViewerFullscreen ? pptxFullscreenSlideRef.current : pptxSlideRef.current;
      if (!container) return;
      container.replaceChildren();
      try {
        const viewer = await PptxViewer.open(buf, container, {
          renderMode: 'slide',
          fitMode: 'contain'
        });
        pptxViewerRef.current = viewer;
        const v = viewer as any;
        setPptxSlideCount(v.slideCount || 1);
        if (typeof v.goToSlide === 'function') {
          v.goToSlide(pptxCurrentSlide);
        }
      } catch (_) {}
    }
    const timer = setTimeout(rebuild, 100);
    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mediaViewerFullscreen]);

  const colorPicker = (
    <div className={styles.colorPickerInline}>
      {[{ color: '#E0E0E0', label: t("game.components.RealmCard.k14") }, { color: '#00FF00', label: t("Knowledge.k225") }, { color: '#FFD700', label: t("Knowledge.k226") }, { color: '#00F0FF', label: t("Knowledge.k227") }, { color: '#B026FF', label: t("game.components.RealmCard.k17") }, { color: '#FF6B6B', label: t("game.components.RealmCard.k16") }].map(c => <button key={c.color} className={`${styles.colorDot} ${mediaViewerTextColor === c.color ? styles.colorDotActive : ''}`} style={{ backgroundColor: c.color }} onClick={() => setMediaViewerTextColor(c.color)} title={c.label} />)}
    </div>
  );

  /** 内联媒体预览（预览区 body，编辑态之后） */
  const renderInline = (editors: EditorsApi) => {
    if (!mediaViewerEntry) return null;
    return (
      <div className={styles.mediaInlineViewer}>
        <div className={styles.mediaInlineHeader}>
          <div className={styles.mediaInlineHeaderLeft}>
            <span className={styles.mediaInlineIcon}>{core.getTypeIcon(mediaViewerEntry.entry_type)}</span>
            <span className={styles.mediaInlineTitle} style={{ color: mediaViewerTextColor }}>{mediaViewerEntry.name}</span>
          </div>
          <div className={styles.mediaInlineActions}>
            {(() => {
              const url = mediaViewerEntry.path_url || '';
              const ext = url.split('.').pop()?.toLowerCase() || '';
              const isExternal = mediaViewerEntry.path_url && !mediaViewerEntry.path_url.startsWith('kb://') && !mediaViewerEntry.path_url.startsWith('http');
              const canEdit = isExternal && editors.editSupportedExts.has(ext);
              const isEditing = editors.fileEditMode;
              if (!canEdit) return null;
              return <button className={styles.modeToggleIcon} onClick={() => {
                if (isEditing) {
                  editors.handleStopFileEdit();
                } else {
                  editors.handleStartFileEdit(mediaViewerEntry);
                }
              }} title={isEditing ? t("Knowledge.k223") : t("Knowledge.k224")}>
                {isEditing ? '✏️' : '📖'}
              </button>;
            })()}
            {colorPicker}
            <button className={styles.modeToggleBtn} onClick={close} title={t("Knowledge.k228")}>
              ✕
            </button>
          </div>
        </div>
        <div className={styles.mediaInlineBody}>
          {mediaViewerLoading ? <div className={styles.mediaLoadingWrap}>
              <div className={styles.spinner} />
              <span>{t("common.loading")}</span>
            </div> : mediaViewerError ? <div className={styles.mediaErrorWrap}>
              <span>⚠ {mediaViewerError}</span>
              <button className={styles.mediaExternalBtn} onClick={() => systemtools.openFile(mediaViewerEntry.path_url)}>
                {t("Knowledge.k229")}
              </button>
            </div> : mediaViewerMime.startsWith('video/') || mediaViewerMime.startsWith('audio/') ? <MediaPlayer src={mediaViewerFileUrl} mimeType={mediaViewerMime} fileName={mediaViewerEntry.name} className={mediaViewerMime.startsWith('video/') ? styles.mediaInlineVideo : styles.mediaInlineAudioWrap} /> : mediaViewerMime.startsWith('image/') ? <img src={mediaViewerFileUrl} alt={mediaViewerEntry.name} className={styles.mediaInlineImage} /> : mediaViewerMime === 'application/pdf' ? <embed src={`data:application/pdf;base64,${mediaViewerBase64}`} type="application/pdf" className={styles.mediaInlinePdf} /> : mediaViewerMime === 'table' && tableData ? <div className={styles.mediaInlineTable}>
              <table className={styles.dataTable}>
                <thead>
                  <tr>
                    <th className={styles.dataTableTh}>#</th>
                    {Array.from({ length: tableData.totalCols || 0 }).map((_, i) => <th key={i} className={styles.dataTableTh}>{excelColName(i)}</th>)}
                  </tr>
                </thead>
                <tbody>
                  {tableData.rows.map((row, ri) => <tr key={ri}>
                      <td className={styles.dataTableTdRowNum}>{ri + 1}</td>
                      {row.map((cell, ci) => <td key={ci} className={styles.dataTableTd} style={{ color: mediaViewerTextColor }}>{cell}</td>)}
                    </tr>)}
                </tbody>
              </table>
            </div> : mediaViewerMime === 'pptx' ? <div className={styles.pptxViewerInline}>
              <div className={styles.pptxNav}>
                <button className={styles.pptxNavBtn} disabled={pptxCurrentSlide === 0} onClick={() => setPptxCurrentSlide(prev => Math.max(0, prev - 1))}>
                  {t("Knowledge.k230")}
                </button>
                <span className={styles.pptxCounter}>
                  {pptxCurrentSlide + 1} / {pptxSlideCount}
                </span>
                <button className={styles.pptxNavBtn} disabled={pptxCurrentSlide >= pptxSlideCount - 1} onClick={() => setPptxCurrentSlide(prev => Math.min(pptxSlideCount - 1, prev + 1))}>
                  {t("Knowledge.k231")}
                </button>
                <button className={styles.pptxNavBtn} onClick={() => setMediaViewerFullscreen(true)}>
                  {t("Knowledge.k232")}
                </button>
              </div>
              <div className={styles.pptxSlideContent} ref={pptxSlideRef} />
            </div> : mediaViewerMime === 'text/plain' ? <pre className={styles.mediaInlineText} style={{ color: mediaViewerTextColor }}>{mediaViewerTextContent}</pre> : mediaViewerMime === 'text/html' ? <div className={styles.mediaInlineHtml} dangerouslySetInnerHTML={{ __html: sanitizeHtml(mediaViewerHtmlContent) }} /> : null}
        </div>
      </div>
    );
  };

  /** 全屏媒体浮层 */
  const renderFullscreen = (editors: EditorsApi) => {
    if (!mediaViewerFullscreen || !mediaViewerEntry) return null;
    return (
      <div className={styles.mediaFullscreenOverlay} onClick={() => setMediaViewerFullscreen(false)}>
        <div className={styles.mediaFullscreenContainer} onClick={e => e.stopPropagation()}>
          <div className={styles.mediaFullscreenHeader}>
            <span className={styles.mediaFullscreenTitle} style={{ color: mediaViewerTextColor }}>{mediaViewerEntry.name}</span>
            <div className={styles.mediaFullscreenActions}>
              {(() => {
                const url = mediaViewerEntry.path_url || '';
                const ext = url.split('.').pop()?.toLowerCase() || '';
                const isExternal = mediaViewerEntry.path_url && !mediaViewerEntry.path_url.startsWith('kb://') && !mediaViewerEntry.path_url.startsWith('http');
                const canEdit = isExternal && editors.editSupportedExts.has(ext);
                const isEditing = editors.fileEditMode;
                if (!canEdit) return null;
                return <button className={styles.modeToggleIcon} onClick={() => {
                  if (isEditing) {
                    editors.handleStopFileEdit();
                  } else {
                    editors.handleStartFileEdit(mediaViewerEntry);
                  }
                }} title={isEditing ? t("Knowledge.k223") : t("Knowledge.k224")}>
                  {isEditing ? '✏️' : '📖'}
                </button>;
              })()}
              {colorPicker}
              <button className={styles.mediaFullscreenClose} onClick={() => setMediaViewerFullscreen(false)}>✕</button>
            </div>
          </div>
          <div className={styles.mediaFullscreenBody}>
            {mediaViewerMime.startsWith('video/') || mediaViewerMime.startsWith('audio/') ? <MediaPlayer src={mediaViewerFileUrl} mimeType={mediaViewerMime} fileName={mediaViewerEntry.name} className={mediaViewerMime.startsWith('video/') ? styles.mediaFullscreenVideo : styles.mediaFullscreenAudioWrap} /> : mediaViewerMime.startsWith('image/') ? <ImageViewer src={mediaViewerFileUrl} alt={mediaViewerEntry.name} className={styles.mediaFullscreenImage} /> : mediaViewerMime === 'application/pdf' ? <PdfViewer base64Data={`data:application/pdf;base64,${mediaViewerBase64}`} fileName={mediaViewerEntry.name} className={styles.mediaFullscreenPdf} /> : mediaViewerMime === 'table' && tableData ? <div className={styles.mediaFullscreenTable}>
              <table className={styles.dataTable}>
                <thead>
                  <tr>
                    <th className={styles.dataTableTh}>#</th>
                    {Array.from({ length: tableData.totalCols || 0 }).map((_, i) => <th key={i} className={styles.dataTableTh}>{excelColName(i)}</th>)}
                  </tr>
                </thead>
                <tbody>
                  {tableData.rows.map((row, ri) => <tr key={ri}>
                      <td className={styles.dataTableTdRowNum}>{ri + 1}</td>
                      {row.map((cell, ci) => <td key={ci} className={styles.dataTableTd} style={{ color: mediaViewerTextColor }}>{cell}</td>)}
                    </tr>)}
                </tbody>
              </table>
            </div> : mediaViewerMime === 'pptx' && pptxSlideCount > 0 ? <div className={styles.pptxViewerFullscreen}>
              <div className={styles.pptxNav}>
                <button className={styles.pptxNavBtn} disabled={pptxCurrentSlide === 0} onClick={() => setPptxCurrentSlide(prev => Math.max(0, prev - 1))}>
                  {t("Knowledge.k230")}
                </button>
                <span className={styles.pptxCounter}>
                  {pptxCurrentSlide + 1} / {pptxSlideCount}
                </span>
                <button className={styles.pptxNavBtn} disabled={pptxCurrentSlide >= pptxSlideCount - 1} onClick={() => setPptxCurrentSlide(prev => Math.min(pptxSlideCount - 1, prev + 1))}>
                  {t("Knowledge.k231")}
                </button>
                <button className={styles.pptxNavBtn} onClick={() => setMediaViewerFullscreen(false)}>
                  {t("Knowledge.k290")}
                </button>
              </div>
              <div className={styles.pptxSlideContentFullscreen} ref={pptxFullscreenSlideRef} />
            </div> : mediaViewerMime === 'text/plain' ? <pre className={styles.mediaFullscreenText} style={{ color: mediaViewerTextColor }}>{mediaViewerTextContent}</pre> : mediaViewerMime === 'text/html' ? <div className={styles.mediaFullscreenHtml} dangerouslySetInnerHTML={{ __html: sanitizeHtml(mediaViewerHtmlContent) }} /> : null}
          </div>
        </div>
      </div>
    );
  };

  return {
    open,
    close,
    hasEntry: mediaViewerEntry !== null,
    textColor: mediaViewerTextColor,
    renderInline,
    renderFullscreen
  };
}
