export type UiLocaleTag = 'en-US' | 'es-ES' | 'ar';

export type UiStringKey =
  | 'about'
  | 'about900Word'
  | 'all'
  | 'applyPage'
  | 'autosave'
  | 'autosaveUpdated'
  | 'blockFormatting'
  | 'blocks'
  | 'bold'
  | 'bottom'
  | 'case'
  | 'characters'
  | 'checkSpelling'
  | 'close'
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
  | 'heading1'
  | 'heading2'
  | 'headingApplied'
  | 'headingUnchanged'
  | 'height'
  | 'highContrast'
  | 'history'
  | 'italic'
  | 'language'
  | 'left'
  | 'license'
  | 'matchCount'
  | 'matchReplaced'
  | 'matchesReplaced'
  | 'markToggled'
  | 'markUnavailable'
  | 'new'
  | 'next'
  | 'noIssues'
  | 'noMatches'
  | 'no'
  | 'off'
  | 'offlineDictionaryFallback'
  | 'odtPath'
  | 'odtPathPlaceholder'
  | 'open'
  | 'openRecent'
  | 'page'
  | 'paragraph'
  | 'paragraphApplied'
  | 'paragraphUnchanged'
  | 'pageSetupUpdated'
  | 'previous'
  | 'print'
  | 'printFrame'
  | 'printPrepared'
  | 'projection'
  | 'ready'
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
  | 'save'
  | 'saveAs'
  | 'saveSettings'
  | 'saved'
  | 'settings'
  | 'settingsUpdated'
  | 'spellIssueCount'
  | 'spelling'
  | 'starting'
  | 'statusNoSpellingIssues'
  | 'stats'
  | 'strikethrough'
  | 'subscript'
  | 'superscript'
  | 'telemetry'
  | 'templateLoaded'
  | 'templates'
  | 'textFormatting'
  | 'top'
  | 'uiLocale'
  | 'underline'
  | 'undo'
  | 'undoApplied'
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
  applyPage: 'Apply Page',
  autosave: 'Autosave',
  autosaveUpdated: 'Recovery draft updated',
  blockFormatting: 'Block formatting',
  blocks: 'Blocks',
  bold: 'Bold',
  bottom: 'Bottom',
  case: 'Case',
  characters: 'Characters',
  checkSpelling: 'Check Spelling',
  close: 'Close',
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
  heading1: 'H1',
  heading2: 'H2',
  headingApplied: 'Heading {level} applied',
  headingUnchanged: 'Heading {level} unchanged',
  height: 'Height',
  highContrast: 'High contrast',
  history: 'History',
  italic: 'Italic',
  language: 'Language',
  left: 'Left',
  license: 'License',
  matchCount: '{current}/{total}',
  matchReplaced: 'Match replaced',
  matchesReplaced: '{count} match(es) replaced',
  markToggled: '{label} toggled',
  markUnavailable: '{label} unavailable',
  new: 'New',
  next: 'Next',
  no: 'No',
  noIssues: 'No issues.',
  noMatches: 'No matches',
  off: 'Off',
  offlineDictionaryFallback: 'Dictionary fallback used',
  odtPath: 'ODT path',
  odtPathPlaceholder: 'Document .odt path',
  open: 'Open',
  openRecent: 'Open',
  page: 'Page',
  paragraph: 'P',
  paragraphApplied: 'Paragraph applied',
  paragraphUnchanged: 'Paragraph unchanged',
  pageSetupUpdated: 'Page setup updated',
  previous: 'Prev',
  print: 'Print',
  printFrame: 'Print document',
  printPrepared: 'Print view prepared',
  projection: 'Projection',
  ready: 'Ready',
  recent: 'Recent',
  recover: 'Open',
  recovery: 'Recovery',
  recoveryDiscarded: 'Recovery draft discarded',
  recoveryOpened: 'Recovery draft opened',
  redo: 'Redo',
  redoApplied: 'Redo applied',
  replace: 'Replace',
  replaceAll: 'All',
  right: 'Right',
  save: 'Save',
  saveAs: 'Save As',
  saveSettings: 'Save Settings',
  saved: 'Saved',
  settings: 'Settings',
  settingsUpdated: 'Settings updated',
  spellIssueCount: '{count} spelling issue(s)',
  spelling: 'Spelling',
  starting: 'Starting...',
  statusNoSpellingIssues: 'No spelling issues found',
  stats: 'Stats',
  strikethrough: 'Strike',
  subscript: 'Sub',
  superscript: 'Sup',
  telemetry: 'Telemetry',
  templateLoaded: 'Template loaded',
  templates: 'Templates',
  textFormatting: 'Text formatting',
  top: 'Top',
  uiLocale: 'UI language',
  underline: 'Underline',
  undo: 'Undo',
  undoApplied: 'Undo applied',
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
  italic: 'Cursiva',
  language: 'Idioma',
  left: 'Izquierda',
  license: 'Licencia',
  new: 'Nuevo',
  next: 'Siguiente',
  noIssues: 'Sin problemas.',
  noMatches: 'Sin coincidencias',
  no: 'No',
  off: 'Apagado',
  odtPath: 'Ruta ODT',
  odtPathPlaceholder: 'Ruta del documento .odt',
  open: 'Abrir',
  page: 'Pagina',
  pageSetupUpdated: 'Pagina actualizada',
  print: 'Imprimir',
  printPrepared: 'Vista de impresion preparada',
  ready: 'Listo',
  recent: 'Recientes',
  recovery: 'Recuperacion',
  recoveryDiscarded: 'Borrador de recuperacion descartado',
  recoveryOpened: 'Borrador de recuperacion abierto',
  redo: 'Rehacer',
  replace: 'Reemplazar',
  right: 'Derecha',
  save: 'Guardar',
  saveAs: 'Guardar como',
  saveSettings: 'Guardar ajustes',
  saved: 'Guardado',
  settings: 'Ajustes',
  settingsUpdated: 'Ajustes guardados',
  spelling: 'Ortografia',
  starting: 'Iniciando...',
  statusNoSpellingIssues: 'No se encontraron problemas de ortografia',
  stats: 'Estadisticas',
  telemetry: 'Telemetria',
  templateLoaded: 'Plantilla cargada',
  templates: 'Plantillas',
  top: 'Superior',
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
