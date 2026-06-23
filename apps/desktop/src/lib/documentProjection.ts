import { sanitizeEditorHref } from './editorSecurity';

export interface Inline {
  text: string;
  marks?: string[];
  link?: string | null;
}

export interface ParagraphBlock {
  type: 'Paragraph';
  value: {
    style?: string;
    inlines: Inline[];
  };
}

export interface HeadingBlock {
  type: 'Heading';
  value: {
    level: number;
    inlines: Inline[];
  };
}

export type Block = ParagraphBlock | HeadingBlock | { type: string; value?: unknown };

export interface DocumentState {
  meta: {
    title: string;
  };
  sections: Array<{
    blocks: Block[];
  }>;
}

export interface EditorTextNode {
  type: 'text';
  text: string;
  marks?: Array<{ type: string; attrs?: Record<string, string> }>;
}

export interface EditorBlockNode {
  type: 'paragraph' | 'heading';
  attrs?: {
    level?: number;
    style?: string;
  };
  content?: EditorTextNode[];
}

export interface EditorDoc {
  type: 'doc';
  content: EditorBlockNode[];
}

export interface EditorProjectedChange {
  text: string;
  blocks: Array<ParagraphBlock | HeadingBlock>;
}

export type DocumentCommand =
  | {
      type: 'replace_block';
      section_index: number;
      block_index: number;
      block: ParagraphBlock | HeadingBlock;
    }
  | {
      type: 'insert_block';
      section_index: number;
      block_index: number;
      block: ParagraphBlock | HeadingBlock;
    }
  | {
      type: 'delete_block';
      section_index: number;
      block_index: number;
    };

export function documentToText(document: DocumentState): string {
  const blocks: Block[] = [];
  for (const section of document.sections) {
    blocks.push(...section.blocks);
  }
  return blocks.map(blockToText).filter(Boolean).join('\n');
}

export function documentProjectionWarnings(document: DocumentState): string[] {
  const warnings = [];
  if (document.sections.length !== 1) {
    warnings.push('Multiple sections are preserved but read-only in the Sprint 002 editor projection.');
  }
  for (const section of document.sections) {
    for (const block of section.blocks) {
      if (!hasInlineContent(block)) {
        warnings.push(`${block.type} blocks are preserved but read-only in the Sprint 002 editor projection.`);
      }
    }
  }
  return warnings;
}

export function canEditProjectedDocument(document: DocumentState): boolean {
  return documentProjectionWarnings(document).length === 0;
}

export function documentToEditorDoc(document: DocumentState): EditorDoc {
  const content: EditorBlockNode[] = [];
  for (const section of document.sections) {
    for (const block of section.blocks) {
      const node = blockToEditorNode(block);
      content.push(node);
    }
  }

  return {
    type: 'doc',
    content: content.length > 0 ? content : [{ type: 'paragraph' }]
  };
}

export function editorDocToWordCoreBlocks(editorDoc: EditorDoc): Array<ParagraphBlock | HeadingBlock> {
  return editorDoc.content.map((block) => {
    const inlines = (block.content ?? []).map(editorTextToInline).filter((inline) => inline.text.length > 0);
    if (block.type === 'heading') {
      return {
        type: 'Heading',
        value: {
          level: clampHeadingLevel(block.attrs?.level ?? 1),
          inlines
        }
      };
    }

    return {
      type: 'Paragraph',
      value: {
        style: block.attrs?.style || 'body',
        inlines
      }
    };
  });
}

export function buildEditorSyncCommands(
  document: DocumentState,
  nextBlocks: Array<ParagraphBlock | HeadingBlock>
): DocumentCommand[] {
  if (!canEditProjectedDocument(document)) {
    return [];
  }

  const currentBlocks = document.sections[0]?.blocks ?? [];
  const commands: DocumentCommand[] = [];
  for (let index = currentBlocks.length - 1; index >= nextBlocks.length; index -= 1) {
    commands.push({ type: 'delete_block', section_index: 0, block_index: index });
  }

  const replaceCount = Math.min(currentBlocks.length, nextBlocks.length);
  for (let index = 0; index < replaceCount; index += 1) {
    commands.push({
      type: 'replace_block',
      section_index: 0,
      block_index: index,
      block: nextBlocks[index]
    });
  }

  for (let index = currentBlocks.length; index < nextBlocks.length; index += 1) {
    commands.push({
      type: 'insert_block',
      section_index: 0,
      block_index: index,
      block: nextBlocks[index]
    });
  }

  return commands;
}

function blockToText(block: Block): string {
  if (hasInlineContent(block)) {
    return block.value.inlines.map((inline) => inline.text).join('');
  }
  return '';
}

function hasInlineContent(block: Block): block is ParagraphBlock | HeadingBlock {
  return block.type === 'Paragraph' || block.type === 'Heading';
}

function blockToEditorNode(block: Block): EditorBlockNode {
  if (!hasInlineContent(block)) {
    return {
      type: 'paragraph',
      content: [
        {
          type: 'text',
          text: `[${block.type} block preserved read-only]`
        }
      ]
    };
  }

  const content = block.value.inlines.map(inlineToEditorText).filter((inline) => inline.text.length > 0);
  if (block.type === 'Heading') {
    return {
      type: 'heading',
      attrs: { level: clampHeadingLevel(block.value.level) },
      content
    };
  }

  return {
    type: 'paragraph',
    attrs: { style: block.value.style || 'body' },
    content
  };
}

function editorTextToInline(textNode: EditorTextNode): Inline {
  const marks: string[] = [];
  let link: string | null = null;
  for (const mark of textNode.marks ?? []) {
    const mapped = mapEditorMark(mark.type);
    if (mapped) {
      marks.push(mapped);
    }
    if (mark.type === 'link' && mark.attrs?.href) {
      link = sanitizeEditorHref(mark.attrs.href) ?? null;
    }
  }

  return {
    text: textNode.text,
    marks,
    link
  };
}

function inlineToEditorText(inline: Inline): EditorTextNode {
  const marks = [];
  for (const mark of inline.marks ?? []) {
    const mapped = mapInlineMark(mark);
    if (mapped) {
      marks.push({ type: mapped });
    }
  }
  if (inline.link) {
    const href = sanitizeEditorHref(inline.link);
    if (href) {
      marks.push({ type: 'link', attrs: { href } });
    }
  }

  return {
    type: 'text',
    text: inline.text,
    ...(marks.length > 0 ? { marks } : {})
  };
}

function mapEditorMark(mark: string): string | undefined {
  const supportedMarks: Record<string, string> = {
    bold: 'Bold',
    italic: 'Italic',
    underline: 'Underline',
    strikethrough: 'Strikethrough',
    superscript: 'Superscript',
    subscript: 'Subscript'
  };
  return supportedMarks[mark];
}

function mapInlineMark(mark: string): string | undefined {
  const supportedMarks: Record<string, string> = {
    Bold: 'bold',
    Italic: 'italic',
    Underline: 'underline',
    Strikethrough: 'strikethrough',
    Superscript: 'superscript',
    Subscript: 'subscript'
  };
  return supportedMarks[mark];
}

function clampHeadingLevel(level: number): number {
  return Math.min(6, Math.max(1, level));
}
