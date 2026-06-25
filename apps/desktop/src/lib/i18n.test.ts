import { describe, expect, it } from 'vitest';
import { localeDirection, translate } from './i18n';

describe('i18n', () => {
  it('falls back to English for unknown locales', () => {
    expect(translate('unknown', 'save')).toBe('Save');
  });

  it('uses initial Spanish translations', () => {
    expect(translate('es-ES', 'save')).toBe('Guardar');
  });

  it('interpolates status values', () => {
    expect(translate('en-US', 'matchCount', { current: 2, total: 5 })).toBe('2/5');
  });

  it('keeps oversized image guidance generic', () => {
    expect(translate('en-US', 'imageTooLarge')).toBe(
      'Image is too large. Compress or resize it before inserting.'
    );
  });

  it('labels smart typing settings', () => {
    expect(translate('en-US', 'smartTyping')).toBe('Smart typing');
    expect(translate('es-ES', 'smartQuotes')).toBe('Usar comillas tipograficas');
  });

  it('labels accessibility and low-resource settings', () => {
    expect(translate('en-US', 'largeToolbar')).toBe('Large toolbar and controls');
    expect(translate('en-US', 'reducedMotion')).toBe('Reduced motion');
    expect(translate('en-US', 'lowResourceMode')).toBe('Low-resource mode');
    expect(translate('es-ES', 'accessibilityAndPerformance')).toBe('Accesibilidad y rendimiento');
  });

  it('labels expanded stats estimates clearly', () => {
    expect(translate('en-US', 'estimatedPages')).toBe('Estimated pages');
    expect(translate('en-US', 'statsEstimateNote')).toContain('estimates');
  });

  it('provides an RTL smoke locale', () => {
    expect(localeDirection('ar')).toBe('rtl');
  });
});
