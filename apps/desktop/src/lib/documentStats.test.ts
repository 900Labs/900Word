import { describe, expect, it } from 'vitest';
import {
  buildExpandedDocumentStats,
  countCharactersWithoutWhitespace,
  estimatePageCount,
  estimateReadingMinutes,
  formatPageSize,
  type CoreDocumentStats
} from './documentStats';
import type { DocumentState } from './documentProjection';

const baseStats: CoreDocumentStats = {
  word_count: 0,
  character_count: 0,
  block_count: 0
};

describe('document stats helpers', () => {
  it('counts visible non-whitespace characters and bounded estimates', () => {
    expect(countCharactersWithoutWhitespace('A b\nc\t!')).toBe(4);
    expect(estimatePageCount(0)).toBe(0);
    expect(estimatePageCount(1)).toBe(1);
    expect(estimatePageCount(501)).toBe(2);
    expect(estimateReadingMinutes(0)).toBe(0);
    expect(estimateReadingMinutes(201)).toBe(2);
  });

  it('builds expanded stats from local document projection without path metadata', () => {
    const document: DocumentState = {
      meta: { title: 'Stats test' },
      track_changes: { recording: true },
      assets: {
        'asset-image': {
          id: 'asset-image',
          media_type: 'image/png',
          byte_len: 4,
          bytes: [1, 2, 3, 4],
          original_name: '{PRIVATE_DOC_DIR}/private-chart.png'
        },
        'asset-unused': { id: 'asset-unused', media_type: 'image/webp', byte_len: 2, bytes: [5, 6] }
      },
      comments: {
        'comment-open': {
          id: 'comment-open',
          author: 'Local User',
          body: 'Needs review',
          created_at: '2026-01-01T00:00:00Z',
          updated_at: '2026-01-01T00:00:00Z',
          resolved: false
        },
        'comment-resolved': {
          id: 'comment-resolved',
          author: 'Local User',
          body: 'Done',
          created_at: '2026-01-02T00:00:00Z',
          updated_at: '2026-01-02T00:00:00Z',
          resolved: true
        }
      },
      notes: {
        'note-foot': { id: 'note-foot', kind: 'footnote', body: 'Footnote body' },
        'note-end': { id: 'note-end', kind: 'endnote', body: 'Endnote body' },
        'note-orphan': { id: 'note-orphan', kind: 'footnote', body: 'Hidden orphan' }
      },
      sections: [
        {
          page: {
            width_mm: 216,
            height_mm: 279,
            margin_top_mm: 20,
            margin_right_mm: 20,
            margin_bottom_mm: 20,
            margin_left_mm: 20
          },
          blocks: [
            { type: 'Paragraph', value: { inlines: [{ text: 'Intro' }] } },
            {
              type: 'List',
              value: {
                definition_id: 'list',
                items: [{ level: 0, blocks: [{ type: 'Paragraph', value: { inlines: [{ text: 'List item' }] } }] }]
              }
            },
            {
              type: 'Table',
              value: {
                rows: [
                  {
                    cells: [
                      {
                        blocks: [{ type: 'Heading', value: { level: 2, inlines: [{ text: 'Cell heading' }] } }]
                      }
                    ]
                  }
                ]
              }
            },
            { type: 'Image', value: { asset_id: 'asset-image', alt_text: 'Chart' } },
            {
              type: 'Paragraph',
              value: {
                inlines: [
                  {
                    text: 'Changed',
                    tracked_change: {
                      id: 'chg-1',
                      kind: 'insertion',
                      author: 'Local User',
                      created_at: '2026-01-03T00:00:00Z'
                    }
                  },
                  { text: '1', note_reference: { id: 'note-foot', kind: 'footnote', label: '1' } },
                  { text: 'i', note_reference: { id: 'note-end', kind: 'endnote', label: 'i' } }
                ]
              }
            }
          ]
        }
      ]
    };

    const expanded = buildExpandedDocumentStats({
      coreStats: { word_count: 501, character_count: 1200, block_count: 5 },
      document,
      plainText: 'Intro List item Cell heading Changed1i',
      selectionWordCount: 3,
      pageSetup: document.sections[0].page
    });

    expect(expanded).toMatchObject({
      wordCount: 501,
      characterCountWithSpaces: 1200,
      characterCountWithoutSpaces: 61,
      paragraphCount: 4,
      blockCount: 5,
      estimatedPageCount: 2,
      estimatedReadingMinutes: 3,
      selectionWordCount: 3,
      commentCount: 2,
      unresolvedCommentCount: 1,
      trackChangesRecording: true,
      trackedChangeCount: 1,
      imageCount: 1,
      assetCount: 2,
      footnoteCount: 1,
      endnoteCount: 1,
      pageSize: '216 x 279 mm'
    });
    expect(JSON.stringify(expanded)).not.toContain('{PRIVATE_DOC_DIR}/private-chart.png');
    expect(JSON.stringify(expanded)).not.toContain('private-chart.png');
  });

  it('formats page sizes without exposing document names or paths', () => {
    expect(
      formatPageSize({
        width_mm: 210,
        height_mm: 297.5,
        margin_top_mm: 25,
        margin_right_mm: 25,
        margin_bottom_mm: 25,
        margin_left_mm: 25
      })
    ).toBe('210 x 297.5 mm');
  });

  it('returns zeroed document indicators when no document is loaded', () => {
    expect(
      buildExpandedDocumentStats({
        coreStats: baseStats,
        document: undefined,
        plainText: '',
        selectionWordCount: -1,
        pageSetup: undefined
      })
    ).toMatchObject({
      wordCount: 0,
      characterCountWithSpaces: 0,
      characterCountWithoutSpaces: 0,
      paragraphCount: 0,
      estimatedPageCount: 0,
      estimatedReadingMinutes: 0,
      selectionWordCount: 0,
      commentCount: 0,
      unresolvedCommentCount: 0,
      trackChangesRecording: false,
      trackedChangeCount: 0,
      imageCount: 0,
      assetCount: 0,
      footnoteCount: 0,
      endnoteCount: 0,
      pageSize: ''
    });
  });
});
