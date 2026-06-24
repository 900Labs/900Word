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
      'PageBreak blocks are preserved but read-only in the editor projection.'
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

  it('projects paragraph direct formatting and inline style to word-core JSON', () => {
    expect(
      editorDocToWordCoreBlocks({
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            attrs: {
              style: 'quote',
              align: 'justify',
              lineSpacing: 1500,
              spacingBefore: 2,
              spacingAfter: 5,
              indentStart: 10,
              firstLineIndent: 4
            },
            content: [
              {
                type: 'text',
                text: 'Formatted',
                marks: [
                  {
                    type: 'textStyle',
                    attrs: {
                      fontFamily: 'serif',
                      fontSizePt: 14,
                      textColor: '#1f2937',
                      highlightColor: '#fff3bf'
                    }
                  }
                ]
              }
            ]
          }
        ]
      })
    ).toEqual([
      {
        type: 'Paragraph',
        value: {
          style: 'quote',
          format: {
            alignment: 'justify',
            line_spacing_per_mille: 1500,
            spacing_before_mm: 2,
            spacing_after_mm: 5,
            indent_start_mm: 10,
            first_line_indent_mm: 4
          },
          inlines: [
            {
              text: 'Formatted',
              marks: [],
              link: null,
              style: {
                font_family: 'serif',
                font_size_pt: 14,
                text_color: '#1f2937',
                highlight_color: '#fff3bf'
              }
            }
          ]
        }
      }
    ]);
  });

  it('applies paragraph style properties to editor paragraphs while direct formatting overrides them', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      styles: {
        quote: {
          id: 'quote',
          name: 'Quote',
          kind: 'Paragraph',
          parent: null,
          properties: {
            paragraph: {
              alignment: 'center',
              line_spacing_per_mille: 1500,
              spacing_before_mm: 0,
              spacing_after_mm: 4,
              first_line_indent_mm: -3
            }
          }
        }
      },
      sections: [
        {
          blocks: [
            {
              type: 'Paragraph',
              value: {
                style: 'quote',
                format: { alignment: 'right' },
                inlines: [{ text: 'Styled paragraph' }]
              }
            }
          ]
        }
      ]
    };

    expect(documentToEditorDoc(document).content[0]).toEqual({
      type: 'paragraph',
      attrs: {
        style: 'quote',
        align: 'right',
        lineSpacing: 1500,
        spacingBefore: 0,
        spacingAfter: 4,
        firstLineIndent: -3
      },
      content: [{ type: 'text', text: 'Styled paragraph' }]
    });
    expect(editorDocToWordCoreBlocks(documentToEditorDoc(document), document.styles)).toEqual([
      {
        type: 'Paragraph',
        value: {
          style: 'quote',
          format: { alignment: 'right' },
          inlines: [{ text: 'Styled paragraph', marks: [], link: null }]
        }
      }
    ]);
  });

  it('does not write inherited paragraph style properties back as direct formatting', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      styles: {
        quote: {
          id: 'quote',
          name: 'Quote',
          kind: 'Paragraph',
          parent: null,
          properties: {
            paragraph: {
              alignment: 'justify',
              line_spacing_per_mille: 1500,
              spacing_after_mm: 4
            }
          }
        }
      },
      sections: [
        {
          blocks: [
            {
              type: 'Paragraph',
              value: {
                style: 'quote',
                inlines: [{ text: 'Styled paragraph' }]
              }
            }
          ]
        }
      ]
    };
    const editorDoc = documentToEditorDoc(document);

    expect(editorDoc.content[0]).toMatchObject({
      attrs: {
        style: 'quote',
        align: 'justify',
        lineSpacing: 1500,
        spacingAfter: 4
      }
    });
    expect(editorDocToWordCoreBlocks(editorDoc, document.styles)).toEqual([
      {
        type: 'Paragraph',
        value: {
          style: 'quote',
          inlines: [{ text: 'Styled paragraph', marks: [], link: null }]
        }
      }
    ]);
  });

  it('projects word-core lists to editable list nodes and back', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'List',
              value: {
                definition_id: '900w-ordered',
                items: [
                  {
                    level: 1,
                    blocks: [{ type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'One', marks: [], link: null }] } }]
                  },
                  {
                    level: 2,
                    blocks: [{ type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'Two', marks: [], link: null }] } }]
                  }
                ]
              }
            }
          ]
        }
      ]
    };

    const editorDoc = documentToEditorDoc(document);
    expect(editorDoc.content[0].type).toBe('ordered_list');
    expect(editorDocToWordCoreBlocks(editorDoc)).toEqual(document.sections[0].blocks);
    expect(documentProjectionWarnings(document)).toEqual([]);
  });

  it('projects default unordered lists as bullet lists, not ordered lists', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      lists: {
        '900w-unordered': { ordered: false }
      },
      sections: [
        {
          blocks: [
            {
              type: 'List',
              value: {
                definition_id: '900w-unordered',
                items: [
                  {
                    level: 1,
                    blocks: [{ type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'Bullet', marks: [], link: null }] } }]
                  }
                ]
              }
            }
          ]
        }
      ]
    };

    expect(documentToEditorDoc(document).content[0].type).toBe('bullet_list');
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

  it('does not build commands for unchanged projected blocks', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [{ type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'Same', marks: [], link: null }] } }]
        }
      ]
    };

    expect(
      buildEditorSyncCommands(document, [
        { type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'Same', marks: [], link: null }] } }
      ])
    ).toEqual([]);
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
