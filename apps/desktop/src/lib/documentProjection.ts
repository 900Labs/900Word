import { sanitizeBookmarkId, sanitizeEditorHref } from './editorSecurity';

export interface Inline {
  text: string;
  marks?: string[];
  link?: string | null;
  comment_ids?: string[];
  style?: InlineStyle;
  field?: PageField | null;
  note_reference?: InlineNoteReference | null;
  tracked_change?: TrackedChange | null;
}

export type PageField = 'page_number' | 'page_count' | 'date';
export type NoteKind = 'footnote' | 'endnote';

export interface InlineNoteReference {
  id: string;
  kind: NoteKind;
  label: string;
}

export interface Note {
  id: string;
  kind: NoteKind;
  body: string;
}

export interface NoteSummary extends Note {
  label: string;
}

export type TrackedChangeKind = 'insertion' | 'deletion';

export interface TrackedChange {
  id: string;
  kind: TrackedChangeKind;
  author: string;
  created_at: string;
}

export interface TrackChangesState {
  recording?: boolean;
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
    bookmark_id?: string | null;
    style?: string;
    format?: ParagraphFormat;
    inlines: Inline[];
  };
}

export interface HeadingBlock {
  type: 'Heading';
  value: {
    bookmark_id?: string | null;
    level: number;
    inlines: Inline[];
  };
}

export interface TableOfContentsEntry {
  level: number;
  text: string;
  target_bookmark_id: string;
}

