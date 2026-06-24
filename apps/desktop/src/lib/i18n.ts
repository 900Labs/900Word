export type UiLocaleTag = 'en-US' | 'es-ES' | 'ar';

export type UiStringKey =
  | 'about'
  | 'about900Word'
  | 'all'
  | 'after'
  | 'alignCenter'
  | 'alignJustify'
  | 'alignLeft'
  | 'alignRight'
  | 'applyPage'
  | 'autosave'
  | 'autosaveUpdated'
  | 'before'
  | 'blockFormatting'
  | 'blocks'
  | 'bold'
  | 'bottom'
  | 'bulletList'
  | 'bulletListApplied'
  | 'case'
  | 'characters'
  | 'charactersNoSpaces'
  | 'checkSpelling'
  | 'clear'
  | 'clearFormatting'
  | 'close'
  | 'custom'
  | 'decreaseIndent'
  | 'dictionary'
  | 'documentActions'
  | 'documentFormat'
  | 'documentOpened'
  | 'documentSaved'
  | 'documentSavedAs'
  | 'dirty'
  | 'discard'
  | 'editor'
  | 'editorReadOnly'
  | 'editingToolbar'
  | 'export'
  | 'exportHtml'
  | 'exportHtmlPrepared'
  | 'exportHtmlSaved'
  | 'exportPath'
  | 'exportPathPlaceholder'
  | 'exportPdf'
  | 'exportPdfPrepared'
  | 'exportPdfSaved'
  | 'exportTxt'
  | 'exportTxtPrepared'
  | 'exportTxtSaved'
  | 'file'
  | 'find'
  | 'findAndReplace'
  | 'firstLine'
  | 'firstLineIndent'
  | 'fontControls'
  | 'fontFamily'
  | 'fontSize'
  | 'formattingCleared'
  | 'addToDictionary'
  | 'heading1'
  | 'heading2'
  | 'heading3'
  | 'headingApplied'
  | 'headingUnchanged'
  | 'height'
  | 'highContrast'
  | 'highlightColor'
  | 'html'
  | 'history'
  | 'increaseIndent'
  | 'ignoreAll'
  | 'ignoreOnce'
  | 'italic'
  | 'language'
  | 'left'
  | 'license'
  | 'lineSpacing'
  | 'listLevelChanged'
  | 'lists'
  | 'matchCount'
  | 'matchReplaced'
  | 'matchesReplaced'
  | 'markToggled'
  | 'markUnavailable'
  | 'mainMenu'
  | 'new'
  | 'next'
  | 'noIssues'
  | 'noMatches'
  | 'noSuggestions'
  | 'no'
  | 'numberedList'
  | 'numberedListApplied'
  | 'off'
  | 'offlineDictionaryFallback'
  | 'open'
  | 'openRecent'
  | 'page'
  | 'paragraph'
  | 'paragraphApplied'
  | 'paragraphControls'
  | 'paragraphFormatApplied'
  | 'paragraphs'
  | 'paragraphUnchanged'
  | 'pageSetupUpdated'
  | 'pageSize'
  | 'pdf'
  | 'previous'
  | 'print'
  | 'printFrame'
  | 'printPrepared'
  | 'projection'
  | 'recent'
  | 'recover'
  | 'recovery'
  | 'recoveryDiscarded'
  | 'recoveryOpened'
  | 'redo'
  | 'redoApplied'
  | 'replace'
  | 'replaceAll'
  | 'right'
  | 'readingMinutes'
  | 'readingTime'
  | 'save'
  | 'saveAs'
  | 'saveSettings'
  | 'saved'
  | 'settings'
  | 'settingsUpdated'
  | 'selectionWords'
  | 'spellAddedToDictionary'
  | 'spellIgnoredAll'
  | 'spellIgnoredOnce'
  | 'spellIssueCount'
  | 'spelling'
  | 'spellingSuggestionApplied'
  | 'starting'
  | 'startWriting'
  | 'statusNoSpellingIssues'
  | 'stats'
  | 'styleApplied'
  | 'styleUpdated'
  | 'styleUpdateNeedsParagraph'
  | 'styleUnchanged'
  | 'styles'
  | 'spacingAfter'
  | 'spacingBefore'
  | 'strikethrough'
  | 'subscript'
  | 'superscript'
  | 'telemetry'
  | 'templateLoaded'
  | 'templates'
  | 'textFormatting'
  | 'textColor'
  | 'top'
  | 'txt'
  | 'uiLocale'
  | 'underline'
  | 'undo'
  | 'undoApplied'
  | 'updateStyle'
  | 'updateStyleFromSelection'
  | 'useTemplate'
  | 'userDictionarySuffix'
  | 'version'
  | 'width'
  | 'words'
  | 'workspaceViews'
  | 'documentWorkspace'
  | 'documentStatistics'
  | 'yes';

