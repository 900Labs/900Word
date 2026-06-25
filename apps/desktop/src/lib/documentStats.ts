import {
  noteSummariesInDocument,
  trackedChangesInDocument,
  type Block,
  type DocumentState,
  type HeadingBlock,
  type Inline,
  type ListBlock,
  type PageSetup,
  type ParagraphBlock,
  type TableBlock
} from './documentProjection';

export interface CoreDocumentStats {
  word_count: number;
  character_count: number;
  block_count: number;
}

export interface ExpandedDocumentStats {
  wordCount: number;
  characterCountWithSpaces: number;
  characterCountWithoutSpaces: number;
  paragraphCount: number;
  blockCount: number;
  estimatedPageCount: number;
  estimatedReadingMinutes: number;
  selectionWordCount: number;
  commentCount: number;
  unresolvedCommentCount: number;
  trackChangesRecording: boolean;
  trackedChangeCount: number;
  imageCount: number;
  assetCount: number;
  footnoteCount: number;
  endnoteCount: number;
  pageSize: string;
}

export interface ExpandedDocumentStatsInput {
  coreStats: CoreDocumentStats;
  document: DocumentState | undefined;
  plainText: string;
  selectionWordCount: number;
  pageSetup: PageSetup | undefined;
}

const estimatedWordsPerPage = 500;
const estimatedReadingWordsPerMinute = 200;

export function buildExpandedDocumentStats(input: ExpandedDocumentStatsInput): ExpandedDocumentStats {
  const notes = noteSummariesInDocument(input.document);
  const comments = Object.values(input.document?.comments ?? {});
  const characterSourceText = input.document ? documentStatsText(input.document, notes) : input.plainText;

  return {
    wordCount: input.coreStats.word_count,
    characterCountWithSpaces: input.coreStats.character_count,
    characterCountWithoutSpaces: countCharactersWithoutWhitespace(characterSourceText),
    paragraphCount: countDocumentParagraphs(input.document),
    blockCount: input.coreStats.block_count,
    estimatedPageCount: estimatePageCount(input.coreStats.word_count),
    estimatedReadingMinutes: estimateReadingMinutes(input.coreStats.word_count),
    selectionWordCount: Math.max(0, Math.trunc(input.selectionWordCount)),
    commentCount: comments.length,
    unresolvedCommentCount: comments.filter((comment) => !comment.resolved).length,
    trackChangesRecording: Boolean(input.document?.track_changes?.recording),
    trackedChangeCount: trackedChangesInDocument(input.document).length,
    imageCount: countDocumentImages(input.document),
    assetCount: Object.keys(input.document?.assets ?? {}).length,
    footnoteCount: notes.filter((note) => note.kind === 'footnote').length,
    endnoteCount: notes.filter((note) => note.kind === 'endnote').length,
    pageSize: formatPageSize(input.pageSetup ?? input.document?.sections[0]?.page)
  };
}

function documentStatsText(document: DocumentState, notes = noteSummariesInDocument(document)): string {
  const blockText = document.sections.flatMap((section) => section.blocks.map(blockStatsText));
  const noteText = notes.map((note) => note.body);
  return [...blockText, ...noteText].join('\n');
}

function blockStatsText(block: Block): string {
  if (hasInlineContent(block)) {
    return block.value.inlines.map(inlineStatsText).join('');
  }
  if (isListBlock(block)) {
    return block.value.items.map((item) => item.blocks.map(blockStatsText).join('\n')).join('\n');
  }
  if (isTableBlock(block)) {
    return block.value.rows
      .map((row) => row.cells.map((cell) => cell.blocks.map(blockStatsText).join('\t')).join('\t'))
      .join('\n');
  }
  if (block.type === 'Image' && typeof block.value === 'object' && block.value !== null) {
    const image = block.value as { alt_text?: string | null; presentation?: { caption?: string | null } | null };
    return [image.alt_text, image.presentation?.caption].map((value) => value?.trim() ?? '').filter(Boolean).join('\n');
  }
  if (block.type === 'TableOfContents' && typeof block.value === 'object' && block.value !== null) {
    const value = block.value as { title?: string | null; entries?: Array<{ level?: number; text?: string }> };
    return [
      value.title?.trim() ?? '',
      ...(Array.isArray(value.entries)
        ? value.entries.map((entry) => `${'  '.repeat(Math.max(0, Math.min(2, (entry.level ?? 1) - 1)))}${entry.text ?? ''}`)
        : [])
    ]
      .filter(Boolean)
      .join('\n');
  }
  return '';
}

