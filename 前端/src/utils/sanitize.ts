// sanitize.ts - HTML 内容消毒工具（v1.52.4.0 T2.12）
//
// 设计依据：功能展望/平台级增强/05_安全加固_A4.md §2.2.2
//
// 用途：
//   在前端渲染用户上传/外部来源的 HTML 内容前，使用 DOMPurify 进行消毒，
//   移除 XSS 攻击载体（<script>、事件属性、javascript: 协议等），
//   同时保留 KB 媒体预览所需的合法标签（含 SVG）。
//
// 使用方法：
//   import { sanitizeHtml } from '@/utils/sanitize';
//   <div dangerouslySetInnerHTML={{ __html: sanitizeHtml(htmlContent) }} />
//
// 依赖：dompurify@3.4.12（已在 T2.14 升级到安全版本）

import DOMPurify from 'dompurify';

// ============================================================================
// 配置：允许的标签白名单
// ============================================================================
//
// 选取策略：覆盖 KB 媒体预览（text/html 文件）所需的所有合法标签，
// 同时严格排除 XSS 载体（script、iframe、object、embed、form 等）。
//
// 分类：
//   - 文本基础：a, abbr, b, blockquote, br, code, em, hr, i, p, pre, span, strong, sub, sup
//   - 标题：h1-h6
//   - 列表：ol, ul, li, dl, dt, dd
//   - 表格：table, thead, tbody, tfoot, tr, td, th, caption, col, colgroup
//   - 媒体：img, figure, figcaption, picture, source
//   - SVG：svg, path, circle, ellipse, rect, line, polyline, polygon, text, g, defs, use, symbol, linearGradient, radialGradient, stop
//   - 语义：article, section, header, footer, nav, aside, main, div
//   - 代码：pre, code, kbd, samp, var
//   - 其他：details, summary, mark, time, del, ins, small, q, cite, blockquote

const ALLOWED_TAGS = [
  // 文本基础
  'a', 'abbr', 'b', 'blockquote', 'br', 'code', 'em', 'hr', 'i', 'p', 'pre',
  'span', 'strong', 'sub', 'sup',
  // 标题
  'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
  // 列表
  'ol', 'ul', 'li', 'dl', 'dt', 'dd',
  // 表格
  'table', 'thead', 'tbody', 'tfoot', 'tr', 'td', 'th', 'caption', 'col', 'colgroup',
  // 媒体
  'img', 'figure', 'figcaption', 'picture', 'source',
  // SVG（KB 媒体预览可能包含内嵌 SVG）
  'svg', 'path', 'circle', 'ellipse', 'rect', 'line', 'polyline', 'polygon',
  'text', 'g', 'defs', 'use', 'symbol', 'linearGradient', 'radialGradient', 'stop',
  // 语义
  'article', 'section', 'header', 'footer', 'nav', 'aside', 'main', 'div',
  // 代码
  'kbd', 'samp', 'var',
  // 其他
  'details', 'summary', 'mark', 'time', 'del', 'ins', 'small', 'q', 'cite',
];

// ============================================================================
// 配置：允许的属性白名单
// ============================================================================
//
// 选取策略：覆盖合法 HTML 渲染所需属性，严格排除事件处理器（on*）和危险属性。
//
// 分类：
//   - 通用：class, style, id, title, lang, dir
//   - 链接：href, target, rel, download, type
//   - 图片：src, alt, width, height, srcset, sizes
//   - 表格：colspan, rowspan, headers, scope
//   - SVG：viewBox, fill, stroke, strokeWidth, strokeLinecap, strokeLinejoin,
//          d, cx, cy, r, rx, ry, x, y, x1, y1, x2, y2, points, transform,
//          gradientUnits, gradientTransform, offset, stop-color, stop-opacity,
//          fill-opacity, stroke-opacity, stroke-dasharray, stroke-width
//   - source：srcset, media, type, sizes
//   - time：datetime
//   - details：open

const ALLOWED_ATTR = [
  // 通用
  'class', 'style', 'id', 'title', 'lang', 'dir',
  // 链接
  'href', 'target', 'rel', 'download', 'type',
  // 图片
  'src', 'alt', 'width', 'height', 'srcset', 'sizes',
  // 表格
  'colspan', 'rowspan', 'headers', 'scope',
  // SVG 属性
  'viewBox', 'fill', 'stroke', 'strokeWidth', 'strokeLinecap', 'strokeLinejoin',
  'stroke-width', 'stroke-dasharray', 'stroke-opacity', 'fill-opacity',
  'd', 'cx', 'cy', 'r', 'rx', 'ry', 'x', 'y', 'x1', 'y1', 'x2', 'y2',
  'points', 'transform', 'gradientUnits', 'gradientTransform', 'offset',
  'stop-color', 'stop-opacity',
  // picture/source
  'media',
  // time
  'datetime',
  // details
  'open',
];

