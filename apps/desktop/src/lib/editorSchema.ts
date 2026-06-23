import { Schema, type MarkSpec, type NodeSpec } from 'prosemirror-model';
import { sanitizeEditorHref } from './editorSecurity';

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
      }
    },
    content: 'text*',
    group: 'block',
    parseDOM: [
      {
        tag: 'p',
        getAttrs(node) {
          return { style: node.getAttribute('data-style') || 'body' };
        }
      }
    ],
    toDOM(node) {
      return ['p', { 'data-style': node.attrs.style }, 0];
    }
  },
  heading: {
    attrs: {
      level: { default: 1 }
    },
    content: 'text*',
    defining: true,
    group: 'block',
    parseDOM: [
      { tag: 'h1', attrs: { level: 1 } },
      { tag: 'h2', attrs: { level: 2 } },
      { tag: 'h3', attrs: { level: 3 } },
      { tag: 'h4', attrs: { level: 4 } },
      { tag: 'h5', attrs: { level: 5 } },
      { tag: 'h6', attrs: { level: 6 } }
    ],
    toDOM(node) {
      return [`h${node.attrs.level}`, 0];
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
  }
};

export const supportedSchema = new Schema({
  nodes,
  marks
});

export const supportedBlockTypes = ['paragraph', 'heading'] as const;
export const supportedMarkTypes = [
  'bold',
  'italic',
  'underline',
  'strikethrough',
  'superscript',
  'subscript',
  'link'
] as const;
