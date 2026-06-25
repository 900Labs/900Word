export type ShortcutPlatform = 'macos' | 'windows' | 'linux' | 'other';

export type GlobalShortcutCommand =
  | 'newDocument'
  | 'openDocument'
  | 'saveDocument'
  | 'saveDocumentAs'
  | 'printDocument'
  | 'undo'
  | 'redo'
  | 'bold'
  | 'italic'
  | 'underline'
  | 'find'
  | 'replace'
  | 'heading1'
  | 'heading2'
  | 'heading3'
  | 'insertLink'
  | 'bulletList'
  | 'numberedList'
  | 'increaseIndent'
  | 'decreaseIndent'
  | 'exportPdf';

export interface KeyboardShortcutEventLike {
  key: string;
  code?: string;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
  target?: unknown;
}

export interface ShortcutLabelDescriptor {
  primary?: boolean;
  shift?: boolean;
  alt?: boolean;
  key: string;
}

const allowedInFormFields = new Set<GlobalShortcutCommand>([
  'saveDocument',
  'saveDocumentAs',
  'printDocument',
  'find'
]);

const commandLabelDescriptors: Record<Exclude<GlobalShortcutCommand, 'redo' | 'replace'>, ShortcutLabelDescriptor> = {
  newDocument: { primary: true, key: 'N' },
  openDocument: { primary: true, key: 'O' },
  saveDocument: { primary: true, key: 'S' },
  saveDocumentAs: { primary: true, shift: true, key: 'S' },
  printDocument: { primary: true, key: 'P' },
  undo: { primary: true, key: 'Z' },
  bold: { primary: true, key: 'B' },
  italic: { primary: true, key: 'I' },
  underline: { primary: true, key: 'U' },
  find: { primary: true, key: 'F' },
  heading1: { primary: true, alt: true, key: '1' },
  heading2: { primary: true, alt: true, key: '2' },
  heading3: { primary: true, alt: true, key: '3' },
  insertLink: { primary: true, key: 'K' },
  bulletList: { primary: true, shift: true, key: '8' },
  numberedList: { primary: true, shift: true, key: '7' },
  increaseIndent: { primary: true, key: ']' },
  decreaseIndent: { primary: true, key: '[' },
  exportPdf: { primary: true, shift: true, key: 'E' }
};

export function normalizeShortcutLabel(
  descriptor: ShortcutLabelDescriptor,
  platform: ShortcutPlatform = 'macos'
): string {
  const parts: string[] = [];
  if (descriptor.primary) {
    parts.push(platform === 'macos' ? 'Cmd' : 'Ctrl');
  }
  if (descriptor.shift) {
    parts.push('Shift');
  }
  if (descriptor.alt) {
    parts.push(platform === 'macos' ? 'Option' : 'Alt');
  }
  parts.push(descriptor.key);
  return parts.join('+');
}

export function shortcutLabels(
  command: GlobalShortcutCommand,
  platform: ShortcutPlatform = 'macos'
): string[] {
  if (command === 'redo') {
    return [
      normalizeShortcutLabel({ primary: true, shift: true, key: 'Z' }, platform),
      normalizeShortcutLabel({ primary: true, key: 'Y' }, platform)
    ];
  }
  if (command === 'replace') {
    return platform === 'macos'
      ? [normalizeShortcutLabel({ primary: true, alt: true, key: 'F' }, platform)]
      : [normalizeShortcutLabel({ primary: true, key: 'H' }, platform)];
  }
  return [normalizeShortcutLabel(commandLabelDescriptors[command], platform)];
}

export function shortcutLabel(command: GlobalShortcutCommand, platform: ShortcutPlatform = 'macos'): string {
  return shortcutLabels(command, platform)[0];
}

export function shortcutPlatformFromNavigator(
  navigatorLike: Pick<Navigator, 'platform' | 'userAgent'> | undefined = globalThis.navigator
): ShortcutPlatform {
  const platform = navigatorLike?.platform?.toLowerCase() ?? '';
  const userAgent = navigatorLike?.userAgent?.toLowerCase() ?? '';
  if (platform.includes('mac') || userAgent.includes('mac os')) {
    return 'macos';
  }
  if (platform.includes('win') || userAgent.includes('windows')) {
    return 'windows';
  }
  if (platform.includes('linux') || userAgent.includes('linux')) {
    return 'linux';
  }
  return 'other';
}

