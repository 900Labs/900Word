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

  it('provides an RTL smoke locale', () => {
    expect(localeDirection('ar')).toBe('rtl');
  });
});