type UiStrings = Record<UiStringKey, string>;

export interface UiLocaleInfo {
  tag: UiLocaleTag;
  display_name: string;
  direction: 'ltr' | 'rtl';
}

const english: UiStrings = {
  about: 'About',
  about900Word: 'About 900Word',
  all: 'All',
  after: 'After',
  alignCenter: 'Align center',
  alignJustify: 'Justify',
  alignLeft: 'Align left',
  alignRight: 'Align right',
  applyPage: 'Apply Page',
  autosave: 'Autosave',
  autosaveUpdated: 'Recovery draft updated',
  before: 'Before',
  blockFormatting: 'Block formatting',
  blocks: 'Blocks',
  bold: 'Bold',
  bottom: 'Bottom',
  bulletList: 'Bullet list',
  bulletListApplied: 'Bullet list applied',
  case: 'Case',
  characters: 'Characters',
  charactersNoSpaces: 'Characters without spaces',
  checkSpelling: 'Check Spelling',
  clear: 'Clear',
  clearFormatting: 'Clear formatting',
  close: 'Close',
  custom: 'Custom',
  decreaseIndent: 'Decrease indent',
  dictionary: 'Dictionary',
  documentActions: 'Document actions',
  documentFormat: 'Document format',
  documentOpened: 'Document opened',
  documentSaved: 'Document saved',
  documentSavedAs: 'Document saved as',
  dirty: 'Dirty',
  discard: 'Discard',
  editor: 'Editor',
  editorReadOnly: 'Editor is read-only',
  editingToolbar: 'Editing toolbar',
  export: 'Export',
  exportHtml: 'Export HTML',
  exportHtmlPrepared: 'HTML export prepared ({characters} characters)',
  exportHtmlSaved: 'HTML export written ({bytes} bytes)',
  exportPath: 'Export path',
  exportPathPlaceholder: 'Export .txt, .html, or .pdf path',
  exportPdf: 'Export PDF',
  exportPdfPrepared: 'PDF export prepared ({bytes} bytes)',
  exportPdfSaved: 'PDF export written ({bytes} bytes)',
  exportTxt: 'Export TXT',
  exportTxtPrepared: 'TXT export prepared ({characters} characters)',
  exportTxtSaved: 'TXT export written ({bytes} bytes)',
  file: 'File',
  find: 'Find',
  findAndReplace: 'Find and replace',
  firstLine: 'First',
  firstLineIndent: 'First-line indent',
  fontControls: 'Font controls',
  fontFamily: 'Font family',
  fontSize: 'Font size',
  formattingCleared: 'Formatting cleared',
  addToDictionary: 'Add',
  heading1: 'H1',
  heading2: 'H2',
  heading3: 'H3',
  headingApplied: 'Heading {level} applied',
  headingUnchanged: 'Heading {level} unchanged',
  height: 'Height',
  highContrast: 'High contrast',
  highlightColor: 'Highlight color',
  html: 'HTML',
  history: 'History',
  increaseIndent: 'Increase indent',
  ignoreAll: 'Ignore all',
  ignoreOnce: 'Ignore once',
  italic: 'Italic',
  language: 'Language',
  left: 'Left',
  license: 'License',
  lineSpacing: 'Line spacing',
  listLevelChanged: 'List level changed',
  lists: 'Lists',
  matchCount: '{current}/{total}',
  matchReplaced: 'Match replaced',
  matchesReplaced: '{count} match(es) replaced',
  markToggled: '{label} toggled',
  markUnavailable: '{label} unavailable',
  mainMenu: 'Main menu',
  new: 'New',
  next: 'Next',
  no: 'No',
  noIssues: 'No issues.',
  noMatches: 'No matches',
  noSuggestions: 'No suggestions',
  numberedList: 'Numbered list',
  numberedListApplied: 'Numbered list applied',
  off: 'Off',
  offlineDictionaryFallback: 'Dictionary fallback used',
  open: 'Open',
  openRecent: 'Open',
  page: 'Page',
  paragraph: 'P',
  paragraphApplied: 'Paragraph applied',
  paragraphControls: 'Paragraph controls',
  paragraphFormatApplied: '{label} applied',
  paragraphUnchanged: 'Paragraph unchanged',
  paragraphs: 'Paragraphs',
  pageSetupUpdated: 'Page setup updated',
  pageSize: 'Page size',
  pdf: 'PDF',
  previous: 'Prev',
  print: 'Print',
  printFrame: 'Print document',
  printPrepared: 'Print view prepared',
  projection: 'Projection',
  recent: 'Recent',
  recover: 'Open',
  recovery: 'Recovery',
  recoveryDiscarded: 'Recovery draft discarded',
  recoveryOpened: 'Recovery draft opened',
  redo: 'Redo',
  redoApplied: 'Redo applied',
  replace: 'Replace',
  replaceAll: 'All',
  readingMinutes: '{count} min',
  readingTime: 'Reading time',
  right: 'Right',
  save: 'Save',
  saveAs: 'Save As...',
  saveSettings: 'Save Settings',
  saved: 'Saved',
  settings: 'Settings',
  settingsUpdated: 'Settings updated',
  selectionWords: 'Selection words',
  spellAddedToDictionary: 'Added "{word}" to the personal dictionary',
  spellIgnoredAll: 'Word ignored for this session',
  spellIgnoredOnce: 'Spelling issue ignored once',
  spellIssueCount: '{count} spelling issue(s)',
  spelling: 'Spelling',
  spellingSuggestionApplied: 'Spelling suggestion applied',
  starting: 'Starting...',
  startWriting: 'Start writing...',
  statusNoSpellingIssues: 'No spelling issues found',
  stats: 'Stats',
  styleApplied: '{style} applied',
  styleUpdated: '{style} updated from selection',
  styleUpdateNeedsParagraph: 'Select a paragraph style to update',
  styleUnchanged: 'Style unchanged',
  styles: 'Styles',
  spacingAfter: 'Spacing after',
  spacingBefore: 'Spacing before',
  strikethrough: 'Strike',
  subscript: 'Sub',
  superscript: 'Sup',
  telemetry: 'Telemetry',
  templateLoaded: 'Template loaded',
  templates: 'Templates',
  textFormatting: 'Text formatting',
  textColor: 'Text color',
  top: 'Top',
  txt: 'TXT',
  uiLocale: 'UI language',
  underline: 'Underline',
  undo: 'Undo',
  undoApplied: 'Undo applied',
  updateStyle: 'Update',
  updateStyleFromSelection: 'Update style from selection',
  useTemplate: 'Use Template',
  userDictionarySuffix: 'user',
  version: 'Version',
  width: 'Width',
  words: 'Words',
  workspaceViews: 'Workspace views',
  documentWorkspace: 'Document workspace',
  documentStatistics: 'Document statistics',
  yes: 'Yes'
};

