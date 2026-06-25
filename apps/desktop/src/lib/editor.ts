import { Fragment, type Mark, type MarkType, type Node as ProseMirrorNode, type ResolvedPos } from 'prosemirror-model';
import { EditorState, NodeSelection, type Transaction } from 'prosemirror-state';
import { TextSelection } from 'prosemirror-state';
import { Decoration, DecorationSet, EditorView } from 'prosemirror-view';
import 'prosemirror-view/style/prosemirror.css';
import {
  documentToEditorDoc,
  editorDocToWordCoreBlocks,
  sanitizeCommentId,
  type DocumentState,
  type EditorDoc,
  type EditorProjectedChange
} from './documentProjection';
import { findTextRanges, type FindRange } from './findReplace';
import { supportedSchema } from './editorSchema';
import { sanitizeBookmarkId, sanitizeEditorHref } from './editorSecurity';

export type SupportedMarkName =
  | 'bold'
  | 'italic'
  | 'underline'
  | 'strikethrough'
  | 'superscript'
  | 'subscript';

export type SupportedBlockName = 'paragraph' | 'heading';
export type SupportedListName = 'bullet_list' | 'ordered_list';

export interface SupportedTextStyleAttrs {
  fontFamily?: string | null;
  fontSizePt?: number | null;
  textColor?: string | null;
  highlightColor?: string | null;
}

export interface SupportedParagraphAttrs {
  align?: 'left' | 'center' | 'right' | 'justify' | null;
  lineSpacing?: number | null;
  spacingBefore?: number | null;
  spacingAfter?: number | null;
  indentStart?: number | null;
  indentEnd?: number | null;
  firstLineIndent?: number | null;
}

export interface SupportedImageAttrs {
  altText?: string | null;
  alignment?: 'inline' | 'left' | 'center' | 'right' | null;
  scalePercent?: number | null;
  caption?: string | null;
}

export interface EditorFindMatch extends FindRange {
  from: number;
  to: number;
}

export interface EditorSelectionSnapshot {
  from: number;
  to: number;
  empty: boolean;
}

export interface EditorFormattingSnapshot {
  blockType: SupportedBlockName | null;
  styleId: string;
  paragraphFormat: SupportedParagraphAttrs;
  textStyle: SupportedTextStyleAttrs;
  marks: Record<SupportedMarkName, boolean>;
  linkHref: string | null;
  blockBookmarkId: string | null;
  list: {
    type: SupportedListName;
    level: number;
  } | null;
  table: EditorTableSnapshot | null;
  image: SupportedImageAttrs | null;
  selectionWordCount: number;
}

export type SupportedTableEditAction =
  | 'add_row_above'
  | 'add_row_below'
  | 'delete_row'
  | 'add_column_left'
  | 'add_column_right'
  | 'delete_column'
  | 'delete_table';

export interface EditorTableSnapshot {
  rows: number;
  columns: number;
  canAddRow: boolean;
  canDeleteRow: boolean;
  canAddColumn: boolean;
  canDeleteColumn: boolean;
  canDeleteTable: boolean;
}

export interface EditorSpellIssue {
  word: string;
  byte_start: number;
  byte_end: number;
  suggestions?: string[];
}

export interface EditorSpellIssueRange extends EditorSpellIssue {
  from: number;
  to: number;
}

export interface ImageResizeDragSnapshot {
  initialScalePercent: number;
  initialWidthPx: number;
  deltaXPx: number;
}

interface CreateEditorOptions {
  editable: boolean;
  trackChanges?: {
    recording: boolean;
    author?: string;
  };
  onInteraction?: () => void;
  onSelectionChange?: (selection: EditorSelectionSnapshot) => void;
}

interface SpellDecorationHolder {
  value: DecorationSet;
}

export const MIN_TABLE_ROWS = 1;
export const MAX_TABLE_ROWS = 8;
export const MIN_TABLE_COLUMNS = 1;
export const MAX_TABLE_COLUMNS = 8;

const spellDecorationStore = new WeakMap<EditorView, SpellDecorationHolder>();
const REFRESH_SELECTION_FORMATTING_META = '900word-refresh-selection-formatting';
const LOCAL_TRACKED_CHANGE_AUTHOR = 'Local User';

export function createEditor(
  host: HTMLElement,
  document: DocumentState,
  onChange: (change: EditorProjectedChange) => void,
  options: CreateEditorOptions = { editable: true }
): EditorView {
  const state = EditorState.create({
    doc: supportedSchema.nodeFromJSON(documentToEditorDoc(document))
  });

  const spellDecorations: SpellDecorationHolder = { value: DecorationSet.empty };
  const view = new EditorView(host, {
    state,
    attributes: {
      spellcheck: 'true',
      autocapitalize: 'sentences'
    },
    editable: () => options.editable,
    decorations() {
      return spellDecorations.value;
    },
    handleTextInput(editorView, from, to, text) {
      if (!options.trackChanges?.recording) {
        return false;
      }
      const transaction = recordTextInsertionTransaction(
        editorView.state,
        text,
        { from, to, empty: from === to },
        options.trackChanges.author
      );
      if (!transaction) {
        return false;
      }
      editorView.dispatch(transaction.scrollIntoView());
      return true;
    },
    handleKeyDown(editorView, event) {
      if (
        options.trackChanges?.recording &&
        (event.key === 'Backspace' || event.key === 'Delete') &&
        !editorView.state.selection.empty
      ) {
        const transaction = recordSelectedDeletionTransaction(
          editorView.state,
          undefined,
          options.trackChanges.author
        );
        if (transaction) {
          event.preventDefault();
          editorView.dispatch(transaction.scrollIntoView());
          return true;
        }
      }
      if (event.key !== 'Enter' || event.shiftKey || event.metaKey || event.ctrlKey || event.altKey) {
        return false;
      }
      const transaction = continueListOnEnterTransaction(editorView.state);
      if (!transaction) {
        return false;
      }
      editorView.dispatch(transaction.scrollIntoView());
      return true;
    },
    handlePaste(editorView, event) {
      const text = event.clipboardData?.getData('text/plain') ?? '';
      const transaction = pastePlainTextAsBlocksTransaction(editorView.state, text);
      if (!transaction) {
        return false;
      }
      event.preventDefault();
      editorView.dispatch(transaction.scrollIntoView());
      return true;
    },
    handleDOMEvents: {
      focus() {
        options.onInteraction?.();
        return false;
      },
      pointerdown(editorView, event) {
        options.onInteraction?.();
        return options.editable ? handleImageResizePointerDown(editorView, event) : false;
      },
      keydown() {
        options.onInteraction?.();
        return false;
      },
      mousedown() {
        options.onInteraction?.();
        return false;
      }
    },
    dispatchTransaction(transaction) {
      const nextState = view.state.apply(transaction);
      view.updateState(nextState);
      if (transaction.selectionSet || transaction.getMeta(REFRESH_SELECTION_FORMATTING_META) === true) {
        options.onSelectionChange?.(snapshotEditorSelection(view));
      }
      if (!transaction.docChanged) {
        return;
      }
      onChange({
        text: editorDocPlainText(view.state.doc),
        blocks: editorDocToWordCoreBlocks(view.state.doc.toJSON() as EditorDoc, document.styles)
      });
    }
  });

  spellDecorationStore.set(view, spellDecorations);
  options.onSelectionChange?.(snapshotEditorSelection(view));
  return view;
}

export function snapshotEditorSelection(view: EditorView): EditorSelectionSnapshot {
  return {
    from: view.state.selection.from,
    to: view.state.selection.to,
    empty: view.state.selection.empty
  };
}

export function snapshotEditorFormatting(
  view: EditorView | undefined,
  fallbackSelection?: EditorSelectionSnapshot
): EditorFormattingSnapshot {
  if (!view) {
    return emptyEditorFormattingSnapshot();
  }
  return editorStateSelectionFormatting(view.state, fallbackSelection);
}

export function editorStateSelectionFormatting(
  state: EditorState,
  fallbackSelection?: EditorSelectionSnapshot
): EditorFormattingSnapshot {
  const transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const selection = transaction.selection;
  const textblock = selectedTextblock(transaction);
  const image = selectedImageAttrs(transaction, fallbackSelection);
  const list = selectedListContext(selection.$from);
  const table = selectedTableSnapshot(transaction);
  const textStyle = compactTextStyleAttrs(
    textStyleAttrsNearSelection(transaction, supportedSchema.marks.textStyle)
  ) as SupportedTextStyleAttrs;

  let blockType: SupportedBlockName | null = null;
  let styleId = 'body';
  const paragraphFormat: SupportedParagraphAttrs = {};

  if (textblock?.node.type.name === 'heading') {
    blockType = 'heading';
    styleId = `heading-${textblock.node.attrs.level ?? 1}`;
  } else if (textblock?.node.type.name === 'paragraph') {
    blockType = 'paragraph';
    styleId = textblock.node.attrs.style || 'body';
    assignDefinedParagraphAttrs(paragraphFormat, textblock.node.attrs as SupportedParagraphAttrs);
  }

  return {
    blockType,
    styleId,
    paragraphFormat,
    textStyle,
    marks: {
      bold: markActive(transaction, 'bold'),
      italic: markActive(transaction, 'italic'),
      underline: markActive(transaction, 'underline'),
      strikethrough: markActive(transaction, 'strikethrough'),
      superscript: markActive(transaction, 'superscript'),
      subscript: markActive(transaction, 'subscript')
    },
    linkHref: linkHrefNearSelection(transaction),
    blockBookmarkId: selectedBlockBookmarkId(transaction),
    list,
    table,
    image,
    selectionWordCount: countSelectionWords(transaction)
  };
}

