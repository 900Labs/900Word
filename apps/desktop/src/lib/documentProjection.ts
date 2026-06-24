import { sanitizeEditorHref } from './editorSecurity';

export interface Inline {
  text: string;
  marks?: string[];
  link?: string | null;
  style?: InlineStyle;
  field?: PageField | null;
}

export type PageField = 'page_number' | 'page_count' | 'date';

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

export interface TableCell {
  blocks: Block[];
}

export interface TableRow {
  cells: TableCell[];
}

export interface TableBlock {
  type: 'Table';
  value: {
    rows: TableRow[];
  };
}

export interface ImageBlock {
  type: 'Image';
  value: {
    asset_id: string;
    presentation?: ImagePresentation | null;
    alt_text?: string | null;
  };
}

export interface ImagePresentation {
  alignment?: 'inline' | 'left' | 'center' | 'right' | null;
  scale_percent?: number | null;
  caption?: string | null;
}

export interface AssetRef {
  id: string;
  media_type: string;
  byte_len: number;
  bytes: number[];
  original_name?: string | null;
}

export type TableCellEditableBlock = ParagraphBlock | HeadingBlock | ListBlock;
export type EditableBlock = ParagraphBlock | HeadingBlock | ListBlock | TableBlock | ImageBlock;
export type Block = EditableBlock | { type: string; value?: unknown };

export interface PageRegionParagraphBlock {
  type: 'Paragraph';
  value: {
    inlines: Inline[];
  };
}

export type PageRegionBlock = PageRegionParagraphBlock;

export interface PageRegion {
  blocks: PageRegionBlock[];
  read_only?: boolean;
}

export interface PageRegions {
  header?: PageRegion;
  footer?: PageRegion;
  first_header?: PageRegion;
  first_footer?: PageRegion;
  different_first_page?: boolean;
}

