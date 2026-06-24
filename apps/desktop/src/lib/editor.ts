import { Fragment, type Node as ProseMirrorNode, type ResolvedPos } from 'prosemirror-model';
import { EditorState, type Transaction } from 'prosemirror-state';
import { TextSelection } from 'prosemirror-state';
import { Decoration, DecorationSet, EditorView } from 'prosemirror-view';
import 'prosemirror-view/style/prosemirror.css';
import {
  documentToEditorDoc,
  editorDocToWordCoreBlocks,
  type DocumentState,
  type EditorDoc,
  type EditorProjectedChange
} from './documentProjection';
import { findTextRanges, type FindRange } from './findReplace';
import { supportedSchema } from './editorSchema';

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
  list: {
    type: SupportedListName;
    level: number;
  } | null;
  selectionWordCount: number;
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

interface CreateEditorOptions {
  editable: boolean;
  onInteraction?: () => void;
  onSelectionChange?: (selection: EditorSelectionSnapshot) => void;
}

interface SpellDecorationHolder {
  value: DecorationSet;
}

const spellDecorationStore = new WeakMap<EditorView, SpellDecorationHolder>();

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
    handleKeyDown(editorView, event) {
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
      if (transaction.selectionSet) {
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
  const list = selectedListContext(selection.$from);
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
    list,
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
    if (node.type !== nodeType || JSON.stringify(node.attrs) !== JSON.stringify(attrs)) {
      transaction = transaction.setNodeMarkup(pos, nodeType, attrs);
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
      if (markType.name === 'link') {
        continue;
      }
      transaction = transaction.removeStoredMark(markType);
    }
  } else {
    for (const markType of Object.values(supportedSchema.marks)) {
      if (markType.name === 'link') {
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
            style: node.type.name === 'paragraph' ? node.attrs.style || 'body' : 'body'
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
    list: null,
    selectionWordCount: 0
  };
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
