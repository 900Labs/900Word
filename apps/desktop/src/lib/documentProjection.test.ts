import { describe, expect, it } from 'vitest';
import {
  buildEditorSyncCommands,
  canEditProjectedDocument,
  documentLinkTargets,
  documentLinkTargetsFromEditableBlocks,
  documentOutline,
  documentOutlineFromEditableBlocks,
  documentProjectionWarnings,
  documentToEditorDoc,
  documentToText,
  editorDocToWordCoreBlocks,
  pageFieldTokens,
  pageRegionTextToBlocks,
  pageRegionToText,
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

  it('converts page region tokens to typed field inlines and back', () => {
    const blocks = pageRegionTextToBlocks(`Page ${pageFieldTokens.page_number} of ${pageFieldTokens.page_count}\n${pageFieldTokens.date}`);

    expect(blocks).toEqual([
      {
        type: 'Paragraph',
        value: {
          inlines: [
            { text: 'Page ', marks: [], link: null },
            { text: '1', marks: [], link: null, field: 'page_number' },
            { text: ' of ', marks: [], link: null },
            { text: '1', marks: [], link: null, field: 'page_count' }
          ]
        }
      },
      {
        type: 'Paragraph',
        value: {
          inlines: [{ text: '1970-01-01', marks: [], link: null, field: 'date' }]
        }
      }
    ]);
    expect(pageRegionToText({ blocks })).toBe(`Page ${pageFieldTokens.page_number} of ${pageFieldTokens.page_count}\n${pageFieldTokens.date}`);
  });

  it('warns when imported page regions are read-only', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          page_regions: {
            header: { read_only: true, blocks: [{ type: 'Paragraph', value: { inlines: [{ text: 'Header' }] } }] }
          },
          blocks: [{ type: 'Paragraph', value: { inlines: [{ text: 'Body' }] } }]
        }
      ]
    };

    expect(documentProjectionWarnings(document)).toContain(
      'Header/footer content with unsupported ODT structure is preserved read-only.'
    );
  });

  it('warns instead of flattening semantic page fields in body content', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Paragraph',
              value: {
                inlines: [{ text: '1', field: 'page_number' }]
              }
            }
          ]
        }
      ]
    };

    expect(documentProjectionWarnings(document)).toContain(
      'Page fields in the document body are preserved but read-only in the editor projection.'
    );
    expect(buildEditorSyncCommands(document, [])).toEqual([]);
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

  it('projects image blocks as atom nodes without making the whole document read-only', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      assets: {
        'image-1.png': {
          id: 'image-1.png',
          media_type: 'image/png',
          byte_len: 8,
          bytes: [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a],
          original_name: null
        }
      },
      sections: [
        {
          blocks: [
            { type: 'Paragraph', value: { inlines: [{ text: 'Before' }] } },
            {
              type: 'Image',
              value: {
                asset_id: 'image-1.png',
                presentation: {
                  alignment: 'center',
                  scale_percent: 80,
                  caption: 'Diagram caption'
                },
                alt_text: 'Diagram alt'
              }
            },
            { type: 'Paragraph', value: { inlines: [{ text: 'After' }] } }
          ]
        }
      ]
    };

    expect(documentProjectionWarnings(document)).toEqual([]);
    expect(canEditProjectedDocument(document)).toBe(true);
    expect(documentToEditorDoc(document).content[1]).toEqual({
      type: 'image',
      attrs: {
        assetId: 'image-1.png',
        altText: 'Diagram alt',
        alignment: 'center',
        scalePercent: 80,
        caption: 'Diagram caption',
        src: 'data:image/png;base64,iVBORw0KGgo='
      }
    });
  });

  it('round-trips image atom nodes back to word-core image blocks', () => {
    expect(
      editorDocToWordCoreBlocks({
        type: 'doc',
        content: [
          {
            type: 'image',
            attrs: {
              assetId: 'image-1.png',
              altText: 'Image',
              alignment: 'right',
              scalePercent: 125,
              caption: 'Visible caption',
              src: 'data:image/png;base64,iVBORw0KGgo='
            }
          }
        ]
      })
    ).toEqual([
      {
        type: 'Image',
        value: {
          asset_id: 'image-1.png',
          presentation: {
            alignment: 'right',
            scale_percent: 125,
            caption: 'Visible caption'
          },
          alt_text: 'Image'
        }
      }
    ]);
  });

  it('builds a navigator outline from non-empty Heading 1-3 blocks', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            { type: 'Heading', value: { level: 1, inlines: [{ text: 'Overview' }] } },
            { type: 'Paragraph', value: { inlines: [{ text: 'Body' }] } },
            { type: 'Heading', value: { level: 3, inlines: [{ text: 'Details' }] } },
            { type: 'Heading', value: { level: 4, inlines: [{ text: 'Too deep' }] } },
            { type: 'Heading', value: { level: 2, inlines: [{ text: '   ' }] } }
          ]
        }
      ]
    };

    expect(documentOutline(document)).toEqual([
      { sectionIndex: 0, blockIndex: 0, editorBlockIndex: 0, level: 1, text: 'Overview' },
      { sectionIndex: 0, blockIndex: 2, editorBlockIndex: 2, level: 3, text: 'Details' }
    ]);
  });

  it('builds a live navigator outline from editable projection blocks', () => {
    expect(
      documentOutlineFromEditableBlocks([
        { type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'Intro' }] } },
        { type: 'Heading', value: { level: 2, inlines: [{ text: 'Live heading', marks: [], link: null }] } }
      ])
    ).toEqual([{ sectionIndex: 0, blockIndex: 1, editorBlockIndex: 1, level: 2, text: 'Live heading' }]);
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

    const firstBlock = documentToEditorDoc(document).content[0];
    expect(firstBlock.type).toBe('paragraph');
    expect(firstBlock.type === 'paragraph' ? firstBlock.content : undefined).toEqual([
      { type: 'text', text: 'Unsafe link' }
    ]);
  });

  it('round-trips safe bookmark ids and internal fragment links through editor projection', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Heading',
              value: {
                bookmark_id: 'bm-heading',
                level: 2,
                inlines: [{ text: 'Target' }]
              }
            },
            {
              type: 'Paragraph',
              value: {
                bookmark_id: 'bm-body',
                inlines: [{ text: 'Jump', link: '#bm-heading' }]
              }
            }
          ]
        }
      ]
    };

    const editorDoc = documentToEditorDoc(document);
    expect(editorDoc.content[0]).toMatchObject({ type: 'heading', attrs: { bookmarkId: 'bm-heading' } });
    expect(editorDoc.content[1]).toMatchObject({
      type: 'paragraph',
      attrs: { bookmarkId: 'bm-body' },
      content: [{ marks: [{ type: 'link', attrs: { href: '#bm-heading' } }] }]
    });
    expect(editorDocToWordCoreBlocks(editorDoc)).toEqual([
      {
        type: 'Heading',
        value: {
          bookmark_id: 'bm-heading',
          level: 2,
          inlines: [{ text: 'Target', marks: [], link: null }]
        }
      },
      {
        type: 'Paragraph',
        value: {
          bookmark_id: 'bm-body',
          style: 'body',
          inlines: [{ text: 'Jump', marks: [], link: '#bm-heading' }]
        }
      }
    ]);
    expect(documentLinkTargets(document)).toEqual([
      {
        id: 'bm-heading',
        label: 'Target',
        kind: 'heading',
        sectionIndex: 0,
        blockIndex: 0,
        editorBlockIndex: 0,
        level: 2
      },
      {
        id: 'bm-body',
        label: 'Jump',
        kind: 'bookmark',
        sectionIndex: 0,
        blockIndex: 1,
        editorBlockIndex: 1
      }
    ]);
  });

  it('does not expose unsafe imported bookmark ids as link targets', () => {
    expect(
      documentLinkTargetsFromEditableBlocks([
        { type: 'Paragraph', value: { bookmark_id: '../bad', style: 'body', inlines: [{ text: 'Bad' }] } },
        { type: 'Paragraph', value: { bookmark_id: 'bm-good', style: 'body', inlines: [{ text: 'Good' }] } }
      ])
    ).toEqual([
      {
        id: 'bm-good',
        label: 'Good',
        kind: 'bookmark',
        sectionIndex: 0,
        blockIndex: 1,
        editorBlockIndex: 1
      }
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

  it('keeps structurally empty lists read-only', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [{ blocks: [{ type: 'List', value: { definition_id: '900w-unordered', items: [] } }] }]
    };

    expect(documentProjectionWarnings(document)).toEqual([
      'List blocks are preserved but read-only in the editor projection.'
    ]);
    expect(documentToEditorDoc(document)).toEqual({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          content: [{ type: 'text', text: '[List block preserved read-only]' }]
        }
      ]
    });
    expect(canEditProjectedDocument(document)).toBe(false);
    expect(buildEditorSyncCommands(document, [])).toEqual([]);
  });

  it('projects word-core tables to editable table nodes and back', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Table',
              value: {
                rows: [
                  {
                    cells: [
                      {
                        blocks: [
                          { type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'A1', marks: [], link: null }] } }
                        ]
                      },
                      {
                        blocks: [
                          { type: 'Heading', value: { level: 2, inlines: [{ text: 'B1', marks: [], link: null }] } }
                        ]
                      }
                    ]
                  },
                  {
                    cells: [
                      {
                        blocks: [
                          {
                            type: 'List',
                            value: {
                              definition_id: '900w-unordered',
                              items: [
                                {
                                  level: 1,
                                  blocks: [
                                    {
                                      type: 'Paragraph',
                                      value: { style: 'body', inlines: [{ text: 'A2', marks: [], link: null }] }
                                    }
                                  ]
                                }
                              ]
                            }
                          }
                        ]
                      },
                      { blocks: [{ type: 'Paragraph', value: { style: 'body', inlines: [] } }] }
                    ]
                  }
                ]
              }
            }
          ]
        }
      ]
    };

    const editorDoc = documentToEditorDoc(document);

    expect(editorDoc).toEqual({
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
                    {
                      type: 'paragraph',
                      attrs: { style: 'body' },
                      content: [{ type: 'text', text: 'A1' }]
                    }
                  ]
                },
                {
                  type: 'table_cell',
                  attrs: { unsupported: false, sourceEmpty: false },
                  content: [
                    {
                      type: 'heading',
                      attrs: { level: 2 },
                      content: [{ type: 'text', text: 'B1' }]
                    }
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
                    {
                      type: 'bullet_list',
                      attrs: { definitionId: '900w-unordered' },
                      content: [
                        {
                          type: 'list_item',
                          attrs: { level: 1 },
                          content: [
                            {
                              type: 'paragraph',
                              attrs: { style: 'body' },
                              content: [{ type: 'text', text: 'A2' }]
                            }
                          ]
                        }
                      ]
                    }
                  ]
                },
                {
                  type: 'table_cell',
                  attrs: { unsupported: false, sourceEmpty: false },
                  content: [{ type: 'paragraph', attrs: { style: 'body' }, content: [] }]
                }
              ]
            }
          ]
        }
      ]
    });
    expect(editorDocToWordCoreBlocks(editorDoc)).toEqual(document.sections[0].blocks);
    expect(documentProjectionWarnings(document)).toEqual([]);
    expect(canEditProjectedDocument(document)).toBe(true);
  });

  it('syncs edited table cell text back to word-core table blocks', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Table',
              value: {
                rows: [
                  {
                    cells: [
                      {
                        blocks: [
                          { type: 'Paragraph', value: { style: 'body', inlines: [{ text: 'Old', marks: [], link: null }] } }
                        ]
                      }
                    ]
                  }
                ]
              }
            }
          ]
        }
      ]
    };
    const editorDoc = documentToEditorDoc(document);
    const table = editorDoc.content[0];
    if (table.type !== 'table') {
      throw new Error('Expected table projection');
    }
    const paragraph = table.content[0].content[0].content[0];
    if (paragraph.type !== 'paragraph') {
      throw new Error('Expected paragraph cell content');
    }
    paragraph.content = [{ type: 'text', text: 'New' }];
    const nextBlocks = editorDocToWordCoreBlocks(editorDoc);

    expect(nextBlocks).toEqual([
      {
        type: 'Table',
        value: {
          rows: [
            {
              cells: [
                {
                  blocks: [
                    {
                      type: 'Paragraph',
                      value: { style: 'body', inlines: [{ text: 'New', marks: [], link: null }] }
                    }
                  ]
                }
              ]
            }
          ]
        }
      }
    ]);
    expect(buildEditorSyncCommands(document, nextBlocks)).toEqual([
      {
        type: 'replace_block',
        section_index: 0,
        block_index: 0,
        block: nextBlocks[0]
      }
    ]);
  });

  it('round-trips formatting, links, and text style inside table cells', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Table',
              value: {
                rows: [
                  {
                    cells: [
                      {
                        blocks: [
                          {
                            type: 'Paragraph',
                            value: {
                              style: 'quote',
                              format: { alignment: 'center', spacing_after_mm: 4 },
                              inlines: [
                                {
                                  text: 'Linked bold',
                                  marks: ['Bold'],
                                  link: 'https://example.invalid',
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
                        ]
                      }
                    ]
                  }
                ]
              }
            }
          ]
        }
      ]
    };

    const editorDoc = documentToEditorDoc(document);
    expect(editorDocToWordCoreBlocks(editorDoc)).toEqual(document.sections[0].blocks);
  });

  it('preserves untouched source-empty table cells during sync command building', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Table',
              value: {
                rows: [
                  {
                    cells: [{ blocks: [] }]
                  }
                ]
              }
            }
          ]
        }
      ]
    };

    const editorDoc = documentToEditorDoc(document);
    const table = editorDoc.content[0];
    if (table.type !== 'table') {
      throw new Error('Expected table projection');
    }
    expect(table.content[0].content[0].attrs?.sourceEmpty).toBe(true);

    const nextBlocks = editorDocToWordCoreBlocks(editorDoc);

    expect(nextBlocks).toEqual(document.sections[0].blocks);
    expect(buildEditorSyncCommands(document, nextBlocks)).toEqual([]);
  });

  it('keeps newly inserted empty table paragraphs as cell content', () => {
    expect(
      editorDocToWordCoreBlocks({
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
                    attrs: { unsupported: false },
                    content: [{ type: 'paragraph', attrs: { style: 'body' }, content: [] }]
                  }
                ]
              }
            ]
          }
        ]
      })
    ).toEqual([
      {
        type: 'Table',
        value: {
          rows: [
            {
              cells: [
                {
                  blocks: [{ type: 'Paragraph', value: { style: 'body', inlines: [] } }]
                }
              ]
            }
          ]
        }
      }
    ]);
  });

  it('marks unsupported table-cell content read-only instead of enabling sync', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Table',
              value: {
                rows: [
                  {
                    cells: [
                      {
                        blocks: [{ type: 'Image', value: { asset_id: 'asset-1', alt_text: 'Diagram' } }]
                      }
                    ]
                  }
                ]
              }
            }
          ]
        }
      ]
    };

    expect(documentToEditorDoc(document).content[0]).toEqual({
      type: 'table',
      content: [
        {
          type: 'table_row',
          content: [
            {
              type: 'table_cell',
              attrs: { unsupported: true },
              content: [
                {
                  type: 'paragraph',
                  attrs: { style: 'body' },
                  content: [{ type: 'text', text: '[Unsupported table cell content preserved read-only]' }]
                }
              ]
            }
          ]
        }
      ]
    });
    expect(documentProjectionWarnings(document)).toEqual([
      'Tables with unsupported or structurally empty content are preserved but read-only in the editor projection.'
    ]);
    expect(canEditProjectedDocument(document)).toBe(false);
    expect(buildEditorSyncCommands(document, [])).toEqual([]);
  });

  it('keeps structurally empty table-cell lists read-only', () => {
    const document: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [
        {
          blocks: [
            {
              type: 'Table',
              value: {
                rows: [
                  {
                    cells: [
                      {
                        blocks: [{ type: 'List', value: { definition_id: '900w-unordered', items: [] } }]
                      }
                    ]
                  }
                ]
              }
            }
          ]
        }
      ]
    };

    expect(documentProjectionWarnings(document)).toEqual([
      'Tables with unsupported or structurally empty content are preserved but read-only in the editor projection.'
    ]);
    expect(canEditProjectedDocument(document)).toBe(false);
    expect(documentToEditorDoc(document).content[0]).toEqual({
      type: 'table',
      content: [
        {
          type: 'table_row',
          content: [
            {
              type: 'table_cell',
              attrs: { unsupported: true },
              content: [
                {
                  type: 'paragraph',
                  attrs: { style: 'body' },
                  content: [{ type: 'text', text: '[Unsupported table cell content preserved read-only]' }]
                }
              ]
            }
          ]
        }
      ]
    });
  });

  it('keeps structurally empty source tables read-only', () => {
    const emptyTable: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [{ blocks: [{ type: 'Table', value: { rows: [] } }] }]
    };
    const emptyRow: DocumentState = {
      meta: { title: 'Generated test' },
      sections: [{ blocks: [{ type: 'Table', value: { rows: [{ cells: [] }] } }] }]
    };

    expect(documentProjectionWarnings(emptyTable)).toEqual([
      'Tables with unsupported or structurally empty content are preserved but read-only in the editor projection.'
    ]);
    expect(documentProjectionWarnings(emptyRow)).toEqual([
      'Tables with unsupported or structurally empty content are preserved but read-only in the editor projection.'
    ]);
    expect(canEditProjectedDocument(emptyTable)).toBe(false);
    expect(canEditProjectedDocument(emptyRow)).toBe(false);
    expect(buildEditorSyncCommands(emptyTable, editorDocToWordCoreBlocks(documentToEditorDoc(emptyTable)))).toEqual([]);
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
          blocks: [{ type: 'PageBreak' }]
        }
      ]
    };

    expect(buildEditorSyncCommands(document, [])).toEqual([]);
  });
});
