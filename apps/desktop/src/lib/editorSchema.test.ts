import { describe, expect, it } from 'vitest';
import { sanitizeEditorHref } from './editorSecurity';
import {
  supportedBlockTypes,
  supportedImageNodeTypes,
  supportedInlineNodeTypes,
  supportedListNodeTypes,
  supportedMarkTypes,
  supportedSchema,
  supportedTableNodeTypes
} from './editorSchema';

describe('supportedSchema', () => {
  it('contains only word-core projected block nodes plus doc and text', () => {
    expect(Object.keys(supportedSchema.nodes).sort()).toEqual([
      'bullet_list',
      'doc',
      'heading',
      'image',
      'list_item',
      'note_reference',
      'ordered_list',
      'paragraph',
      'table',
      'table_cell',
      'table_of_contents',
      'table_row',
      'text'
    ]);
    expect(supportedBlockTypes).toEqual(['paragraph', 'heading']);
    expect(supportedListNodeTypes).toEqual(['bullet_list', 'ordered_list', 'list_item']);
    expect(supportedTableNodeTypes).toEqual(['table', 'table_row', 'table_cell']);
    expect(supportedImageNodeTypes).toEqual(['image']);
    expect(supportedInlineNodeTypes).toEqual(['note_reference']);
  });

  it('contains only word-core inline mark projections', () => {
    expect(Object.keys(supportedSchema.marks).sort()).toEqual([
      'bold',
      'comment',
      'italic',
      'link',
      'strikethrough',
      'subscript',
      'superscript',
      'textStyle',
      'trackedChange',
      'underline'
    ]);
    expect(supportedMarkTypes).toEqual([
      'bold',
      'italic',
      'underline',
      'strikethrough',
      'superscript',
      'subscript',
      'textStyle',
      'link',
      'comment',
      'trackedChange'
    ]);
  });

  it('accepts only bounded local comment marks', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [{ type: 'text', text: 'Commented', marks: [{ type: 'comment', attrs: { id: 'cmt-abc123' } }] }]
        }
      ]
    });

    expect(doc.firstChild?.firstChild?.marks[0].type.name).toBe('comment');

    const overlapping = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            {
              type: 'text',
              text: 'Overlap',
              marks: [
                { type: 'comment', attrs: { id: 'cmt-first' } },
                { type: 'comment', attrs: { id: 'cmt-second' } }
              ]
            }
          ]
        }
      ]
    });
    expect(overlapping.firstChild?.firstChild?.marks.map((mark) => mark.attrs.id)).toEqual([
      'cmt-first',
      'cmt-second'
    ]);

    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            attrs: { style: 'body' },
            content: [{ type: 'text', text: 'Unsafe', marks: [{ type: 'comment', attrs: { id: '../bad' } }] }]
          }
        ]
      })
    ).toThrow('unsupported comment id');
  });

  it('accepts only bounded local note reference atoms', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'paragraph',
          attrs: { style: 'body' },
          content: [
            { type: 'text', text: 'Claim' },
            { type: 'note_reference', attrs: { id: 'note-source', kind: 'footnote', label: '1' } }
          ]
        }
      ]
    });

    expect(doc.firstChild?.child(1).type.name).toBe('note_reference');
    expect(doc.textBetween(0, doc.content.size, '\n')).toBe('Claim1');

    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            attrs: { style: 'body' },
            content: [{ type: 'note_reference', attrs: { id: '../bad', kind: 'footnote', label: '1' } }]
          }
        ]
      })
    ).toThrow('unsupported note id');
  });

  it('preserves list nodes and direct text style attrs', () => {
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
                  attrs: { style: 'body', align: 'center', lineSpacing: 1500 },
                  content: [
                    {
                      type: 'text',
                      text: 'Styled item',
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
            }
          ]
        }
      ]
    });

    expect(doc.firstChild?.type.name).toBe('bullet_list');
    expect(doc.firstChild?.firstChild?.attrs.level).toBe(2);
  });

  it('accepts only embedded data URLs for image nodes', () => {
    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [
          {
            type: 'image',
            attrs: {
              assetId: 'image-1.png',
              altText: 'Image',
              src: 'https://example.invalid/image.png'
            }
          }
        ]
      })
    ).toThrow('unsupported image source');

    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'image',
          attrs: {
            assetId: 'image-1.png',
            altText: 'Image',
            alignment: 'center',
            scalePercent: 90,
            caption: 'Caption',
            src: 'data:image/png;base64,iVBORw0KGgo='
          }
        }
      ]
    });
    expect(doc.child(0).type.name).toBe('image');
    expect(doc.child(0).attrs.alignment).toBe('center');
    expect(doc.child(0).attrs.scalePercent).toBe(90);
    expect(doc.child(0).attrs.caption).toBe('Caption');

    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [
          {
            type: 'image',
            attrs: {
              assetId: 'image-1.png',
              alignment: 'float',
              scalePercent: 90
            }
          }
        ]
      })
    ).toThrow('unsupported image alignment');

    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [
          {
            type: 'image',
            attrs: {
              assetId: 'image-1.png',
              alignment: 'inline',
              scalePercent: 300
            }
          }
        ]
      })
    ).toThrow('unsupported image scale');
  });

  it('preserves table nodes with editable paragraph cell content', () => {
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
                    {
                      type: 'paragraph',
                      attrs: { style: 'body' },
                      content: [{ type: 'text', text: 'Cell' }]
                    }
                  ]
                }
              ]
            }
          ]
        }
      ]
    });

    expect(doc.firstChild?.type.name).toBe('table');
    expect(doc.firstChild?.firstChild?.firstChild?.attrs.unsupported).toBe(false);
    expect(doc.firstChild?.firstChild?.firstChild?.attrs.sourceEmpty).toBe(false);
  });

  it('rejects table nodes nested inside list items', () => {
    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [
          {
            type: 'bullet_list',
            content: [
              {
                type: 'list_item',
                content: [
                  { type: 'paragraph', attrs: { style: 'body' }, content: [{ type: 'text', text: 'Item' }] },
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
              }
            ]
          }
        ]
      }).check()
    ).toThrow();
  });

  it('rejects unsupported ProseMirror nodes', () => {
    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [{ type: 'blockquote', content: [{ type: 'paragraph' }] }]
      })
    ).toThrow();
  });

  it('preserves paragraph style attrs and rejects blank style attrs', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [{ type: 'paragraph', attrs: { style: 'caption' }, content: [{ type: 'text', text: 'Styled' }] }]
    });

    expect(doc.firstChild?.attrs.style).toBe('caption');
    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [{ type: 'paragraph', attrs: { style: ' ' }, content: [{ type: 'text', text: 'Bad' }] }]
      })
    ).toThrow();
  });

  it('accepts safe bookmark attrs and fragment links only', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'heading',
          attrs: { level: 1, bookmarkId: 'bm-heading' },
          content: [{ type: 'text', text: 'Target' }]
        },
        {
          type: 'paragraph',
          attrs: { style: 'body', bookmarkId: 'bm-body' },
          content: [{ type: 'text', text: 'Jump', marks: [{ type: 'link', attrs: { href: '#bm-heading' } }] }]
        }
      ]
    });

    expect(doc.child(0).attrs.bookmarkId).toBe('bm-heading');
    expect(sanitizeEditorHref('#bm-heading')).toBe('#bm-heading');
    expect(sanitizeEditorHref('#../bad')).toBeUndefined();

    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [{ type: 'paragraph', attrs: { style: 'body', bookmarkId: '../bad' }, content: [] }]
      })
    ).toThrow('unsupported paragraph bookmark');
  });

  it('accepts safe table of contents entries and rejects unsafe targets', () => {
    const doc = supportedSchema.nodeFromJSON({
      type: 'doc',
      content: [
        {
          type: 'table_of_contents',
          attrs: {
            title: 'Contents',
            entries: [
              { level: 1, text: 'Overview', target_bookmark_id: 'bm-overview' },
              { level: 3, text: 'Details', target_bookmark_id: 'bm-details' }
            ]
          }
        }
      ]
    });

    expect(doc.firstChild?.attrs.entries).toEqual([
      { level: 1, text: 'Overview', target_bookmark_id: 'bm-overview' },
      { level: 3, text: 'Details', target_bookmark_id: 'bm-details' }
    ]);
    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [
          {
            type: 'table_of_contents',
            attrs: {
              title: 'Contents',
              entries: [{ level: 1, text: 'Bad', target_bookmark_id: '../bad' }]
            }
          }
        ]
      })
    ).toThrow('unsupported table of contents entries');
  });

  it('rejects unsafe link schemes before schema projection', () => {
    expect(sanitizeEditorHref('javascript:alert(1)')).toBeUndefined();
    expect(sanitizeEditorHref('file://example.invalid/document.odt')).toBeUndefined();
    expect(sanitizeEditorHref('https://example.invalid')).toBe('https://example.invalid');
  });

  it('rejects unsafe link attrs when loading ProseMirror JSON', () => {
    expect(() =>
      supportedSchema.nodeFromJSON({
        type: 'doc',
        content: [
          {
            type: 'paragraph',
            content: [
              {
                type: 'text',
                text: 'Unsafe',
                marks: [{ type: 'link', attrs: { href: 'javascript:alert(1)' } }]
              }
            ]
          }
        ]
      })
    ).toThrow();
  });
});
