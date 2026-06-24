import { sanitizeEditorHref } from './editorSecurity';

export interface Inline {
  text: string;
  marks?: string[];
  link?: string | null;
  style?: InlineStyle;
}

export interface InlineStyle {
  font_family?: string | null;
  font_size_pt?: number | null;
  text_color?: string | null;
  highlight_color?: string | null;
}

export interface ParagraphFormat {
  alignment?: 'left' | 'center' | 'right' | 'justify' | null;
  line_spacing_per_mille?: number | null;
  spacing_before_mm?: number | null;
  spacing_after_mm?: number | null;
  indent_start_mm?: number | null;
  indent_end_mm?: number | null;
  first_line_indent_mm?: number | null;
}

export interface StyleProperties {
  paragraph?: ParagraphFormat | null;
  inline?: InlineStyle | null;
  page?: PageSetup | null;
}

export interface DocumentStyle {
  id: string;
  name: string;
  kind: 'Paragraph' | 'Character' | 'Table' | 'Page';
  parent?: string | null;
  properties: StyleProperties;
}

export interface ParagraphBlock {
  type: 'Paragraph';
  value: {
    style?: string;
    format?: ParagraphFormat;
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

export interface ListDefinition {
  ordered: boolean;
  marker?: string | null;
}

export interface ListItem {
  level: number;
  blocks: Array<ParagraphBlock | HeadingBlock>;
}

export interface ListBlock {
  type: 'List';
  value: {
    definition_id: string;
    items: ListItem[];
  };
}

export type EditableBlock = ParagraphBlock | HeadingBlock | ListBlock;
export type Block = EditableBlock | { type: string; value?: unknown };

export interface PageSetup {
  width_mm: number;
  height_mm: number;
  margin_top_mm: number;
  margin_right_mm: number;
  margin_bottom_mm: number;
  margin_left_mm: number;
}

export interface DocumentState {
  meta: {
    title: string;
  };
  styles?: Record<string, DocumentStyle>;
  lists?: Record<string, ListDefinition>;
  sections: Array<{
    blocks: Block[];
    page?: PageSetup;
  }>;
  warnings?: Array<{
    code: string;
    message: string;
  }>;
}

export interface EditorTextNode {
  type: 'text';
  text: string;
  marks?: Array<{ type: string; attrs?: Record<string, string | number | null> }>;
}

export interface EditorParagraphNode {
  type: 'paragraph';
  attrs?: EditorParagraphAttrs;
  content?: EditorTextNode[];
}

export interface EditorHeadingNode {
  type: 'heading';
  attrs?: {
    level?: number;
  };
  content?: EditorTextNode[];
}

export interface EditorListItemNode {
  type: 'list_item';
  attrs?: {
    level?: number;
  };
  content?: Array<EditorParagraphNode | EditorHeadingNode>;
}

export interface EditorListNode {
  type: 'bullet_list' | 'ordered_list';
  attrs?: {
    definitionId?: string;
  };
  content: EditorListItemNode[];
}

export interface EditorParagraphAttrs {
  style?: string;
  align?: 'left' | 'center' | 'right' | 'justify' | null;
  lineSpacing?: number | null;
  spacingBefore?: number | null;
  spacingAfter?: number | null;
  indentStart?: number | null;
  indentEnd?: number | null;
  firstLineIndent?: number | null;
}

export type EditorBlockNode = EditorParagraphNode | EditorHeadingNode | EditorListNode;

export interface EditorDoc {
  type: 'doc';
  content: EditorBlockNode[];
}

export interface EditorProjectedChange {
  text: string;
  blocks: EditableBlock[];
}

export interface DocumentOutlineEntry {
  sectionIndex: number;
  blockIndex: number;
  editorBlockIndex: number;
  level: 1 | 2 | 3;
  text: string;
}

export type DocumentCommand =
  | {
      type: 'replace_block';
      section_index: number;
      block_index: number;
      block: EditableBlock;
    }
  | {
      type: 'insert_block';
      section_index: number;
      block_index: number;
      block: EditableBlock;
    }
  | {
      type: 'delete_block';
      section_index: number;
      block_index: number;
    }
  | {
      type: 'update_page_setup';
      section_index: number;
      page: PageSetup;
    }
  | {
      type: 'update_style';
      style: DocumentStyle;
    };

export function documentToText(document: DocumentState): string {
  const blocks: Block[] = [];
  for (const section of document.sections) {
    blocks.push(...section.blocks);
  }
  return blocks.map(blockToText).filter(Boolean).join('\n');
}

export function documentOutline(document: DocumentState): DocumentOutlineEntry[] {
  const entries: DocumentOutlineEntry[] = [];
  let editorBlockIndex = 0;
  document.sections.forEach((section, sectionIndex) => {
    section.blocks.forEach((block, blockIndex) => {
      const entry = outlineEntryFromBlock(block, sectionIndex, blockIndex, editorBlockIndex);
      if (entry) {
        entries.push(entry);
      }
      editorBlockIndex += 1;
    });
  });
  return entries;
}

export function documentOutlineFromEditableBlocks(blocks: EditableBlock[]): DocumentOutlineEntry[] {
  return blocks
    .map((block, index) => outlineEntryFromBlock(block, 0, index, index))
    .filter((entry): entry is DocumentOutlineEntry => entry !== undefined);
}

export function documentProjectionWarnings(document: DocumentState): string[] {
  const warnings = [];
  if (document.sections.length !== 1) {
    warnings.push('Multiple sections are preserved but read-only in the current editor projection.');
  }
  for (const section of document.sections) {
    for (const block of section.blocks) {
      if (!hasInlineContent(block) && !isListBlock(block)) {
        warnings.push(`${block.type} blocks are preserved but read-only in the editor projection.`);
      }
    }
  }
  return warnings;
}

function outlineEntryFromBlock(
  block: Block,
  sectionIndex: number,
  blockIndex: number,
  editorBlockIndex: number
): DocumentOutlineEntry | undefined {
  if (!hasInlineContent(block) || block.type !== 'Heading') {
    return undefined;
  }
  const level = Math.trunc(Number(block.value.level));
  if (level < 1 || level > 3) {
    return undefined;
  }
  const text = block.value.inlines.map((inline) => inline.text).join('').trim();
  if (text.length === 0) {
    return undefined;
  }
  return {
    sectionIndex,
    blockIndex,
    editorBlockIndex,
    level: level as 1 | 2 | 3,
    text
  };
}

export function canEditProjectedDocument(document: DocumentState): boolean {
  return documentProjectionWarnings(document).length === 0;
}

export function documentToEditorDoc(document: DocumentState): EditorDoc {
  const content: EditorBlockNode[] = [];
  for (const section of document.sections) {
    for (const block of section.blocks) {
      const node = blockToEditorNode(block, document.lists, document.styles);
      content.push(node);
    }
  }

  return {
    type: 'doc',
    content: content.length > 0 ? content : [{ type: 'paragraph' }]
  };
}

export function editorDocToWordCoreBlocks(
  editorDoc: EditorDoc,
  styles?: Record<string, DocumentStyle>
): EditableBlock[] {
  return editorDoc.content.map((block) => {
    if (block.type === 'bullet_list' || block.type === 'ordered_list') {
      return editorListToWordCoreBlock(block, styles);
    }
    if (block.type === 'heading') {
      const inlines = (block.content ?? []).map(editorTextToInline).filter((inline) => inline.text.length > 0);
      return {
        type: 'Heading',
        value: {
          level: clampHeadingLevel(block.attrs?.level ?? 1),
          inlines
        }
      };
    }

    if (block.type !== 'paragraph') {
      return { type: 'Paragraph', value: { style: 'body', inlines: [] } };
    }
    const inlines = (block.content ?? []).map(editorTextToInline).filter((inline) => inline.text.length > 0);
    const format = editorParagraphAttrsToFormat(block.attrs, styles);
    return {
      type: 'Paragraph',
      value: {
        style: block.attrs?.style || 'body',
        ...(format ? { format } : {}),
        inlines
      }
    };
  });
}

export function buildEditorSyncCommands(
  document: DocumentState,
  nextBlocks: EditableBlock[]
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
    if (blocksEqual(currentBlocks[index], nextBlocks[index])) {
      continue;
    }
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

function blocksEqual(left: Block, right: EditableBlock): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

function blockToText(block: Block): string {
  if (hasInlineContent(block)) {
    return block.value.inlines.map((inline) => inline.text).join('');
  }
  if (isListBlock(block)) {
    return block.value.items
      .map((item) => item.blocks.map(blockToText).filter(Boolean).join(' '))
      .filter(Boolean)
      .join('\n');
  }
  return '';
}

function hasInlineContent(block: Block): block is ParagraphBlock | HeadingBlock {
  return block.type === 'Paragraph' || block.type === 'Heading';
}

function isListBlock(block: Block): block is ListBlock {
  return block.type === 'List' && typeof block.value === 'object' && block.value !== null && 'items' in block.value;
}

function blockToEditorNode(
  block: Block,
  lists?: Record<string, ListDefinition>,
  styles?: Record<string, DocumentStyle>
): EditorBlockNode {
  if (isListBlock(block)) {
    return wordCoreListToEditorNode(block, lists, styles);
  }

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
    attrs: paragraphAttrsFromFormat(block.value.style || 'body', block.value.format, styles),
    content
  };
}

function editorTextToInline(textNode: EditorTextNode): Inline {
  const marks: string[] = [];
  let link: string | null = null;
  const style: InlineStyle = {};
  for (const mark of textNode.marks ?? []) {
    const mapped = mapEditorMark(mark.type);
    if (mapped) {
      marks.push(mapped);
    }
    if (mark.type === 'link' && mark.attrs?.href) {
      link = sanitizeEditorHref(String(mark.attrs.href)) ?? null;
    }
    if (mark.type === 'textStyle') {
      if (typeof mark.attrs?.fontFamily === 'string') {
        style.font_family = mark.attrs.fontFamily;
      }
      if (typeof mark.attrs?.fontSizePt === 'number') {
        style.font_size_pt = mark.attrs.fontSizePt;
      }
      if (typeof mark.attrs?.textColor === 'string') {
        style.text_color = mark.attrs.textColor;
      }
      if (typeof mark.attrs?.highlightColor === 'string') {
        style.highlight_color = mark.attrs.highlightColor;
      }
    }
  }

  const inline: Inline = {
    text: textNode.text,
    marks,
    link
  };
  if (!inlineStyleIsEmpty(style)) {
    inline.style = style;
  }
  return inline;
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
  if (inline.style && !inlineStyleIsEmpty(inline.style)) {
    marks.push({
      type: 'textStyle',
      attrs: {
        fontFamily: inline.style.font_family ?? null,
        fontSizePt: inline.style.font_size_pt ?? null,
        textColor: inline.style.text_color ?? null,
        highlightColor: inline.style.highlight_color ?? null
      }
    });
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

function wordCoreListToEditorNode(
  block: ListBlock,
  lists?: Record<string, ListDefinition>,
  styles?: Record<string, DocumentStyle>
): EditorListNode {
  const registryOrdered = lists?.[block.value.definition_id]?.ordered;
  const ordered =
    registryOrdered ?? (block.value.definition_id === '900w-ordered' || block.value.definition_id.endsWith('-ol'));
  return {
    type: ordered ? 'ordered_list' : 'bullet_list',
    attrs: { definitionId: block.value.definition_id },
    content: block.value.items.map((item) => ({
      type: 'list_item',
      attrs: { level: clampListLevel(item.level) },
      content:
        item.blocks.length > 0
          ? item.blocks.map((child): EditorParagraphNode | EditorHeadingNode => {
              const node = blockToEditorNode(child, lists, styles);
              if (node.type === 'paragraph' || node.type === 'heading') {
                return node;
              }
              return {
                type: 'paragraph' as const,
                attrs: { style: 'body' },
                content: [{ type: 'text' as const, text: '[Nested list preserved read-only]' }]
              };
            })
          : [{ type: 'paragraph', attrs: { style: 'body' }, content: [] }]
    }))
  };
}

function editorListToWordCoreBlock(block: EditorListNode, styles?: Record<string, DocumentStyle>): ListBlock {
  const definitionId =
    block.attrs?.definitionId || (block.type === 'ordered_list' ? '900w-ordered' : '900w-unordered');
  return {
    type: 'List',
    value: {
      definition_id: definitionId,
      items: block.content.map((item) => ({
        level: clampListLevel(item.attrs?.level ?? 1),
        blocks: (item.content ?? [])
          .map((child) => editorChildBlockToWordCore(child, styles))
          .filter((child): child is ParagraphBlock | HeadingBlock => child !== undefined)
      }))
    }
  };
}

function editorChildBlockToWordCore(
  block: EditorParagraphNode | EditorHeadingNode,
  styles?: Record<string, DocumentStyle>
): ParagraphBlock | HeadingBlock | undefined {
  const inlines = (block.content ?? []).map(editorTextToInline).filter((inline) => inline.text.length > 0);
  if (block.type === 'heading') {
    return {
      type: 'Heading',
      value: { level: clampHeadingLevel(block.attrs?.level ?? 1), inlines }
    };
  }
  const format = editorParagraphAttrsToFormat(block.attrs, styles);
  return {
    type: 'Paragraph',
    value: {
      style: block.attrs?.style || 'body',
      ...(format ? { format } : {}),
      inlines
    }
  };
}

function paragraphAttrsFromFormat(
  style: string,
  format: ParagraphFormat | undefined,
  styles?: Record<string, DocumentStyle>
): EditorParagraphAttrs {
  const attrs: EditorParagraphAttrs = { style };
  const styleFormat = styles?.[style]?.kind === 'Paragraph' ? styles?.[style]?.properties.paragraph : undefined;
  const merged = mergeParagraphFormats(styleFormat ?? undefined, format);
  if (merged.alignment !== undefined && merged.alignment !== null) attrs.align = merged.alignment;
  if (merged.line_spacing_per_mille !== undefined && merged.line_spacing_per_mille !== null) {
    attrs.lineSpacing = merged.line_spacing_per_mille;
  }
  if (merged.spacing_before_mm !== undefined && merged.spacing_before_mm !== null) {
    attrs.spacingBefore = merged.spacing_before_mm;
  }
  if (merged.spacing_after_mm !== undefined && merged.spacing_after_mm !== null) {
    attrs.spacingAfter = merged.spacing_after_mm;
  }
  if (merged.indent_start_mm !== undefined && merged.indent_start_mm !== null) {
    attrs.indentStart = merged.indent_start_mm;
  }
  if (merged.indent_end_mm !== undefined && merged.indent_end_mm !== null) {
    attrs.indentEnd = merged.indent_end_mm;
  }
  if (merged.first_line_indent_mm !== undefined && merged.first_line_indent_mm !== null) {
    attrs.firstLineIndent = merged.first_line_indent_mm;
  }
  return attrs;
}

function editorParagraphAttrsToFormat(
  attrs: EditorParagraphAttrs | undefined,
  styles?: Record<string, DocumentStyle>
): ParagraphFormat | undefined {
  const format: ParagraphFormat = {};
  if (attrs?.align !== undefined && attrs.align !== null) format.alignment = attrs.align;
  if (attrs?.lineSpacing !== undefined && attrs.lineSpacing !== null) format.line_spacing_per_mille = attrs.lineSpacing;
  if (attrs?.spacingBefore !== undefined && attrs.spacingBefore !== null) format.spacing_before_mm = attrs.spacingBefore;
  if (attrs?.spacingAfter !== undefined && attrs.spacingAfter !== null) format.spacing_after_mm = attrs.spacingAfter;
  if (attrs?.indentStart !== undefined && attrs.indentStart !== null) format.indent_start_mm = attrs.indentStart;
  if (attrs?.indentEnd !== undefined && attrs.indentEnd !== null) format.indent_end_mm = attrs.indentEnd;
  if (attrs?.firstLineIndent !== undefined && attrs.firstLineIndent !== null) {
    format.first_line_indent_mm = attrs.firstLineIndent;
  }
  const inherited =
    attrs?.style && styles?.[attrs.style]?.kind === 'Paragraph'
      ? styles[attrs.style]?.properties.paragraph
      : undefined;
  const direct = subtractInheritedParagraphFormat(format, inherited ?? undefined);
  return Object.keys(direct).length > 0 ? direct : undefined;
}

function mergeParagraphFormats(
  styleFormat: ParagraphFormat | undefined,
  directFormat: ParagraphFormat | undefined
): ParagraphFormat {
  return {
    ...(styleFormat ?? {}),
    ...(directFormat ?? {})
  };
}

function subtractInheritedParagraphFormat(format: ParagraphFormat, inherited: ParagraphFormat | undefined): ParagraphFormat {
  if (!inherited) {
    return format;
  }
  const direct: ParagraphFormat = {};
  if (format.alignment !== undefined && format.alignment !== inherited.alignment) direct.alignment = format.alignment;
  if (
    format.line_spacing_per_mille !== undefined &&
    format.line_spacing_per_mille !== inherited.line_spacing_per_mille
  ) {
    direct.line_spacing_per_mille = format.line_spacing_per_mille;
  }
  if (format.spacing_before_mm !== undefined && format.spacing_before_mm !== inherited.spacing_before_mm) {
    direct.spacing_before_mm = format.spacing_before_mm;
  }
  if (format.spacing_after_mm !== undefined && format.spacing_after_mm !== inherited.spacing_after_mm) {
    direct.spacing_after_mm = format.spacing_after_mm;
  }
  if (format.indent_start_mm !== undefined && format.indent_start_mm !== inherited.indent_start_mm) {
    direct.indent_start_mm = format.indent_start_mm;
  }
  if (format.indent_end_mm !== undefined && format.indent_end_mm !== inherited.indent_end_mm) {
    direct.indent_end_mm = format.indent_end_mm;
  }
  if (
    format.first_line_indent_mm !== undefined &&
    format.first_line_indent_mm !== inherited.first_line_indent_mm
  ) {
    direct.first_line_indent_mm = format.first_line_indent_mm;
  }
  return direct;
}

function inlineStyleIsEmpty(style: InlineStyle): boolean {
  return (
    !style.font_family &&
    !style.font_size_pt &&
    !style.text_color &&
    !style.highlight_color
  );
}

function clampListLevel(level: number): number {
  return Math.min(8, Math.max(1, Math.trunc(level)));
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
