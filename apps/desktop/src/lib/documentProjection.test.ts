import { describe, expect, it } from 'vitest';
import { documentToText, type DocumentState } from './documentProjection';

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
});
