//! 装饰器管理 — 对标 VSCode Decoration API
//!
//! 提供代码透镜、内嵌提示、自定义装饰等功能。

export interface DecorationOptions {
  range: {
    startLineNumber: number;
    startColumn: number;
    endLineNumber: number;
    endColumn: number;
  };
  hoverMessage?: string;
  className?: string;
  glyphMarginClassName?: string;
  linesDecorationsClassName?: string;
  inlineClassName?: string;
  before?: {
    content: string;
    inlineClassName?: string;
  };
  after?: {
    content: string;
    inlineClassName?: string;
  };
}

export class DecorationManager {
  private decorations: string[] = [];
  private editor: any;

  constructor(editor: any) {
    this.editor = editor;
  }

  /** 设置装饰 */
  setDecorations(options: DecorationOptions[]): void {
    this.decorations = this.editor.deltaDecorations(
      this.decorations,
      options.map((opt) => ({
        range: new (this.editor as any).Range(
          opt.range.startLineNumber,
          opt.range.startColumn,
          opt.range.endLineNumber,
          opt.range.endColumn
        ),
        options: {
          hoverMessage: opt.hoverMessage ? { value: opt.hoverMessage } : undefined,
          className: opt.className,
          glyphMarginClassName: opt.glyphMarginClassName,
          linesDecorationsClassName: opt.linesDecorationsClassName,
          inlineClassName: opt.inlineClassName,
          before: opt.before
            ? {
                content: opt.before.content,
                inlineClassName: opt.before.inlineClassName,
              }
            : undefined,
          after: opt.after
            ? {
                content: opt.after.content,
                inlineClassName: opt.after.inlineClassName,
              }
            : undefined,
        },
      }))
    );
  }

  /** 添加代码透镜 */
  addCodeLens(line: number, text: string, _command?: () => void): void {
    this.editor.deltaDecorations(
      [],
      [
        {
          range: {
            startLineNumber: line,
            startColumn: 1,
            endLineNumber: line,
            endColumn: 1,
          },
          options: {
            isWholeLine: true,
            after: {
              content: text,
              inlineClassName: 'codelens-decoration',
            },
          },
        },
      ]
    );
  }

  /** 添加内嵌提示 */
  addInlayHint(line: number, column: number, text: string): void {
    this.editor.deltaDecorations(
      [],
      [
        {
          range: {
            startLineNumber: line,
            startColumn: column,
            endLineNumber: line,
            endColumn: column,
          },
          options: {
            before: {
              content: text,
              inlineClassName: 'inlay-hint-decoration',
            },
          },
        },
      ]
    );
  }

  /** 高亮行 */
  highlightLine(line: number, className: string): void {
    this.editor.deltaDecorations(
      [],
      [
        {
          range: {
            startLineNumber: line,
            startColumn: 1,
            endLineNumber: line,
            endColumn: 1,
          },
          options: {
            isWholeLine: true,
            className,
          },
        },
      ]
    );
  }

  /** 清除所有装饰 */
  clear(): void {
    this.decorations = this.editor.deltaDecorations(this.decorations, []);
  }
}

/** 括号匹配高亮 */
export function setupBracketHighlighting(editor: any): void {
  editor.updateOptions({
    bracketPairColorization: {
      enabled: true,
    },
    matchBrackets: 'always' as const,
    autoClosingBrackets: 'always' as const,
    guides: {
      bracketPairs: true,
      indentation: true,
      bracketPairsHorizontal: 'active' as const,
    },
  });
}

/** 代码透镜配置 */
export function setupCodeLens(editor: any): void {
  editor.updateOptions({
    codeLens: true,
    lightbulb: {
      enabled: true,
    },
  });
}

/** 内嵌提示配置 */
export function setupInlayHints(editor: any): void {
  editor.updateOptions({
    inlayHints: {
      enabled: 'on' as const,
    },
  });
}

/** 缩进参考线配置 */
export function setupIndentGuides(editor: any): void {
  editor.updateOptions({
    renderIndentGuides: true,
    guides: {
      indentation: true,
      bracketPairs: true,
      bracketPairsHorizontal: 'active' as const,
      highlightActiveIndentation: true,
    },
  });
}

/** 光标增强配置 */
export function setupCursorEnhancement(editor: any): void {
  editor.updateOptions({
    cursorBlinking: 'smooth',
    cursorSmoothCaretAnimation: 'on',
    cursorStyle: 'line',
    cursorWidth: 2,
    cursorSurroundingLines: 0,
    cursorSurroundingLinesStyle: 'default',
  });
}

/** 应用所有视觉增强 */
export function applyVisualEnhancements(editor: any): void {
  setupBracketHighlighting(editor);
  setupCodeLens(editor);
  setupInlayHints(editor);
  setupIndentGuides(editor);
  setupCursorEnhancement(editor);
}