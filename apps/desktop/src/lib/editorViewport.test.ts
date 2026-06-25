import { describe, expect, it } from 'vitest';
import { clampEditorZoom, editorViewportStyle, fitWidthZoomPercent } from './editorViewport';
import type { PageSetup } from './documentProjection';

const a4Page: PageSetup = {
  width_mm: 210,
  height_mm: 297,
  margin_top_mm: 20,
  margin_right_mm: 20,
  margin_bottom_mm: 20,
  margin_left_mm: 20
};

describe('editor viewport controls', () => {
  it('clamps custom zoom to the supported lightweight range', () => {
    expect(clampEditorZoom(10)).toBe(75);
    expect(clampEditorZoom(125.4)).toBe(125);
    expect(clampEditorZoom(999)).toBe(175);
    expect(clampEditorZoom(Number.NaN)).toBe(100);
  });

  it('builds page viewport CSS variables without document metadata', () => {
    const style = editorViewportStyle(a4Page, 125);

    expect(style).toContain('--editor-zoom: 1.25');
    expect(style).toContain('--editor-font-size: 1.25rem');
    expect(style).toContain('--page-fit-width: 210mm');
    expect(style).toContain('--page-width: 262.5mm');
    expect(style).toContain('--margin-left: 25mm');
    expect(style).not.toContain('document');
    expect(style).not.toContain('odt');
  });

  it('allows fit-width render styles below the custom zoom minimum', () => {
    const style = editorViewportStyle(a4Page, 50);

    expect(style).toContain('--editor-zoom: 0.5');
    expect(style).toContain('--editor-font-size: 0.5rem');
    expect(style).toContain('--page-width: 105mm');
  });

  it('calculates fit width zoom from the local viewport width', () => {
    expect(fitWidthZoomPercent(a4Page, 794)).toBe(100);
    expect(fitWidthZoomPercent(a4Page, 397)).toBe(50);
    expect(fitWidthZoomPercent(a4Page, 1600)).toBe(175);
    expect(fitWidthZoomPercent(a4Page, 0)).toBe(100);
  });

  it('normalizes invalid page numbers before emitting CSS units', () => {
    const style = editorViewportStyle(
      {
        width_mm: 0,
        height_mm: Number.NaN,
        margin_top_mm: -1,
        margin_right_mm: 0,
        margin_bottom_mm: Number.POSITIVE_INFINITY,
        margin_left_mm: 10
      },
      100
    );

    expect(style).toContain('--page-fit-width: 1mm');
    expect(style).toContain('--page-fit-height: 1mm');
    expect(style).toContain('--margin-top: 0mm');
    expect(style).toContain('--margin-right: 0mm');
    expect(style).toContain('--margin-left: 10mm');
  });
});
