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

  it('labels settings reset actions in supported locales', () => {
    expect(translate('en-US', 'resetSettings')).toBe('Reset Settings');
    expect(translate('en-US', 'settingsReset')).toBe('Settings reset to defaults');
    expect(translate('es-ES', 'resetSettings')).toBe('Restablecer ajustes');
    expect(translate('es-ES', 'settingsReset')).toBe('Ajustes restablecidos');
    expect(translate('ar', 'resetSettings')).toBe('Reset Settings');
  });

  it('labels the offline dictionary manager in supported locales', () => {
    expect(translate('en-US', 'dictionaryManager')).toBe('Dictionary manager');
    expect(translate('en-US', 'dictionaryOfflineState')).toContain('Offline/local only');
    expect(translate('en-US', 'dictionaryInstallLocal')).toBe('Install local Hunspell dictionary');
    expect(translate('en-US', 'dictionaryAffSelected')).toBe('AFF selected');
    expect(translate('en-US', 'dictionaryDicNotSelected')).toBe('No DIC selected');
    expect(translate('en-US', 'dictionaryInstallUnsupportedFile')).toBe(
      'Dictionary files are unsupported'
    );
    expect(translate('en-US', 'dictionaryRemove')).toBe('Remove');
    expect(translate('en-US', 'dictionaryRemoveSuccess')).toBe('Dictionary removed');
    expect(translate('es-ES', 'dictionarySourceUserFolder')).toBe(
      'Carpeta local de diccionarios del usuario'
    );
    expect(translate('es-ES', 'dictionaryInstallSuccess')).toBe('Diccionario instalado');
    expect(translate('es-ES', 'dictionaryRemoveFailed')).toBe(
      'No se pudo quitar el diccionario'
    );
    expect(translate('ar', 'dictionaryRefresh')).toBe('Refresh');
    expect(translate('ar', 'dictionaryChooseAff')).toBe('Choose AFF');
    expect(translate('ar', 'dictionaryRemove')).toBe('Remove');
    expect(translate('en-US', 'dictionaryUnavailableOption', { languageTag: 'sv-SE' })).toBe(
      'Unavailable (sv-SE)'
    );
  });

  it('labels the personal dictionary manager in supported locales', () => {
    expect(translate('en-US', 'personalDictionary')).toBe('Personal dictionary');
    expect(translate('en-US', 'personalDictionaryWordRemoved', { word: 'qwerty' })).toBe(
      'Removed word from the personal dictionary'
    );
    expect(translate('es-ES', 'personalDictionaryRefresh')).toBe('Actualizar palabras');
    expect(translate('ar', 'personalDictionaryRemove')).toBe('Remove');
  });

  it('labels expanded stats estimates clearly', () => {
    expect(translate('en-US', 'estimatedPages')).toBe('Estimated pages');
    expect(translate('en-US', 'statsEstimateNote')).toContain('estimates');
  });

  it('provides an RTL smoke locale', () => {
    expect(localeDirection('ar')).toBe('rtl');
  });
});
