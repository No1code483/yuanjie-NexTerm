// knowledge.editors L2 功能域：编辑器套件（文本/表格/PPT/PDF/图像/音频）。
// 局部 state + handlers + JSX 物理落在本目录；核心共享数据经 KnowledgeCore 注入。
import { t } from "i18next";
import { useState } from 'react';
import { ipc } from '@/lib/ipc';
import TextEditor from '@/components/TextEditor';
import TableEditor from '@/components/TableEditor';
import PptEditor from '@/components/PptEditor';
import PdfEditor from '@/components/PdfEditor';
import ImageEditor from '@/components/ImageEditor';
import AudioEditor from '@/components/AudioEditor';
import type { KbEntry, KnowledgeCore } from '../../knowledge/types';

export const editSupportedExts = new Set(['md', 'markdown', 'txt', 'text', 'json', 'jsonc', 'json5', 'xml', 'xaml', 'xsl', 'xslt', 'xsd', 'yaml', 'yml', 'toml', 'cfg', 'conf', 'ini', 'inf', 'cnf', 'env', 'envrc', 'properties', 'prop', 'csv', 'tsv', 'html', 'htm', 'xhtml', 'shtml', 'css', 'scss', 'sass', 'less', 'styl', 'js', 'jsx', 'mjs', 'cjs', 'ts', 'tsx', 'vue', 'svelte', 'astro', 'py', 'pyw', 'pyx', 'rs', 'rlib', 'go', 'java', 'kt', 'kts', 'scala', 'sc', 'groovy', 'gradle', 'php', 'phtml', 'phps', 'phpt', 'rb', 'rbw', 'rake', 'gemspec', 'pl', 'pm', 'pod', 'swift', 'dart', 'jl', 'lua', 'r', 'rprofile', 'rmd', 'rnw', 'erl', 'hrl', 'ex', 'exs', 'eex', 'heex', 'hs', 'lhs', 'ml', 'mli', 'clj', 'cljs', 'cljc', 'edn', 'elm', 'fs', 'fsx', 'fsi', 'fsscript', 'nim', 'nims', 'zig', 'sh', 'bash', 'zsh', 'fish', 'bat', 'cmd', 'ps1', 'psm1', 'psd1', 'makefile', 'mk', 'dockerfile', 'containerfile', 'cmake', 'sql', 'psql', 'mysql', 'hql', 'prql', 'sparql', 'rq', 'graphql', 'gql', 'ttl', 'nt', 'n3', 'rdf', 'owl', 'cypher', 'cql', 'proto', 'protobuf', 'thrift', 'avsc', 'avdl', 'wsdl', 'wadl', 'raml', 'oas', 'openapi', 'grpc', 'cue', 'dhall', 'nix', 'tf', 'tfvars', 'hcl', 'nomad', 'sentinel', 'smithy', 'handlebars', 'hbs', 'hbrs', 'mustache', 'ejs', 'ect', 'pug', 'jade', 'twig', 'jinja', 'jinja2', 'j2', 'liquid', 'njk', 'nunjucks', 'dust', 'haml', 'slim', 'erb', 'rhtml', 'volt', 'latte', 'blade', 'mjml', 'rst', 'rest', 'restructuredtext', 'asciidoc', 'adoc', 'textile', 'org', 'wiki', 'mediawiki', 'creole', 'typ', 'tex', 'latex', 'ltx', 'sty', 'cls', 'bib', 'bibtex', 'nfo', 'diz', 'log', 'srt', 'vtt', 'ass', 'ssa', 'sub', 'smi', 'lrc', 'patch', 'diff', 'rej', 'ron', 'eml', 'mbox', 'vcard', 'vcf', 'ics', 'ical', 'ifb', 'prisma', 'coffee', 'cson', 'iced', 'litcoffee', 'peg', 'pegjs', 'ohm', 'rnc', 'rng', 'dtd', 'sgml', 'sgm', 'ent', 'g4', 'ebnf', 'bnf', 'abnf', 'wast', 'wat', 'asm', 'sage', 'sagews', 'm', 'mat', 'octave', 'matlab', 'mma', 'nb', 'wl', 'wls', 'cirru', 'idr', 'lidr', 'agda', 'lagda', 'v', 'vhdl', 'vhd', 'sv', 'svh', 'glsl', 'vert', 'frag', 'tesc', 'tese', 'geom', 'comp', 'hlsl', 'fx', 'fxh', 'vsh', 'psh', 'wgsl', 'metal', 'opencl', 'cuh', 'ispc', 'plist', 'strings', 'dic', 'aff', 'po', 'pot', 'mo', 'lang', 'resx', 'resw', 'resjson', 'xliff', 'xlf', 'arb', 'ftl', 'fxml', 'mxml', 'xib', 'storyboard', 'abc', 'ly', 'ily', 'schemas', 'mod', 'pest', 'pomsky', 'nearley', 'a51', 'bsv', 'semgrep', 'smel', 'editorconfig', 'gitignore', 'gitattributes', 'gitmodules', 'dockerignore', 'npmrc', 'yarnrc', 'lock', 'psv', 'h', 'c', 'cpp', 'cc', 'cxx', 'hpp', 'hh', 'hxx']);

