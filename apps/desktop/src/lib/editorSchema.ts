import { Schema, type MarkSpec, type NodeSpec } from 'prosemirror-model';
import { sanitizeCommentId, sanitizeNoteId, sanitizeNoteLabel, sanitizeTrackedChangeId } from './documentProjection';
import { sanitizeBookmarkId, sanitizeEditorHref } from './editorSecurity';

const nodes: Record<string, NodeSpec> = {
  doc: {
    content: 'block+'
  },
  paragraph: {
    attrs: {
      style: {
        default: 'body',
        validate(value: unknown) {
          if (typeof value !== 'string' || value.trim().length === 0) {
            throw new RangeError('unsupported paragraph style');
          }
        }
      },
      align: {
        default: null,
        validate(value: unknown) {
          if (value !== null && !['left', 'center', 'right', 'justify'].includes(String(value))) {
            throw new RangeError('unsupported paragraph alignment');
          }
        }
      },
      lineSpacing: {
        default: null,
        validate(value: unknown) {
          if (value !== null && ![1000, 1150, 1500, 2000].includes(Number(value))) {
            throw new RangeError('unsupported paragraph line spacing');
          }
        }
      },
      spacingBefore: { default: null },
      spacingAfter: { default: null },
      indentStart: { default: null },
      indentEnd: { default: null },
      firstLineIndent: { default: null },
      bookmarkId: {
        default: null,
        validate(value: unknown) {
          if (value !== null && (typeof value !== 'string' || !sanitizeBookmarkId(value))) {
            throw new RangeError('unsupported paragraph bookmark');
          }
        }
      }
    },
    content: 'inline*',
    group: 'block',
    parseDOM: [
      {
        tag: 'p',
        getAttrs(node) {
          return {
            style: node.getAttribute('data-style') || 'body',
            align: node.getAttribute('data-align'),
            lineSpacing: numberAttr(node, 'data-line-spacing'),
            spacingBefore: numberAttr(node, 'data-spacing-before'),
            spacingAfter: numberAttr(node, 'data-spacing-after'),
            indentStart: numberAttr(node, 'data-indent-start'),
            indentEnd: numberAttr(node, 'data-indent-end'),
            firstLineIndent: numberAttr(node, 'data-first-line-indent'),
            bookmarkId: sanitizeBookmarkId(node.getAttribute('data-bookmark-id') ?? node.getAttribute('id') ?? '') ?? null
          };
        }
      }
    ],
    toDOM(node) {
      return ['p', paragraphDomAttrs(node.attrs), 0];
    }
  },
  heading: {
    attrs: {
      level: { default: 1 },
      bookmarkId: {
        default: null,
        validate(value: unknown) {
          if (value !== null && (typeof value !== 'string' || !sanitizeBookmarkId(value))) {
            throw new RangeError('unsupported heading bookmark');
          }
        }
      }
    },
    content: 'inline*',
    defining: true,
    group: 'block',
    parseDOM: [
      { tag: 'h1', getAttrs: (node) => headingAttrsFromDom(node, 1) },
      { tag: 'h2', getAttrs: (node) => headingAttrsFromDom(node, 2) },
      { tag: 'h3', getAttrs: (node) => headingAttrsFromDom(node, 3) },
      { tag: 'h4', getAttrs: (node) => headingAttrsFromDom(node, 4) },
      { tag: 'h5', getAttrs: (node) => headingAttrsFromDom(node, 5) },
      { tag: 'h6', getAttrs: (node) => headingAttrsFromDom(node, 6) }
    ],
    toDOM(node) {
      return [`h${node.attrs.level}`, blockBookmarkDomAttrs(node.attrs), 0];
    }
  },
  table_of_contents: {
    attrs: {
      title: {
        default: 'Contents',
        validate(value: unknown) {
          if (typeof value !== 'string' || !safeTocTitle(value)) {
            throw new RangeError('unsupported table of contents title');
          }
        }
      },
      entries: {
        default: [],
        validate(value: unknown) {
          if (!Array.isArray(value) || !value.every(safeTocEntry)) {
            throw new RangeError('unsupported table of contents entries');
          }
        }
      }
    },
    atom: true,
    selectable: true,
    group: 'block',
    parseDOM: [
      {
        tag: 'nav[data-900word-block="table-of-contents"]',
        getAttrs(node) {
          const title = node.getAttribute('data-toc-title') ?? '';
          return {
            title: safeTocTitle(title) ? title : 'Contents',
            entries: tocEntriesFromDom(node)
          };
        }
      }
    ],
    toDOM(node) {
      const entries = normalizeTocEntries(node.attrs.entries);
      return [
        'nav',
        tocDomAttrs(node.attrs, entries),
        ['div', { class: 'toc-title' }, safeTocTitle(String(node.attrs.title)) ? String(node.attrs.title).trim() : 'Contents'],
        [
          'ol',
          { class: 'toc-list' },
          ...entries.map((entry) => [
            'li',
            { class: `toc-level-${entry.level}`, 'data-toc-level': String(entry.level) },
            ['a', { href: `#${entry.target_bookmark_id}` }, entry.text]
          ])
        ]
      ];
    },
    leafText(node) {
      return tocPlainText(String(node.attrs.title ?? 'Contents'), normalizeTocEntries(node.attrs.entries));
    }
  },
  bullet_list: {
    attrs: {
      definitionId: { default: '900w-unordered' }
    },
    content: 'list_item+',
    group: 'block',
    parseDOM: [
      {
        tag: 'ul',
        getAttrs(node) {
          return { definitionId: node.getAttribute('data-definition-id') || '900w-unordered' };
        }
      }
    ],
    toDOM(node) {
      return ['ul', { 'data-definition-id': node.attrs.definitionId }, 0];
    }
  },
  ordered_list: {
    attrs: {
      definitionId: { default: '900w-ordered' }
    },
    content: 'list_item+',
    group: 'block',
    parseDOM: [
      {
        tag: 'ol',
        getAttrs(node) {
          return { definitionId: node.getAttribute('data-definition-id') || '900w-ordered' };
        }
      }
    ],
    toDOM(node) {
      return ['ol', { 'data-definition-id': node.attrs.definitionId }, 0];
    }
  },
  list_item: {
    attrs: {
      level: {
        default: 1,
        validate(value: unknown) {
          const level = Number(value);
          if (!Number.isInteger(level) || level < 1 || level > 8) {
            throw new RangeError('unsupported list level');
          }
        }
      }
    },
    content: 'paragraph (paragraph | heading)*',
    defining: true,
    parseDOM: [
      {
        tag: 'li',
        getAttrs(node) {
          return { level: numberAttr(node, 'data-level') ?? 1 };
        }
      }
    ],
    toDOM(node) {
      return ['li', { 'data-level': node.attrs.level }, 0];
    }
  },
  table: {
    content: 'table_row+',
    group: 'block',
    isolating: true,
    parseDOM: [{ tag: 'table' }],
    toDOM() {
      return ['table', ['tbody', 0]];
    }
  },
  table_row: {
    content: 'table_cell+',
    parseDOM: [{ tag: 'tr' }],
    toDOM() {
      return ['tr', 0];
    }
  },
  table_cell: {
    attrs: {
      unsupported: {
        default: false,
        validate(value: unknown) {
          if (typeof value !== 'boolean') {
            throw new RangeError('unsupported table cell state');
          }
        }
      },
      sourceEmpty: {
        default: false,
        validate(value: unknown) {
          if (typeof value !== 'boolean') {
            throw new RangeError('unsupported table cell source state');
          }
        }
      }
    },
    content: '(paragraph | heading | bullet_list | ordered_list)+',
    isolating: true,
    parseDOM: [
      {
        tag: 'td',
        getAttrs(node) {
          return {
            unsupported: node.getAttribute('data-unsupported') === 'true',
            sourceEmpty: node.getAttribute('data-source-empty') === 'true'
          };
        }
      },
      {
        tag: 'th',
        getAttrs(node) {
          return {
            unsupported: node.getAttribute('data-unsupported') === 'true',
            sourceEmpty: node.getAttribute('data-source-empty') === 'true'
          };
        }
      }
    ],
    toDOM(node) {
      return ['td', tableCellDomAttrs(node.attrs), 0];
    }
  },
  image: {
    attrs: {
      assetId: {
        validate(value: unknown) {
          if (typeof value !== 'string' || !safeAssetId(value)) {
            throw new RangeError('unsupported image asset');
          }
        }
      },
      altText: {
        default: 'Image',
        validate(value: unknown) {
          if (value !== null && value !== undefined && typeof value !== 'string') {
            throw new RangeError('unsupported image alt text');
          }
        }
      },
      alignment: {
        default: 'inline',
        validate(value: unknown) {
          if (!['inline', 'left', 'center', 'right'].includes(String(value))) {
            throw new RangeError('unsupported image alignment');
          }
        }
      },
      scalePercent: {
        default: 100,
        validate(value: unknown) {
          const scale = Number(value);
          if (!Number.isInteger(scale) || scale < 25 || scale > 200) {
            throw new RangeError('unsupported image scale');
          }
        }
      },
      caption: {
        default: null,
        validate(value: unknown) {
          if (value !== null && value !== undefined && typeof value !== 'string') {
            throw new RangeError('unsupported image caption');
          }
        }
      },
      src: {
        default: null,
        validate(value: unknown) {
          if (value !== null && (typeof value !== 'string' || !safeImageSrc(value))) {
            throw new RangeError('unsupported image source');
          }
        }
      }
    },
    atom: true,
    draggable: false,
    group: 'block',
    isolating: true,
    parseDOM: [
      {
        tag: 'figure[data-asset-id]',
        getAttrs(node) {
          const assetId = node.getAttribute('data-asset-id') ?? '';
          return safeAssetId(assetId)
            ? {
                assetId,
                altText: node.getAttribute('data-alt-text') || 'Image',
                alignment: imageAlignmentAttr(node.getAttribute('data-align')),
                scalePercent: imageScaleAttr(node.getAttribute('data-scale')),
                caption: node.getAttribute('data-caption'),
                src: safeImageSrc(node.querySelector('img')?.getAttribute('src') ?? '')
                  ? node.querySelector('img')?.getAttribute('src')
                  : null
              }
            : false;
        }
      }
    ],
    toDOM(node) {
      const attrs = imageDomAttrs(node.attrs);
      const altText = typeof node.attrs.altText === 'string' && node.attrs.altText.trim().length > 0
        ? node.attrs.altText
        : 'Image';
      const caption = typeof node.attrs.caption === 'string' && node.attrs.caption.trim().length > 0
        ? node.attrs.caption
        : altText;
      const image = safeImageSrc(String(node.attrs.src ?? ''))
        ? ['img', { src: node.attrs.src, alt: altText }]
        : ['span', { class: 'image-placeholder-text' }, altText];
      return [
        'figure',
        attrs,
        image,
        ['figcaption', caption],
        ['span', { class: 'image-resize-handle', 'aria-hidden': 'true' }]
      ];
    }
  },
  note_reference: {
    inline: true,
    group: 'inline',
    atom: true,
    selectable: false,
    attrs: {
      id: {
        validate(value: unknown) {
          if (typeof value !== 'string' || !sanitizeNoteId(value)) {
            throw new RangeError('unsupported note id');
          }
        }
      },
      kind: {
        validate(value: unknown) {
          if (value !== 'footnote' && value !== 'endnote') {
            throw new RangeError('unsupported note kind');
          }
        }
      },
      label: {
        validate(value: unknown) {
          if (typeof value !== 'string' || !sanitizeNoteLabel(value)) {
            throw new RangeError('unsupported note label');
          }
        }
      }
    },
    parseDOM: [
      {
        tag: 'sup[data-note-reference-id]',
        getAttrs(node) {
          const id = sanitizeNoteId(node.getAttribute('data-note-reference-id') ?? '');
          const kind = noteKindAttr(node.getAttribute('data-note-kind'));
          const label = sanitizeNoteLabel(node.getAttribute('data-note-label') ?? node.textContent ?? '');
          return id && kind && label ? { id, kind, label } : false;
        }
      }
    ],
    toDOM(node) {
      const id = sanitizeNoteId(node.attrs.id) ?? '';
      const kind = noteKindAttr(node.attrs.kind) ?? 'footnote';
      const label = sanitizeNoteLabel(node.attrs.label) ?? '1';
      return ['sup', noteReferenceDomAttrs(id, kind, label), label];
    },
    leafText(node) {
      return sanitizeNoteLabel(node.attrs.label) ?? '';
    }
  },
  text: {
    group: 'inline'
  }
};