function hasInlineContent(block: Block): block is ParagraphBlock | HeadingBlock {
  return (
    (block.type === 'Paragraph' || block.type === 'Heading') &&
    typeof block.value === 'object' &&
    block.value !== null &&
    'inlines' in block.value &&
    Array.isArray(block.value.inlines)
  );
}

function inlineStatsText(inline: Inline): string {
  if (inline.note_reference) {
    return inline.note_reference.label;
  }
  if (inline.field) {
    return pageFieldFallbackText(inline.field);
  }
  return inline.text;
}

function pageFieldFallbackText(field: Inline['field']): string {
  switch (field) {
    case 'page_number':
    case 'page_count':
      return '1';
    case 'date':
      return '1970-01-01';
    default:
      return '';
  }
}

export function countCharactersWithoutWhitespace(text: string): number {
  return Array.from(text).filter((char) => !/\s/.test(char)).length;
}

export function estimatePageCount(wordCount: number): number {
  const normalized = Math.max(0, Math.trunc(wordCount));
  return normalized === 0 ? 0 : Math.max(1, Math.ceil(normalized / estimatedWordsPerPage));
}

export function estimateReadingMinutes(wordCount: number): number {
  const normalized = Math.max(0, Math.trunc(wordCount));
  return normalized === 0 ? 0 : Math.max(1, Math.ceil(normalized / estimatedReadingWordsPerMinute));
}

export function countDocumentParagraphs(document: DocumentState | undefined): number {
  return document?.sections.reduce((total, section) => total + countParagraphsInBlocks(section.blocks), 0) ?? 0;
}

export function countDocumentImages(document: DocumentState | undefined): number {
  return document?.sections.reduce((total, section) => total + countImagesInBlocks(section.blocks), 0) ?? 0;
}

export function formatPageSize(pageSetup: PageSetup | undefined): string {
  if (!pageSetup) {
    return '';
  }
  return `${formatMillimeters(pageSetup.width_mm)} x ${formatMillimeters(pageSetup.height_mm)} mm`;
}

function countParagraphsInBlocks(blocks: Block[]): number {
  return blocks.reduce((total, block) => {
    if (block.type === 'Paragraph' || block.type === 'Heading') {
      return total + 1;
    }
    if (isListBlock(block)) {
      return total + block.value.items.reduce((itemTotal, item) => itemTotal + countParagraphsInBlocks(item.blocks), 0);
    }
    if (isTableBlock(block)) {
      return total + block.value.rows.reduce(
        (rowTotal, row) =>
          rowTotal + row.cells.reduce((cellTotal, cell) => cellTotal + countParagraphsInBlocks(cell.blocks), 0),
        0
      );
    }
    return total;
  }, 0);
}

function countImagesInBlocks(blocks: Block[]): number {
  return blocks.reduce((total, block) => {
    if (block.type === 'Image') {
      return total + 1;
    }
    if (isListBlock(block)) {
      return total + block.value.items.reduce((itemTotal, item) => itemTotal + countImagesInBlocks(item.blocks), 0);
    }
    if (isTableBlock(block)) {
      return total + block.value.rows.reduce(
        (rowTotal, row) =>
          rowTotal + row.cells.reduce((cellTotal, cell) => cellTotal + countImagesInBlocks(cell.blocks), 0),
        0
      );
    }
    return total;
  }, 0);
}

function isListBlock(block: Block): block is ListBlock {
  return (
    block.type === 'List' &&
    typeof block.value === 'object' &&
    block.value !== null &&
    'items' in block.value &&
    Array.isArray(block.value.items)
  );
}

function isTableBlock(block: Block): block is TableBlock {
  return (
    block.type === 'Table' &&
    typeof block.value === 'object' &&
    block.value !== null &&
    'rows' in block.value &&
    Array.isArray(block.value.rows)
  );
}

function formatMillimeters(value: number): string {
  const normalized = Math.max(0, Number.isFinite(value) ? value : 0);
  return Number.isInteger(normalized) ? String(normalized) : normalized.toFixed(1);
}
