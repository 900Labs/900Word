export interface PdfExportOptions {
  pageStart?: number;
  pageEnd?: number;
}

export type PdfPageRangeMode = 'all' | 'range';

export function buildPdfExportOptions(
  mode: PdfPageRangeMode,
  startInput: string,
  endInput: string
): PdfExportOptions | null {
  if (mode === 'all') {
    return {};
  }

  const pageStart = parsePositiveIntegerInput(startInput);
  const pageEnd = parsePositiveIntegerInput(endInput);
  if (pageStart === null || pageEnd === null || pageEnd < pageStart) {
    return null;
  }

  return { pageStart, pageEnd };
}

export function parsePositiveIntegerInput(value: string): number | null {
  const trimmed = value.trim();
  if (!/^[1-9]\d*$/.test(trimmed)) {
    return null;
  }

  const parsed = Number.parseInt(trimmed, 10);
  return Number.isSafeInteger(parsed) ? parsed : null;
}
