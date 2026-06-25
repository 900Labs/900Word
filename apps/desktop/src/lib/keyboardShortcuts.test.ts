import { describe, expect, it } from 'vitest';
import {
  identifyGlobalShortcut,
  isFormFieldTarget,
  normalizeShortcutLabel,
  shortcutLabel,
  shortcutLabels,
  shortcutPlatformFromNavigator
} from './keyboardShortcuts';

describe('keyboard shortcut labels', () => {
  it('normalizes primary shortcuts for macOS and non-mac platforms', () => {
    expect(normalizeShortcutLabel({ primary: true, shift: true, key: 'S' }, 'macos')).toBe(
      'Cmd+Shift+S'
    );
    expect(normalizeShortcutLabel({ primary: true, shift: true, key: 'S' }, 'windows')).toBe(
      'Ctrl+Shift+S'
    );
  });

  it('exposes command labels including alternate redo forms', () => {
    expect(shortcutLabel('exportPdf')).toBe('Cmd+Shift+E');
    expect(shortcutLabels('redo')).toEqual(['Cmd+Shift+Z', 'Cmd+Y']);
    expect(shortcutLabels('replace', 'windows')).toEqual(['Ctrl+H']);
    expect(shortcutLabel('heading1')).toBe('Cmd+Option+1');
    expect(shortcutLabel('heading1', 'windows')).toBe('Ctrl+Alt+1');
  });

  it('detects the current shortcut label platform from browser metadata', () => {
    expect(shortcutPlatformFromNavigator({ platform: 'MacIntel', userAgent: '' })).toBe('macos');
    expect(shortcutPlatformFromNavigator({ platform: 'Win32', userAgent: '' })).toBe('windows');
    expect(shortcutPlatformFromNavigator({ platform: 'Linux x86_64', userAgent: '' })).toBe('linux');
    expect(shortcutPlatformFromNavigator({ platform: '', userAgent: 'Unknown' })).toBe('other');
  });
});

describe('identifyGlobalShortcut', () => {
  it('matches existing formatting and file commands with Cmd or Ctrl', () => {
    expect(identifyGlobalShortcut({ key: 'b', metaKey: true })).toBe('bold');
    expect(identifyGlobalShortcut({ key: 'B', ctrlKey: true })).toBe('bold');
    expect(identifyGlobalShortcut({ key: 's', metaKey: true, shiftKey: true })).toBe('saveDocumentAs');
    expect(identifyGlobalShortcut({ key: 'p', ctrlKey: true })).toBe('printDocument');
  });

  it('matches redo through Shift+Z and Y', () => {
    expect(identifyGlobalShortcut({ key: 'z', metaKey: true, shiftKey: true })).toBe('redo');
    expect(identifyGlobalShortcut({ key: 'y', ctrlKey: true })).toBe('redo');
  });

  it('uses physical digit keys for headings and list shortcuts', () => {
    expect(identifyGlobalShortcut({ key: 'Unidentified', code: 'Digit1', metaKey: true, altKey: true })).toBe(
      'heading1'
    );
    expect(identifyGlobalShortcut({ key: '1', code: 'Digit1', altKey: true })).toBeNull();
    expect(identifyGlobalShortcut({ key: '&', code: 'Digit7', metaKey: true, shiftKey: true })).toBe(
      'numberedList'
    );
    expect(identifyGlobalShortcut({ key: '*', code: 'Digit8', ctrlKey: true, shiftKey: true })).toBe(
      'bulletList'
    );
  });

  it('matches replace and export PDF shortcuts', () => {
    expect(identifyGlobalShortcut({ key: 'Unidentified', code: 'KeyF', metaKey: true, altKey: true })).toBe(
      'replace'
    );
    expect(identifyGlobalShortcut({ key: 'h', ctrlKey: true })).toBe('replace');
    expect(identifyGlobalShortcut({ key: 'e', metaKey: true, shiftKey: true })).toBe('exportPdf');
  });

  it('guards editor-destructive shortcuts while typing in form fields', () => {
    const input = { tagName: 'INPUT' };
    expect(isFormFieldTarget(input)).toBe(true);
    expect(identifyGlobalShortcut({ key: 'b', metaKey: true, target: input })).toBeNull();
    expect(identifyGlobalShortcut({ key: 'z', metaKey: true, target: input })).toBeNull();
    expect(identifyGlobalShortcut({ key: 'y', ctrlKey: true, target: input })).toBeNull();
    expect(identifyGlobalShortcut({ key: 'n', metaKey: true, target: input })).toBeNull();
    expect(identifyGlobalShortcut({ key: 'o', metaKey: true, target: input })).toBeNull();
    expect(identifyGlobalShortcut({ key: 's', metaKey: true, target: input })).toBe('saveDocument');
    expect(identifyGlobalShortcut({ key: 's', metaKey: true, shiftKey: true, target: input })).toBe(
      'saveDocumentAs'
    );
    expect(identifyGlobalShortcut({ key: 'f', metaKey: true, target: input })).toBe('find');
  });

  it('guards shortcuts in textareas and selects', () => {
    expect(identifyGlobalShortcut({ key: 'k', metaKey: true, target: { tagName: 'TEXTAREA' } })).toBeNull();
    expect(
      identifyGlobalShortcut({ key: '1', code: 'Digit1', altKey: true, target: { tagName: 'SELECT' } })
    ).toBeNull();
  });
});
