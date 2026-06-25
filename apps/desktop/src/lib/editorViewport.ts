import type { PageSetup } from './documentProjection';

export type EditorViewMode = 'draft' | 'page-layout';
export type EditorZoomChoice = 'fit-width' | '100' | 'custom';

export const minEditorZoom = 75;
export const minFitWidthZoom = 35;
export const maxEditorZoom = 175;
export const editorZoomStep = 5;
const cssPixelsPerMillimeter = 96 / 25.4;

export function clampEditorZoom(value: number): number {
  return clampZoom(value, minEditorZoom, maxEditorZoom);
}

export function fitWidthZoomPercent(page: PageSetup, viewportWidthPx: number, reservedWidthPx = 0): number {
  if (!Number.isFinite(viewportWidthPx) || viewportWidthPx <= 0) {
    return 100;
  }
  const pageWidthPx = safeDimensionMillimeters(page.width_mm) * cssPixelsPerMillimeter;
  const availableWidthPx = Math.max(1, viewportWidthPx - Math.max(0, reservedWidthPx));
  return clampZoom(Math.floor((availableWidthPx / pageWidthPx) * 100), minFitWidthZoom, maxEditorZoom);
}

function clampZoom(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) {
    return 100;
  }
  return Math.min(max, Math.max(min, Math.round(value)));
}

export function editorViewportStyle(page: PageSetup, zoomPercent: number): string {
  const zoom = clampZoom(zoomPercent, minFitWidthZoom, maxEditorZoom) / 100;
  const width = safeDimensionMillimeters(page.width_mm);
  const height = safeDimensionMillimeters(page.height_mm);
  const marginTop = safeMarginMillimeters(page.margin_top_mm);
  const marginRight = safeMarginMillimeters(page.margin_right_mm);
  const marginBottom = safeMarginMillimeters(page.margin_bottom_mm);
  const marginLeft = safeMarginMillimeters(page.margin_left_mm);

  return [
    `--editor-zoom: ${zoom}`,
    `--editor-font-size: ${zoom}rem`,
    `--page-fit-width: ${width}mm`,
    `--page-fit-height: ${height}mm`,
    `--page-width: ${width * zoom}mm`,
    `--page-height: ${height * zoom}mm`,
    `--margin-top: ${marginTop * zoom}mm`,
    `--margin-right: ${marginRight * zoom}mm`,
    `--margin-bottom: ${marginBottom * zoom}mm`,
    `--margin-left: ${marginLeft * zoom}mm`
  ].join('; ');
}

function safeDimensionMillimeters(value: number): number {
  return Number.isFinite(value) && value > 0 ? value : 1;
}

function safeMarginMillimeters(value: number): number {
  return Number.isFinite(value) && value >= 0 ? value : 0;
}