export function snapshotEditorDomSelection(view: EditorView | undefined): EditorSelectionSnapshot | undefined {
  if (!view) {
    return undefined;
  }

  const selection = view.dom.ownerDocument.getSelection();
  if (!selection?.anchorNode || !selection.focusNode) {
    return undefined;
  }

  if (!view.dom.contains(selection.anchorNode) || !view.dom.contains(selection.focusNode)) {
    return undefined;
  }

  try {
    const anchor = view.posAtDOM(selection.anchorNode, selection.anchorOffset);
    const focus = view.posAtDOM(selection.focusNode, selection.focusOffset);
    const from = Math.min(anchor, focus);
    const to = Math.max(anchor, focus);
    if (!isValidDocumentRange(view.state.doc, from, to)) {
      return undefined;
    }
    return {
      from,
      to,
      empty: from === to
    };
  } catch {
    return undefined;
  }
}

export function restoreEditorSelection(
  view: EditorView | undefined,
  selection: EditorSelectionSnapshot | undefined
): boolean {
  if (!view || !selection || !isValidDocumentRange(view.state.doc, selection.from, selection.to)) {
    return false;
  }

  if (view.state.selection.from === selection.from && view.state.selection.to === selection.to) {
    return true;
  }

  try {
    view.dispatch(
      view.state.tr.setSelection(TextSelection.create(view.state.doc, selection.from, selection.to))
    );
    return true;
  } catch {
    return false;
  }
}

