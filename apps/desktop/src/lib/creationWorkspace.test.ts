import { describe, expect, it } from 'vitest';
import {
  buildTemplateCardMetadata,
  classifyTemplatePreviewKind,
  type CreationTemplateSummary
} from './creationWorkspace';

const baseTemplate: CreationTemplateSummary = {
  id: 'project-report',
  name: 'Project Report',
  source: 'Generated',
  description: 'NGO or project update with milestone table'
};

describe('creation workspace helpers', () => {
  it('builds deterministic bounded preview metadata from template summary fields', () => {
    const first = buildTemplateCardMetadata(baseTemplate);
    const second = buildTemplateCardMetadata({ ...baseTemplate });

    expect(first).toEqual(second);
    expect(first.previewKind).toBe('report');
    expect(first.titleWidthPercent).toBeGreaterThanOrEqual(42);
    expect(first.titleWidthPercent).toBeLessThanOrEqual(70);
    expect(first.lineWidthsPercent).toHaveLength(6);
    expect(first.lineWidthsPercent.every((width) => width >= 34 && width <= 92)).toBe(true);
  });

  it('classifies common starter templates without backend document inspection', () => {
    expect(classifyTemplatePreviewKind({ ...baseTemplate, id: 'invoice', name: 'Invoice' })).toBe(
      'invoice'
    );
    expect(classifyTemplatePreviewKind({ ...baseTemplate, id: 'resume', name: 'CV / Resume' })).toBe(
      'resume'
    );
    expect(
      classifyTemplatePreviewKind({
        ...baseTemplate,
        id: 'meeting-minutes',
        name: 'Meeting Minutes',
        description: 'Agenda, decisions, and action-item tracker'
      })
    ).toBe('minutes');
    expect(classifyTemplatePreviewKind({ ...baseTemplate, id: 'flyer', name: 'Flyer One-Pager' })).toBe(
      'flyer'
    );
  });

  it('does not echo raw template summary strings in derived metadata', () => {
    const sensitiveTemplate: CreationTemplateSummary = {
      id: 'user-template',
      name: 'SENSITIVE_TEMPLATE_NAME_TOKEN',
      source: 'SENSITIVE_TEMPLATE_SOURCE_TOKEN',
      description: 'SENSITIVE_TEMPLATE_DESCRIPTION_TOKEN'
    };

    const metadata = buildTemplateCardMetadata(sensitiveTemplate);
    const serialized = JSON.stringify(metadata);

    expect(serialized).not.toContain('SENSITIVE_TEMPLATE_NAME_TOKEN');
    expect(serialized).not.toContain('SENSITIVE_TEMPLATE_SOURCE_TOKEN');
    expect(serialized).not.toContain('SENSITIVE_TEMPLATE_DESCRIPTION_TOKEN');
    expect(metadata.previewKind).toBe('custom');
  });
});
