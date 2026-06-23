import { describe, expect, it } from 'vitest';
import { EditorState, TextSelection } from 'prosemirror-state';
import { findEditorDocMatches, setEditorBlockTypeTransaction, toggleEditorMarkTransaction } from './editor';
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
});
