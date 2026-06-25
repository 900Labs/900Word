import { describe, expect, it } from 'vitest';
import { EditorState, NodeSelection, TextSelection, type Transaction } from 'prosemirror-state';
import {
  findEditorDocMatches,
  addEditorCommentTransaction,
  clearEditorDirectFormattingTransaction,
  continueListOnEnterTransaction,
  editorDocPlainText,
  editorStateSelectionFormatting,
  editTableStructureTransaction,
  imageScalePercentFromResizeDrag,
  insertDefaultTableTransaction,
  insertTableTransaction,
  mapSpellIssuesToEditorRanges,
  pastePlainTextAsBlocksTransaction,
  removeEditorCommentTransaction,
  removeEditorLinkTransaction,
  removeEditorBlockBookmarkTransaction,
  setEditorParagraphFormatTransaction,
  setEditorBlockBookmarkTransaction,
  setSelectedImageAttrsTransaction,
  restoreEditorSelection,
  selectEditorTopLevelBlock,
  setEditorBlockType,
  setEditorBlockTypeTransaction,
  setEditorLinkTransaction,
  setEditorTextStyleTransaction,
  selectedEditorText,
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

  it('applies and removes safe link marks on a selected range', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Visit example' }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const linked = setEditorLinkTransaction(state, 'https://example.invalid', { from: 1, to: 6, empty: false });
    expect(linked).toBeDefined();
    let nextState = state.apply(linked!);
    expect(nextState.doc.firstChild?.firstChild?.marks[0].type.name).toBe('link');
    expect(nextState.doc.firstChild?.firstChild?.marks[0].attrs.href).toBe('https://example.invalid');

    const removed = removeEditorLinkTransaction(nextState, { from: 1, to: 6, empty: false });
    expect(removed).toBeDefined();
    nextState = nextState.apply(removed!);
    expect(nextState.doc.firstChild?.firstChild?.marks).toEqual([]);
  });

  it('adds a comment mark only to non-empty selected text', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Comment this text' }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = addEditorCommentTransaction(state, 'cmt-abc123', {
      from: 1,
      to: 8,
      empty: false
    });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.firstChild?.marks.map((mark) => mark.type.name)).toContain('comment');
    expect(nextState.doc.firstChild?.firstChild?.marks.find((mark) => mark.type.name === 'comment')?.attrs.id).toBe(
      'cmt-abc123'
    );
    expect(selectedEditorText({ state: nextState } as Parameters<typeof selectedEditorText>[0], {
      from: 1,
      to: 8,
      empty: false
    })).toBe('Comment');
    expect(addEditorCommentTransaction(state, 'cmt-abc123', { from: 1, to: 1, empty: true })).toBeUndefined();
    expect(addEditorCommentTransaction(state, '../bad', { from: 1, to: 8, empty: false })).toBeUndefined();
  });

  it('keeps overlapping comment marks from editor operations', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'abcdef' }]
        }
      ]
    });
    let state = EditorState.create({ doc });

    const first = addEditorCommentTransaction(state, 'cmt-first', { from: 1, to: 5, empty: false });
    expect(first).toBeDefined();
    state = state.apply(first!);

    state = state.apply(state.tr.setSelection(TextSelection.create(state.doc, 3, 7)));
    const second = addEditorCommentTransaction(state, 'cmt-second');
    expect(second).toBeDefined();
    state = state.apply(second!);

    const segments: Array<{ text: string | undefined; commentIds: string[] }> = [];
    state.doc.descendants((node) => {
      if (node.isText) {
        segments.push({
          text: node.text,
          commentIds: node.marks
            .filter((mark) => mark.type.name === 'comment')
            .map((mark) => String(mark.attrs.id))
            .sort()
        });
      }
      return true;
    });
    expect(segments).toEqual([
      { text: 'ab', commentIds: ['cmt-first'] },
      { text: 'cd', commentIds: ['cmt-first', 'cmt-second'] },
      { text: 'ef', commentIds: ['cmt-second'] }
    ]);
  });

  it('removes all anchors for a deleted comment id', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            { type: 'text', text: 'One', marks: [{ type: 'comment', attrs: { id: 'cmt-remove' } }] },
            { type: 'text', text: ' two', marks: [{ type: 'comment', attrs: { id: 'cmt-keep' } }] }
          ]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = removeEditorCommentTransaction(state, 'cmt-remove');

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.child(0).marks).toEqual([]);
    expect(nextState.doc.firstChild?.child(1).marks[0].attrs.id).toBe('cmt-keep');
  });

  it('removes only the requested comment mark from overlapping anchors', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            {
              type: 'text',
              text: 'Shared',
              marks: [
                { type: 'comment', attrs: { id: 'cmt-remove' } },
                { type: 'comment', attrs: { id: 'cmt-keep' } }
              ]
            }
          ]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = removeEditorCommentTransaction(state, 'cmt-remove');

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.firstChild?.marks.map((mark) => mark.attrs.id)).toEqual(['cmt-keep']);
  });

  it('rejects unsafe link marks from toolbar transactions', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Unsafe' }] }]
    });
    const state = EditorState.create({ doc });

    expect(setEditorLinkTransaction(state, 'javascript:alert(1)', { from: 1, to: 7, empty: false })).toBeUndefined();
  });

  it('applies and removes a bookmark id on the selected text block', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Target' }] }]
    });
    const state = EditorState.create({ doc });

    const added = setEditorBlockBookmarkTransaction(state, 'bm-target', { from: 1, to: 3, empty: false });
    expect(added).toBeDefined();
    let nextState = state.apply(added!);
    expect(nextState.doc.firstChild?.attrs.bookmarkId).toBe('bm-target');

    const removed = removeEditorBlockBookmarkTransaction(nextState, { from: 1, to: 3, empty: false });
    expect(removed).toBeDefined();
    nextState = nextState.apply(removed!);
    expect(nextState.doc.firstChild?.attrs.bookmarkId).toBeNull();
  });

  it('preserves a bookmark id when changing paragraph to heading', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body', bookmarkId: 'bm-existing' },
          content: [{ type: 'text', text: 'Heading text' }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = setEditorBlockTypeTransaction(state, 'heading', { level: 2 }, { from: 1, to: 8, empty: false });
    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);

    expect(nextState.doc.firstChild?.type.name).toBe('heading');
    expect(nextState.doc.firstChild?.attrs.bookmarkId).toBe('bm-existing');
  });

  it('edits an existing link when the cursor is inside linked text', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            {
              type: 'text',
              text: 'Linked',
              marks: [{ type: 'link', attrs: { href: 'https://old.example.invalid' } }]
            }
          ]
        }
      ]
    });
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, 3))
    );

    const transaction = setEditorLinkTransaction(state, 'https://new.example.invalid');
    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);

    expect(nextState.doc.firstChild?.firstChild?.marks[0].attrs.href).toBe('https://new.example.invalid');
  });

  it('does not treat a cursor immediately before a link as inside the link', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            { type: 'text', text: 'Intro ' },
            {
              type: 'text',
              text: 'Linked',
              marks: [{ type: 'link', attrs: { href: 'https://old.example.invalid' } }]
            }
          ]
        }
      ]
    });
    const beforeLink = 1 + 'Intro '.length;
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, beforeLink))
    );

    expect(editorStateSelectionFormatting(state).linkHref).toBeNull();
    expect(removeEditorLinkTransaction(state)).toBeUndefined();

    const transaction = setEditorLinkTransaction(state, 'https://new.example.invalid');
    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.child(1).marks[0].attrs.href).toBe('https://old.example.invalid');
  });

  it('does not treat a cursor immediately after a link as inside the link', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            {
              type: 'text',
              text: 'Linked',
              marks: [{ type: 'link', attrs: { href: 'https://old.example.invalid' } }]
            },
            { type: 'text', text: ' plain' }
          ]
        }
      ]
    });
    const afterLink = 1 + 'Linked'.length;
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, afterLink))
    );

    expect(editorStateSelectionFormatting(state).linkHref).toBeNull();
    expect(removeEditorLinkTransaction(state)).toBeUndefined();

    const transaction = setEditorLinkTransaction(state, 'https://new.example.invalid');
    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.firstChild?.marks[0].attrs.href).toBe('https://old.example.invalid');
  });

  it('detects active link formatting from the selected text', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            {
              type: 'text',
              text: 'Linked',
              marks: [{ type: 'link', attrs: { href: 'mailto:test@example.invalid' } }]
            }
          ]
        }
      ]
    });
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, 2, 5))
    );

    expect(editorStateSelectionFormatting(state).linkHref).toBe('mailto:test@example.invalid');
  });

  it('does not report adjacent links as active for plain selected text', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            {
              type: 'text',
              text: 'Linked',
              marks: [{ type: 'link', attrs: { href: 'https://example.invalid' } }]
            },
            { type: 'text', text: ' plain' }
          ]
        }
      ]
    });
    const plainStart = 1 + 'Linked '.length;
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, plainStart, plainStart + 5))
    );

    expect(editorStateSelectionFormatting(state).linkHref).toBeNull();
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

  it('updates selected image atom metadata', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'image',
          attrs: {
            assetId: 'image-1.png',
            altText: 'Image',
            alignment: 'inline',
            scalePercent: 100,
            caption: null,
            src: 'data:image/png;base64,iVBORw0KGgo='
          }
        }
      ]
    });
    const state = EditorState.create({ doc });
    const selectedState = state.apply(state.tr.setSelection(NodeSelection.create(doc, 0)));

    const snapshot = editorStateSelectionFormatting(selectedState);
    expect(snapshot.image?.alignment).toBe('inline');

    const transaction = setSelectedImageAttrsTransaction(selectedState, {
      altText: 'Chart alt',
      alignment: 'right',
      scalePercent: 125,
      caption: 'Chart caption'
    });

    expect(transaction).toBeDefined();
    const nextState = selectedState.apply(transaction!);
    expect(nextState.doc.firstChild?.attrs.altText).toBe('Chart alt');
    expect(nextState.doc.firstChild?.attrs.alignment).toBe('right');
    expect(nextState.doc.firstChild?.attrs.scalePercent).toBe(125);
    expect(nextState.doc.firstChild?.attrs.caption).toBe('Chart caption');
  });

  it('bounds direct image resize drag scale metadata', () => {
    expect(
      imageScalePercentFromResizeDrag({
        initialScalePercent: 100,
        initialWidthPx: 400,
        deltaXPx: 100
      })
    ).toBe(125);
    expect(
      imageScalePercentFromResizeDrag({
        initialScalePercent: 100,
        initialWidthPx: 400,
        deltaXPx: -380
      })
    ).toBe(25);
    expect(
      imageScalePercentFromResizeDrag({
        initialScalePercent: 150,
        initialWidthPx: 300,
        deltaXPx: 300
      })
    ).toBe(200);
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

  it('clears visual formatting without removing comment anchors', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body', align: 'center' },
          content: [
            {
              type: 'text',
              text: 'Commented',
              marks: [
                { type: 'bold' },
                { type: 'comment', attrs: { id: 'cmt-abc123' } }
              ]
            }
          ]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = clearEditorDirectFormattingTransaction(state, { from: 1, to: 10, empty: false });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.attrs.align).toBeNull();
    expect(nextState.doc.firstChild?.firstChild?.marks.map((mark) => mark.type.name)).toEqual(['comment']);
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

  it('detects active formatting from the selected text and containing list item', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'bullet_list',
          content: [
            {
              type: 'list_item',
              attrs: { level: 2 },
              content: [
                {
                  type: 'paragraph',
                  attrs: { style: 'quote', align: 'center', lineSpacing: 1500 },
                  content: [
                    {
                      type: 'text',
                      text: 'Item',
                      marks: [
                        { type: 'bold' },
                        { type: 'textStyle', attrs: { fontFamily: 'serif', fontSizePt: 14 } }
                      ]
                    }
                  ]
                }
              ]
            }
          ]
        }
      ]
    });
    const range = textRange(doc, 'Item');
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, range.from, range.to))
    );

    const snapshot = editorStateSelectionFormatting(state);

    expect(snapshot.styleId).toBe('quote');
    expect(snapshot.paragraphFormat.align).toBe('center');
    expect(snapshot.textStyle.fontFamily).toBe('serif');
    expect(snapshot.marks.bold).toBe(true);
    expect(snapshot.list).toEqual({ type: 'bullet_list', level: 2 });
    expect(snapshot.selectionWordCount).toBe(1);
  });

  it('continues a non-empty list item on Enter', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'ordered_list',
          attrs: { definitionId: '900w-ordered' },
          content: [
            {
              type: 'list_item',
              attrs: { level: 1 },
              content: [{ type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'One' }] }]
            }
          ]
        }
      ]
    });
    const cursor = textEnd(doc, 'One');
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, cursor))
    );

    const transaction = continueListOnEnterTransaction(state);

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.childCount).toBe(2);
    expect(nextState.doc.firstChild?.child(1).attrs.level).toBe(1);
  });

  it('exits a list when Enter is pressed on an empty list item', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'bullet_list',
          attrs: { definitionId: '900w-unordered' },
          content: [
            {
              type: 'list_item',
              attrs: { level: 1 },
              content: [{ type: 'paragraph', attrs: { style: 'body' } }]
            }
          ]
        }
      ]
    });
    const cursor = firstTextblockStart(doc);
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, cursor))
    );

    const transaction = continueListOnEnterTransaction(state);

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.type.name).toBe('paragraph');
  });

  it('pastes simple bullet lines as a list', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'body' } }]
    });
    const state = EditorState.create({ doc });

    const transaction = pastePlainTextAsBlocksTransaction(state, '- One\n- Two');

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.firstChild?.type.name).toBe('bullet_list');
    expect(nextState.doc.firstChild?.childCount).toBe(2);
  });

  it('preserves a bookmark id when converting a paragraph into a list', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body', bookmarkId: 'bm-target' },
          content: [{ type: 'text', text: 'Bookmarked item' }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = toggleEditorListTransaction(state, 'bullet_list', { from: 1, to: 5, empty: false });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    const listParagraph = nextState.doc.firstChild?.firstChild?.firstChild;
    expect(listParagraph?.type.name).toBe('paragraph');
    expect(listParagraph?.attrs.bookmarkId).toBe('bm-target');
  });

  it('pastes simple newline text as paragraphs', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'body' } }]
    });
    const state = EditorState.create({ doc });

    const transaction = pastePlainTextAsBlocksTransaction(state, 'One\nTwo');

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.childCount).toBe(2);
    expect(nextState.doc.child(1).textContent).toBe('Two');
  });

  it('inserts a default 2x2 table at the toolbar selection', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Intro' }]
        }
      ]
    });
    const state = EditorState.create({ doc });

    const transaction = insertDefaultTableTransaction(state, { from: 1, to: 1, empty: true });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.childCount).toBe(2);
    expect(nextState.doc.child(0).textContent).toBe('Intro');
    const table = nextState.doc.child(1);
    expect(table.type.name).toBe('table');
    expect(table.childCount).toBe(2);
    expect(table.child(0).childCount).toBe(2);
    expect(nextState.selection.from).toBeGreaterThan(doc.child(0).nodeSize);
    expect(selectionAncestorNames(nextState)).toContain('table_cell');
    expect(nextState.selection.$from.parent.type.name).toBe('paragraph');
  });

  it('replaces an empty paragraph with the default table', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'body' } }]
    });
    const state = EditorState.create({ doc });

    const transaction = insertDefaultTableTransaction(state, { from: 1, to: 1, empty: true });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.childCount).toBe(1);
    expect(nextState.doc.child(0).type.name).toBe('table');
    expect(selectionAncestorNames(nextState)).toContain('table_cell');
  });

  it('inserts a bounded custom-size table', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'body' } }]
    });
    const state = EditorState.create({ doc });

    const transaction = insertTableTransaction(state, 3, 4, { from: 1, to: 1, empty: true });

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    const table = nextState.doc.child(0);
    expect(table.type.name).toBe('table');
    expect(table.childCount).toBe(3);
    expect(table.child(0).childCount).toBe(4);
    expect(selectionAncestorNames(nextState)).toContain('table_cell');
  });

  it('rejects unsafe table insertion dimensions', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'body' } }]
    });
    const state = EditorState.create({ doc });

    expect(insertTableTransaction(state, 0, 2)).toBeUndefined();
    expect(insertTableTransaction(state, 2, 9)).toBeUndefined();
  });

  it('adds and deletes rows around the selected table cell', () => {
    const state = tableStateWithSelection('A1');

    const added = editTableStructureTransaction(state, 'add_row_below');

    expect(added).toBeDefined();
    const withRow = state.apply(added!);
    expect(withRow.doc.child(0).childCount).toBe(3);
    expect(withRow.doc.textContent).toContain('A1');
    expect(withRow.doc.textContent).toContain('B2');
    expect(selectionAncestorNames(withRow)).toContain('table_cell');

    const deleted = editTableStructureTransaction(withRow, 'delete_row');

    expect(deleted).toBeDefined();
    const withoutInsertedRow = withRow.apply(deleted!);
    expect(withoutInsertedRow.doc.child(0).childCount).toBe(2);
  });

  it('adds and deletes columns around the selected table cell', () => {
    const state = tableStateWithSelection('A2');

    const added = editTableStructureTransaction(state, 'add_column_left');

    expect(added).toBeDefined();
    const withColumn = state.apply(added!);
    expect(withColumn.doc.child(0).child(0).childCount).toBe(3);
    expect(withColumn.doc.child(0).child(1).childCount).toBe(3);
    expect(withColumn.doc.textContent).toContain('A1');
    expect(withColumn.doc.textContent).toContain('B2');
    expect(selectionAncestorNames(withColumn)).toContain('table_cell');

    const deleted = editTableStructureTransaction(withColumn, 'delete_column');

    expect(deleted).toBeDefined();
    const withoutInsertedColumn = withColumn.apply(deleted!);
    expect(withoutInsertedColumn.doc.child(0).child(0).childCount).toBe(2);
    expect(withoutInsertedColumn.doc.child(0).child(1).childCount).toBe(2);
  });

  it('deletes a selected table and leaves an editable paragraph when it is the only block', () => {
    const state = tableStateWithSelection('A1');

    const transaction = editTableStructureTransaction(state, 'delete_table');

    expect(transaction).toBeDefined();
    const nextState = state.apply(transaction!);
    expect(nextState.doc.childCount).toBe(1);
    expect(nextState.doc.child(0).type.name).toBe('paragraph');
    expect(nextState.selection.$from.parent.type.name).toBe('paragraph');
  });

  it('refuses to delete the last row or last column', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'table',
          content: [
            {
              type: 'table_row',
              content: [
                {
                  type: 'table_cell',
                  attrs: { unsupported: false, sourceEmpty: false },
                  content: [
                    { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Only' }] }
                  ]
                }
              ]
            }
          ]
        }
      ]
    });
    const cursor = textRange(doc, 'Only').from;
    const initial = EditorState.create({ doc });
    const state = initial.apply(initial.tr.setSelection(TextSelection.create(doc, cursor)));

    expect(editorStateSelectionFormatting(state).table).toMatchObject({
      rows: 1,
      columns: 1,
      canDeleteRow: false,
      canDeleteColumn: false
    });
    expect(editTableStructureTransaction(state, 'delete_row')).toBeUndefined();
    expect(editTableStructureTransaction(state, 'delete_column')).toBeUndefined();
  });

  it('refuses structure edits for irregular or source-empty tables', () => {
    const irregular = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'table',
          content: [
            {
              type: 'table_row',
              content: [
                {
                  type: 'table_cell',
                  attrs: { unsupported: false, sourceEmpty: false },
                  content: [
                    { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'A1' }] }
                  ]
                },
                {
                  type: 'table_cell',
                  attrs: { unsupported: false, sourceEmpty: false },
                  content: [
                    { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'A2' }] }
                  ]
                }
              ]
            },
            {
              type: 'table_row',
              content: [
                {
                  type: 'table_cell',
                  attrs: { unsupported: false, sourceEmpty: false },
                  content: [
                    { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'B1' }] }
                  ]
                }
              ]
            }
          ]
        }
      ]
    });
    const irregularCursor = textRange(irregular, 'A1').from;
    const irregularInitial = EditorState.create({ doc: irregular });
    const irregularState = irregularInitial.apply(
      irregularInitial.tr.setSelection(TextSelection.create(irregular, irregularCursor))
    );

    expect(editorStateSelectionFormatting(irregularState).table).toBeNull();
    expect(editTableStructureTransaction(irregularState, 'add_column_right')).toBeUndefined();

    const sourceEmpty = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'table',
          content: [
            {
              type: 'table_row',
              content: [
                {
                  type: 'table_cell',
                  attrs: { unsupported: false, sourceEmpty: true },
                  content: [{ type: 'paragraph', attrs: { style: 'body' } }]
                }
              ]
            }
          ]
        }
      ]
    });
    const sourceEmptyInitial = EditorState.create({ doc: sourceEmpty });
    const sourceEmptyState = sourceEmptyInitial.apply(sourceEmptyInitial.tr.setSelection(TextSelection.create(sourceEmpty, 3)));

    expect(editorStateSelectionFormatting(sourceEmptyState).table).toBeNull();
    expect(editTableStructureTransaction(sourceEmptyState, 'add_row_below')).toBeUndefined();
  });

  it('keeps table structure commands harmless outside editable tables', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Outside' }] }]
    });
    const state = EditorState.create({ doc });

    expect(editorStateSelectionFormatting(state).table).toBeNull();
    expect(editTableStructureTransaction(state, 'add_row_below')).toBeUndefined();
  });

  it('lets the native paste path handle multiline text inside non-empty paragraphs', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Existing text' }]
        }
      ]
    });
    const cursor = textEnd(doc, 'Existing text');
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, cursor))
    );

    const transaction = pastePlainTextAsBlocksTransaction(state, 'One\nTwo');

    expect(transaction).toBeUndefined();
  });

  it('lets the native paste path handle partial multiline replacements inside paragraphs', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Hello world!' }]
        }
      ]
    });
    const range = textRange(doc, 'Hello world!');
    const state = EditorState.create({ doc }).apply(
      EditorState.create({ doc }).tr.setSelection(TextSelection.create(doc, range.from + 6, range.to - 1))
    );

    const transaction = pastePlainTextAsBlocksTransaction(state, 'One\nTwo');

    expect(transaction).toBeUndefined();
  });

  it('uses the same newline-separated text representation for spell input and mapping', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Hello' }] },
        { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'qwerty' }] }
      ]
    });

    expect(editorDocPlainText(doc)).toBe('Hello\nqwerty');
  });

  it('selects a top-level block by navigator index', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        { type: 'heading', attrs: { level: 1 }, content: [{ type: 'text', text: 'One' }] },
        { type: 'heading', attrs: { level: 2 }, content: [{ type: 'text', text: 'Two' }] }
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

    expect(selectEditorTopLevelBlock(view as unknown as Parameters<typeof selectEditorTopLevelBlock>[0], 1)).toBe(true);
    expect(view.state.selection.from).toBe(doc.child(0).nodeSize + 1);
  });

  it('maps byte-based spell issues to editor decoration ranges', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Hello' }] },
        { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'qwerty' }] }
      ]
    });

    const ranges = mapSpellIssuesToEditorRanges(
      doc,
      [{ word: 'qwerty', byte_start: 6, byte_end: 12, suggestions: ['query'] }],
      'Hello\nqwerty'
    );

    expect(ranges).toHaveLength(1);
    expect(ranges[0].word).toBe('qwerty');
    expect(ranges[0].to).toBeGreaterThan(ranges[0].from);
  });
});