export interface TableOfContentsBlock {
  type: 'TableOfContents';
  value: {
    title: string;
    entries: TableOfContentsEntry[];
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

export interface CommentThread {
  id: string;
  author: string;
  body: string;
  created_at: string;
  updated_at: string;
  resolved?: boolean;
}

export type TableCellEditableBlock = ParagraphBlock | HeadingBlock | ListBlock;
export type EditableBlock = ParagraphBlock | HeadingBlock | TableOfContentsBlock | ListBlock | TableBlock | ImageBlock;
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
  track_changes?: TrackChangesState;
  styles?: Record<string, DocumentStyle>;
  lists?: Record<string, ListDefinition>;
  assets?: Record<string, AssetRef>;
  comments?: Record<string, CommentThread>;
  notes?: Record<string, Note>;
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

export interface EditorNoteReferenceNode {
  type: 'note_reference';
  attrs: InlineNoteReference;
}

export type EditorInlineNode = EditorTextNode | EditorNoteReferenceNode;

export interface EditorParagraphNode {
  type: 'paragraph';
  attrs?: EditorParagraphAttrs;
  content?: EditorInlineNode[];
}

export interface EditorHeadingNode {
  type: 'heading';
  attrs?: {
    level?: number;
    bookmarkId?: string | null;
  };
  content?: EditorInlineNode[];
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

export interface EditorTableOfContentsNode {
  type: 'table_of_contents';
  attrs: {
    title: string;
    entries: TableOfContentsEntry[];
  };
}

export interface EditorParagraphAttrs {
  bookmarkId?: string | null;
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
  | EditorTableOfContentsNode
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

export interface TrackedChangeSummary extends TrackedChange {
  text: string;
}

export interface DocumentOutlineEntry {
  sectionIndex: number;
  blockIndex: number;
  editorBlockIndex: number;
  level: 1 | 2 | 3;
  text: string;
  bookmarkId?: string | null;
}

export interface DocumentLinkTarget {
  id: string;
  label: string;
  kind: 'heading' | 'bookmark';
  sectionIndex: number;
  blockIndex: number;
  editorBlockIndex: number;
  level?: 1 | 2 | 3;
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
      type: 'insert_or_update_table_of_contents';
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
    }
  | {
      type: 'set_track_changes_recording';
      enabled: boolean;
    }
  | {
      type: 'accept_tracked_change';
      id: string;
    }
  | {
      type: 'reject_tracked_change';
      id: string;
    }
  | {
      type: 'accept_all_tracked_changes';
    }
  | {
      type: 'reject_all_tracked_changes';
    }
  | {
      type: 'insert_note';
      section_index: number;
      block_index: number;
      inline_index: number;
      id: string;
      kind: NoteKind;
      body: string;
      label?: string | null;
    }
  | {
      type: 'add_note';
      id: string;
      kind: NoteKind;
      body: string;
    }
  | {
      type: 'update_note';
      id: string;
      body: string;
    }
  | {
      type: 'delete_note';
      id: string;
    }
  | {
      type: 'add_comment';
      id: string;
      author?: string | null;
      body: string;
    }
  | {
      type: 'set_comment_resolved';
      id: string;
      resolved: boolean;
    }
  | {
      type: 'delete_comment';
      id: string;
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
  const text = blocks.map(blockToText).filter(Boolean).join('\n');
  const notes = documentNotesToText(document);
  return [text, notes].filter(Boolean).join('\n\n');
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

export function documentLinkTargets(document: DocumentState): DocumentLinkTarget[] {
  const targets: DocumentLinkTarget[] = [];
  let editorBlockIndex = 0;
  document.sections.forEach((section, sectionIndex) => {
    section.blocks.forEach((block, blockIndex) => {
      targets.push(...linkTargetsFromBlock(block, sectionIndex, blockIndex, editorBlockIndex));
      editorBlockIndex += 1;
    });
  });
  return uniqueLinkTargets(targets);
}

export function trackedChangesInDocument(document: DocumentState | undefined): TrackedChangeSummary[] {
  if (!document) {
    return [];
  }
  const changes = new Map<string, TrackedChangeSummary>();
  for (const section of document.sections) {
    for (const block of section.blocks) {
      collectTrackedChangesFromBlock(block, changes);
    }
  }
  return [...changes.values()].sort((left, right) => left.created_at.localeCompare(right.created_at));
}

export function noteSummariesInDocument(document: DocumentState | undefined): NoteSummary[] {
  if (!document) {
    return [];
  }
  return orderedNoteReferencesInDocument(document)
    .map((reference) => {
      const note = normalizeNote(document.notes?.[reference.id]);
      if (!note || note.kind !== reference.kind) {
        return null;
      }
      return {
        ...note,
        label: reference.label
      };
    })
    .filter((note): note is NoteSummary => note !== null);
}

export function nextNoteLabelInDocument(document: DocumentState | undefined, kind: NoteKind): string {
  const references = document ? orderedNoteReferencesInDocument(document).filter((reference) => reference.kind === kind) : [];
  const maxNumericLabel = references.reduce((max, reference) => {
    const parsed = Number.parseInt(reference.label, 10);
    return Number.isFinite(parsed) && String(parsed) === reference.label ? Math.max(max, parsed) : max;
  }, 0);
  return String(Math.max(maxNumericLabel, references.length) + 1);
}

export function documentOutlineFromEditableBlocks(blocks: EditableBlock[]): DocumentOutlineEntry[] {
  return blocks
    .map((block, index) => outlineEntryFromBlock(block, 0, index, index))
    .filter((entry): entry is DocumentOutlineEntry => entry !== undefined);
}

export function documentLinkTargetsFromEditableBlocks(blocks: EditableBlock[]): DocumentLinkTarget[] {
  return uniqueLinkTargets(
    blocks
      .flatMap((block, index) => linkTargetsFromBlock(block, 0, index, index))
  );
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
      if (isTableOfContentsBlock(block)) {
        if (normalizeTableOfContentsEntries(block.value.entries).length !== block.value.entries.length) {
          warnings.push('Table of contents entries with unsafe targets are preserved but read-only in the editor projection.');
        }
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
    text,
    ...optionalBookmarkId(block)
  };
}

function linkTargetFromBlock(
  block: Block,
  sectionIndex: number,
  blockIndex: number,
  editorBlockIndex: number
): DocumentLinkTarget | undefined {
  if (!hasInlineContent(block)) {
    return undefined;
  }
  const id = blockBookmarkId(block);
  if (!id) {
    return undefined;
  }
  const text = block.value.inlines.map(inlineToPlainText).join('').trim();
  const fallback = block.type === 'Heading' ? `Heading ${blockIndex + 1}` : `Bookmark ${blockIndex + 1}`;
  const level = block.type === 'Heading' ? Math.trunc(Number(block.value.level)) : undefined;
  return {
    id,
    label: text.length > 0 ? text : fallback,
    kind: block.type === 'Heading' ? 'heading' : 'bookmark',
    sectionIndex,
    blockIndex,
    editorBlockIndex,
    ...(level !== undefined && level >= 1 && level <= 3 ? { level: level as 1 | 2 | 3 } : {})
  };
}

function linkTargetsFromBlock(
  block: Block,
  sectionIndex: number,
  blockIndex: number,
  editorBlockIndex: number
): DocumentLinkTarget[] {
  const targets: DocumentLinkTarget[] = [];
  const ownTarget = linkTargetFromBlock(block, sectionIndex, blockIndex, editorBlockIndex);
  if (ownTarget) {
    targets.push(ownTarget);
  }
  if (isListBlock(block)) {
    for (const item of block.value.items) {
      for (const child of item.blocks) {
        targets.push(...linkTargetsFromBlock(child, sectionIndex, blockIndex, editorBlockIndex));
      }
    }
  } else if (isTableBlock(block)) {
    for (const row of block.value.rows) {
      for (const cell of row.cells) {
        for (const child of cell.blocks) {
          targets.push(...linkTargetsFromBlock(child, sectionIndex, blockIndex, editorBlockIndex));
        }
      }
    }
  }
  return targets;
}

function blockBookmarkId(block: Block): string | null {
  if (!hasInlineContent(block)) {
    return null;
  }
  return sanitizeBookmarkId(block.value.bookmark_id ?? '') ?? null;
}

function optionalBookmarkId(block: Block): { bookmarkId?: string } {
  const bookmarkId = blockBookmarkId(block);
  return bookmarkId ? { bookmarkId } : {};
}

function uniqueLinkTargets(targets: DocumentLinkTarget[]): DocumentLinkTarget[] {
  const seen = new Set<string>();
  const unique: DocumentLinkTarget[] = [];
  for (const target of targets) {
    if (seen.has(target.id)) {
      continue;
    }
    seen.add(target.id);
    unique.push(target);
  }
  return unique;
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
  if (isTableOfContentsBlock(block)) {
    return tableOfContentsToText(block);
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

function isTableOfContentsBlock(block: Block): block is TableOfContentsBlock {
  return (
    block.type === 'TableOfContents' &&
    typeof block.value === 'object' &&
    block.value !== null &&
    Array.isArray((block.value as { entries?: unknown }).entries)
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

  if (isTableOfContentsBlock(block)) {
    return wordCoreTableOfContentsToEditorNode(block);
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

  const content = block.value.inlines.map(inlineToEditorInline).filter(editorInlineHasContent);
  if (block.type === 'Heading') {
    return {
      type: 'heading',
      attrs: {
        level: clampHeadingLevel(block.value.level),
        ...bookmarkEditorAttr(block.value.bookmark_id)
      },
      content
    };
  }

  return {
    type: 'paragraph',
    attrs: {
      ...paragraphAttrsFromFormat(block.value.style || 'body', block.value.format, styles),
      ...bookmarkEditorAttr(block.value.bookmark_id)
    },
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
    case 'table_of_contents':
      return {
        type: 'TableOfContents',
        value: {
          title: normalizeOptionalString(block.attrs.title) ?? 'Contents',
          entries: normalizeTableOfContentsEntries(block.attrs.entries)
        }
      };
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
      const inlines = (block.content ?? []).map(editorInlineToInline).filter(inlineHasContent);
      return {
        type: 'Heading',
        value: {
          ...bookmarkWordCoreValue(block.attrs?.bookmarkId),
          level: clampHeadingLevel(block.attrs?.level ?? 1),
          inlines
        }
      };
    }
    case 'paragraph': {
      const inlines = (block.content ?? []).map(editorInlineToInline).filter(inlineHasContent);
      const format = editorParagraphAttrsToFormat(block.attrs, styles);
      return {
        type: 'Paragraph',
        value: {
          ...bookmarkWordCoreValue(block.attrs?.bookmarkId),
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

function wordCoreTableOfContentsToEditorNode(block: TableOfContentsBlock): EditorTableOfContentsNode {
  return {
    type: 'table_of_contents',
    attrs: {
      title: normalizeOptionalString(block.value.title) ?? 'Contents',
      entries: normalizeTableOfContentsEntries(block.value.entries)
    }
  };
}

function normalizeTableOfContentsEntries(value: unknown): TableOfContentsEntry[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value
    .map((entry): TableOfContentsEntry | undefined => {
      if (typeof entry !== 'object' || entry === null) {
        return undefined;
      }
      const level = Math.trunc(Number((entry as { level?: unknown }).level));
      const text = normalizeOptionalString((entry as { text?: unknown }).text);
      const target = sanitizeBookmarkId(String((entry as { target_bookmark_id?: unknown }).target_bookmark_id ?? ''));
      if (!text || !target || level < 1 || level > 3) {
        return undefined;
      }
      return { level, text, target_bookmark_id: target };
    })
    .filter((entry): entry is TableOfContentsEntry => entry !== undefined);
}

function imageToText(block: ImageBlock): string {
  return [block.value.alt_text, block.value.presentation?.caption]
    .map((value) => value?.trim() ?? '')
    .filter(Boolean)
    .join('\n');
}

function tableOfContentsToText(block: TableOfContentsBlock): string {
  return [
    block.value.title?.trim() ?? '',
    ...block.value.entries.map((entry) => `${'  '.repeat(Math.max(0, Math.min(2, entry.level - 1)))}${entry.text}`)
  ]
    .filter(Boolean)
    .join('\n');
}

function documentNotesToText(document: DocumentState): string {
  const references = orderedNoteReferencesInDocument(document);
  const footnotes = noteLinesForKind(references, document, 'footnote');
  const endnotes = noteLinesForKind(references, document, 'endnote');
  return [
    footnotes.length > 0 ? ['Footnotes', ...footnotes].join('\n') : '',
    endnotes.length > 0 ? ['Endnotes', ...endnotes].join('\n') : ''
  ]
    .filter(Boolean)
    .join('\n\n');
}

function noteLinesForKind(
  references: InlineNoteReference[],
  document: DocumentState,
  kind: NoteKind
): string[] {
  return references
    .filter((reference) => reference.kind === kind)
    .map((reference) => {
      const note = normalizeNote(document.notes?.[reference.id]);
      if (!note || note.kind !== reference.kind) {
        return null;
      }
      return `[${reference.label}] ${note.body}`;
    })
    .filter((line): line is string => line !== null);
}

function orderedNoteReferencesInDocument(document: DocumentState): InlineNoteReference[] {
  const references: InlineNoteReference[] = [];
  const seen = new Set<string>();
  for (const section of document.sections) {
    for (const block of section.blocks) {
      collectNoteReferencesFromBlock(block, references, seen);
    }
  }
  return references;
}

function collectNoteReferencesFromBlock(
  block: Block,
  references: InlineNoteReference[],
  seen: Set<string>
) {
  if (hasInlineContent(block)) {
    collectNoteReferencesFromInlines(block.value.inlines, references, seen);
    return;
  }
  if (isListBlock(block)) {
    for (const item of block.value.items) {
      for (const child of item.blocks) {
        collectNoteReferencesFromBlock(child, references, seen);
      }
    }
  } else if (isTableBlock(block)) {
    for (const row of block.value.rows) {
      for (const cell of row.cells) {
        for (const child of cell.blocks) {
          collectNoteReferencesFromBlock(child, references, seen);
        }
      }
    }
  }
}

function collectNoteReferencesFromInlines(
  inlines: Inline[],
  references: InlineNoteReference[],
  seen: Set<string>
) {
  for (const inline of inlines) {
    const reference = normalizeNoteReference(inline.note_reference);
    if (!reference || seen.has(reference.id)) {
      continue;
    }
    seen.add(reference.id);
    references.push(reference);
  }
}

function normalizeNote(value: unknown): Note | null {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const candidate = value as Partial<Note>;
  const id = sanitizeNoteId(candidate.id);
  const kind = normalizeNoteKind(candidate.kind);
  const body = typeof candidate.body === 'string' ? candidate.body.trim() : '';
  if (!id || !kind || body.length === 0 || Array.from(body).length > 4000) {
    return null;
  }
  return { id, kind, body };
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

function editorInlineToInline(node: EditorInlineNode): Inline {
  if (node.type === 'note_reference') {
    const reference = normalizeNoteReference(node.attrs);
    if (!reference) {
      return { text: '', marks: [], link: null };
    }
    return {
      text: reference.label,
      marks: [],
      link: null,
      note_reference: reference
    };
  }
  return editorTextToInline(node);
}

function editorTextToInline(textNode: EditorTextNode): Inline {
  const marks: string[] = [];
  let link: string | null = null;
  const commentIds: string[] = [];
  const style: InlineStyle = {};
  let trackedChange: TrackedChange | null = null;
  for (const mark of textNode.marks ?? []) {
    const mapped = mapEditorMark(mark.type);
    if (mapped) {
      marks.push(mapped);
    }
    if (mark.type === 'link' && mark.attrs?.href) {
      link = sanitizeEditorHref(String(mark.attrs.href)) ?? null;
    }
    if (mark.type === 'comment' && typeof mark.attrs?.id === 'string') {
      const commentId = sanitizeCommentId(mark.attrs.id);
      if (commentId && !commentIds.includes(commentId)) {
        commentIds.push(commentId);
      }
    }
    if (mark.type === 'trackedChange') {
      const change = trackedChangeFromMarkAttrs(mark.attrs);
      if (change) {
        trackedChange = change;
      }
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
  if (commentIds.length > 0) {
    inline.comment_ids = commentIds;
  }
  if (!inlineStyleIsEmpty(style)) {
    inline.style = style;
  }
  if (trackedChange) {
    inline.tracked_change = trackedChange;
  }
  return inline;
}

function inlineToEditorInline(inline: Inline): EditorInlineNode {
  const reference = normalizeNoteReference(inline.note_reference);
  if (reference) {
    return {
      type: 'note_reference',
      attrs: reference
    };
  }

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
  for (const commentId of inline.comment_ids ?? []) {
    const id = sanitizeCommentId(commentId);
    if (id) {
      marks.push({ type: 'comment', attrs: { id } });
    }
  }
  const trackedChange = normalizeTrackedChange(inline.tracked_change);
  if (trackedChange) {
    marks.push({
      type: 'trackedChange',
      attrs: {
        id: trackedChange.id,
        kind: trackedChange.kind,
        author: trackedChange.author,
        createdAt: trackedChange.created_at
      }
    });
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

function editorInlineHasContent(node: EditorInlineNode): boolean {
  return node.type === 'note_reference' || node.text.length > 0;
}

function inlineHasContent(inline: Inline): boolean {
  return inline.text.length > 0 || normalizeNoteReference(inline.note_reference) !== null;
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
  const reference = normalizeNoteReference(inline.note_reference);
  if (reference) {
    return reference.label;
  }
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

function collectTrackedChangesFromBlock(block: Block, changes: Map<string, TrackedChangeSummary>) {
  if (hasInlineContent(block)) {
    collectTrackedChangesFromInlines(block.value.inlines, changes);
    return;
  }
  if (isListBlock(block)) {
    for (const item of block.value.items) {
      for (const child of item.blocks) {
        collectTrackedChangesFromBlock(child, changes);
      }
    }
  } else if (isTableBlock(block)) {
    for (const row of block.value.rows) {
      for (const cell of row.cells) {
        for (const child of cell.blocks) {
          collectTrackedChangesFromBlock(child, changes);
        }
      }
    }
  }
}

function collectTrackedChangesFromInlines(inlines: Inline[], changes: Map<string, TrackedChangeSummary>) {
  for (const inline of inlines) {
    const change = normalizeTrackedChange(inline.tracked_change);
    if (!change) {
      continue;
    }
    const existing = changes.get(change.id);
    if (existing) {
      existing.text += inline.text;
    } else {
      changes.set(change.id, { ...change, text: inline.text });
    }
  }
}

function trackedChangeFromMarkAttrs(attrs: Record<string, string | number | null> | undefined): TrackedChange | null {
  const id = sanitizeTrackedChangeId(String(attrs?.id ?? ''));
  const kind = normalizeTrackedChangeKind(attrs?.kind);
  if (!id || !kind) {
    return null;
  }
  return {
    id,
    kind,
    author: normalizeTrackedChangeAuthor(attrs?.author),
    created_at: normalizeTrackedChangeTimestamp(attrs?.createdAt)
  };
}

function normalizeTrackedChange(value: unknown): TrackedChange | null {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const candidate = value as Partial<TrackedChange>;
  const id = sanitizeTrackedChangeId(candidate.id ?? '');
  const kind = normalizeTrackedChangeKind(candidate.kind);
  if (!id || !kind) {
    return null;
  }
  return {
    id,
    kind,
    author: normalizeTrackedChangeAuthor(candidate.author),
    created_at: normalizeTrackedChangeTimestamp(candidate.created_at)
  };
}

export function sanitizeTrackedChangeId(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }
  const trimmed = value.trim();
  const suffix = trimmed.startsWith('chg-') ? trimmed.slice(4) : '';
  if (trimmed.length > 64 || suffix.length === 0) {
    return null;
  }
  return /^[A-Za-z0-9_-]+$/.test(suffix) ? trimmed : null;
}

export function sanitizeNoteId(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }
  const trimmed = value.trim();
  const suffix = trimmed.startsWith('note-') ? trimmed.slice(5) : '';
  if (trimmed.length > 64 || suffix.length === 0) {
    return null;
  }
  return /^[A-Za-z0-9_-]+$/.test(suffix) ? trimmed : null;
}

export function sanitizeNoteLabel(value: unknown): string | null {
  if (typeof value !== 'string') {
    return null;
  }
  const label = value.replace(/[\u0000-\u001f\u007f]/g, '').trim();
  return label.length > 0 && Array.from(label).length <= 16 ? label : null;
}

function normalizeNoteKind(value: unknown): NoteKind | null {
  return value === 'footnote' || value === 'endnote' ? value : null;
}

function normalizeNoteReference(value: unknown): InlineNoteReference | null {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const candidate = value as Partial<InlineNoteReference>;
  const id = sanitizeNoteId(candidate.id);
  const kind = normalizeNoteKind(candidate.kind);
  const label = sanitizeNoteLabel(candidate.label);
  if (!id || !kind || !label) {
    return null;
  }
  return { id, kind, label };
}

function normalizeTrackedChangeKind(value: unknown): TrackedChangeKind | null {
  return value === 'insertion' || value === 'deletion' ? value : null;
}

function normalizeTrackedChangeAuthor(value: unknown): string {
  if (typeof value !== 'string') {
    return 'Local User';
  }
  const author = value.replace(/[\u0000-\u001f\u007f]/g, '').trim();
  return author.length > 0 && Array.from(author).length <= 80 ? author : 'Local User';
}

function normalizeTrackedChangeTimestamp(value: unknown): string {
  if (typeof value !== 'string') {
    return new Date(0).toISOString();
  }
  const timestamp = Date.parse(value);
  return Number.isFinite(timestamp) ? new Date(timestamp).toISOString() : new Date(0).toISOString();
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
  const inlines = (block.content ?? []).map(editorInlineToInline).filter(inlineHasContent);
  if (block.type === 'heading') {
    return {
      type: 'Heading',
      value: {
        ...bookmarkWordCoreValue(block.attrs?.bookmarkId),
        level: clampHeadingLevel(block.attrs?.level ?? 1),
        inlines
      }
    };
  }
  const format = editorParagraphAttrsToFormat(block.attrs, styles);
  return {
    type: 'Paragraph',
    value: {
      ...bookmarkWordCoreValue(block.attrs?.bookmarkId),
      style: block.attrs?.style || 'body',
      ...(format ? { format } : {}),
      inlines
    }
  };
}

function bookmarkEditorAttr(bookmarkId: unknown): { bookmarkId?: string } {
  const sanitized = typeof bookmarkId === 'string' ? sanitizeBookmarkId(bookmarkId) : undefined;
  return sanitized ? { bookmarkId: sanitized } : {};
}

function bookmarkWordCoreValue(bookmarkId: unknown): { bookmark_id?: string } {
  const sanitized = typeof bookmarkId === 'string' ? sanitizeBookmarkId(bookmarkId) : undefined;
  return sanitized ? { bookmark_id: sanitized } : {};
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

export function sanitizeCommentId(value: string): string | null {
  const trimmed = value.trim();
  if (trimmed.length === 0 || trimmed.length > 64 || !trimmed.startsWith('cmt-')) {
    return null;
  }
  const suffix = trimmed.slice(4);
  return /^[A-Za-z0-9_-]+$/.test(suffix) ? trimmed : null;
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