const spanish: Partial<UiStrings> = {
  about: 'Acerca de',
  about900Word: 'Acerca de 900Word',
  applyPage: 'Aplicar pagina',
  autosave: 'Autoguardar',
  autosaveUpdated: 'Borrador de recuperacion actualizado',
  blocks: 'Bloques',
  bold: 'Negrita',
  bottom: 'Inferior',
  case: 'Mayusculas',
  characters: 'Caracteres',
  checkSpelling: 'Revisar ortografia',
  custom: 'Personalizado',
  dictionary: 'Diccionario',
  documentActions: 'Acciones del documento',
  documentFormat: 'Formato de documento',
  documentOpened: 'Documento abierto',
  documentSaved: 'Documento guardado',
  documentSavedAs: 'Documento guardado como',
  dirty: 'Cambios',
  discard: 'Descartar',
  editor: 'Editor',
  editorReadOnly: 'El editor es de solo lectura',
  editingToolbar: 'Barra de edicion',
  export: 'Exportar',
  exportHtmlPrepared: 'Exportacion HTML preparada ({characters} caracteres)',
  exportHtmlSaved: 'Exportacion HTML escrita ({bytes} bytes)',
  exportPath: 'Ruta de exportacion',
  exportPathPlaceholder: 'Ruta .txt, .html o .pdf',
  exportPdfPrepared: 'Exportacion PDF preparada ({bytes} bytes)',
  exportPdfSaved: 'Exportacion PDF escrita ({bytes} bytes)',
  exportTxtPrepared: 'Exportacion TXT preparada ({characters} caracteres)',
  exportTxtSaved: 'Exportacion TXT escrita ({bytes} bytes)',
  file: 'Archivo',
  find: 'Buscar',
  findAndReplace: 'Buscar y reemplazar',
  height: 'Alto',
  highContrast: 'Alto contraste',
  html: 'HTML',
  italic: 'Cursiva',
  language: 'Idioma',
  left: 'Izquierda',
  license: 'Licencia',
  mainMenu: 'Menu principal',
  new: 'Nuevo',
  next: 'Siguiente',
  noIssues: 'Sin problemas.',
  noMatches: 'Sin coincidencias',
  no: 'No',
  off: 'Apagado',
  open: 'Abrir',
  page: 'Pagina',
  pageSetupUpdated: 'Pagina actualizada',
  pageSize: 'Tamano de pagina',
  pdf: 'PDF',
  print: 'Imprimir',
  printPrepared: 'Vista de impresion preparada',
  recent: 'Recientes',
  recovery: 'Recuperacion',
  recoveryDiscarded: 'Borrador de recuperacion descartado',
  recoveryOpened: 'Borrador de recuperacion abierto',
  redo: 'Rehacer',
  replace: 'Reemplazar',
  right: 'Derecha',
  save: 'Guardar',
  saveAs: 'Guardar como...',
  saveSettings: 'Guardar ajustes',
  saved: 'Guardado',
  settings: 'Ajustes',
  settingsUpdated: 'Ajustes guardados',
  spelling: 'Ortografia',
  starting: 'Iniciando...',
  startWriting: 'Empieza a escribir...',
  statusNoSpellingIssues: 'No se encontraron problemas de ortografia',
  stats: 'Estadisticas',
  telemetry: 'Telemetria',
  templateLoaded: 'Plantilla cargada',
  templates: 'Plantillas',
  top: 'Superior',
  txt: 'TXT',
  uiLocale: 'Idioma de interfaz',
  underline: 'Subrayado',
  undo: 'Deshacer',
  useTemplate: 'Usar plantilla',
  userDictionarySuffix: 'usuario',
  version: 'Version',
  width: 'Ancho',
  words: 'Palabras',
  workspaceViews: 'Vistas del espacio de trabajo',
  documentWorkspace: 'Espacio de documento',
  documentStatistics: 'Estadisticas del documento',
  yes: 'Si'
};

