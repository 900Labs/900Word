import { describe, expect, it } from 'vitest';
import {
  buildEditorSyncCommands,
  documentProjectionWarnings,
  documentToEditorDoc,
  documentToText,
  editorDocToWordCoreBlocks,
  type DocumentState
} from './documentProjection';

describe('documentToText', () => {
  it('projects supported blocks to plain text', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            { type: 'Heading', value: { level: 1, inlines: [{ text: 'Title' }] } },
            { type: 'Paragraph', value: { inlines: [{ text: 'Body' }] } }
          ]
        }
      ]
    };

    expect(documentToText(document)).toBe('Title\nBody');
  });

  it('projects supported word-core blocks to ProseMirror JSON', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            { type: 'Heading', value: { level: 2, inlines: [{ text: 'Title', marks: ['Bold'] }] } },
            {
              type: 'Paragraph',
              value: { style: 'caption', inlines: [{ text: 'Body', marks: ['Italic'], link: 'https://example.invalid' }] }
            },
            { type: 'PageBreak' }
          ]
        }
      ]
    };

    expect(documentToEditorDoc(document)).toEqual({
      type: 'doc',
      content: [
        {
          type: 'heading',
          attrs: { level: 2 },
          content: [{ type: 'text', text: 'Title', marks: [{ type: 'bold' }] }]
        },
        {
          type: 'paragraph',
          attrs: { style: 'caption' },
          content: [
            {
              type: 'text',
              text: 'Body',
              marks: [
                { type: 'italic' },
                { type: 'link', attrs: { href: 'https://example.invalid' } }
              ]
            }
          ]
        },
        {
          type: 'paragraph',
          content: [{ type: 'text', text: '[PageBreak block preserved read-only]' }]
        }
      ]
    });
    expect(documentProjectionWarnings(document)).toEqual([
      'PageBreak blocks are preserved but read-only in the Sprint 002 editor projection.'
    ]);
  });

  it('drops unsafe link schemes from editor projection', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Paragraph',
              value: { inlines: [{ text: 'Unsafe link', link: 'javascript:alert(1)' }] }
            }
          ]
        }
      ]
    };

    expect(documentToEditorDoc(document).content[0].content).toEqual([
      { type: 'text', text: 'Unsafe link' }
    ]);
  });

  it('projects ProseMirror JSON back to word-core editable blocks', () => {
    expect(
      editorDocToWordCoreBlocks({
        type: 'doc',
        content: [
          {
            type: 'heading',
            attrs: { level: 3 },
            content: [{ type: 'text', text: 'Heading', marks: [{ type: 'underline' }] }]
          },
          {
            type: 'paragraph',
            attrs: { style: 'caption' },
            content: [{ type: 'text', text: 'Body' }]
          }
        ]
      })
    ).toEqual([
      {
        type: 'Heading',
        value: {
          level: 3,
          inlines: [{ text: 'Heading', marks: ['Underline'], link: null }]
        }
      },
      {
        type: 'Paragraph',
        value: {
          style: 'caption',
          inlines: [{ text: 'Body', marks: [], link: null }]
        }
      }
    ]);
  });

  it('builds document commands for editable projection changes', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [{ type: 'Paragraph', value: { inlines: [{ text: 'Old' }] } }]
        }
      ]
    };

    expect(
      buildEditorSyncCommands(document, [
        { type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'New', marks: [], link: null }] } },
        { type: 'Heading', value: { level: 1, inlines: [{ text: 'Next', marks: [], link: null }] } }
      ])
    ).toEqual([
      {
        type: 'replace_block',
        section_index: 0,
        block_index: 0,
        block: { type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'New', marks: [], link: null }] } }
      },
      {
        type: 'insert_block',
        section_index: 0,
        block_index: 1,
        block: { type: 'Heading', value: { level: 1, inlines: [{ text: 'Next', marks: [], link: null }] } }
      }
    ]);
  });

  it('blocks sync commands when unprojected blocks are present', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [{ type: 'Table', value: { rows: [] } }]
        }
      ]
    };

    expect(buildEditorSyncCommands(document, [])).toEqual([]);
  });
});
