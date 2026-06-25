export interface SmartTypingSettings {
  capitalize_sentences: boolean;
  smart_quotes: boolean;
  smart_dashes: boolean;
  typo_replacements: boolean;
  list_triggers: boolean;
}

export interface SmartTypingInput {
  settings: SmartTypingSettings;
  text: string;
  textBefore: string;
  textAfter?: string;
}

export interface SmartTypingReplacement {
  insertText: string;
  replaceBefore: number;
  listTrigger?: 'bullet_list' | 'ordered_list';
}

const TYPO_REPLACEMENTS: Record<string, string> = {
  adress: 'address',
  becuase: 'because',
  recieve: 'receive',
  seperate: 'separate',
  teh: 'the',
  untill: 'until',
  wich: 'which'
};

export function defaultSmartTypingSettings(): SmartTypingSettings {
  return {
    capitalize_sentences: false,
    smart_quotes: false,
    smart_dashes: false,
    typo_replacements: false,
    list_triggers: false
  };
}

export function smartTypingEnabled(settings: SmartTypingSettings | undefined): settings is SmartTypingSettings {
  return Boolean(
    settings &&
      (settings.capitalize_sentences ||
        settings.smart_quotes ||
        settings.smart_dashes ||
        settings.typo_replacements ||
        settings.list_triggers)
  );
}

export function smartTypingReplacement(input: SmartTypingInput): SmartTypingReplacement | undefined {
  const { settings, text, textBefore, textAfter = '' } = input;
  if (!smartTypingEnabled(settings) || text.length === 0 || text.length > 2) {
    return undefined;
  }

  if (settings.list_triggers && text === ' ' && textAfter.length === 0) {
    if (textBefore === '-') {
      return { insertText: '', replaceBefore: 1, listTrigger: 'bullet_list' };
    }
    if (textBefore === '1.') {
      return { insertText: '', replaceBefore: 2, listTrigger: 'ordered_list' };
    }
  }

  if (isProbablyUrlContext(textBefore)) {
    return undefined;
  }

  if (settings.typo_replacements && isWhitespace(text)) {
    const corrected = typoReplacement(textBefore);
    if (corrected) {
      return {
        insertText: `${corrected.replacement}${text}`,
        replaceBefore: corrected.original.length
      };
    }
  }

  if (settings.smart_dashes && text === '-' && textBefore.endsWith('-')) {
    return { insertText: '—', replaceBefore: 1 };
  }

  if (settings.smart_quotes && (text === '"' || text === "'")) {
    return { insertText: smartQuoteFor(text, textBefore, textAfter), replaceBefore: 0 };
  }

  if (settings.capitalize_sentences && isSingleLowercaseLetter(text) && isSentenceStart(textBefore)) {
    return { insertText: text.toLocaleUpperCase('en-US'), replaceBefore: 0 };
  }

  return undefined;
}

function typoReplacement(textBefore: string): { original: string; replacement: string } | undefined {
  const match = /([A-Za-z]{2,32})$/.exec(textBefore);
  if (!match) {
    return undefined;
  }
  const original = match[1];
  const replacement = TYPO_REPLACEMENTS[original.toLocaleLowerCase('en-US')];
  if (!replacement) {
    return undefined;
  }
  return { original, replacement: preserveWordCase(original, replacement) };
}

function preserveWordCase(original: string, replacement: string): string {
  if (original === original.toLocaleUpperCase('en-US')) {
    return replacement.toLocaleUpperCase('en-US');
  }
  if (original[0] === original[0].toLocaleUpperCase('en-US')) {
    return `${replacement[0].toLocaleUpperCase('en-US')}${replacement.slice(1)}`;
  }
  return replacement;
}

function smartQuoteFor(text: '"' | "'", textBefore: string, textAfter: string): string {
  const opening = quoteShouldOpen(textBefore, textAfter);
  if (text === '"') {
    return opening ? '“' : '”';
  }
  return opening ? '‘' : '’';
}

function quoteShouldOpen(textBefore: string, textAfter: string): boolean {
  if (textAfter.length > 0 && /^[A-Za-z0-9]/.test(textAfter)) {
    return true;
  }
  if (textBefore.length === 0) {
    return true;
  }
  return /[\s([{\u201c\u2018]$/.test(textBefore);
}

function isSentenceStart(textBefore: string): boolean {
  if (textBefore.trim().length === 0) {
    return true;
  }
  return /[.!?]["')\]\u201d\u2019]*\s+$/.test(textBefore);
}

function isSingleLowercaseLetter(text: string): boolean {
  return /^[a-z]$/.test(text);
}

function isWhitespace(text: string): boolean {
  return /^[\t\n\r ]+$/.test(text);
}

function isProbablyUrlContext(textBefore: string): boolean {
  const token = textBefore.split(/\s+/).at(-1) ?? '';
  return (
    /^(?:[a-z][a-z0-9+.-]*:\/\/|www\.)/i.test(token) ||
    /(?:^|[^\w-])(?:[a-z0-9-]+\.)+[a-z]{2,}(?::\d+)?(?:[/?#]|$)/i.test(token) ||
    token.includes('@')
  );
}