const marks: Record<string, MarkSpec> = {
  bold: {
    parseDOM: [
      { tag: 'strong' },
      {
        tag: 'b',
        getAttrs: (node) => node.style.fontWeight !== 'normal' && null
      },
      {
        style: 'font-weight',
        getAttrs: (value) => /^(bold(er)?|[5-9]\d{2,})$/.test(String(value)) && null
      }
    ],
    toDOM() {
      return ['strong', 0];
    }
  },
  italic: {
    parseDOM: [{ tag: 'i' }, { tag: 'em' }, { style: 'font-style=italic' }],
    toDOM() {
      return ['em', 0];
    }
  },
  underline: {
    parseDOM: [{ tag: 'u' }, { style: 'text-decoration=underline' }],
    toDOM() {
      return ['u', 0];
    }
  },
  strikethrough: {
    parseDOM: [{ tag: 's' }, { tag: 'del' }, { style: 'text-decoration=line-through' }],
    toDOM() {
      return ['s', 0];
    }
  },
  superscript: {
    parseDOM: [{ tag: 'sup' }],
    toDOM() {
      return ['sup', 0];
    }
  },
  subscript: {
    parseDOM: [{ tag: 'sub' }],
    toDOM() {
      return ['sub', 0];
    }
  },
  textStyle: {
    attrs: {
      fontFamily: {
        default: null,
        validate(value: unknown) {
          if (value !== null && !safeTextStyleValue(String(value))) {
            throw new RangeError('unsupported font family');
          }
        }
      },
      fontSizePt: {
        default: null,
        validate(value: unknown) {
          if (value !== null && ![9, 10, 11, 12, 14, 16, 18, 24, 32].includes(Number(value))) {
            throw new RangeError('unsupported font size');
          }
        }
      },
      textColor: {
        default: null,
        validate(value: unknown) {
          if (value !== null && !safeColor(String(value))) {
            throw new RangeError('unsupported text color');
          }
        }
      },
      highlightColor: {
        default: null,
        validate(value: unknown) {
          if (value !== null && !safeColor(String(value))) {
            throw new RangeError('unsupported highlight color');
          }
        }
      }
    },
    parseDOM: [
      {
        tag: 'span[data-text-style]',
        getAttrs(node) {
          return {
            fontFamily: safeTextStyleValue(node.getAttribute('data-font-family') ?? '')
              ? node.getAttribute('data-font-family')
              : null,
            fontSizePt: numberAttr(node, 'data-font-size-pt'),
            textColor: safeColor(node.getAttribute('data-text-color') ?? '')
              ? node.getAttribute('data-text-color')
              : null,
            highlightColor: safeColor(node.getAttribute('data-highlight-color') ?? '')
              ? node.getAttribute('data-highlight-color')
              : null
          };
        }
      }
    ],
    toDOM(mark) {
      const attrs = textStyleDomAttrs(mark.attrs);
      return ['span', attrs, 0];
    }
  },
  link: {
    attrs: {
      href: {
        validate(value: unknown) {
          if (typeof value !== 'string' || !sanitizeEditorHref(value)) {
            throw new RangeError('unsupported link href');
          }
        }
      }
    },
    inclusive: false,
    parseDOM: [
      {
        tag: 'a[href]',
        getAttrs(node) {
          const href = sanitizeEditorHref(node.getAttribute('href') ?? '');
          return href ? { href } : false;
        }
      }
    ],
    toDOM(node) {
      return ['a', { href: sanitizeEditorHref(String(node.attrs.href)) ?? '', rel: 'noreferrer' }, 0];
    }
  },
  comment: {
    excludes: '',
    attrs: {
      id: {
        validate(value: unknown) {
          if (typeof value !== 'string' || !sanitizeCommentId(value)) {
            throw new RangeError('unsupported comment id');
          }
        }
      }
    },
    inclusive: false,
    parseDOM: [
      {
        tag: 'span[data-comment-id]',
        getAttrs(node) {
          const id = sanitizeCommentId(node.getAttribute('data-comment-id') ?? '');
          return id ? { id } : false;
        }
      }
    ],
    toDOM(mark) {
      const id = sanitizeCommentId(String(mark.attrs.id)) ?? '';
      return ['span', { class: 'comment-marker', 'data-comment-id': id }, 0];
    }
  },
  trackedChange: {
    excludes: '',
    attrs: {
      id: {
        validate(value: unknown) {
          if (typeof value !== 'string' || !sanitizeTrackedChangeId(value)) {
            throw new RangeError('unsupported tracked change id');
          }
        }
      },
      kind: {
        validate(value: unknown) {
          if (value !== 'insertion' && value !== 'deletion') {
            throw new RangeError('unsupported tracked change kind');
          }
        }
      },
      author: {
        default: 'Local User',
        validate(value: unknown) {
          if (typeof value !== 'string' || !safeLocalAuthor(value)) {
            throw new RangeError('unsupported tracked change author');
          }
        }
      },
      createdAt: {
        validate(value: unknown) {
          if (typeof value !== 'string' || !Number.isFinite(Date.parse(value))) {
            throw new RangeError('unsupported tracked change timestamp');
          }
        }
      }
    },
    inclusive: false,
    parseDOM: [
      {
        tag: 'span[data-tracked-change-id]',
        getAttrs(node) {
          const id = sanitizeTrackedChangeId(node.getAttribute('data-tracked-change-id') ?? '');
          const kind = trackedChangeKindAttr(node.getAttribute('data-tracked-change-kind'));
          const author = safeLocalAuthor(node.getAttribute('data-tracked-change-author') ?? '')
            ? node.getAttribute('data-tracked-change-author')
            : 'Local User';
          const createdAt = node.getAttribute('data-tracked-change-created-at') ?? '';
          return id && kind && Number.isFinite(Date.parse(createdAt))
            ? { id, kind, author, createdAt }
            : false;
        }
      }
    ],
    toDOM(mark) {
      const id = sanitizeTrackedChangeId(String(mark.attrs.id)) ?? '';
      const kind = trackedChangeKindAttr(mark.attrs.kind) ?? 'insertion';
      const author = safeLocalAuthor(String(mark.attrs.author ?? '')) ? String(mark.attrs.author) : 'Local User';
      const createdAt = Number.isFinite(Date.parse(String(mark.attrs.createdAt))) ? String(mark.attrs.createdAt) : new Date(0).toISOString();
      return [
        'span',
        {
          class: `tracked-change tracked-${kind}`,
          'data-tracked-change-id': id,
          'data-tracked-change-kind': kind,
          'data-tracked-change-author': author,
          'data-tracked-change-created-at': createdAt
        },
        0
      ];
    }
  }
};

