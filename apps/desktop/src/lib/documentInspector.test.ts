import { describe, expect, it } from 'vitest';
import {
  buildDocumentInspectorSummary,
  documentSavedStatus,
  formatBytes,
  type DocumentInspectorFileState
} from './documentInspector';
import type { DocumentState } from './documentProjection';

describe('document inspector helpers', () => {
  it('builds a local-first summary without leaking asset filenames or paths', () => {
    const document: DocumentState = {
      meta: {
        title: 'Inspector test',
        created_at: '2026-06-25T05:00:00Z',
        modified_at: '2026-06-25T05:30:00Z'
      },
      track_changes: { recording: true },
      assets: {
        'image-a': {
          id: 'image-a',
          media_type: 'image/png',
          byte_len: 1024,
          bytes: [1, 2, 3],
          original_name: '{PRIVATE_DOC_DIR}/private-chart.png'
        },
        'image-b': {
          id: 'image-b',
          media_type: 'image/jpeg',
          byte_len: 3,
          bytes: [4, 5, 6],
          original_name: 'private-photo.jpg'
        },
        'text-a': {
          id: 'text-a',
          media_type: 'text/plain',
          byte_len: 9000,
          bytes: [7],
          original_name: '{PRIVATE_DOC_DIR}/notes.txt'
        }
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
        'note-end': { id: 'note-end', kind: 'endnote', body: 'Endnote body' }
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
            { type: 'Paragraph', value: { inlines: [{ text: 'Intro text' }] } },
            { type: 'Image', value: { asset_id: 'image-a', alt_text: 'Chart' } },
            { type: 'Image', value: { asset_id: 'image-b', alt_text: 'Photo' } },
            {
              type: 'Paragraph',
              value: {
                inlines: [
                  {
                    text: 'Changed',
                    tracked_change: {
                      id: 'chg-1',
                      kind: 'deletion',
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

    const summary = buildDocumentInspectorSummary({
      coreStats: { word_count: 12, character_count: 80, block_count: 4 },
      document,
      fileState: {
        has_current_path: true,
        dirty: true,
        recovery_documents: [
          {
            token:
              'recovery-v1-00000000000040008000000000000071-00000000000040008000000000000072.odt',
            label: 'Recovery draft 1'
          }
        ]
      },
      plainText: 'Intro text Chart Photo Changed1i',
      selectionWordCount: 2
    });

    expect(summary).toMatchObject({
      format: 'OpenDocument Text (.odt)',
      savedStatus: 'saved_with_unsaved_changes',
      locationStatus: 'backend_only',
      pageSize: '216 x 279 mm',
      createdAt: '2026-06-25T05:00:00Z',
      modifiedAt: '2026-06-25T05:30:00Z',
      wordCount: 12,
      characterCount: 80,
      paragraphCount: 2,
      blockCount: 4,
      selectionWordCount: 2,
      embeddedImageCount: 2,
      embeddedImageBytes: 1027,
      embeddedImageBytesLabel: '1 KB',
      commentCount: 2,
      unresolvedCommentCount: 1,
      trackChangesRecording: true,
      trackedChangeCount: 1,
      footnoteCount: 1,
      endnoteCount: 1,
      privacyWarnings: ['comments', 'tracked_changes', 'metadata', 'recovery', 'unsaved']
    });
    expect(JSON.stringify(summary)).not.toContain('{PRIVATE_DOC_DIR}');
    expect(JSON.stringify(summary)).not.toContain('private-chart.png');
    expect(JSON.stringify(summary)).not.toContain('private-photo.jpg');
    expect(JSON.stringify(summary)).not.toContain('notes.txt');
  });

  it('summarizes clean saved documents with generic location status', () => {
    const fileState: DocumentInspectorFileState = {
      has_current_path: true,
      dirty: false,
      recovery_documents: []
    };

    const summary = buildDocumentInspectorSummary({
      coreStats: { word_count: 0, character_count: 0, block_count: 0 },
      document: {
        meta: { title: 'Untitled Document' },
        sections: [{ blocks: [] }]
      },
      fileState,
      plainText: '',
      selectionWordCount: -10
    });

    expect(documentSavedStatus(fileState)).toBe('saved');
    expect(summary.savedStatus).toBe('saved');
    expect(summary.locationStatus).toBe('backend_only');
    expect(summary.createdAt).toBe('');
    expect(summary.modifiedAt).toBe('');
    expect(summary.privacyWarnings).toEqual(['metadata']);
  });

  it('formats byte counts with bounded labels', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(999)).toBe('999 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(10 * 1024)).toBe('10 KB');
    expect(formatBytes(2 * 1024 * 1024)).toBe('2 MB');
    expect(formatBytes(-1)).toBe('0 B');
  });
});