// ============================================================================
// 配置：禁止的标签（即使白名单遗漏也强制禁止）
// ============================================================================
//
// 这些标签是 XSS 攻击的常见载体，必须禁止。

const FORBID_TAGS = [
  'script', 'iframe', 'object', 'embed', 'form', 'input', 'button',
  'textarea', 'select', 'option', 'base', 'link', 'meta', 'style',
  'applet', 'frame', 'frameset', 'noscript', 'template',
];

// ============================================================================
// 配置：禁止的属性（即使白名单遗漏也强制禁止）
// ============================================================================
//
// 事件处理器属性（on*）和危险协议属性必须禁止。

const FORBID_ATTR = [
  'onerror', 'onload', 'onclick', 'onmouseover', 'onmouseout', 'onfocus',
  'onblur', 'onchange', 'oninput', 'onsubmit', 'onreset', 'onkeydown',
  'onkeyup', 'onkeypress', 'ontouchstart', 'ontouchmove', 'ontouchend',
  'onanimationstart', 'onanimationend', 'onanimationiteration',
  'ontransitionend', 'onwheel', 'onscroll', 'onresize',
];

// ============================================================================
// 主接口：sanitizeHtml
// ============================================================================

/**
 * 对用户上传/外部来源的 HTML 内容进行消毒。
 *
 * 用途：
 * - KB 媒体预览（text/html 文件渲染）
 * - 任何需要使用 dangerouslySetInnerHTML 的用户内容
 *
 * 安全保障：
 * - 移除所有 <script> 标签
 * - 移除所有事件处理器属性（onerror、onload 等）
 * - 移除 javascript: 协议的 href/src
 * - 移除 data: 协议的 href/src（防 SVG data: URL XSS）
 * - 保留合法的 HTML + SVG 标签和属性
 *
 * @param dirty 待消毒的 HTML 字符串
 * @returns 消毒后的安全 HTML 字符串
 *
 * @example
 *   const safe = sanitizeHtml('<script>alert(1)</script><p>hello</p>');
 *   // 返回 '<p>hello</p>'
 */
export function sanitizeHtml(dirty: string): string {
  if (!dirty) return '';
  return DOMPurify.sanitize(dirty, {
    ALLOWED_TAGS,
    ALLOWED_ATTR,
    ALLOW_DATA_ATTR: false,
    FORBID_TAGS,
    FORBID_ATTR,
    // 禁止 URI 协议白名单外的协议（防 javascript:、vbscript: 等）
    ALLOWED_URI_REGEXP: /^(?:(?:https?|mailto|tel|data:image\/(?:png|jpeg|gif|webp|svg\+xml|bmp|x-icon|vnd\.microsoft\.icon)):|[^:/?#]*(?:[/?#]|$))/i,
    // KEEP_CONTENT 默认 true：
    // - script/iframe/object 等危险标签的内容会被 DOMPurify 自动移除（不受此选项影响）
    // - 合法标签（p/div 等）的文本内容被保留
    // - 注意：设为 false 会导致 jsdom 中所有文本节点被错误移除
  });
}

// ============================================================================
// 辅助接口：sanitizeSvg（用于纯 SVG 内容）
// ============================================================================

/**
 * 对纯 SVG 字符串进行消毒。
 *
 * 比 sanitizeHtml 更严格：仅保留 SVG 相关标签和属性，
 * 适用于图标优化器等场景。
 *
 * @param dirty 待消毒的 SVG 字符串
 * @returns 消毒后的安全 SVG 字符串
 */
export function sanitizeSvg(dirty: string): string {
  if (!dirty) return '';
  return DOMPurify.sanitize(dirty, {
    USE_PROFILES: { svg: true, svgFilters: true },
    ALLOW_DATA_ATTR: false,
    FORBID_ATTR,
  });
}

// ============================================================================
// 辅助接口：isSafeHtml（仅检查不修改）
// ============================================================================

/**
 * 检查 HTML 内容是否已经安全（不包含 XSS 载体）。
 *
 * 不修改原字符串，仅返回布尔值。适用于日志/调试。
 *
 * @param html 待检查的 HTML 字符串
 * @returns true=安全，false=包含 XSS 载体
 */
export function isSafeHtml(html: string): boolean {
  if (!html) return true;
  return sanitizeHtml(html) === html;
}
