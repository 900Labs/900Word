export interface Inline {
  text: string;
}

export interface ParagraphBlock {
  type: 'Paragraph';
  value: {
    inlines: Inline[];
  };
}

export interface HeadingBlock {
  type: 'Heading';
  value: {
    level: number;
    inlines: Inline[];
  };
}

export type Block = ParagraphBlock | HeadingBlock | { type: string; value?: unknown };

export interface DocumentState {
  meta: {
    title: string;
  };
  sections: Array<{
    blocks: Block[];
  }>;
}

export function documentToText(document: DocumentState): string {
  const blocks: Block[] = [];
  for (const section of document.sections) {
    blocks.push(...section.blocks);
  }
  return blocks.map(blockToText).filter(Boolean).join('\n');
}

function blockToText(block: Block): string {
  if (hasInlineContent(block)) {
    return block.value.inlines.map((inline) => inline.text).join('');
  }
  return '';
}

function hasInlineContent(block: Block): block is ParagraphBlock | HeadingBlock {
  return block.type === 'Paragraph' || block.type === 'Heading';
}
