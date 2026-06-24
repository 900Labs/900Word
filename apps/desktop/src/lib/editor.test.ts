import { describe, expect, it } from 'vitest';
import { EditorState, TextSelection, type Transaction } from 'prosemirror-state';
import {
  findEditorDocMatches,
  clearEditorDirectFormattingTransaction,
  setEditorParagraphFormatTransaction,
  restoreEditorSelection,
  setEditorBlockType,
  setEditorBlockTypeTransaction,
  setEditorTextStyleTransaction,
  toggleEditorMark,
  toggleEditorListTransaction,
  toggleEditorMarkTransaction
} from './editor';
import { supportedSchema } from './editorSchema';

describe('findEditorDocMatches', () => {
  it('does not match across paragraph boundaries', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'ab' }] },
        { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'cd' }] }
      ]
    });

    expect(findEditorDocMatches(doc, 'bc')).toEqual([]);
  });

  it('matches within one paragraph even when marks split text nodes', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            { type: 'text', text: 'ab' },
            { type: 'text', text: 'cd', marks: [{ type: 'bold' }] }
          ]
        }
      ]
    });

    const matches = findEditorDocMatches(doc, 'bc');

    expect(matches).toHaveLength(1);
    expect(matches[0].length).toBe(2);
    expect(matches[0].to).toBeGreaterThan(matches[0].from);
  });

  it('applies marks to a fallback text selection when toolbar focus collapses the live selection', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Hello world' }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = toggleEditorMarkTransaction(state, 'bold', {
      from: 1,
      to: 6,
      empty: false
    });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    const firstText = nextState.doc.firstChild?.firstChild;
    expect(firstText?.marks.map((mark) => mark.type.name)).toEqual(['bold']);
    expect(firstText?.text).toBe('Hello');
  });

  it('changes the selected block type from a fallback toolbar selection', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Heading text' }]
        }
      ]
    });
    const selectedState = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, 1, 8))
    );
    const collapsedState = selectedState.apply(
      selectedState.tr.setSelection(TextSelection.create(selectedState.doc, 1, 1))
    );

    const transaction = setEditorBlockTypeTransaction(collapsedState, 'heading', { level: 1 }, {
      from: 1,
      to: 8,
      empty: false
    });

    expect(transaction).toBeDefined();
    const nextState = collapsedState.apply(transaction!);
    expect(nextState.doc.firstChild?.type.name).toBe('heading');
    expect(nextState.doc.firstChild?.attrs.level).toBe(1);
  });

  it('restores a toolbar selection before applying editor commands', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Hello world' }]
        }
      ]
    });
    let state = EditorState.create({ doc });
    const view = {
      state,
      dispatch(transaction: Transaction) {
        state = state.apply(transaction);
        this.state = state;
      }
    };

    const restored = restoreEditorSelection(view as unknown as Parameters<typeof restoreEditorSelection>[0], {
      from: 1,
      to: 6,
      empty: false
    });

    expect(restored).toBe(true);
    expect(view.state.selection.empty).toBe(false);
    expect(view.state.selection.from).toBe(1);
    expect(view.state.selection.to).toBe(6);
  });

  it('toggles a mark through the toolbar command path with a fallback selection', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Hello world' }]
        }
      ]
    });
    let state = EditorState.create({ doc });
    const view = {
      state,
      dispatch(transaction: Transaction) {
        state = state.apply(transaction);
        this.state = state;
      },
      focus() {}
    };

    const changed = toggleEditorMark(view as unknown as Parameters<typeof toggleEditorMark>[0], 'bold', {
      from: 1,
      to: 6,
      empty: false
    });

    expect(changed).toBe(true);
    expect(view.state.doc.firstChild?.firstChild?.marks.map((mark) => mark.type.name)).toEqual(['bold']);
  });

  it('changes block type through the toolbar command path with a fallback selection', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Heading text' }]
        }
      ]
    });
    let state = EditorState.create({ doc });
    const view = {
      state,
      dispatch(transaction: Transaction) {
        state = state.apply(transaction);
        this.state = state;
      },
      focus() {}
    };

    const changed = setEditorBlockType(
      view as unknown as Parameters<typeof setEditorBlockType>[0],
      'heading',
      { level: 1 },
      {
        from: 1,
        to: 8,
        empty: false
      }
    );

    expect(changed).toBe(true);
    expect(view.state.doc.firstChild?.type.name).toBe('heading');
    expect(view.state.doc.firstChild?.attrs.level).toBe(1);
  });

  it('applies direct text style to the selected text', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Hello world' }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = setEditorTextStyleTransaction(
      state,
      { fontFamily: 'serif', fontSizePt: 14, textColor: '#1f2937' },
      { from: 1, to: 6, empty: false }
    );

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.firstChild?.marks[0].type.name).toBe('textStyle');
    expect(nextState.doc.firstChild?.firstChild?.marks[0].attrs.fontFamily).toBe('serif');
  });

  it('composes sequential direct text style updates on the same selection', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Hello world' }]
        }
      ]
    });
    let state = EditorState.create({ doc });

    const family = setEditorTextStyleTransaction(state, { fontFamily: 'serif' }, { from: 1, to: 6, empty: false });
    state = state.apply(family!);
    const size = setEditorTextStyleTransaction(state, { fontSizePt: 14 }, { from: 1, to: 6, empty: false });
    state = state.apply(size!);
    const color = setEditorTextStyleTransaction(state, { textColor: '#1f2937' }, { from: 1, to: 6, empty: false });
    state = state.apply(color!);

    const attrs = state.doc.firstChild?.firstChild?.marks[0].attrs;
    expect(attrs?.fontFamily).toBe('serif');
    expect(attrs?.fontSizePt).toBe(14);
    expect(attrs?.textColor).toBe('#1f2937');
  });

  it('applies paragraph format attrs to selected paragraphs', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Paragraph' }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = setEditorParagraphFormatTransaction(
      state,
      { align: 'center', lineSpacing: 1500, spacingAfter: 4 },
      { from: 1, to: 4, empty: false }
    );

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.attrs.align).toBe('center');
    expect(nextState.doc.firstChild?.attrs.lineSpacing).toBe(1500);
    expect(nextState.doc.firstChild?.attrs.spacingAfter).toBe(4);
  });

  it('clears direct inline and paragraph formatting without changing paragraph style', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'caption', align: 'right' },
          content: [{ type: 'text', text: 'Styled', marks: [{ type: 'bold' }] }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = clearEditorDirectFormattingTransaction(state, { from: 1, to: 6, empty: false });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.attrs.style).toBe('caption');
    expect(nextState.doc.firstChild?.attrs.align).toBeNull();
    expect(nextState.doc.firstChild?.firstChild?.marks).toEqual([]);
  });

  it('converts selected top-level paragraphs into a real list node', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'One' }] },
        { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Two' }] }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = toggleEditorListTransaction(state, 'bullet_list', { from: 1, to: doc.content.size - 1, empty: false });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.type.name).toBe('bullet_list');
    expect(nextState.doc.firstChild?.childCount).toBe(2);
  });

  it('unwraps every child block from list items instead of dropping extra blocks', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'bullet_list',
          content: [
            {
              type: 'list_item',
              attrs: { level: 1 },
              content: [
                { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'One' }] },
                { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Two' }] }
              ]
            }
          ]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = toggleEditorListTransaction(state, 'bullet_list', { from: 1, to: doc.content.size - 1, empty: false });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.childCount).toBe(2);
    expect(nextState.doc.child(0).textContent).toBe('One');
    expect(nextState.doc.child(1).textContent).toBe('Two');
  });
});