export const supportedSchema = new Schema({
  nodes,
  marks
});

export const supportedBlockTypes = ['paragraph', 'heading'] as const;
export const supportedListNodeTypes = ['bullet_list', 'ordered_list', 'list_item'] as const;
export const supportedTableNodeTypes = ['table', 'table_row', 'table_cell'] as const;
export const supportedImageNodeTypes = ['image'] as const;
export const supportedInlineNodeTypes = ['note_reference'] as const;
export const supportedMarkTypes = [
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
] as const;

function numberAttr(node: Element, name: string): number | null {
  const value = node.getAttribute(name);
  if (value === null || value.trim() === '') {
    return null;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function paragraphDomAttrs(attrs: Record<string, unknown>): Record<string, string> {
  const domAttrs: Record<string, string> = { 'data-style': String(attrs.style || 'body') };
  Object.assign(domAttrs, blockBookmarkDomAttrs(attrs));
  setStringAttr(domAttrs, 'data-align', attrs.align);
  setNumberAttr(domAttrs, 'data-line-spacing', attrs.lineSpacing);
  setNumberAttr(domAttrs, 'data-spacing-before', attrs.spacingBefore);
  setNumberAttr(domAttrs, 'data-spacing-after', attrs.spacingAfter);
  setNumberAttr(domAttrs, 'data-indent-start', attrs.indentStart);
  setNumberAttr(domAttrs, 'data-indent-end', attrs.indentEnd);
  setNumberAttr(domAttrs, 'data-first-line-indent', attrs.firstLineIndent);

  const css: string[] = [];
  if (attrs.align) {
    css.push(`text-align: ${attrs.align}`);
  }
  if (attrs.lineSpacing) {
    css.push(`line-height: ${Number(attrs.lineSpacing) / 1000}`);
  }
  if (attrs.spacingBefore) {
    css.push(`margin-top: ${attrs.spacingBefore}mm`);
  }
  if (attrs.spacingAfter) {
    css.push(`margin-bottom: ${attrs.spacingAfter}mm`);
  }
  if (attrs.indentStart) {
    css.push(`margin-left: ${attrs.indentStart}mm`);
  }
  if (attrs.indentEnd) {
    css.push(`margin-right: ${attrs.indentEnd}mm`);
  }
  if (attrs.firstLineIndent) {
    css.push(`text-indent: ${attrs.firstLineIndent}mm`);
  }
  if (css.length > 0) {
    domAttrs.style = css.join('; ');
  }
  return domAttrs;
}

function headingAttrsFromDom(node: Element, level: number): Record<string, string | number | null> {
  return {
    level,
    bookmarkId: sanitizeBookmarkId(node.getAttribute('data-bookmark-id') ?? node.getAttribute('id') ?? '') ?? null
  };
}

interface TocDomEntry {
  level: number;
  text: string;
  target_bookmark_id: string;
}

function tocEntriesFromDom(node: Element): TocDomEntry[] {
  const raw = node.getAttribute('data-toc-entries');
  if (!raw) {
    return [];
  }
  try {
    return normalizeTocEntries(JSON.parse(raw));
  } catch {
    return [];
  }
}

function tocDomAttrs(attrs: Record<string, unknown>, entries: TocDomEntry[]): Record<string, string> {
  const title = safeTocTitle(String(attrs.title ?? '')) ? String(attrs.title).trim() : 'Contents';
  return {
    class: 'toc-block',
    'data-900word-block': 'table-of-contents',
    'data-toc-title': title,
    'data-toc-entries': JSON.stringify(entries),
    contenteditable: 'false',
    'aria-label': title
  };
}

function tocPlainText(title: string, entries: TocDomEntry[]): string {
  const lines = [safeTocTitle(title) ? title.trim() : 'Contents'];
  for (const entry of entries) {
    lines.push(`${'  '.repeat(entry.level - 1)}${entry.text}`);
  }
  return lines.filter(Boolean).join('\n');
}

function normalizeTocEntries(value: unknown): TocDomEntry[] {
  if (!Array.isArray(value)) {
    return [];
  }
  return value
    .map((entry): TocDomEntry | undefined => {
      if (!safeTocEntry(entry)) {
        return undefined;
      }
      return {
        level: Math.trunc(Number(entry.level)),
        text: safeTocText(String(entry.text)),
        target_bookmark_id: sanitizeBookmarkId(String(entry.target_bookmark_id)) ?? ''
      };
    })
    .filter((entry): entry is TocDomEntry => entry !== undefined);
}

function safeTocTitle(value: string): boolean {
  const trimmed = value.replace(/[\u0000-\u001f\u007f]/g, '').trim();
  return trimmed.length > 0 && Array.from(trimmed).length <= 120;
}

function safeTocText(value: string): string {
  return value.replace(/[\u0000-\u001f\u007f]/g, ' ').trim();
}

function safeTocEntry(value: unknown): value is TocDomEntry {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const entry = value as Partial<TocDomEntry>;
  const level = Math.trunc(Number(entry.level));
  const text = typeof entry.text === 'string' ? safeTocText(entry.text) : '';
  const target = typeof entry.target_bookmark_id === 'string'
    ? sanitizeBookmarkId(entry.target_bookmark_id)
    : null;
  return level >= 1 && level <= 3 && text.length > 0 && Boolean(target);
}

function blockBookmarkDomAttrs(attrs: Record<string, unknown>): Record<string, string> {
  if (typeof attrs.bookmarkId !== 'string') {
    return {};
  }
  const bookmarkId = sanitizeBookmarkId(attrs.bookmarkId);
  return bookmarkId ? { id: bookmarkId, 'data-bookmark-id': bookmarkId } : {};
}

function tableCellDomAttrs(attrs: Record<string, unknown>): Record<string, string> {
  const domAttrs: Record<string, string> = {};
  if (attrs.unsupported === true) {
    domAttrs['data-unsupported'] = 'true';
    domAttrs.contenteditable = 'false';
  }
  if (attrs.sourceEmpty === true) {
    domAttrs['data-source-empty'] = 'true';
  }
  return domAttrs;
}

function imageDomAttrs(attrs: Record<string, unknown>): Record<string, string> {
  const alignment = imageAlignmentAttr(attrs.alignment);
  const scale = imageScaleAttr(attrs.scalePercent);
  const domAttrs: Record<string, string> = {
    'data-asset-id': String(attrs.assetId),
    'data-alt-text': typeof attrs.altText === 'string' ? attrs.altText : 'Image',
    'data-align': alignment,
    'data-scale': String(scale),
    contenteditable: 'false'
  };
  if (typeof attrs.caption === 'string' && attrs.caption.trim().length > 0) {
    domAttrs['data-caption'] = attrs.caption;
  }
  const css = [`width: ${scale}%`];
  if (alignment === 'inline') {
    css.push('display: inline-block');
  } else if (alignment === 'left') {
    css.push('margin-left: 0', 'margin-right: auto');
  } else if (alignment === 'center') {
    css.push('margin-left: auto', 'margin-right: auto');
  } else if (alignment === 'right') {
    css.push('margin-left: auto', 'margin-right: 0');
  }
  domAttrs.style = css.join('; ');
  return domAttrs;
}

function noteKindAttr(value: unknown): 'footnote' | 'endnote' | null {
  return value === 'footnote' || value === 'endnote' ? value : null;
}

function noteReferenceDomAttrs(id: string, kind: 'footnote' | 'endnote', label: string): Record<string, string> {
  return {
    class: `note-reference note-${kind}`,
    'data-note-reference-id': id,
    'data-note-kind': kind,
    'data-note-label': label,
    contenteditable: 'false',
    'aria-label': `${kind === 'footnote' ? 'Footnote' : 'Endnote'} ${label}`
  };
}

function imageAlignmentAttr(value: unknown): 'inline' | 'left' | 'center' | 'right' {
  return value === 'left' || value === 'center' || value === 'right' || value === 'inline'
    ? value
    : 'inline';
}

function imageScaleAttr(value: unknown): number {
  const scale = Number(value);
  return Number.isInteger(scale) && scale >= 25 && scale <= 200 ? scale : 100;
}

function textStyleDomAttrs(attrs: Record<string, unknown>): Record<string, string> {
  const domAttrs: Record<string, string> = { 'data-text-style': 'true' };
  setStringAttr(domAttrs, 'data-font-family', attrs.fontFamily);
  setNumberAttr(domAttrs, 'data-font-size-pt', attrs.fontSizePt);
  setStringAttr(domAttrs, 'data-text-color', attrs.textColor);
  setStringAttr(domAttrs, 'data-highlight-color', attrs.highlightColor);

  const css: string[] = [];
  if (attrs.fontFamily) {
    css.push(`font-family: ${attrs.fontFamily}`);
  }
  if (attrs.fontSizePt) {
    css.push(`font-size: ${attrs.fontSizePt}pt`);
  }
  if (attrs.textColor) {
    css.push(`color: ${attrs.textColor}`);
  }
  if (attrs.highlightColor) {
    css.push(`background-color: ${attrs.highlightColor}`);
  }
  if (css.length > 0) {
    domAttrs.style = css.join('; ');
  }
  return domAttrs;
}

function setStringAttr(output: Record<string, string>, name: string, value: unknown) {
  if (typeof value === 'string' && value.trim().length > 0) {
    output[name] = value;
  }
}

function setNumberAttr(output: Record<string, string>, name: string, value: unknown) {
  if (typeof value === 'number' && Number.isFinite(value)) {
    output[name] = String(value);
  }
}

function safeColor(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}

function safeAssetId(value: string): boolean {
  return /^[A-Za-z0-9._@-]+$/.test(value) && !value.includes('..') && !value.includes('/') && !value.includes('\\');
}

function safeImageSrc(value: string): boolean {
  return /^data:image\/(?:png|jpeg|gif|webp);base64,[A-Za-z0-9+/]+=*$/.test(value);
}

function safeTextStyleValue(value: string): boolean {
  return /^[A-Za-z0-9 ,.'"-]{1,80}$/.test(value);
}

function trackedChangeKindAttr(value: unknown): 'insertion' | 'deletion' | null {
  return value === 'insertion' || value === 'deletion' ? value : null;
}

function safeLocalAuthor(value: string): boolean {
  const trimmed = value.replace(/[\u0000-\u001f\u007f]/g, '').trim();
  return trimmed.length > 0 && Array.from(trimmed).length <= 80;
}
