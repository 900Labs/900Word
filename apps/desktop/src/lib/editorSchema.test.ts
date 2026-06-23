import { describe, expect, it } from 'vitest';
import { sanitizeEditorHref } from './editorSecurity';
import { supportedBlockTypes, supportedMarkTypes, supportedSchema } from './editorSchema';

describe('supportedSchema', () => {
  it('contains only word-core projected block nodes plus doc and text', () => {
    expect(Object.keys(supportedSchema.nodes).sort()).toEqual(['doc', 'heading', 'paragraph', 'text']);
    expect(supportedBlockTypes).toEqual(['paragraph', 'heading']);
  });

  it('contains only word-core inline mark projections', () => {
    expect(Object.keys(supportedSchema.marks).sort()).toEqual([
      'bold',
      'italic',
      'link',
      'strikethrough',
      'subscript',
      'superscript',
      'underline'
    ]);
    expect(supportedMarkTypes).toEqual([
      'bold',
      'italic',
      'underline',
      'strikethrough',
      'superscript',
      'subscript',
      'link'
    ]);
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
