import { describe, expect, it } from 'vitest';

import { buildPdfExportOptions, parsePositiveIntegerInput } from './exportWorkflow';

describe('PDF export workflow helpers', () => {
  it('returns empty options for all pages', () => {
    expect(buildPdfExportOptions('all', '', '')).toEqual({});
    expect(buildPdfExportOptions('all', '0', 'not-a-page')).toEqual({});
  });

  it('builds bounded range options from positive integer inputs', () => {
    expect(buildPdfExportOptions('range', '1', '3')).toEqual({ pageStart: 1, pageEnd: 3 });
    expect(buildPdfExportOptions('range', ' 4 ', '4')).toEqual({ pageStart: 4, pageEnd: 4 });
  });

  it('rejects invalid page range options', () => {
    expect(buildPdfExportOptions('range', '', '2')).toBeNull();
    expect(buildPdfExportOptions('range', '0', '2')).toBeNull();
    expect(buildPdfExportOptions('range', '2', '1')).toBeNull();
    expect(buildPdfExportOptions('range', '1.5', '2')).toBeNull();
    expect(buildPdfExportOptions('range', '1', 'two')).toBeNull();
    expect(buildPdfExportOptions('range', '9007199254740992', '9007199254740992')).toBeNull();
  });

  it('parses positive integers only', () => {
    expect(parsePositiveIntegerInput('12')).toBe(12);
    expect(parsePositiveIntegerInput(' 7 ')).toBe(7);
    expect(parsePositiveIntegerInput('0')).toBeNull();
    expect(parsePositiveIntegerInput('-1')).toBeNull();
    expect(parsePositiveIntegerInput('1.0')).toBeNull();
    expect(parsePositiveIntegerInput('9007199254740992')).toBeNull();
  });
});