const tableSupportedExts = new Set(['xlsx', 'xls', 'ods', 'csv']);

export function useEditors(core: KnowledgeCore, mediaClose: () => void) {
  const [fileEditMode, setFileEditMode] = useState(false);
  const [fileEditContent, setFileEditContent] = useState('');
  const [fileEditFormatType, setFileEditFormatType] = useState('');
  const [tableEditMode, setTableEditMode] = useState(false);
  const [tableEditSheets, setTableEditSheets] = useState<Array<{ name: string; rows: string[][]; row_count: number; col_count: number }>>([]);
  const [tableEditExt, setTableEditExt] = useState('');
  const [pptEditMode, setPptEditMode] = useState(false);
  const [pptEditSlides, setPptEditSlides] = useState<Array<{ index: number; name: string; text_content: string; raw_xml: string }>>([]);
  const [pdfEditMode, setPdfEditMode] = useState(false);
  const [imageEditMode, setImageEditMode] = useState(false);
  const [audioEditMode, setAudioEditMode] = useState(false);

  const handleStartFileEdit = async (entry: KbEntry) => {
    const url = entry.path_url || '';
    const ext = url.split('.').pop()?.toLowerCase() || '';
    if (!editSupportedExts.has(ext)) {
      core.showStatus('error', t("Knowledge.k59", { ext: ext }));
      return;
    }
    try {
      const res = await ipc.invoke<any>('fileedit_read', {
        request: { path: url, ext }
      });
      if (res.code === 0 && res.data) {
        setFileEditContent(res.data.content);
        setFileEditFormatType(res.data.format_type);
        setFileEditMode(true);
      } else {
        core.showStatus('error', res.message || t("Knowledge.k60"));
      }
    } catch (e: any) {
      core.showStatus('error', e?.toString() || t("Knowledge.k61"));
    }
  };

  const handleStopFileEdit = () => {
    setFileEditMode(false);
    setFileEditContent('');
    setFileEditFormatType('');
  };

  const handleFileEditSaved = () => {
    mediaClose();
  };

  /** 直接以 markdown 模式进入文本编辑（预览区 contentEditor 头部按钮） */
  const startMarkdownEdit = (content: string) => {
    setFileEditContent(content);
    setFileEditFormatType('markdown');
    setFileEditMode(true);
  };

  // @ts-ignore -- reserved for future table editing feature
  const handleStartTableEdit = async (entry: KbEntry) => {
    const url = entry.path_url || '';
    const ext = url.split('.').pop()?.toLowerCase() || '';
    if (!tableSupportedExts.has(ext)) {
      core.showStatus('error', t("Knowledge.k62", { ext: ext }));
      return;
    }
    try {
      const res = await ipc.invoke<any>('tableedit_read', { path: url, ext });
      if (res.code === 0 && res.data) {
        setTableEditSheets(res.data.sheets || []);
        setTableEditExt(res.data.extension || ext);
        setTableEditMode(true);
      } else {
        core.showStatus('error', res.message || t("Knowledge.k63"));
      }
    } catch (e: any) {
      core.showStatus('error', e?.toString() || t("Knowledge.k64"));
    }
  };
  const handleStopTableEdit = () => {
    setTableEditMode(false);
    setTableEditSheets([]);
    setTableEditExt('');
  };
  const handleTableEditSaved = () => {
    mediaClose();
  };

  // @ts-ignore -- reserved for future ppt edit feature
  const handleStartPptEdit = async (entry: KbEntry) => {
    const url = entry.path_url || '';
    const ext = url.split('.').pop()?.toLowerCase() || '';
    if (ext !== 'pptx') {
      core.showStatus('error', t("Knowledge.k59", { ext: ext }));
      return;
    }
    try {
      const res = await ipc.invoke<any>('pptedit_get_slides', { path: url });
      if (res.code === 0 && res.data && res.data.slides) {
        setPptEditSlides(res.data.slides);
        setPptEditMode(true);
      } else {
        core.showStatus('error', res.message || t("Knowledge.k65"));
      }
    } catch (e: any) {
      core.showStatus('error', e?.toString() || t("Knowledge.k66"));
    }
  };
  const handleStopPptEdit = () => {
    setPptEditMode(false);
    setPptEditSlides([]);
  };

  // @ts-expect-error -- reserved for future pdf edit feature
  const handleStartPdfEdit = () => {
    setPdfEditMode(true);
  };
  const handleStopPdfEdit = () => {
    setPdfEditMode(false);
  };

  // @ts-expect-error -- reserved for future image edit feature
  const handleStartImageEdit = () => {
    setImageEditMode(true);
  };
  const handleStopImageEdit = () => {
    setImageEditMode(false);
  };

  // @ts-expect-error -- reserved for future audio edit feature
  const handleStartAudioEdit = () => {
    setAudioEditMode(true);
  };
  const handleStopAudioEdit = () => {
    setAudioEditMode(false);
  };

  /** 供 browse handleSelect 重置全部编辑态 */
  const resetEditors = () => {
    setFileEditMode(false);
    setTableEditMode(false);
    setPptEditMode(false);
    setPdfEditMode(false);
    setImageEditMode(false);
    setAudioEditMode(false);
  };

  /** 预览区编辑器分派（优先级最高；无激活编辑器返回 null） */
  const renderActiveEditor = (entry: KbEntry, wikiEntries: string[], onWikiLinkClick: (name: string) => void) => {
    if (fileEditMode) return <TextEditor filePath={entry.path_url || ''} fileExt={(entry.path_url || '').split('.').pop()?.toLowerCase() || 'txt'} initialContent={fileEditContent} formatType={fileEditFormatType} fileName={entry.name} onSave={handleFileEditSaved} onClose={handleStopFileEdit} showStatus={core.showStatus} wikiEntries={wikiEntries} onWikiLinkClick={onWikiLinkClick} />;
    if (tableEditMode) return <TableEditor filePath={entry.path_url || ''} fileExt={tableEditExt} initialSheets={tableEditSheets} fileName={entry.name} onSave={handleTableEditSaved} onClose={handleStopTableEdit} showStatus={core.showStatus} />;
    if (pptEditMode) return <PptEditor filePath={entry.path_url || ''} initialSlides={pptEditSlides} fileName={entry.name} onSave={() => {}} onClose={handleStopPptEdit} showStatus={core.showStatus} />;
    if (pdfEditMode) return <PdfEditor filePath={entry.path_url || ''} fileName={entry.name} onClose={handleStopPdfEdit} showStatus={core.showStatus} />;
    if (imageEditMode) return <ImageEditor filePath={entry.path_url || ''} fileName={entry.name} onClose={handleStopImageEdit} showStatus={core.showStatus} />;
    if (audioEditMode) return <AudioEditor filePath={entry.path_url || ''} fileName={entry.name} onClose={handleStopAudioEdit} showStatus={core.showStatus} />;
    return null;
  };

  return {
    fileEditMode,
    editSupportedExts,
    handleStartFileEdit,
    handleStopFileEdit,
    startMarkdownEdit,
    renderActiveEditor,
    resetEditors
  };
}
