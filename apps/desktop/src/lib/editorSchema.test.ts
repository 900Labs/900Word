import { describe, expect, it } from 'vitest';
import { sanitizeEditorHref } from './editorSecurity';
import { supportedBlockTypes, supportedListNodeTypes, supportedMarkTypes, supportedSchema } from './editorSchema';

describe('supportedSchema', () => {
  it('contains only word-core projected block nodes plus doc and text', () => {
    expect(Object.keys(supportedSchema.nodes).sort()).toEqual([
      'bullet_list',
      'doc',
      'heading',
      'list_item',
      'ordered_list',
      'paragraph',
      'text'
    ]);
    expect(supportedBlockTypes).toEqual(['paragraph', 'heading']);
    expect(supportedListNodeTypes).toEqual(['bullet_list', 'ordered_list', 'list_item']);
  });

  it('contains only word-core inline mark projections', () => {
    expect(Object.keys(supportedSchema.marks).sort()).toEqual([
      'bold',
      'italic',
      'link',
      'strikethrough',
      'subscript',
      'superscript',
      'textStyle',
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
      'link'
    ]);
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
