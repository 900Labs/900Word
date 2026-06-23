import type { Node as ProseMirrorNode } from 'prosemirror-model';
import { EditorState } from 'prosemirror-state';
import { TextSelection } from 'prosemirror-state';
import { EditorView } from 'prosemirror-view';
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

export interface EditorFindMatch extends FindRange {
  from: number;
  to: number;
}

export function createEditor(
  host: HTMLElement,
  document: DocumentState,
  onChange: (change: EditorProjectedChange) => void,
  options: { editable: boolean } = { editable: true }
): EditorView {
  const state = EditorState.create({
    doc: supportedSchema.nodeFromJSON(documentToEditorDoc(document))
  });

  const view = new EditorView(host, {
    state,
    editable: () => options.editable,
    dispatchTransaction(transaction) {
      const nextState = view.state.apply(transaction);
      view.updateState(nextState);
      if (!transaction.docChanged) {
        return;
      }
      onChange({
        text: view.state.doc.textContent,
        blocks: editorDocToWordCoreBlocks(view.state.doc.toJSON() as EditorDoc)
      });
    }
  });

  return view;
}

export function toggleEditorMark(view: EditorView | undefined, markName: SupportedMarkName): boolean {
  if (!view) {
    return false;
  }

  const markType = supportedSchema.marks[markName];
  if (!markType) {
    return false;
  }

  const { state } = view;
  const { empty, from, to } = state.selection;
  let transaction = state.tr;

  if (empty) {
    const active = markType.isInSet(state.storedMarks ?? state.selection.$from.marks());
    transaction = active
      ? transaction.removeStoredMark(markType)
      : transaction.addStoredMark(markType.create());
  } else if (state.doc.rangeHasMark(from, to, markType)) {
    transaction = transaction.removeMark(from, to, markType);
  } else {
    transaction = transaction.addMark(from, to, markType.create());
  }

  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
}

export function setEditorBlockType(
  view: EditorView | undefined,
  blockName: SupportedBlockName,
  attrs: Record<string, string | number> = {}
): boolean {
  if (!view) {
    return false;
  }

  const nodeType = supportedSchema.nodes[blockName];
  if (!nodeType) {
    return false;
  }

  const { state } = view;
  const { from, to } = selectedTextblockRange(state);
  let changed = false;
  let transaction = state.tr;

  state.doc.nodesBetween(from, to, (node, pos) => {
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
    return false;
  }

  view.dispatch(transaction.scrollIntoView());
  view.focus();
  return true;
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

function selectedTextblockRange(state: EditorState): { from: number; to: number } {
  if (state.selection.$from.depth === 0 || state.selection.$to.depth === 0) {
    return { from: 0, to: state.doc.content.size };
  }

  const fromDepth = Math.max(1, state.selection.$from.depth);
  const toDepth = Math.max(1, state.selection.$to.depth);
  return {
    from: state.selection.$from.before(fromDepth),
    to: state.selection.$to.after(toDepth)
  };
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