function textRange(doc: ReturnType<typeof supportedSchema.nodeFromJSON>, text: string) {
  let range: { from: number; to: number } | undefined;
  doc.descendants((node, pos) => {
    if (range || !node.isTextblock || node.textContent !== text) {
      return !range;
    }
    range = { from: pos + 1, to: pos + 1 + text.length };
    return false;
  });
  if (!range) {
    throw new Error(`Text not found: ${text}`);
  }
  return range;
}

function textEnd(doc: ReturnType<typeof supportedSchema.nodeFromJSON>, text: string) {
  return textRange(doc, text).to;
}

function firstTextblockStart(doc: ReturnType<typeof supportedSchema.nodeFromJSON>) {
  let found: number | undefined;
  doc.descendants((node, pos) => {
    if (found || !node.isTextblock) {
      return !found;
    }
    found = pos + 1;
    return false;
  });
  if (!found) {
    throw new Error('Textblock not found');
  }
  return found;
}

function tableStateWithSelection(cellText: string) {
  const doc = supportedSchema.nodeFromJSON({
    type: 'doc',
    content: [
      {
        type: 'table',
        content: [
          {
            type: 'table_row',
            content: [
              {
                type: 'table_cell',
                attrs: { unsupported: false, sourceEmpty: false },
                content: [
                  { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'A1' }] }
                ]
              },
              {
                type: 'table_cell',
                attrs: { unsupported: false, sourceEmpty: false },
                content: [
                  { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'A2' }] }
                ]
              }
            ]
          },
          {
            type: 'table_row',
            content: [
              {
                type: 'table_cell',
                attrs: { unsupported: false, sourceEmpty: false },
                content: [
                  { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'B1' }] }
                ]
              },
              {
                type: 'table_cell',
                attrs: { unsupported: false, sourceEmpty: false },
                content: [
                  { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'B2' }] }
                ]
              }
            ]
          }
        ]
      }
    ]
  });
  const cursor = textRange(doc, cellText).from;
  const initial = EditorState.create({ doc });
  return initial.apply(initial.tr.setSelection(TextSelection.create(doc, cursor)));
}

function selectionAncestorNames(state: EditorState) {
  const names: string[] = [];
  for (let depth = 0; depth <= state.selection.$from.depth; depth += 1) {
    names.push(state.selection.$from.node(depth).type.name);
  }
  return names;
}