export type PageRegionKind = 'header' | 'footer' | 'first_header' | 'first_footer';

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
  assets?: Record<string, AssetRef>;
  sections: Array<{
    blocks: Block[];
    page?: PageSetup;
    page_regions?: PageRegions;
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

export type EditorTableCellBlockNode = EditorParagraphNode | EditorHeadingNode | EditorListNode;

export interface EditorTableCellNode {
  type: 'table_cell';
  attrs?: {
    unsupported?: boolean;
    sourceEmpty?: boolean;
  };
  content: EditorTableCellBlockNode[];
}

export interface EditorTableRowNode {
  type: 'table_row';
  content: EditorTableCellNode[];
}

export interface EditorTableNode {
  type: 'table';
  content: EditorTableRowNode[];
}

export interface EditorImageNode {
  type: 'image';
  attrs: {
    assetId: string;
    altText?: string | null;
    alignment?: 'inline' | 'left' | 'center' | 'right' | null;
    scalePercent?: number | null;
    caption?: string | null;
    src?: string | null;
  };
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

export type EditorBlockNode =
  | EditorParagraphNode
  | EditorHeadingNode
  | EditorListNode
  | EditorTableNode
  | EditorImageNode;

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
      type: 'update_page_region';
      section_index: number;
      region: PageRegionKind;
      blocks: PageRegionBlock[];
    }
  | {
      type: 'set_different_first_page';
      section_index: number;
      enabled: boolean;
    }
  | {
      type: 'update_style';
      style: DocumentStyle;
    };

export const pageFieldTokens: Record<PageField, string> = {
  page_number: '{{page_number}}',
  page_count: '{{page_count}}',
  date: '{{date}}'
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
    for (const region of Object.values(section.page_regions ?? {})) {
      if (typeof region === 'object' && region !== null && 'read_only' in region && region.read_only) {
        warnings.push('Header/footer content with unsupported ODT structure is preserved read-only.');
        break;
      }
    }
    for (const block of section.blocks) {
      if (blockHasPageField(block)) {
        warnings.push('Page fields in the document body are preserved but read-only in the editor projection.');
        continue;
      }
      if (isTableBlock(block)) {
        if (!isEditableTableBlock(block)) {
          warnings.push('Tables with unsupported or structurally empty content are preserved but read-only in the editor projection.');
        }
        continue;
      }
      if (isImageBlock(block)) {
        continue;
      }
      if (!hasInlineContent(block) && !isEditableListBlock(block)) {
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
  const text = block.value.inlines.map(inlineToPlainText).join('').trim();
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
      const node = blockToEditorNode(block, document.lists, document.styles, document.assets);
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
  return editorDoc.content.map((block) => editorBlockToWordCoreBlock(block, styles));
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
    return block.value.inlines.map(inlineToPlainText).join('');
  }
  if (isListBlock(block)) {
    return block.value.items
      .map((item) => item.blocks.map(blockToText).filter(Boolean).join(' '))
      .filter(Boolean)
      .join('\n');
  }
  if (isTableBlock(block)) {
    return block.value.rows
      .map((row) =>
        row.cells
          .map((cell) => cell.blocks.map(blockToText).filter(Boolean).join(' '))
          .filter(Boolean)
          .join('\t')
      )
      .filter(Boolean)
      .join('\n');
  }
  if (isImageBlock(block)) {
    return imageToText(block);
  }
  return '';
}

function blockHasPageField(block: Block): boolean {
  if (hasInlineContent(block)) {
    return block.value.inlines.some((inline) => inline.field !== undefined && inline.field !== null);
  }
  if (isListBlock(block)) {
    return block.value.items.some((item) => item.blocks.some(blockHasPageField));
  }
  if (isTableBlock(block)) {
    return block.value.rows.some((row) =>
      row.cells.some((cell) => cell.blocks.some(blockHasPageField))
    );
  }
  return false;
}

function hasInlineContent(block: Block): block is ParagraphBlock | HeadingBlock {
  return block.type === 'Paragraph' || block.type === 'Heading';
}

function isListBlock(block: Block): block is ListBlock {
  return block.type === 'List' && typeof block.value === 'object' && block.value !== null && 'items' in block.value;
}

function isEditableListBlock(block: Block): block is ListBlock {
  return (
    isListBlock(block) &&
    block.value.items.length > 0 &&
    block.value.items.every((item) => item.blocks.length > 0 && item.blocks.every((child) => hasInlineContent(child)))
  );
}

function isTableBlock(block: Block): block is TableBlock {
  if (block.type !== 'Table' || typeof block.value !== 'object' || block.value === null) {
    return false;
  }
  const rows = (block.value as { rows?: unknown }).rows;
  return (
    Array.isArray(rows) &&
    rows.every(
      (row) =>
        typeof row === 'object' &&
        row !== null &&
        Array.isArray((row as { cells?: unknown }).cells) &&
        (row as { cells: unknown[] }).cells.every(
          (cell) =>
            typeof cell === 'object' &&
            cell !== null &&
            Array.isArray((cell as { blocks?: unknown }).blocks)
        )
    )
  );
}

function isImageBlock(block: Block): block is ImageBlock {
  return (
    block.type === 'Image' &&
    typeof block.value === 'object' &&
    block.value !== null &&
    typeof (block.value as { asset_id?: unknown }).asset_id === 'string'
  );
}

function blockToEditorNode(
  block: Block,
  lists?: Record<string, ListDefinition>,
  styles?: Record<string, DocumentStyle>,
  assets?: Record<string, AssetRef>
): EditorBlockNode {
  if (isImageBlock(block)) {
    return wordCoreImageToEditorNode(block, assets);
  }

  if (isTableBlock(block)) {
    return wordCoreTableToEditorNode(block, lists, styles);
  }

  if (isListBlock(block)) {
    if (!isEditableListBlock(block)) {
      return readOnlyPlaceholderNode('List');
    }
    return wordCoreListToEditorNode(block, lists, styles);
  }

  if (!hasInlineContent(block)) {
    return readOnlyPlaceholderNode(block.type);
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

function readOnlyPlaceholderNode(blockType: string): EditorParagraphNode {
  return {
    type: 'paragraph',
    content: [
      {
        type: 'text',
        text: `[${blockType} block preserved read-only]`
      }
    ]
  };
}

function editorBlockToWordCoreBlock(block: EditorBlockNode, styles?: Record<string, DocumentStyle>): EditableBlock {
  switch (block.type) {
    case 'image':
      return {
        type: 'Image',
        value: {
          asset_id: String(block.attrs.assetId),
          presentation: {
            alignment: normalizeImageAlignment(block.attrs.alignment),
            scale_percent: normalizeImageScale(block.attrs.scalePercent),
            caption: normalizeOptionalString(block.attrs.caption)
          },
          alt_text: normalizeOptionalString(block.attrs.altText) ?? 'Image'
        }
      };
    case 'table':
      return editorTableToWordCoreBlock(block, styles);
    case 'bullet_list':
    case 'ordered_list':
      return editorListToWordCoreBlock(block, styles);
    case 'heading': {
      const inlines = (block.content ?? []).map(editorTextToInline).filter((inline) => inline.text.length > 0);
      return {
        type: 'Heading',
        value: {
          level: clampHeadingLevel(block.attrs?.level ?? 1),
          inlines
        }
      };
    }
    case 'paragraph': {
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
    }
  }
}

function wordCoreImageToEditorNode(block: ImageBlock, assets?: Record<string, AssetRef>): EditorImageNode {
  return {
    type: 'image',
    attrs: {
      assetId: block.value.asset_id,
      altText: normalizeOptionalString(block.value.alt_text) ?? 'Image',
      alignment: normalizeImageAlignment(block.value.presentation?.alignment),
      scalePercent: normalizeImageScale(block.value.presentation?.scale_percent),
      caption: normalizeOptionalString(block.value.presentation?.caption),
      src: imageDataUrl(assets?.[block.value.asset_id])
    }
  };
}

function imageToText(block: ImageBlock): string {
  return [block.value.alt_text, block.value.presentation?.caption]
    .map((value) => value?.trim() ?? '')
    .filter(Boolean)
    .join('\n');
}

function normalizeImageAlignment(value: unknown): 'inline' | 'left' | 'center' | 'right' {
  return value === 'left' || value === 'center' || value === 'right' || value === 'inline'
    ? value
    : 'inline';
}

function normalizeImageScale(value: unknown): number {
  const scale = Number(value);
  if (!Number.isFinite(scale)) {
    return 100;
  }
  return Math.min(200, Math.max(25, Math.round(scale)));
}

function normalizeOptionalString(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

function imageDataUrl(asset: AssetRef | undefined): string | null {
  if (!asset || !allowlistedImageMediaType(asset.media_type) || asset.byte_len !== asset.bytes.length) {
    return null;
  }
  return `data:${asset.media_type};base64,${base64FromBytes(asset.bytes)}`;
}

function allowlistedImageMediaType(mediaType: string): boolean {
  return ['image/png', 'image/jpeg', 'image/gif', 'image/webp'].includes(mediaType);
}

function base64FromBytes(bytes: number[]): string {
  const alphabet = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  let output = '';
  for (let index = 0; index < bytes.length; index += 3) {
    const first = bytes[index] ?? 0;
    const second = bytes[index + 1] ?? 0;
    const third = bytes[index + 2] ?? 0;
    const value = (first << 16) | (second << 8) | third;
    output += alphabet[(value >> 18) & 63];
    output += alphabet[(value >> 12) & 63];
    output += index + 1 < bytes.length ? alphabet[(value >> 6) & 63] : '=';
    output += index + 2 < bytes.length ? alphabet[value & 63] : '=';
  }
  return output;
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
    text: inlineToPlainText(inline),
    ...(marks.length > 0 ? { marks } : {})
  };
}

export function pageRegionToText(region: PageRegion | undefined): string {
  return (region?.blocks ?? [])
    .map((block) => block.value.inlines.map(inlineToTokenText).join(''))
    .join('\n');
}

export function pageRegionTextToBlocks(text: string): PageRegionBlock[] {
  if (text.trim().length === 0) {
    return [];
  }
  return text.split(/\r?\n/).map((line) => ({
    type: 'Paragraph',
    value: {
      inlines: parsePageFieldTokens(line)
    }
  }));
}

export function pageRegionIsReadOnly(region: PageRegion | undefined): boolean {
  return Boolean(region?.read_only);
}

function inlineToPlainText(inline: Inline): string {
  if (inline.field === 'page_number' || inline.field === 'page_count') {
    return '1';
  }
  if (inline.field === 'date') {
    return pageFieldTokens.date;
  }
  return inline.text;
}

function inlineToTokenText(inline: Inline): string {
  return inline.field ? pageFieldTokens[inline.field] : inline.text;
}

function parsePageFieldTokens(text: string): Inline[] {
  const tokenPattern = /(\{\{page_number\}\}|\{\{page_count\}\}|\{\{date\}\})/g;
  const inlines: Inline[] = [];
  let offset = 0;
  for (const match of text.matchAll(tokenPattern)) {
    if (match.index === undefined) {
      continue;
    }
    if (match.index > offset) {
      inlines.push({ text: text.slice(offset, match.index), marks: [], link: null });
    }
    const field = pageFieldFromToken(match[0]);
    if (field) {
      inlines.push({ text: field === 'date' ? '1970-01-01' : '1', marks: [], link: null, field });
    }
    offset = match.index + match[0].length;
  }
  if (offset < text.length) {
    inlines.push({ text: text.slice(offset), marks: [], link: null });
  }
  return inlines;
}

function pageFieldFromToken(token: string): PageField | undefined {
  return Object.entries(pageFieldTokens).find(([, value]) => value === token)?.[0] as PageField | undefined;
}

function isEditableTableBlock(block: TableBlock): boolean {
  return (
    block.value.rows.length > 0 &&
    block.value.rows.every((row) =>
      row.cells.length > 0 && row.cells.every((cell) => cell.blocks.every((child) => isSupportedTableCellBlock(child)))
    )
  );
}

function isSupportedTableCellBlock(block: Block): block is TableCellEditableBlock {
  return hasInlineContent(block) || isEditableListBlock(block);
}

function wordCoreTableToEditorNode(
  block: TableBlock,
  lists?: Record<string, ListDefinition>,
  styles?: Record<string, DocumentStyle>
): EditorTableNode {
  const rows = block.value.rows.length > 0 ? block.value.rows : [{ cells: [{ blocks: [] }] }];
  return {
    type: 'table',
    content: rows.map((row): EditorTableRowNode => {
      const cells = row.cells.length > 0 ? row.cells : [{ blocks: [] }];
      return {
        type: 'table_row',
        content: cells.map((cell): EditorTableCellNode => {
          if (cell.blocks.some((child) => !isSupportedTableCellBlock(child))) {
            return unsupportedTableCellNode();
          }
          return {
            type: 'table_cell',
            attrs: { unsupported: false, sourceEmpty: cell.blocks.length === 0 },
            content:
              cell.blocks.length > 0
                ? cell.blocks.map((child) => blockToEditorNode(child, lists, styles) as EditorTableCellBlockNode)
                : [emptyEditorParagraphNode()]
          };
        })
      };
    })
  };
}

function unsupportedTableCellNode(): EditorTableCellNode {
  return {
    type: 'table_cell',
    attrs: { unsupported: true },
    content: [
      {
        type: 'paragraph',
        attrs: { style: 'body' },
        content: [{ type: 'text', text: '[Unsupported table cell content preserved read-only]' }]
      }
    ]
  };
}

function emptyEditorParagraphNode(): EditorParagraphNode {
  return { type: 'paragraph', attrs: { style: 'body' }, content: [] };
}

function editorTableToWordCoreBlock(block: EditorTableNode, styles?: Record<string, DocumentStyle>): TableBlock {
  return {
    type: 'Table',
    value: {
      rows: block.content.map((row) => ({
        cells: row.content.map((cell) => ({
          blocks: editorTableCellToWordCoreBlocks(cell, styles)
        }))
      }))
    }
  };
}

function editorTableCellToWordCoreBlocks(
  cell: EditorTableCellNode,
  styles?: Record<string, DocumentStyle>
): TableCellEditableBlock[] {
  if (cell.attrs?.unsupported) {
    return [{ type: 'Paragraph', value: { style: 'body', inlines: [] } }];
  }
  if (cell.attrs?.sourceEmpty && cell.content.length === 1 && isEmptyEditorCellParagraph(cell.content[0])) {
    return [];
  }
  const blocks = (cell.content ?? []).map((child) => editorTableCellBlockToWordCore(child, styles));
  return blocks.length > 0 ? blocks : [{ type: 'Paragraph', value: { style: 'body', inlines: [] } }];
}

function isEmptyEditorCellParagraph(block: EditorTableCellBlockNode): block is EditorParagraphNode {
  return block.type === 'paragraph' && (block.attrs?.style ?? 'body') === 'body' && (block.content ?? []).length === 0;
}

function editorTableCellBlockToWordCore(
  block: EditorTableCellBlockNode,
  styles?: Record<string, DocumentStyle>
): TableCellEditableBlock {
  switch (block.type) {
    case 'bullet_list':
    case 'ordered_list':
      return editorListToWordCoreBlock(block, styles);
    case 'paragraph':
    case 'heading':
      return editorChildBlockToWordCore(block, styles) ?? { type: 'Paragraph', value: { style: 'body', inlines: [] } };
  }
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