const dictionaries: Record<UiLocaleTag, Partial<UiStrings>> = {
  'en-US': english,
  'es-ES': spanish,
  ar: {}
};

export const uiLocales: UiLocaleInfo[] = [
  { tag: 'en-US', display_name: 'English', direction: 'ltr' },
  { tag: 'es-ES', display_name: 'Espanol', direction: 'ltr' },
  { tag: 'ar', display_name: 'Arabic RTL smoke', direction: 'rtl' }
];

export function translate(
  locale: string,
  key: UiStringKey,
  values: Record<string, string | number> = {}
): string {
  const tag = normalizeUiLocale(locale);
  const template = dictionaries[tag][key] ?? english[key];
  return Object.entries(values).reduce(
    (text, [name, value]) => text.replaceAll(`{${name}}`, String(value)),
    template
  );
}

export function normalizeUiLocale(locale: string): UiLocaleTag {
  return isUiLocale(locale) ? locale : 'en-US';
}

export function localeDirection(locale: string): 'ltr' | 'rtl' {
  return uiLocales.find((candidate) => candidate.tag === normalizeUiLocale(locale))?.direction ?? 'ltr';
}

function isUiLocale(locale: string): locale is UiLocaleTag {
  return locale === 'en-US' || locale === 'es-ES' || locale === 'ar';
}