export function toggleEditorMark(
  view: EditorView | undefined,
  markName: SupportedMarkName,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const markType = supportedSchema.marks[markName];
  if (!markType) {
    return false;
  }

  const transaction = toggleEditorMarkTransaction(view.state, markName, fallbackSelection);
  if (!transaction) {
    return false;
  }

  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function toggleEditorMarkTransaction(
  state: EditorState,
  markName: SupportedMarkName,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  const markType = supportedSchema.marks[markName];
  if (!markType) {
    return undefined;
  }

  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { empty, from, to } = transaction.selection;

  if (empty) {
    const active = markType.isInSet(transaction.storedMarks ?? transaction.selection.$from.marks());
    transaction = active
      ? transaction.removeStoredMark(markType)
      : transaction.addStoredMark(markType.create());
  } else if (state.doc.rangeHasMark(from, to, markType)) {
    transaction = transaction.removeMark(from, to, markType);
  } else {
    transaction = transaction.addMark(from, to, markType.create());
  }

  return transaction;
}

export function setEditorLink(
  view: EditorView | undefined,
  href: string,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = setEditorLinkTransaction(view.state, href, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function setEditorLinkTransaction(
  state: EditorState,
  href: string,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  const markType = supportedSchema.marks.link;
  const safeHref = sanitizeEditorHref(href);
  if (!markType || !safeHref) {
    return undefined;
  }

  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { empty, from, to } = transaction.selection;
  const linkMark = markType.create({ href: safeHref });
  if (empty) {
    const activeRange = markRangeAroundPosition(transaction.doc, from, markType);
    if (activeRange) {
      return transaction
        .removeMark(activeRange.from, activeRange.to, markType)
        .addMark(activeRange.from, activeRange.to, linkMark);
    }

    const current = markType.isInSet(transaction.storedMarks ?? transaction.selection.$from.marks());
    if (current) {
      transaction = transaction.removeStoredMark(markType);
    }
    return transaction.addStoredMark(linkMark);
  }

  return transaction.removeMark(from, to, markType).addMark(from, to, linkMark);
}

export function removeEditorLink(
  view: EditorView | undefined,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = removeEditorLinkTransaction(view.state, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function removeEditorLinkTransaction(
  state: EditorState,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  const markType = supportedSchema.marks.link;
  if (!markType) {
    return undefined;
  }

  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { empty, from, to } = transaction.selection;
  if (!empty) {
    return transaction.doc.rangeHasMark(from, to, markType)
      ? transaction.removeMark(from, to, markType)
      : undefined;
  }

  const activeRange = markRangeAroundPosition(transaction.doc, from, markType);
  if (activeRange) {
    return transaction.removeMark(activeRange.from, activeRange.to, markType);
  }

  const stored = markType.isInSet(transaction.storedMarks ?? transaction.selection.$from.marks());
  return stored ? transaction.removeStoredMark(markType) : undefined;
}

export function createEditorBookmarkId(): string {
  const bytes = new Uint8Array(8);
  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  const suffix = Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  return `bm-${suffix}`;
}

export function createEditorCommentId(): string {
  const bytes = new Uint8Array(8);
  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  const suffix = Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  return `cmt-${suffix}`;
}

export function createEditorTrackedChangeId(): string {
  const bytes = new Uint8Array(8);
  if (globalThis.crypto?.getRandomValues) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  const suffix = Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
  return `chg-${suffix}`;
}

export function recordTextInsertionTransaction(
  state: EditorState,
  text: string,
  fallbackSelection?: EditorSelectionSnapshot,
  author = LOCAL_TRACKED_CHANGE_AUTHOR
): Transaction | undefined {
  if (text.length === 0) {
    return undefined;
  }
  const insertionMark = trackedChangeMark('insertion', author);
  if (!insertionMark) {
    return undefined;
  }
  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { empty, from, to } = transaction.selection;
  const insertionPosition = empty ? from : to;
  if (!empty) {
    const deletionMark = trackedChangeMark('deletion', author);
    if (!deletionMark) {
      return undefined;
    }
    transaction = transaction.addMark(from, to, deletionMark);
  }
  const textNode = supportedSchema.text(text, insertionMarks(transaction, insertionMark));
  transaction = transaction.insert(insertionPosition, textNode);
  return transaction.setSelection(TextSelection.create(transaction.doc, insertionPosition + text.length));
}

export function recordSelectedDeletionTransaction(
  state: EditorState,
  fallbackSelection?: EditorSelectionSnapshot,
  author = LOCAL_TRACKED_CHANGE_AUTHOR
): Transaction | undefined {
  const deletionMark = trackedChangeMark('deletion', author);
  if (!deletionMark) {
    return undefined;
  }
  const transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { empty, from, to } = transaction.selection;
  if (empty || transaction.doc.textBetween(from, to, '\n', ' ').length === 0) {
    return undefined;
  }
  const nextTransaction = transaction.addMark(from, to, deletionMark);
  return nextTransaction.docChanged ? nextTransaction : transaction;
}

function trackedChangeMark(kind: 'insertion' | 'deletion', author: string): Mark | undefined {
  const markType = supportedSchema.marks.trackedChange;
  if (!markType) {
    return undefined;
  }
  return markType.create({
    id: createEditorTrackedChangeId(),
    kind,
    author: normalizeLocalTrackedChangeAuthor(author),
    createdAt: new Date().toISOString()
  });
}

function insertionMarks(transaction: Transaction, trackedMark: Mark): Mark[] {
  const activeMarks = transaction.storedMarks ?? transaction.selection.$from.marks();
  return [
    ...activeMarks.filter((mark) => mark.type.name !== 'trackedChange'),
    trackedMark
  ];
}

function normalizeLocalTrackedChangeAuthor(author: string): string {
  const normalized = author.replace(/[\u0000-\u001f\u007f]/g, '').trim();
  return normalized.length > 0 && Array.from(normalized).length <= 80
    ? normalized
    : LOCAL_TRACKED_CHANGE_AUTHOR;
}

export function selectedEditorText(
  view: EditorView | undefined,
  fallbackSelection?: EditorSelectionSnapshot
): string {
  if (!view) {
    return '';
  }
  const transaction = transactionWithFallbackSelection(view.state, fallbackSelection);
  if (transaction.selection.empty) {
    return '';
  }
  return transaction.doc.textBetween(transaction.selection.from, transaction.selection.to, '\n', ' ');
}

export function addEditorCommentToSelection(
  view: EditorView | undefined,
  commentId: string,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = addEditorCommentTransaction(view.state, commentId, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function addEditorCommentTransaction(
  state: EditorState,
  commentId: string,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  const markType = supportedSchema.marks.comment;
  const safeId = sanitizeCommentId(commentId);
  if (!markType || !safeId) {
    return undefined;
  }

  const transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { empty, from, to } = transaction.selection;
  if (empty || transaction.doc.textBetween(from, to, '\n', ' ').trim().length === 0) {
    return undefined;
  }
  return transaction.addMark(from, to, markType.create({ id: safeId }));
}

export function removeEditorCommentFromDocument(
  view: EditorView | undefined,
  commentId: string
): boolean {
  if (!view) {
    return false;
  }
  const transaction = removeEditorCommentTransaction(view.state, commentId);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function removeEditorCommentTransaction(
  state: EditorState,
  commentId: string
): Transaction | undefined {
  const markType = supportedSchema.marks.comment;
  const safeId = sanitizeCommentId(commentId);
  if (!markType || !safeId) {
    return undefined;
  }

  let transaction = state.tr;
  let changed = false;
  state.doc.descendants((node, pos) => {
    if (!node.isText) {
      return true;
    }
    const targetMark = node.marks.find((mark) => mark.type === markType && mark.attrs.id === safeId);
    if (targetMark) {
      transaction = transaction.removeMark(pos, pos + node.nodeSize, targetMark);
      changed = true;
    }
    return true;
  });
  return changed ? transaction : undefined;
}

export function selectEditorCommentRange(view: EditorView | undefined, commentId: string): boolean {
  if (!view) {
    return false;
  }
  const markType = supportedSchema.marks.comment;
  const safeId = sanitizeCommentId(commentId);
  if (!markType || !safeId) {
    return false;
  }
  let range: { from: number; to: number } | undefined;
  view.state.doc.descendants((node, pos) => {
    if (range || !node.isText) {
      return !range;
    }
    if (node.marks.some((mark) => mark.type === markType && mark.attrs.id === safeId)) {
      range = { from: pos, to: pos + node.nodeSize };
      return false;
    }
    return true;
  });
  if (!range) {
    return false;
  }
  view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, range.from, range.to)).scrollIntoView());
  view.focus();
  return true;
}

export function selectEditorTrackedChangeRange(view: EditorView | undefined, changeId: string): boolean {
  if (!view) {
    return false;
  }
  const markType = supportedSchema.marks.trackedChange;
  if (!markType || !/^chg-[A-Za-z0-9_-]+$/.test(changeId)) {
    return false;
  }
  let range: { from: number; to: number } | undefined;
  view.state.doc.descendants((node, pos) => {
    if (range || !node.isText) {
      return !range;
    }
    if (node.marks.some((mark) => mark.type === markType && mark.attrs.id === changeId)) {
      range = { from: pos, to: pos + node.nodeSize };
      return false;
    }
    return true;
  });
  if (!range) {
    return false;
  }
  view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, range.from, range.to)).scrollIntoView());
  view.focus();
  return true;
}

export function setEditorBlockBookmark(
  view: EditorView | undefined,
  bookmarkId: string,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = setEditorBlockBookmarkTransaction(view.state, bookmarkId, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function setEditorBlockBookmarkTransaction(
  state: EditorState,
  bookmarkId: string,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  const safeBookmarkId = sanitizeBookmarkId(bookmarkId);
  if (!safeBookmarkId) {
    return undefined;
  }

  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const selected = selectedTextblock(transaction);
  if (!selected || !bookmarkableTextblock(selected.node)) {
    return undefined;
  }
  if (selected.node.attrs.bookmarkId === safeBookmarkId) {
    return undefined;
  }
  return transaction.setNodeMarkup(
    selected.pos,
    selected.node.type,
    { ...selected.node.attrs, bookmarkId: safeBookmarkId },
    selected.node.marks
  );
}

export function removeEditorBlockBookmark(
  view: EditorView | undefined,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = removeEditorBlockBookmarkTransaction(view.state, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function removeEditorBlockBookmarkTransaction(
  state: EditorState,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const selected = selectedTextblock(transaction);
  if (!selected || !bookmarkableTextblock(selected.node) || !selected.node.attrs.bookmarkId) {
    return undefined;
  }
  return transaction.setNodeMarkup(
    selected.pos,
    selected.node.type,
    { ...selected.node.attrs, bookmarkId: null },
    selected.node.marks
  );
}

export function setEditorBlockType(
  view: EditorView | undefined,
  blockName: SupportedBlockName,
  attrs: Record<string, string | number> = {},
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const nodeType = supportedSchema.nodes[blockName];
  if (!nodeType) {
    return false;
  }

  const transaction = setEditorBlockTypeTransaction(view.state, blockName, attrs, fallbackSelection);
  if (!transaction) {
    return false;
  }

  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function setEditorBlockTypeTransaction(
  state: EditorState,
  blockName: SupportedBlockName,
  attrs: Record<string, string | number> = {},
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  const nodeType = supportedSchema.nodes[blockName];
  if (!nodeType) {
    return undefined;
  }

  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { from, to } = selectedTextblockRange(transaction);
  let changed = false;

  transaction.doc.nodesBetween(from, to, (node, pos) => {
    if (!node.isTextblock) {
      return true;
    }
    const nextAttrs = attrsWithPreservedBookmark(node.attrs, attrs);
    if (node.type !== nodeType || JSON.stringify(node.attrs) !== JSON.stringify(nextAttrs)) {
      transaction = transaction.setNodeMarkup(pos, nodeType, nextAttrs);
      changed = true;
    }
    return false;
  });

  if (!changed) {
    return undefined;
  }

  return transaction;
}

export function setEditorTextStyle(
  view: EditorView | undefined,
  attrs: SupportedTextStyleAttrs,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = setEditorTextStyleTransaction(view.state, attrs, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function setEditorTextStyleTransaction(
  state: EditorState,
  attrs: SupportedTextStyleAttrs,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  const markType = supportedSchema.marks.textStyle;
  if (!markType) {
    return undefined;
  }

  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { empty, from, to } = transaction.selection;
  const normalized = compactTextStyleAttrs(attrs);
  if (Object.keys(normalized).length === 0) {
    return undefined;
  }
  const currentAttrs = textStyleAttrsNearSelection(transaction, markType);
  const nextAttrs = { ...currentAttrs, ...normalized };

  if (empty) {
    const current = markType.isInSet(transaction.storedMarks ?? transaction.selection.$from.marks());
    transaction = current ? transaction.removeStoredMark(markType) : transaction;
    transaction = transaction.addStoredMark(markType.create(nextAttrs));
  } else {
    transaction = transaction.removeMark(from, to, markType).addMark(from, to, markType.create(nextAttrs));
  }

  return transaction;
}

export function setEditorParagraphFormat(
  view: EditorView | undefined,
  attrs: SupportedParagraphAttrs,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = setEditorParagraphFormatTransaction(view.state, attrs, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function setEditorParagraphFormatTransaction(
  state: EditorState,
  attrs: SupportedParagraphAttrs,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { from, to } = selectedTextblockRange(transaction);
  let changed = false;

  transaction.doc.nodesBetween(from, to, (node, pos) => {
    if (node.type.name !== 'paragraph') {
      return true;
    }
    const nextAttrs = { ...node.attrs, ...compactParagraphAttrs(attrs) };
    if (JSON.stringify(node.attrs) !== JSON.stringify(nextAttrs)) {
      transaction = transaction.setNodeMarkup(pos, node.type, nextAttrs, node.marks);
      changed = true;
    }
    return false;
  });

  return changed ? transaction : undefined;
}

export function setSelectedImageAttrs(
  view: EditorView | undefined,
  attrs: SupportedImageAttrs,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  const transaction = setSelectedImageAttrsTransaction(view.state, attrs, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function setSelectedImageAttrsTransaction(
  state: EditorState,
  attrs: SupportedImageAttrs,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  let transaction = state.tr;
  const selected = selectedImageNode(transaction, fallbackSelection);
  if (!selected) {
    return undefined;
  }

  const nextAttrs = {
    ...selected.node.attrs,
    ...compactImageAttrs(attrs)
  };
  if (JSON.stringify(selected.node.attrs) === JSON.stringify(nextAttrs)) {
    return undefined;
  }
  return transaction.setNodeMarkup(selected.pos, selected.node.type, nextAttrs, selected.node.marks);
}

export function imageScalePercentFromResizeDrag(snapshot: ImageResizeDragSnapshot): number {
  const initialWidth = Math.max(1, snapshot.initialWidthPx);
  const nextWidth = initialWidth + snapshot.deltaXPx;
  const ratio = nextWidth / initialWidth;
  return normalizeImageScale(snapshot.initialScalePercent * ratio);
}

export function clearEditorDirectFormatting(
  view: EditorView | undefined,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = clearEditorDirectFormattingTransaction(view.state, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function clearEditorDirectFormattingTransaction(
  state: EditorState,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { from, to } = transaction.selection;
  let changed = false;

  if (transaction.selection.empty) {
    for (const markType of Object.values(supportedSchema.marks)) {
      if (markType.name === 'link' || markType.name === 'comment' || markType.name === 'trackedChange') {
        continue;
      }
      transaction = transaction.removeStoredMark(markType);
    }
  } else {
    for (const markType of Object.values(supportedSchema.marks)) {
      if (markType.name === 'link' || markType.name === 'comment' || markType.name === 'trackedChange') {
        continue;
      }
      transaction = transaction.removeMark(from, to, markType);
    }
    changed = true;
  }

  const blockRange = selectedTextblockRange(transaction);
  transaction.doc.nodesBetween(blockRange.from, blockRange.to, (node, pos) => {
    if (node.type.name !== 'paragraph') {
      return true;
    }
    const nextAttrs = {
      style: node.attrs.style || 'body',
      bookmarkId: node.attrs.bookmarkId ?? null,
      align: null,
      lineSpacing: null,
      spacingBefore: null,
      spacingAfter: null,
      indentStart: null,
      indentEnd: null,
      firstLineIndent: null
    };
    if (JSON.stringify(node.attrs) !== JSON.stringify(nextAttrs)) {
      transaction = transaction.setNodeMarkup(pos, node.type, nextAttrs, node.marks);
      changed = true;
    }
    return false;
  });

  return changed ? transaction : undefined;
}

export function toggleEditorList(
  view: EditorView | undefined,
  listName: SupportedListName,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = toggleEditorListTransaction(view.state, listName, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function toggleEditorListTransaction(
  state: EditorState,
  listName: SupportedListName,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const listType = supportedSchema.nodes[listName];
  const itemType = supportedSchema.nodes.list_item;
  const paragraphType = supportedSchema.nodes.paragraph;
  if (!listType || !itemType || !paragraphType) {
    return undefined;
  }

  const { from, to } = selectedTopLevelRange(transaction);
  const topLevelNodes: Array<{ node: ProseMirrorNode; pos: number }> = [];
  transaction.doc.nodesBetween(from, to, (node, pos, parent) => {
    if (parent === transaction.doc) {
      topLevelNodes.push({ node, pos });
      return false;
    }
    return true;
  });

  if (topLevelNodes.length === 0) {
    return undefined;
  }

  if (topLevelNodes.length === 1 && topLevelNodes[0].node.type === listType) {
    const blocks: ProseMirrorNode[] = [];
    topLevelNodes[0].node.forEach((item) => {
      item.forEach((child) => {
        blocks.push(child.isTextblock ? child : paragraphType.create({ style: 'body' }, child.content));
      });
    });
    if (blocks.length === 0) {
      return undefined;
    }
    return transaction.replaceWith(from, to, blocks);
  }

  const items = topLevelNodes
    .filter(({ node }) => node.isTextblock)
    .map(({ node }) =>
      itemType.create(
        { level: 1 },
        paragraphType.create(
          {
            style: node.type.name === 'paragraph' ? node.attrs.style || 'body' : 'body',
            bookmarkId: node.attrs.bookmarkId ?? null
          },
          node.content
        )
      )
    );
  if (items.length === 0) {
    return undefined;
  }

  const definitionId = listName === 'ordered_list' ? '900w-ordered' : '900w-unordered';
  return transaction.replaceWith(from, to, listType.create({ definitionId }, items));
}

export function adjustSelectedListLevel(
  view: EditorView | undefined,
  delta: number,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = adjustSelectedListLevelTransaction(view.state, delta, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function adjustSelectedListLevelTransaction(
  state: EditorState,
  delta: number,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { from, to } = transaction.selection;
  let changed = false;
  transaction.doc.nodesBetween(from, to, (node, pos) => {
    if (node.type.name !== 'list_item') {
      return true;
    }
    const nextLevel = Math.min(8, Math.max(1, Number(node.attrs.level || 1) + delta));
    if (nextLevel !== node.attrs.level) {
      transaction = transaction.setNodeMarkup(pos, node.type, { ...node.attrs, level: nextLevel }, node.marks);
      changed = true;
    }
    return false;
  });

  return changed ? transaction : undefined;
}

export function insertDefaultTable(
  view: EditorView | undefined,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  return insertTable(view, 2, 2, fallbackSelection);
}

export function insertTable(
  view: EditorView | undefined,
  rows: number,
  columns: number,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = insertTableTransaction(view.state, rows, columns, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function insertDefaultTableTransaction(
  state: EditorState,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  return insertTableTransaction(state, 2, 2, fallbackSelection);
}

export function insertTableTransaction(
  state: EditorState,
  rows: number,
  columns: number,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  const table = createTableNode(rows, columns);
  if (!table) {
    return undefined;
  }

  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const range = selectedTopLevelRange(transaction);
  const selected = topLevelNodesInRange(transaction, range.from, range.to);
  let from = range.from;
  let to = range.to;

  if (transaction.selection.empty) {
    const selectedNode = selected.length === 1 ? selected[0].node : undefined;
    if (!selectedNode || !isEmptyParagraphNode(selectedNode)) {
      from = range.to;
      to = range.to;
    }
  } else if (!selectionCoversTopLevelContent(transaction, range.from, range.to)) {
    from = range.to;
    to = range.to;
  }

  transaction = transaction.replaceWith(from, to, table);
  const cursor = firstTextblockStartBetween(transaction.doc, from, from + table.nodeSize);
  if (cursor !== undefined) {
    transaction = transaction.setSelection(TextSelection.create(transaction.doc, cursor));
  }
  return transaction;
}

export function editTableStructure(
  view: EditorView | undefined,
  action: SupportedTableEditAction,
  fallbackSelection?: EditorSelectionSnapshot
): boolean {
  if (!view) {
    return false;
  }

  restoreEditorSelection(view, fallbackSelection);
  const transaction = editTableStructureTransaction(view.state, action, fallbackSelection);
  if (!transaction) {
    return false;
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function editTableStructureTransaction(
  state: EditorState,
  action: SupportedTableEditAction,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const context = selectedEditableTableContext(transaction);
  if (!context) {
    return undefined;
  }

  if (action === 'delete_table') {
    return deleteSelectedTable(transaction, context);
  }

  const { table, rowIndex, cellIndex } = context;
  const rows = nodeChildren(table.node);
  const rowCount = rows.length;
  const columnCount = context.columns;

  if (action === 'add_row_above' || action === 'add_row_below') {
    if (rowCount >= MAX_TABLE_ROWS) {
      return undefined;
    }
    const insertionIndex = action === 'add_row_above' ? rowIndex : rowIndex + 1;
    const nextRows = [
      ...rows.slice(0, insertionIndex),
      createEmptyTableRow(columnCount),
      ...rows.slice(insertionIndex)
    ];
    return replaceSelectedTable(transaction, context, nextRows, insertionIndex, cellIndex);
  }

  if (action === 'delete_row') {
    if (rowCount <= MIN_TABLE_ROWS) {
      return undefined;
    }
    const nextRows = rows.filter((_, index) => index !== rowIndex);
    return replaceSelectedTable(
      transaction,
      context,
      nextRows,
      Math.min(rowIndex, nextRows.length - 1),
      Math.min(cellIndex, columnCount - 1)
    );
  }

  if (action === 'add_column_left' || action === 'add_column_right') {
    if (columnCount >= MAX_TABLE_COLUMNS) {
      return undefined;
    }
    const insertionIndex = action === 'add_column_left' ? cellIndex : cellIndex + 1;
    const nextRows = rows.map((row) => {
      const cells = nodeChildren(row);
      return row.type.create(row.attrs, [
        ...cells.slice(0, insertionIndex),
        createEmptyTableCell(),
        ...cells.slice(insertionIndex)
      ], row.marks);
    });
    return replaceSelectedTable(transaction, context, nextRows, rowIndex, insertionIndex);
  }

  if (action === 'delete_column') {
    if (columnCount <= MIN_TABLE_COLUMNS) {
      return undefined;
    }
    const nextRows = rows.map((row) => {
      const cells = nodeChildren(row).filter((_, index) => index !== cellIndex);
      return row.type.create(row.attrs, cells, row.marks);
    });
    return replaceSelectedTable(
      transaction,
      context,
      nextRows,
      rowIndex,
      Math.min(cellIndex, columnCount - 2)
    );
  }

  return undefined;
}

export function continueListOnEnterTransaction(
  state: EditorState,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  let transaction = transactionWithFallbackSelection(state, fallbackSelection);
  if (!transaction.selection.empty) {
    return undefined;
  }

  const context = selectedListPositionContext(transaction.selection.$from);
  if (!context || context.textblock.node.type.name !== 'paragraph') {
    return undefined;
  }

  const paragraph = context.textblock.node;
  const itemBlocks = nodeChildren(context.item.node);
  const paragraphIndex = itemBlocks.findIndex((node) => node === paragraph);
  if (paragraphIndex < 0) {
    return undefined;
  }

  if (paragraph.textContent.length === 0 && itemBlocks.length === 1) {
    return exitEmptyListItem(transaction, context);
  }

  const splitOffset = Math.max(0, Math.min(paragraph.content.size, transaction.selection.$from.parentOffset));
  const beforeParagraph = paragraph.type.create(
    paragraph.attrs,
    paragraph.content.cut(0, splitOffset),
    paragraph.marks
  );
  const afterParagraph = paragraph.type.create(
    paragraph.attrs,
    paragraph.content.cut(splitOffset),
    paragraph.marks
  );
  const beforeBlocks = [...itemBlocks.slice(0, paragraphIndex), beforeParagraph];
  const afterBlocks = [afterParagraph, ...itemBlocks.slice(paragraphIndex + 1)];
  const items = nodeChildren(context.list.node);
  const nextItems = [
    ...items.slice(0, context.itemIndex),
    context.item.node.type.create(context.item.node.attrs, beforeBlocks, context.item.node.marks),
    context.item.node.type.create(context.item.node.attrs, afterBlocks, context.item.node.marks),
    ...items.slice(context.itemIndex + 1)
  ];
  const nextList = context.list.node.type.create(context.list.node.attrs, nextItems, context.list.node.marks);
  transaction = transaction.replaceWith(context.list.pos, context.list.pos + context.list.node.nodeSize, nextList);

  const cursor = listItemParagraphStart(transaction.doc, context.list.pos, context.itemIndex + 1);
  if (cursor !== undefined) {
    transaction = transaction.setSelection(TextSelection.create(transaction.doc, cursor));
  }
  return transaction;
}

export function pastePlainTextAsBlocksTransaction(
  state: EditorState,
  text: string,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction | undefined {
  if (!text.includes('\n')) {
    return undefined;
  }

  const parsed = parsePlainTextBlocks(text);
  if (parsed.length === 0) {
    return undefined;
  }

  const transaction = transactionWithFallbackSelection(state, fallbackSelection);
  const { from, to } = selectedTopLevelRange(transaction);
  const selected = topLevelNodesInRange(transaction, from, to);
  if (transaction.selection.empty) {
    if (selected.length !== 1 || !selected[0].node.isTextblock || selected[0].node.textContent.trim().length > 0) {
      return undefined;
    }
  } else if (!selectionCoversTopLevelContent(transaction, from, to)) {
    return undefined;
  }
  return transaction.replaceWith(from, to, Fragment.fromArray(parsed));
}

export function setEditorSpellIssues(
  view: EditorView | undefined,
  issues: EditorSpellIssue[],
  plainText: string
): EditorSpellIssueRange[] {
  if (!view) {
    return [];
  }

  const ranges = mapSpellIssuesToEditorRanges(view.state.doc, issues, plainText);
  const decorations = ranges.map((issue) =>
    Decoration.inline(issue.from, issue.to, {
      class: 'spell-misspelled',
      'data-spell-word': issue.word
    })
  );
  const holder = spellDecorationStore.get(view);
  if (holder) {
    holder.value = DecorationSet.create(view.state.doc, decorations);
  }
  view.updateState(view.state);
  return ranges;
}

export function mapSpellIssuesToEditorRanges(
  doc: ProseMirrorNode,
  issues: EditorSpellIssue[],
  plainText: string
): EditorSpellIssueRange[] {
  return issues
    .map((issue) => {
      const start = utf8ByteOffsetToStringIndex(plainText, issue.byte_start);
      const end = utf8ByteOffsetToStringIndex(plainText, issue.byte_end);
      const range = mapPlainTextRangeInsideDoc(doc, start, end);
      return range ? { ...issue, ...range } : undefined;
    })
    .filter((issue): issue is EditorSpellIssueRange => issue !== undefined);
}

export function findEditorTextMatches(
  view: EditorView | undefined,
  query: string,
  caseSensitive = false
): EditorFindMatch[] {
  return view ? findEditorDocMatches(view.state.doc, query, caseSensitive) : [];
}

export function findEditorDocMatches(
  doc: ProseMirrorNode,
  query: string,
  caseSensitive = false
): EditorFindMatch[] {
  if (query.length === 0) {
    return [];
  }

  const matches: EditorFindMatch[] = [];
  let documentTextIndex = 0;
  doc.descendants((node, pos) => {
    if (!node.isTextblock) {
      return true;
    }

    const blockText = node.textContent;
    for (const range of findTextRanges(blockText, query, caseSensitive)) {
      const mapped = mapTextRangeInsideTextblock(node, pos, range.index, range.length);
      if (mapped) {
        matches.push({
          index: documentTextIndex + range.index,
          length: range.length,
          from: mapped.from,
          to: mapped.to
        });
      }
    }
    documentTextIndex += blockText.length + 1;
    return false;
  });

  return matches;
}

export function selectEditorTextRange(view: EditorView | undefined, from: number, to: number): boolean {
  if (!view) {
    return false;
  }

  if (!isValidDocumentRange(view.state.doc, from, to)) {
    return false;
  }

  view.dispatch(
    view.state.tr.setSelection(TextSelection.create(view.state.doc, from, to)).scrollIntoView()
  );
  view.focus();
  return true;
}

export function selectEditorTopLevelBlock(view: EditorView | undefined, blockIndex: number): boolean {
  if (!view || !Number.isInteger(blockIndex) || blockIndex < 0 || blockIndex >= view.state.doc.childCount) {
    return false;
  }

  let position = 0;
  for (let index = 0; index < blockIndex; index += 1) {
    position += view.state.doc.child(index).nodeSize;
  }
  const block = view.state.doc.child(blockIndex);
  const cursor = Math.min(position + 1, position + block.nodeSize - 1);
  if (!isValidDocumentRange(view.state.doc, cursor, cursor)) {
    return false;
  }

  view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, cursor)).scrollIntoView());
  view.focus();
  return true;
}

export function editorTopLevelInsertionIndex(
  view: EditorView | undefined,
  fallbackSelection?: EditorSelectionSnapshot
): number | undefined {
  if (!view) {
    return undefined;
  }
  let transaction = transactionWithFallbackSelection(view.state, fallbackSelection);
  const range = selectedTopLevelRange(transaction);
  let index = 0;
  let insertionIndex: number | undefined;
  transaction.doc.nodesBetween(0, transaction.doc.content.size, (node, pos, parent) => {
    if (parent !== transaction.doc) {
      return true;
    }
    const end = pos + node.nodeSize;
    if (range.to <= end && insertionIndex === undefined) {
      insertionIndex = index + 1;
      return false;
    }
    index += 1;
    return false;
  });
  return insertionIndex ?? transaction.doc.childCount;
}

export function replaceEditorTextRange(
  view: EditorView | undefined,
  from: number,
  to: number,
  replacement: string
): boolean {
  if (!view) {
    return false;
  }

  if (!isValidDocumentRange(view.state.doc, from, to)) {
    return false;
  }

  view.dispatch(view.state.tr.insertText(replacement, from, to).scrollIntoView());
  view.focus();
  return true;
}

export function replaceAllEditorText(
  view: EditorView | undefined,
  ranges: EditorFindMatch[],
  replacement: string
): boolean {
  if (!view || ranges.length === 0) {
    return false;
  }

  const mappedRanges = ranges
    .filter((range) => isValidDocumentRange(view.state.doc, range.from, range.to))
    .sort((left, right) => right.from - left.from);

  if (mappedRanges.length === 0) {
    return false;
  }

  let transaction = view.state.tr;
  for (const range of mappedRanges) {
    transaction = transaction.insertText(replacement, range.from, range.to);
  }
  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function editorDocPlainText(doc: ProseMirrorNode): string {
  return doc.textBetween(0, doc.content.size, '\n', ' ');
}

function transactionWithFallbackSelection(
  state: EditorState,
  fallbackSelection?: EditorSelectionSnapshot
): Transaction {
  let transaction = state.tr;
  if (
    state.selection.empty &&
    fallbackSelection &&
    !fallbackSelection.empty &&
    isValidDocumentRange(state.doc, fallbackSelection.from, fallbackSelection.to)
  ) {
    try {
      transaction = transaction.setSelection(
        TextSelection.create(state.doc, fallbackSelection.from, fallbackSelection.to)
      );
    } catch {
      return transaction;
    }
  }
  return transaction;
}

function selectedTextblockRange(transaction: Transaction): { from: number; to: number } {
  if (transaction.selection.$from.depth === 0 || transaction.selection.$to.depth === 0) {
    return { from: 0, to: transaction.doc.content.size };
  }

  const fromDepth = Math.max(1, transaction.selection.$from.depth);
  const toDepth = Math.max(1, transaction.selection.$to.depth);
  return {
    from: transaction.selection.$from.before(fromDepth),
    to: transaction.selection.$to.after(toDepth)
  };
}

function selectedTopLevelRange(transaction: Transaction): { from: number; to: number } {
  const fromDepth = transaction.selection.$from.depth;
  const toDepth = transaction.selection.$to.depth;
  const from = fromDepth === 0 ? 0 : transaction.selection.$from.before(1);
  const to = toDepth === 0 ? transaction.doc.content.size : transaction.selection.$to.after(1);
  return { from, to };
}

function topLevelNodesInRange(
  transaction: Transaction,
  from: number,
  to: number
): Array<{ node: ProseMirrorNode; pos: number }> {
  const nodes: Array<{ node: ProseMirrorNode; pos: number }> = [];
  transaction.doc.nodesBetween(from, to, (node, pos, parent) => {
    if (parent === transaction.doc) {
      nodes.push({ node, pos });
      return false;
    }
    return true;
  });
  return nodes;
}

function selectionCoversTopLevelContent(transaction: Transaction, from: number, to: number): boolean {
  return transaction.selection.from <= from + 1 && transaction.selection.to >= Math.max(from + 1, to - 1);
}

function selectedTextblock(
  transaction: Transaction
): { node: ProseMirrorNode; pos: number } | undefined {
  const { $from } = transaction.selection;
  for (let depth = $from.depth; depth >= 0; depth -= 1) {
    const node = $from.node(depth);
    if (node.isTextblock) {
      return { node, pos: $from.before(depth) };
    }
  }
  return undefined;
}

function bookmarkableTextblock(node: ProseMirrorNode): boolean {
  return node.type.name === 'paragraph' || node.type.name === 'heading';
}

function selectedBlockBookmarkId(transaction: Transaction): string | null {
  const selected = selectedTextblock(transaction);
  if (!selected || !bookmarkableTextblock(selected.node) || typeof selected.node.attrs.bookmarkId !== 'string') {
    return null;
  }
  return sanitizeBookmarkId(selected.node.attrs.bookmarkId) ?? null;
}

function attrsWithPreservedBookmark(
  currentAttrs: Record<string, unknown>,
  nextAttrs: Record<string, string | number>
): Record<string, string | number | null> {
  const bookmarkId = sanitizeBookmarkId(String(currentAttrs.bookmarkId ?? ''));
  return bookmarkId ? { ...nextAttrs, bookmarkId } : nextAttrs;
}

function selectedImageAttrs(
  transaction: Transaction,
  fallbackSelection?: EditorSelectionSnapshot
): SupportedImageAttrs | null {
  const selected = selectedImageNode(transaction, fallbackSelection);
  if (!selected) {
    return null;
  }
  return {
    altText: typeof selected.node.attrs.altText === 'string' ? selected.node.attrs.altText : 'Image',
    alignment: normalizeImageAlignment(selected.node.attrs.alignment),
    scalePercent: normalizeImageScale(selected.node.attrs.scalePercent),
    caption: typeof selected.node.attrs.caption === 'string' ? selected.node.attrs.caption : null
  };
}

function selectedImageNode(
  transaction: Transaction,
  fallbackSelection?: EditorSelectionSnapshot
): { node: ProseMirrorNode; pos: number } | undefined {
  if (transaction.selection instanceof NodeSelection && transaction.selection.node.type.name === 'image') {
    return { node: transaction.selection.node, pos: transaction.selection.from };
  }

  const from = fallbackSelection?.from ?? transaction.selection.from;
  const to = fallbackSelection?.to ?? transaction.selection.to;
  let found: { node: ProseMirrorNode; pos: number } | undefined;
  transaction.doc.nodesBetween(Math.max(0, from), Math.min(transaction.doc.content.size, to), (node, pos, parent) => {
    if (parent === transaction.doc && node.type.name === 'image') {
      found = { node, pos };
      return false;
    }
    return true;
  });
  return found;
}

function handleImageResizePointerDown(view: EditorView, event: Event): boolean {
  if (!(event instanceof PointerEvent) || event.button !== 0) {
    return false;
  }

  const target = event.target;
  if (!(target instanceof Element) || !target.closest('.image-resize-handle')) {
    return false;
  }

  const figure = target.closest('figure[data-asset-id]');
  if (!(figure instanceof HTMLElement)) {
    return false;
  }

  let position: number;
  try {
    position = view.posAtDOM(figure, 0);
  } catch {
    return false;
  }

  const selected = imageNodeAtOrBeforePosition(view.state.doc, position);
  if (!selected) {
    return false;
  }

  event.preventDefault();
  try {
    target.setPointerCapture?.(event.pointerId);
  } catch {
    // The resize still works through document-level pointer listeners if capture is unavailable.
  }

  const initialX = event.clientX;
  const initialWidth = figure.getBoundingClientRect().width;
  const initialScale = normalizeImageScale(selected.node.attrs.scalePercent);

  view.dispatch(view.state.tr.setSelection(NodeSelection.create(view.state.doc, selected.pos)));
  view.focus();

  const ownerDocument = view.dom.ownerDocument;
  const updateScale = (pointerEvent: PointerEvent) => {
    const scalePercent = imageScalePercentFromResizeDrag({
      initialScalePercent: initialScale,
      initialWidthPx: initialWidth,
      deltaXPx: pointerEvent.clientX - initialX
    });
    const transaction = setSelectedImageAttrsTransaction(view.state, { scalePercent });
    if (transaction) {
      transaction.setMeta(REFRESH_SELECTION_FORMATTING_META, true);
      view.dispatch(transaction);
    }
  };
  const cleanupResize = () => {
    ownerDocument.removeEventListener('pointermove', updateScale);
    ownerDocument.removeEventListener('pointerup', finishResize);
    ownerDocument.removeEventListener('pointercancel', cancelResize);
    try {
      target.releasePointerCapture?.(event.pointerId);
    } catch {
      // The handle may have been redrawn after node-attribute updates.
    }
  };
  const finishResize = (pointerEvent: PointerEvent) => {
    try {
      updateScale(pointerEvent);
    } finally {
      cleanupResize();
    }
  };
  const cancelResize = () => {
    cleanupResize();
  };

  ownerDocument.addEventListener('pointermove', updateScale);
  ownerDocument.addEventListener('pointerup', finishResize, { once: true });
  ownerDocument.addEventListener('pointercancel', cancelResize, { once: true });
  return true;
}

function imageNodeAtOrBeforePosition(
  doc: ProseMirrorNode,
  position: number
): { node: ProseMirrorNode; pos: number } | undefined {
  const bounded = Math.min(Math.max(0, position), doc.content.size);
  const direct = doc.nodeAt(bounded);
  if (direct?.type.name === 'image') {
    return { node: direct, pos: bounded };
  }
  if (bounded === 0) {
    return undefined;
  }
  const nodeBefore = doc.resolve(bounded).nodeBefore;
  if (nodeBefore?.type.name !== 'image') {
    return undefined;
  }
  return { node: nodeBefore, pos: bounded - nodeBefore.nodeSize };
}

function selectedListContext($pos: ResolvedPos): EditorFormattingSnapshot['list'] {
  const context = selectedListPositionContext($pos);
  if (!context) {
    return null;
  }
  return {
    type: context.list.node.type.name as SupportedListName,
    level: Number(context.item.node.attrs.level || 1)
  };
}

function selectedListPositionContext($pos: ResolvedPos):
  | {
      list: { node: ProseMirrorNode; pos: number };
      item: { node: ProseMirrorNode; pos: number };
      itemIndex: number;
      textblock: { node: ProseMirrorNode; pos: number };
    }
  | undefined {
  let itemDepth = -1;
  let textblockDepth = -1;
  for (let depth = $pos.depth; depth > 0; depth -= 1) {
    const node = $pos.node(depth);
    if (textblockDepth < 0 && node.isTextblock) {
      textblockDepth = depth;
    }
    if (node.type.name === 'list_item') {
      itemDepth = depth;
      break;
    }
  }
  if (itemDepth < 0 || textblockDepth < 0) {
    return undefined;
  }

  const listDepth = itemDepth - 1;
  const listNode = $pos.node(listDepth);
  if (listNode.type.name !== 'bullet_list' && listNode.type.name !== 'ordered_list') {
    return undefined;
  }

  return {
    list: { node: listNode, pos: $pos.before(listDepth) },
    item: { node: $pos.node(itemDepth), pos: $pos.before(itemDepth) },
    itemIndex: $pos.index(listDepth),
    textblock: { node: $pos.node(textblockDepth), pos: $pos.before(textblockDepth) }
  };
}

function selectedTableSnapshot(transaction: Transaction): EditorTableSnapshot | null {
  const context = selectedEditableTableContext(transaction);
  if (!context) {
    return null;
  }

  return {
    rows: context.rows,
    columns: context.columns,
    canAddRow: context.rows < MAX_TABLE_ROWS,
    canDeleteRow: context.rows > MIN_TABLE_ROWS,
    canAddColumn: context.columns < MAX_TABLE_COLUMNS,
    canDeleteColumn: context.columns > MIN_TABLE_COLUMNS,
    canDeleteTable: true
  };
}

function selectedEditableTableContext(transaction: Transaction):
  | {
      table: { node: ProseMirrorNode; pos: number };
      row: { node: ProseMirrorNode; pos: number };
      cell: { node: ProseMirrorNode; pos: number };
      rowIndex: number;
      cellIndex: number;
      rows: number;
      columns: number;
    }
  | undefined {
  const fromContext = tablePositionContext(transaction.selection.$from);
  const toContext = tablePositionContext(transaction.selection.$to);
  if (!fromContext || !toContext || fromContext.table.pos !== toContext.table.pos) {
    return undefined;
  }
  if (!editableRectangularTable(fromContext.table.node)) {
    return undefined;
  }

  return {
    ...fromContext,
    rows: fromContext.table.node.childCount,
    columns: fromContext.row.node.childCount
  };
}

function tablePositionContext($pos: ResolvedPos):
  | {
      table: { node: ProseMirrorNode; pos: number };
      row: { node: ProseMirrorNode; pos: number };
      cell: { node: ProseMirrorNode; pos: number };
      rowIndex: number;
      cellIndex: number;
    }
  | undefined {
  let tableDepth = -1;
  let rowDepth = -1;
  let cellDepth = -1;
  for (let depth = $pos.depth; depth > 0; depth -= 1) {
    const name = $pos.node(depth).type.name;
    if (cellDepth < 0 && name === 'table_cell') {
      cellDepth = depth;
    } else if (rowDepth < 0 && name === 'table_row') {
      rowDepth = depth;
    } else if (tableDepth < 0 && name === 'table') {
      tableDepth = depth;
    }
  }

  if (tableDepth < 0 || rowDepth < 0 || cellDepth < 0) {
    return undefined;
  }

  return {
    table: { node: $pos.node(tableDepth), pos: $pos.before(tableDepth) },
    row: { node: $pos.node(rowDepth), pos: $pos.before(rowDepth) },
    cell: { node: $pos.node(cellDepth), pos: $pos.before(cellDepth) },
    rowIndex: $pos.index(tableDepth),
    cellIndex: $pos.index(rowDepth)
  };
}

function editableRectangularTable(table: ProseMirrorNode): boolean {
  if (table.type.name !== 'table' || table.childCount < MIN_TABLE_ROWS) {
    return false;
  }

  const columns = table.child(0).childCount;
  if (columns < MIN_TABLE_COLUMNS) {
    return false;
  }

  let editable = true;
  table.forEach((row) => {
    if (row.type.name !== 'table_row' || row.childCount !== columns) {
      editable = false;
      return;
    }
    row.forEach((cell) => {
      if (
        cell.type.name !== 'table_cell' ||
        cell.attrs.unsupported === true ||
        cell.attrs.sourceEmpty === true
      ) {
        editable = false;
      }
    });
  });
  return editable;
}

function deleteSelectedTable(
  transaction: Transaction,
  context: NonNullable<ReturnType<typeof selectedEditableTableContext>>
): Transaction {
  const paragraph = supportedSchema.nodes.paragraph.create({ style: 'body' });
  if (transaction.doc.childCount === 1) {
    transaction = transaction.replaceWith(
      context.table.pos,
      context.table.pos + context.table.node.nodeSize,
      paragraph
    );
    return transaction.setSelection(TextSelection.create(transaction.doc, context.table.pos + 1));
  }

  transaction = transaction.delete(context.table.pos, context.table.pos + context.table.node.nodeSize);
  const cursor =
    firstTextblockStartBetween(transaction.doc, context.table.pos, transaction.doc.content.size) ??
    firstTextblockStartBetween(transaction.doc, 0, Math.min(context.table.pos, transaction.doc.content.size));
  if (cursor !== undefined) {
    transaction = transaction.setSelection(TextSelection.create(transaction.doc, cursor));
  }
  return transaction;
}

function replaceSelectedTable(
  transaction: Transaction,
  context: NonNullable<ReturnType<typeof selectedEditableTableContext>>,
  rows: ProseMirrorNode[],
  targetRowIndex: number,
  targetCellIndex: number
): Transaction | undefined {
  const nextTable = context.table.node.type.create(context.table.node.attrs, rows, context.table.node.marks);
  transaction = transaction.replaceWith(
    context.table.pos,
    context.table.pos + context.table.node.nodeSize,
    nextTable
  );

  const cursor = tableCellTextblockStart(transaction.doc, context.table.pos, targetRowIndex, targetCellIndex);
  return cursor === undefined ? undefined : transaction.setSelection(TextSelection.create(transaction.doc, cursor));
}

function tableCellTextblockStart(
  doc: ProseMirrorNode,
  tablePos: number,
  rowIndex: number,
  cellIndex: number
): number | undefined {
  const table = doc.nodeAt(tablePos);
  if (!table || rowIndex < 0 || rowIndex >= table.childCount) {
    return undefined;
  }

  let rowPos = tablePos + 1;
  for (let index = 0; index < rowIndex; index += 1) {
    rowPos += table.child(index).nodeSize;
  }

  const row = table.child(rowIndex);
  if (cellIndex < 0 || cellIndex >= row.childCount) {
    return undefined;
  }

  let cellPos = rowPos + 1;
  for (let index = 0; index < cellIndex; index += 1) {
    cellPos += row.child(index).nodeSize;
  }

  let found: number | undefined;
  row.child(cellIndex).descendants((node, pos) => {
    if (found !== undefined) {
      return false;
    }
    if (node.isTextblock) {
      found = cellPos + 1 + pos + 1;
      return false;
    }
    return true;
  });
  return found;
}

function exitEmptyListItem(
  transaction: Transaction,
  context: NonNullable<ReturnType<typeof selectedListPositionContext>>
): Transaction | undefined {
  const paragraphType = supportedSchema.nodes.paragraph;
  if (!paragraphType) {
    return undefined;
  }

  const items = nodeChildren(context.list.node);
  const replacement: ProseMirrorNode[] = [];
  const beforeItems = items.slice(0, context.itemIndex);
  const afterItems = items.slice(context.itemIndex + 1);

  if (beforeItems.length > 0) {
    replacement.push(context.list.node.type.create(context.list.node.attrs, beforeItems, context.list.node.marks));
  }

  const paragraph = paragraphType.create({ style: 'body' });
  replacement.push(paragraph);

  if (afterItems.length > 0) {
    replacement.push(context.list.node.type.create(context.list.node.attrs, afterItems, context.list.node.marks));
  }

  transaction = transaction.replaceWith(
    context.list.pos,
    context.list.pos + context.list.node.nodeSize,
    Fragment.fromArray(replacement)
  );

  const beforeSize = beforeItems.length > 0 ? replacement[0].nodeSize : 0;
  const paragraphPos = context.list.pos + beforeSize;
  return transaction.setSelection(TextSelection.create(transaction.doc, paragraphPos + 1));
}

function listItemParagraphStart(
  doc: ProseMirrorNode,
  listPos: number,
  itemIndex: number
): number | undefined {
  const list = doc.nodeAt(listPos);
  if (!list || itemIndex < 0 || itemIndex >= list.childCount) {
    return undefined;
  }

  let itemPos = listPos + 1;
  for (let index = 0; index < itemIndex; index += 1) {
    itemPos += list.child(index).nodeSize;
  }

  const item = list.child(itemIndex);
  if (item.childCount === 0 || !item.child(0).isTextblock) {
    return undefined;
  }
  return itemPos + 2;
}

function nodeChildren(node: ProseMirrorNode): ProseMirrorNode[] {
  const children: ProseMirrorNode[] = [];
  node.forEach((child) => children.push(child));
  return children;
}

function createTableNode(rows: number, columns: number): ProseMirrorNode | undefined {
  const tableType = supportedSchema.nodes.table;
  const rowType = supportedSchema.nodes.table_row;
  if (
    !tableType ||
    !rowType ||
    !validTableDimension(rows, MIN_TABLE_ROWS, MAX_TABLE_ROWS) ||
    !validTableDimension(columns, MIN_TABLE_COLUMNS, MAX_TABLE_COLUMNS)
  ) {
    return undefined;
  }

  return tableType.create(
    null,
    Array.from({ length: rows }, () => createEmptyTableRow(columns))
  );
}

function createEmptyTableRow(columns: number): ProseMirrorNode {
  return supportedSchema.nodes.table_row.create(
    null,
    Array.from({ length: columns }, () => createEmptyTableCell())
  );
}

function createEmptyTableCell(): ProseMirrorNode {
  return supportedSchema.nodes.table_cell.create(
    { unsupported: false, sourceEmpty: false },
    supportedSchema.nodes.paragraph.create({ style: 'body' })
  );
}

function validTableDimension(value: number, min: number, max: number): boolean {
  return Number.isInteger(value) && value >= min && value <= max;
}

function isEmptyParagraphNode(node: ProseMirrorNode): boolean {
  return node.type.name === 'paragraph' && node.textContent.length === 0;
}

function firstTextblockStartBetween(doc: ProseMirrorNode, from: number, to: number): number | undefined {
  let found: number | undefined;
  doc.nodesBetween(from, Math.min(to, doc.content.size), (node, pos) => {
    if (found !== undefined) {
      return false;
    }
    if (node.isTextblock) {
      found = pos + 1;
      return false;
    }
    return true;
  });
  return found;
}

function parsePlainTextBlocks(text: string): ProseMirrorNode[] {
  const lines = text
    .replace(/\r\n/g, '\n')
    .replace(/\r/g, '\n')
    .split('\n')
    .map((line) => line.trimEnd())
    .filter((line) => line.trim().length > 0);
  if (lines.length === 0) {
    return [];
  }

  const parsedListLines = lines.map(parsePlainTextListLine);
  if (parsedListLines.every((line) => line !== undefined)) {
    const listLines = parsedListLines as Array<NonNullable<ReturnType<typeof parsePlainTextListLine>>>;
    const ordered = listLines.every((line) => line.ordered);
    const listType = supportedSchema.nodes[ordered ? 'ordered_list' : 'bullet_list'];
    const itemType = supportedSchema.nodes.list_item;
    const paragraphType = supportedSchema.nodes.paragraph;
    const definitionId = ordered ? '900w-ordered' : '900w-unordered';
    return [
      listType.create(
        { definitionId },
        listLines.map((line) =>
          itemType.create(
            { level: line.level },
            paragraphType.create({ style: 'body' }, line.text ? supportedSchema.text(line.text) : undefined)
          )
        )
      )
    ];
  }

  return lines.map((line) =>
    supportedSchema.nodes.paragraph.create(
      { style: 'body' },
      line.trim().length > 0 ? supportedSchema.text(line.trim()) : undefined
    )
  );
}

function parsePlainTextListLine(line: string): { ordered: boolean; text: string; level: number } | undefined {
  const match = /^(\s*)(?:(?:[-*•])|(?:(?:\d+|[A-Za-z])[.)]))\s+(.+)$/.exec(line);
  if (!match) {
    return undefined;
  }
  const marker = line.trimStart();
  const ordered = /^(?:\d+|[A-Za-z])[.)]\s+/.test(marker);
  const indentUnits = match[1].replace(/\t/g, '    ').length;
  return {
    ordered,
    text: match[2].trim(),
    level: Math.min(8, Math.max(1, Math.floor(indentUnits / 2) + 1))
  };
}

function emptyEditorFormattingSnapshot(): EditorFormattingSnapshot {
  return {
    blockType: null,
    styleId: 'body',
    paragraphFormat: {},
    textStyle: {},
    marks: {
      bold: false,
      italic: false,
      underline: false,
      strikethrough: false,
      superscript: false,
      subscript: false
    },
    linkHref: null,
    blockBookmarkId: null,
    list: null,
    table: null,
    image: null,
    selectionWordCount: 0
  };
}

function compactImageAttrs(attrs: SupportedImageAttrs): SupportedImageAttrs {
  const compact: SupportedImageAttrs = {};
  if (typeof attrs.altText === 'string') {
    compact.altText = attrs.altText.trim().length > 0 ? attrs.altText : 'Image';
  }
  if (attrs.alignment !== undefined && attrs.alignment !== null) {
    compact.alignment = normalizeImageAlignment(attrs.alignment);
  }
  if (attrs.scalePercent !== undefined && attrs.scalePercent !== null) {
    compact.scalePercent = normalizeImageScale(attrs.scalePercent);
  }
  if (typeof attrs.caption === 'string') {
    compact.caption = attrs.caption.trim().length > 0 ? attrs.caption : null;
  }
  return compact;
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

function markActive(transaction: Transaction, markName: SupportedMarkName): boolean {
  const markType = supportedSchema.marks[markName];
  if (!markType) {
    return false;
  }

  if (transaction.selection.empty) {
    return Boolean(markType.isInSet(transaction.storedMarks ?? transaction.selection.$from.marks()));
  }
  return transaction.doc.rangeHasMark(transaction.selection.from, transaction.selection.to, markType);
}

function linkHrefNearSelection(transaction: Transaction): string | null {
  const markType = supportedSchema.marks.link;
  if (!markType) {
    return null;
  }

  if (!transaction.selection.empty) {
    let href: string | null = null;
    transaction.doc.nodesBetween(transaction.selection.from, transaction.selection.to, (node) => {
      if (!node.isText) {
        return true;
      }
      const mark = markType.isInSet(node.marks);
      if (typeof mark?.attrs.href === 'string') {
        href = mark.attrs.href;
        return false;
      }
      return true;
    });
    return href;
  }

  const stored = markType.isInSet(transaction.storedMarks ?? transaction.selection.$from.marks());
  if (typeof stored?.attrs.href === 'string') {
    return stored.attrs.href;
  }

  const activeRange = markRangeAroundPosition(transaction.doc, transaction.selection.from, markType);
  if (activeRange) {
    return activeRange.href;
  }
  return null;
}

function markRangeAroundPosition(
  doc: ProseMirrorNode,
  position: number,
  markType: MarkType
): { from: number; to: number; href: string } | undefined {
  const activeMark = markType.isInSet(doc.resolve(position).marks());
  if (typeof activeMark?.attrs.href !== 'string') {
    return undefined;
  }

  const activeHref = activeMark.attrs.href;
  let range: { from: number; to: number; href: string } | undefined;
  const from = Math.max(0, position - 1);
  const to = Math.min(doc.content.size, position + 1);
  doc.nodesBetween(from, to, (node, pos, parent, index) => {
    if (range || !node.isText || !parent || index === undefined) {
      return !range;
    }

    const mark = markType.isInSet(node.marks);
    if (mark?.attrs.href !== activeHref) {
      return true;
    }

    let start = pos;
    let previousOffset = pos;
    for (let previous = index - 1; previous >= 0; previous -= 1) {
      const sibling = parent.child(previous);
      previousOffset -= sibling.nodeSize;
      const siblingMark = markType.isInSet(sibling.marks);
      if (siblingMark?.attrs.href !== activeHref) {
        break;
      }
      start = previousOffset;
    }

    let end = pos + node.nodeSize;
    let nextOffset = end;
    for (let next = index + 1; next < parent.childCount; next += 1) {
      const sibling = parent.child(next);
      const siblingMark = markType.isInSet(sibling.marks);
      if (siblingMark?.attrs.href !== activeHref) {
        break;
      }
      end = nextOffset + sibling.nodeSize;
      nextOffset = end;
    }

    range = { from: start, to: end, href: activeHref };
    return false;
  });
  return range;
}

function countSelectionWords(transaction: Transaction): number {
  if (transaction.selection.empty) {
    return 0;
  }
  return transaction.doc
    .textBetween(transaction.selection.from, transaction.selection.to, '\n', ' ')
    .trim()
    .split(/\s+/)
    .filter(Boolean).length;
}

function assignDefinedParagraphAttrs(target: SupportedParagraphAttrs, attrs: SupportedParagraphAttrs) {
  const keys: Array<keyof SupportedParagraphAttrs> = [
    'align',
    'lineSpacing',
    'spacingBefore',
    'spacingAfter',
    'indentStart',
    'indentEnd',
    'firstLineIndent'
  ];
  for (const key of keys) {
    if (attrs[key] !== undefined) {
      target[key] = attrs[key] as never;
    }
  }
}

function mapPlainTextRangeInsideDoc(
  doc: ProseMirrorNode,
  fromIndex: number,
  toIndex: number
): { from: number; to: number } | undefined {
  let documentTextIndex = 0;
  let mapped: { from: number; to: number } | undefined;
  doc.descendants((node, pos) => {
    if (mapped || !node.isTextblock) {
      return !mapped;
    }

    const blockText = node.textContent;
    const blockStart = documentTextIndex;
    const blockEnd = blockStart + blockText.length;
    if (fromIndex >= blockStart && toIndex <= blockEnd) {
      mapped = mapTextRangeInsideTextblock(node, pos, fromIndex - blockStart, toIndex - fromIndex);
      return false;
    }
    documentTextIndex = blockEnd + 1;
    return false;
  });
  return mapped;
}

function utf8ByteOffsetToStringIndex(text: string, byteOffset: number): number {
  if (byteOffset <= 0) {
    return 0;
  }

  const encoder = new TextEncoder();
  let bytes = 0;
  let index = 0;
  for (const char of text) {
    const nextBytes = bytes + encoder.encode(char).length;
    if (nextBytes > byteOffset) {
      return index;
    }
    bytes = nextBytes;
    index += char.length;
  }
  return text.length;
}

function compactTextStyleAttrs(attrs: SupportedTextStyleAttrs): Record<string, string | number> {
  const output: Record<string, string | number> = {};
  if (attrs.fontFamily) {
    output.fontFamily = attrs.fontFamily;
  }
  if (attrs.fontSizePt) {
    output.fontSizePt = attrs.fontSizePt;
  }
  if (attrs.textColor) {
    output.textColor = attrs.textColor;
  }
  if (attrs.highlightColor) {
    output.highlightColor = attrs.highlightColor;
  }
  return output;
}

function compactParagraphAttrs(attrs: SupportedParagraphAttrs): Record<string, string | number | null> {
  const output: Record<string, string | number | null> = {};
  for (const [key, value] of Object.entries(attrs)) {
    output[key] = value ?? null;
  }
  return output;
}

function textStyleAttrsNearSelection(
  transaction: Transaction,
  markType: (typeof supportedSchema.marks)['textStyle']
): Record<string, string | number> {
  const active = markType.isInSet(transaction.storedMarks ?? transaction.selection.$from.marks());
  if (active) {
    return compactTextStyleAttrs(active.attrs as SupportedTextStyleAttrs);
  }

  const { from, to } = transaction.selection;
  let found: Record<string, string | number> = {};
  transaction.doc.nodesBetween(from, to, (node) => {
    if (!node.isText) {
      return true;
    }
    const mark = markType.isInSet(node.marks);
    if (mark) {
      found = compactTextStyleAttrs(mark.attrs as SupportedTextStyleAttrs);
      return false;
    }
    return true;
  });
  return found;
}

function mapTextRangeInsideTextblock(
  textblock: ProseMirrorNode,
  textblockPos: number,
  index: number,
  length: number
): { from: number; to: number } | undefined {
  const end = index + length;
  let textOffset = 0;
  let from: number | undefined;
  let to: number | undefined;

  textblock.descendants((node, pos) => {
    if (!node.isText) {
      return true;
    }

    const text = node.text ?? '';
    const nextOffset = textOffset + text.length;
    if (from === undefined && index >= textOffset && index <= nextOffset) {
      from = textblockPos + 1 + pos + (index - textOffset);
    }
    if (to === undefined && end >= textOffset && end <= nextOffset) {
      to = textblockPos + 1 + pos + (end - textOffset);
      return false;
    }
    textOffset = nextOffset;
    return true;
  });

  return from === undefined || to === undefined ? undefined : { from, to };
}

function isValidDocumentRange(doc: ProseMirrorNode, from: number, to: number): boolean {
  return Number.isInteger(from) && Number.isInteger(to) && from >= 0 && to >= from && to <= doc.content.size;
}
