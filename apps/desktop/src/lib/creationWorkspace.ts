export interface CreationTemplateSummary {
  id: string;
  name: string;
  source: string;
  description: string;
}

export type TemplatePreviewKind =
  | 'blank'
  | 'letter'
  | 'report'
  | 'resume'
  | 'minutes'
  | 'memo'
  | 'invoice'
  | 'flyer'
  | 'table'
  | 'custom';

export type TemplatePreviewTone = 'teal' | 'blue' | 'plum' | 'gold' | 'red' | 'slate';

export interface TemplateCardMetadata {
  previewKind: TemplatePreviewKind;
  tone: TemplatePreviewTone;
  titleWidthPercent: number;
  lineWidthsPercent: number[];
}

const previewTones: TemplatePreviewTone[] = ['teal', 'blue', 'plum', 'gold', 'red', 'slate'];

const kindKeywords: Array<[TemplatePreviewKind, string[]]> = [
  ['invoice', ['invoice', 'line item', 'billing']],
  ['resume', ['resume', 'cv', 'curriculum']],
  ['minutes', ['meeting', 'minutes', 'agenda', 'decisions', 'action-item']],
  ['flyer', ['flyer', 'announcement', 'one-pager']],
  ['letter', ['letter', 'correspondence']],
  ['memo', ['memo']],
  ['report', ['report', 'milestone', 'sections']],
  ['table', ['table', 'tracker']],
  ['blank', ['blank', 'untitled']]
];

const lineCounts: Record<TemplatePreviewKind, number> = {
  blank: 3,
  letter: 5,
  report: 6,
  resume: 6,
  minutes: 5,
  memo: 4,
  invoice: 5,
  flyer: 4,
  table: 4,
  custom: 5
};

export function buildTemplateCardMetadata(template: CreationTemplateSummary): TemplateCardMetadata {
  const fingerprint = stableTemplateFingerprint(template);
  const previewKind = classifyTemplatePreviewKind(template);
  return {
    previewKind,
    tone: previewTones[fingerprint % previewTones.length],
    titleWidthPercent: boundedPercent(42 + (fingerprint % 29), 42, 70),
    lineWidthsPercent: buildPreviewLineWidths(fingerprint, previewKind)
  };
}

export function classifyTemplatePreviewKind(template: CreationTemplateSummary): TemplatePreviewKind {
  const searchableText = normalizeTemplateText(
    `${template.id} ${template.name} ${template.source} ${template.description}`
  );
  return kindKeywords.find(([, keywords]) => keywords.some((keyword) => searchableText.includes(keyword)))?.[0] ?? 'custom';
}

function buildPreviewLineWidths(fingerprint: number, previewKind: TemplatePreviewKind): number[] {
  const count = lineCounts[previewKind];
  return Array.from({ length: count }, (_, index) => {
    const shift = (index % 4) * 6;
    const width = 44 + ((fingerprint >>> shift) % 47);
    return boundedPercent(width - (index % 3) * 4, 34, 92);
  });
}

function stableTemplateFingerprint(template: CreationTemplateSummary): number {
  const text = normalizeTemplateText(`${template.id}|${template.name}|${template.source}|${template.description}`);
  let hash = 2166136261;
  for (const char of text) {
    hash ^= char.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return hash >>> 0;
}

function normalizeTemplateText(value: string): string {
  return value.trim().toLowerCase().replace(/\s+/g, ' ');
}

function boundedPercent(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}