export function identifyGlobalShortcut(event: KeyboardShortcutEventLike): GlobalShortcutCommand | null {
  const command = identifyShortcutCommand(event);
  if (!command) {
    return null;
  }
  if (isFormFieldTarget(event.target) && !allowedInFormFields.has(command)) {
    return null;
  }
  return command;
}

export function isFormFieldTarget(target: unknown): boolean {
  if (!target || typeof target !== 'object' || !('tagName' in target)) {
    return false;
  }
  const tagName = String((target as { tagName?: unknown }).tagName).toUpperCase();
  return tagName === 'INPUT' || tagName === 'TEXTAREA' || tagName === 'SELECT';
}

function identifyShortcutCommand(event: KeyboardShortcutEventLike): GlobalShortcutCommand | null {
  const key = normalizeKey(event.key);
  const primary = hasPrimaryModifier(event);
  const shift = Boolean(event.shiftKey);
  const alt = Boolean(event.altKey);

  if (primary && !alt && !shift && key === 'n') return 'newDocument';
  if (primary && !alt && !shift && key === 'o') return 'openDocument';
  if (primary && !alt && shift && key === 's') return 'saveDocumentAs';
  if (primary && !alt && !shift && key === 's') return 'saveDocument';
  if (primary && !alt && !shift && key === 'p') return 'printDocument';

  if (primary && !alt && shift && key === 'z') return 'redo';
  if (primary && !alt && !shift && key === 'y') return 'redo';
  if (primary && !alt && !shift && key === 'z') return 'undo';

  if (primary && !alt && !shift && key === 'b') return 'bold';
  if (primary && !alt && !shift && key === 'i') return 'italic';
  if (primary && !alt && !shift && key === 'u') return 'underline';

  if (matchesReplaceShortcut(event)) return 'replace';
  if (primary && !alt && !shift && key === 'f') return 'find';

  const headingLevel = headingLevelFromEvent(event);
  if (headingLevel) {
    return `heading${headingLevel}` as GlobalShortcutCommand;
  }

  if (primary && !alt && shift && event.code === 'Digit7') return 'numberedList';
  if (primary && !alt && shift && event.code === 'Digit8') return 'bulletList';
  if (primary && !alt && !shift && (key === '[' || event.code === 'BracketLeft')) return 'decreaseIndent';
  if (primary && !alt && !shift && (key === ']' || event.code === 'BracketRight')) return 'increaseIndent';
  if (primary && !alt && !shift && key === 'k') return 'insertLink';
  if (primary && !alt && shift && key === 'e') return 'exportPdf';

  return null;
}

function matchesReplaceShortcut(event: KeyboardShortcutEventLike): boolean {
  const key = normalizeKey(event.key);
  const shift = Boolean(event.shiftKey);
  const alt = Boolean(event.altKey);
  if (event.metaKey && !event.ctrlKey && alt && !shift && (key === 'f' || event.code === 'KeyF')) {
    return true;
  }
  return Boolean(event.ctrlKey && !event.metaKey && !alt && !shift && key === 'h');
}

function headingLevelFromEvent(event: KeyboardShortcutEventLike): 1 | 2 | 3 | null {
  if (!hasPrimaryModifier(event) || !event.altKey || event.shiftKey) {
    return null;
  }
  if (event.code === 'Digit1') return 1;
  if (event.code === 'Digit2') return 2;
  if (event.code === 'Digit3') return 3;
  if (event.key === '1') return 1;
  if (event.key === '2') return 2;
  if (event.key === '3') return 3;
  return null;
}

function hasPrimaryModifier(event: KeyboardShortcutEventLike): boolean {
  return Boolean(event.metaKey || event.ctrlKey);
}

function normalizeKey(key: string): string {
  return key.length === 1 ? key.toLowerCase() : key;
}
