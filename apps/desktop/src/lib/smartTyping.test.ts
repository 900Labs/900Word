import { describe, expect, it } from 'vitest';
import { defaultSmartTypingSettings, smartTypingReplacement, type SmartTypingSettings } from './smartTyping';

function settings(overrides: Partial<SmartTypingSettings>): SmartTypingSettings {
  return { ...defaultSmartTypingSettings(), ...overrides };
}

describe('smartTypingReplacement', () => {
  it('capitalizes a first typed letter at paragraph start and after sentence punctuation', () => {
    const enabled = settings({ capitalize_sentences: true });

    expect(smartTypingReplacement({ settings: enabled, text: 'h', textBefore: '' })).toEqual({
      insertText: 'H',
      replaceBefore: 0
    });
    expect(smartTypingReplacement({ settings: enabled, text: 'w', textBefore: 'Hello. ' })).toEqual({
      insertText: 'W',
      replaceBefore: 0
    });
  });

  it('converts straight quotes to directional smart quotes', () => {
    const enabled = settings({ smart_quotes: true });

    expect(smartTypingReplacement({ settings: enabled, text: '"', textBefore: '' })?.insertText).toBe('“');
    expect(smartTypingReplacement({ settings: enabled, text: '"', textBefore: 'Hello' })?.insertText).toBe('”');
    expect(smartTypingReplacement({ settings: enabled, text: "'", textBefore: 'don' })?.insertText).toBe('’');
  });

  it('converts a second hyphen to an em dash outside URL tokens', () => {
    const enabled = settings({ smart_dashes: true });

    expect(smartTypingReplacement({ settings: enabled, text: '-', textBefore: '-' })).toEqual({
      insertText: '—',
      replaceBefore: 1
    });
    expect(smartTypingReplacement({ settings: enabled, text: '-', textBefore: 'https://example.invalid/path-' })).toBeUndefined();
    expect(smartTypingReplacement({ settings: enabled, text: '-', textBefore: 'example.invalid/path-' })).toBeUndefined();
  });

  it('replaces only allowlisted typos on a word boundary', () => {
    const enabled = settings({ typo_replacements: true });

    expect(smartTypingReplacement({ settings: enabled, text: ' ', textBefore: 'teh' })).toEqual({
      insertText: 'the ',
      replaceBefore: 3
    });
    expect(smartTypingReplacement({ settings: enabled, text: ' ', textBefore: 'Teh' })).toEqual({
      insertText: 'The ',
      replaceBefore: 3
    });
    expect(smartTypingReplacement({ settings: enabled, text: ' ', textBefore: 'unknown' })).toBeUndefined();
    expect(smartTypingReplacement({ settings: enabled, text: ' ', textBefore: 'https://example.invalid/teh' })).toBeUndefined();
    expect(smartTypingReplacement({ settings: enabled, text: ' ', textBefore: 'example.invalid/teh' })).toBeUndefined();
  });

  it('detects simple list triggers at the start of an otherwise empty paragraph', () => {
    const enabled = settings({ list_triggers: true });

    expect(smartTypingReplacement({ settings: enabled, text: ' ', textBefore: '-' })).toEqual({
      insertText: '',
      replaceBefore: 1,
      listTrigger: 'bullet_list'
    });
    expect(smartTypingReplacement({ settings: enabled, text: ' ', textBefore: '1.' })).toEqual({
      insertText: '',
      replaceBefore: 2,
      listTrigger: 'ordered_list'
    });
  });

  it('stays inactive when features are disabled', () => {
    expect(
      smartTypingReplacement({ settings: defaultSmartTypingSettings(), text: '-', textBefore: '-' })
    ).toBeUndefined();
  });
});
