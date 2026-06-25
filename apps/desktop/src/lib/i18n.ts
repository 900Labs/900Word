export type UiLocaleTag = 'en-US' | 'es-ES' | 'ar';

export type UiStringKey =
  | 'about'
  | 'about900Word'
  | 'accept'
  | 'acceptAll'
  | 'accessibilityAndPerformance'
  | 'all'
  | 'allTrackedChangesAccepted'
  | 'allTrackedChangesRejected'
  | 'addColumnLeft'
  | 'addColumnRight'
  | 'addComment'
  | 'addRowAbove'
  | 'addRowBelow'
  | 'after'
  | 'alignCenter'
  | 'alignJustify'
  | 'alignLeft'
  | 'alignRight'
  | 'apply'
  | 'applyHeadersFooters'
  | 'applyPage'
  | 'autosave'
  | 'autosaveUpdated'
  | 'before'
  | 'blockFormatting'
  | 'blocks'
  | 'bold'
  | 'bookmark'
  | 'bookmarkAdded'
  | 'bookmarkRemoved'
  | 'bottom'
  | 'bulletList'
  | 'bulletListApplied'
  | 'capitalizeSentences'
  | 'case'
  | 'characters'
  | 'charactersWithSpaces'
  | 'charactersNoSpaces'
  | 'checkSpelling'
  | 'clear'
  | 'clearFormatting'
  | 'close'
  | 'columnsShort'
  | 'comments'
  | 'commentAdded'
  | 'commentBody'
  | 'commentBodyRequired'
  | 'commentBodyTooLong'
  | 'commentDeleted'
  | 'commentReopened'
  | 'commentResolved'
  | 'commentSelected'
  | 'commentSelectionRequired'
  | 'custom'
  | 'created'
  | 'dateField'
  | 'decreaseIndent'
  | 'deleteColumn'
  | 'deleteComment'
  | 'deleteRow'
  | 'deleteTable'
  | 'dictionary'
  | 'differentFirstPage'
  | 'documentActions'
  | 'documentFormat'
  | 'documentInspector'
  | 'documentOpened'
  | 'documentSaved'
  | 'documentSavedAs'
  | 'dirty'
  | 'discard'
  | 'deletion'
  | 'draftView'
  | 'editor'
  | 'editorReadOnly'
  | 'editingToolbar'
  | 'emptyChangeText'
  | 'export'
  | 'externalLink'
  | 'exportHtml'
  | 'exportHtmlPrepared'
  | 'exportHtmlSaved'
  | 'exportPath'
  | 'exportPathPlaceholder'
  | 'exportPdf'
  | 'exportPdfPathRequired'
  | 'exportPdfPrepared'
  | 'exportPdfSaved'
  | 'exportTxt'
  | 'exportTxtPrepared'
  | 'exportTxtSaved'
  | 'file'
  | 'find'
  | 'findAndReplace'
  | 'fitWidth'
  | 'firstLine'
  | 'firstLineIndent'
  | 'fontControls'
  | 'fontFamily'
  | 'fontSize'
  | 'footer'
  | 'footerFields'
  | 'footnotes'
  | 'formattingCleared'
  | 'addToDictionary'
  | 'firstFooter'
  | 'firstFooterFields'
  | 'firstHeader'
  | 'firstHeaderFields'
  | 'header'
  | 'headerFields'
  | 'heading1'
  | 'heading2'
  | 'heading3'
  | 'headingApplied'
  | 'headingJumped'
  | 'headingUnchanged'
  | 'height'
  | 'headersFooters'
  | 'highContrast'
  | 'highlightColor'
  | 'html'
  | 'history'
  | 'increaseIndent'
  | 'imageAlignment'
  | 'imageAltText'
  | 'imageCaption'
  | 'imageControls'
  | 'imageInserted'
  | 'imageTooLarge'
  | 'imageScale'
  | 'imageUpdated'
  | 'images'
  | 'inspectorBackendOnlyLocation'
  | 'inspectorNoSavedLocation'
  | 'inspectorPrivacyComments'
  | 'inspectorPrivacyMetadata'
  | 'inspectorPrivacyRecovery'
  | 'inspectorPrivacyTrackedChanges'
  | 'inspectorPrivacyUnsaved'
  | 'inspectorSaved'
  | 'inspectorSavedWithUnsavedChanges'
  | 'inspectorUnsaved'
  | 'insertion'
  | 'insertImage'
  | 'insertFootnote'
  | 'insertEndnote'
  | 'insertOrUpdateTableOfContents'
  | 'insertTable'
  | 'insertTableShort'
  | 'ignoreAll'
  | 'ignoreOnce'
  | 'italic'
  | 'language'
  | 'largeToolbar'
  | 'left'
  | 'license'
  | 'lineSpacing'
  | 'link'
  | 'linkApplied'
  | 'linkHref'
  | 'linkInvalid'
  | 'linkRemoved'
  | 'linkTarget'
  | 'linkTools'
  | 'linkUnchanged'
  | 'listLevelChanged'
  | 'lists'
  | 'lowResourceMode'
  | 'matchCount'
  | 'matchReplaced'
  | 'matchesReplaced'
  | 'markToggled'
  | 'markUnavailable'
  | 'mainMenu'
  | 'modified'
  | 'navigator'
  | 'new'
  | 'next'
  | 'noIssues'
  | 'noMatches'
  | 'noSuggestions'
  | 'no'
  | 'noComments'
  | 'noPrivacyWarnings'
  | 'noTrackedChanges'
  | 'notAvailable'
  | 'notRecording'
  | 'notes'
  | 'noNotes'
  | 'noteBodyRequired'
  | 'noteBodyTooLong'
  | 'noteInsertionUnavailable'
  | 'footnoteInserted'
  | 'footnotePrompt'
  | 'endnoteInserted'
  | 'endnotePrompt'
  | 'embeddedAssets'
  | 'embeddedImageBytes'
  | 'embeddedImages'
  | 'endnotes'
  | 'estimatedPages'
  | 'estimatedReadingTime'
  | 'numberedList'
  | 'numberedListApplied'
  | 'off'
  | 'offlineDictionaryFallback'
  | 'open'
  | 'openLinkPanel'
  | 'openRecent'
  | 'page'
  | 'pageCount'
  | 'pageLayoutView'
  | 'pageNumber'
  | 'pageRegionsReadOnly'
  | 'pageRegionsUpdated'
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
  | 'privacyWarnings'
  | 'projection'
  | 'recent'
  | 'recover'
  | 'recovery'
  | 'recoveryDiscarded'
  | 'recoveryOpened'
  | 'redo'
  | 'redoApplied'
  | 'reopen'
  | 'replace'
  | 'replaceAll'
  | 'recordChanges'
  | 'recording'
  | 'reject'
  | 'rejectAll'
  | 'reducedMotion'
  | 'resolve'
  | 'reviewChanges'
  | 'removeBookmark'
  | 'removeBookmarkShort'
  | 'removeLink'
  | 'right'
  | 'rowsShort'
  | 'readingMinutes'
  | 'readingTime'
  | 'save'
  | 'saveAs'
  | 'saveSettings'
  | 'saved'
  | 'savedLocation'
  | 'savedStatus'
  | 'settings'
  | 'settingsUpdated'
  | 'selectionWords'
  | 'showRulers'
  | 'smartDashes'
  | 'smartListTriggers'
  | 'smartQuotes'
  | 'smartTyping'
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
  | 'statsEstimateNote'
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
  | 'trackChangesPrivacyWarning'
  | 'trackChangesRecordingOff'
  | 'trackChangesRecordingOn'
  | 'trackChangesStatus'
  | 'trackedChanges'
  | 'trackedChangeAccepted'
  | 'trackedChangeRejected'
  | 'trackedChangeSelected'
  | 'typoReplacements'
  | 'tableColumns'
  | 'tableDeleted'
  | 'tableInserted'
  | 'tableOfContents'
  | 'tableOfContentsUpdated'
  | 'tableRows'
  | 'tableUpdated'
  | 'tables'
  | 'templateLoaded'
  | 'templates'
  | 'textFormatting'
  | 'textColor'
  | 'top'
  | 'txt'
  | 'uiLocale'
  | 'underline'
  | 'unlink'
  | 'unresolvedComments'
  | 'undo'
  | 'undoApplied'
  | 'updateStyle'
  | 'updateStyleFromSelection'
  | 'useTemplate'
  | 'userDictionarySuffix'
  | 'version'
  | 'viewMode'
  | 'width'
  | 'words'
  | 'workspaceViews'
  | 'zoom'
  | 'customZoom'
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
  accept: 'Accept',
  acceptAll: 'Accept all',
  accessibilityAndPerformance: 'Accessibility and performance',
  all: 'All',
  allTrackedChangesAccepted: 'All tracked changes accepted',
  allTrackedChangesRejected: 'All tracked changes rejected',
  addColumnLeft: 'Add column left',
  addColumnRight: 'Add column right',
  addComment: 'Add comment',
  addRowAbove: 'Add row above',
  addRowBelow: 'Add row below',
  after: 'After',
  alignCenter: 'Align center',
  alignJustify: 'Justify',
  alignLeft: 'Align left',
  alignRight: 'Align right',
  apply: 'Apply',
  applyHeadersFooters: 'Apply Headers and Footers',
  applyPage: 'Apply Page',
  autosave: 'Autosave',
  autosaveUpdated: 'Recovery draft updated',
  before: 'Before',
  blockFormatting: 'Block formatting',
  blocks: 'Blocks',
  bold: 'Bold',
  bookmark: 'Bookmark',
  bookmarkAdded: 'Bookmark added',
  bookmarkRemoved: 'Bookmark removed',
  bottom: 'Bottom',
  bulletList: 'Bullet list',
  bulletListApplied: 'Bullet list applied',
  capitalizeSentences: 'Capitalize sentences',
  case: 'Case',
  characters: 'Characters',
  charactersWithSpaces: 'Characters with spaces',
  charactersNoSpaces: 'Characters without spaces',
  checkSpelling: 'Check Spelling',
  clear: 'Clear',
  clearFormatting: 'Clear formatting',
  close: 'Close',
  columnsShort: 'C',
  comments: 'Comments',
  commentAdded: 'Comment added',
  commentBody: 'Comment body',
  commentBodyRequired: 'Enter a comment before adding it.',
  commentBodyTooLong: 'Comment is too long. Keep it under {max} characters.',
  commentDeleted: 'Comment deleted',
  commentReopened: 'Comment reopened',
  commentResolved: 'Comment resolved',
  commentSelected: 'Comment selected',
  commentSelectionRequired: 'Select non-empty text before adding a comment.',
  custom: 'Custom',
  created: 'Created',
  dateField: 'Date',
  decreaseIndent: 'Decrease indent',
  deleteColumn: 'Delete column',
  deleteComment: 'Delete comment',
  deleteRow: 'Delete row',
  deleteTable: 'Delete table',
  dictionary: 'Dictionary',
  differentFirstPage: 'Different first page',
  documentActions: 'Document actions',
  documentFormat: 'Document format',
  documentInspector: 'Document Inspector',
  documentOpened: 'Document opened',
  documentSaved: 'Document saved',
  documentSavedAs: 'Document saved as',
  dirty: 'Dirty',
  discard: 'Discard',
  deletion: 'Deletion',
  draftView: 'Draft',
  editor: 'Editor',
  editorReadOnly: 'Editor is read-only',
  editingToolbar: 'Editing toolbar',
  emptyChangeText: 'Empty text change',
  export: 'Export',
  externalLink: 'External link',
  exportHtml: 'Export HTML',
  exportHtmlPrepared: 'HTML export prepared ({characters} characters)',
  exportHtmlSaved: 'HTML export written ({bytes} bytes)',
  exportPath: 'Export path',
  exportPathPlaceholder: 'Export .txt, .html, or .pdf path',
  exportPdf: 'Export PDF',
  exportPdfPathRequired: 'Enter a PDF export path in File > Export.',
  exportPdfPrepared: 'PDF export prepared ({bytes} bytes)',
  exportPdfSaved: 'PDF export written ({bytes} bytes)',
  exportTxt: 'Export TXT',
  exportTxtPrepared: 'TXT export prepared ({characters} characters)',
  exportTxtSaved: 'TXT export written ({bytes} bytes)',
  file: 'File',
  find: 'Find',
  findAndReplace: 'Find and replace',
  fitWidth: 'Fit Width',
  firstLine: 'First',
  firstLineIndent: 'First-line indent',
  fontControls: 'Font controls',
  fontFamily: 'Font family',
  fontSize: 'Font size',
  footer: 'Footer',
  footerFields: 'Footer fields',
  footnotes: 'Footnotes',
  formattingCleared: 'Formatting cleared',
  addToDictionary: 'Add',
  firstFooter: 'First-page footer',
  firstFooterFields: 'First-page footer fields',
  firstHeader: 'First-page header',
  firstHeaderFields: 'First-page header fields',
  header: 'Header',
  headerFields: 'Header fields',
  heading1: 'H1',
  heading2: 'H2',
  heading3: 'H3',
  headingApplied: 'Heading {level} applied',
  headingJumped: 'Jumped to {heading}',
  headingUnchanged: 'Heading {level} unchanged',
  height: 'Height',
  headersFooters: 'Headers and footers',
  highContrast: 'High contrast',
  highlightColor: 'Highlight color',
  html: 'HTML',
  history: 'History',
  increaseIndent: 'Increase indent',
  imageAlignment: 'Image alignment',
  imageAltText: 'Alt text',
  imageCaption: 'Caption',
  imageControls: 'Image controls',
  imageInserted: 'Image inserted',
  imageTooLarge: 'Image is too large. Compress or resize it before inserting.',
  imageScale: 'Scale',
  imageUpdated: 'Image updated',
  images: 'Images',
  inspectorBackendOnlyLocation: 'Saved location known to backend only',
  inspectorNoSavedLocation: 'No saved location',
  inspectorPrivacyComments: 'Comments may contain review notes that are saved with the document.',
  inspectorPrivacyMetadata: 'Document title metadata is saved in the ODT package.',
  inspectorPrivacyRecovery: 'Recovery drafts are local files that may contain document content.',
  inspectorPrivacyTrackedChanges: 'Tracked changes can reveal edit history and deleted text in saved files.',
  inspectorPrivacyUnsaved: 'Unsaved changes may exist only in this session or local recovery drafts.',
  inspectorSaved: 'Saved',
  inspectorSavedWithUnsavedChanges: 'Saved with unsaved changes',
  inspectorUnsaved: 'Unsaved',
  insertion: 'Insertion',
  insertFootnote: 'Footnote',
  insertEndnote: 'Endnote',
  insertImage: 'Insert image',
  insertOrUpdateTableOfContents: 'Insert/update contents',
  insertTable: 'Insert table',
  insertTableShort: 'Insert',
  ignoreAll: 'Ignore all',
  ignoreOnce: 'Ignore once',
  italic: 'Italic',
  language: 'Language',
  largeToolbar: 'Large toolbar and controls',
  left: 'Left',
  license: 'License',
  lineSpacing: 'Line spacing',
  link: 'Link',
  linkApplied: 'Link applied',
  linkHref: 'Link address',
  linkInvalid: 'Use a safe http, https, mailto, or #bookmark link',
  linkRemoved: 'Link removed',
  linkTarget: 'Link target',
  linkTools: 'Link tools',
  linkUnchanged: 'Link unchanged',
  listLevelChanged: 'List level changed',
  lists: 'Lists',
  lowResourceMode: 'Low-resource mode',
  matchCount: '{current}/{total}',
  matchReplaced: 'Match replaced',
  matchesReplaced: '{count} match(es) replaced',
  markToggled: '{label} toggled',
  markUnavailable: '{label} unavailable',
  mainMenu: 'Main menu',
  modified: 'Modified',
  navigator: 'Navigator',
  new: 'New',
  next: 'Next',
  no: 'No',
  noComments: 'No comments',
  noPrivacyWarnings: 'No privacy warnings.',
  noTrackedChanges: 'No tracked changes',
  notAvailable: 'Not available',
  notRecording: 'Not recording',
  notes: 'Notes',
  noNotes: 'No footnotes or endnotes',
  noteBodyRequired: 'Enter note text before inserting it.',
  noteBodyTooLong: 'Note is too long. Keep it under {max} characters.',
  noteInsertionUnavailable: 'Place the cursor in editable text before inserting a note.',
  footnoteInserted: 'Footnote inserted',
  footnotePrompt: 'Footnote text',
  endnoteInserted: 'Endnote inserted',
  endnotePrompt: 'Endnote text',
  embeddedAssets: 'Embedded assets',
  embeddedImageBytes: 'Embedded image bytes',
  embeddedImages: 'Embedded images',
  endnotes: 'Endnotes',
  estimatedPages: 'Estimated pages',
  estimatedReadingTime: 'Estimated reading time',
  noIssues: 'No issues.',
  noMatches: 'No matches',
  noSuggestions: 'No suggestions',
  numberedList: 'Numbered list',
  numberedListApplied: 'Numbered list applied',
  off: 'Off',
  offlineDictionaryFallback: 'Dictionary fallback used',
  open: 'Open',
  openLinkPanel: 'Insert or edit link',
  openRecent: 'Open',
  page: 'Page',
  pageCount: 'Page count',
  pageLayoutView: 'Page Layout',
  pageNumber: 'Page number',
  pageRegionsReadOnly: 'Imported header/footer content is read-only',
  pageRegionsUpdated: 'Headers and footers updated',
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
  privacyWarnings: 'Privacy warnings',
  projection: 'Projection',
  recent: 'Recent',
  recover: 'Open',
  recovery: 'Recovery',
  recoveryDiscarded: 'Recovery draft discarded',
  recoveryOpened: 'Recovery draft opened',
  redo: 'Redo',
  redoApplied: 'Redo applied',
  reopen: 'Reopen',
  replace: 'Replace',
  replaceAll: 'All',
  recordChanges: 'Record changes',
  recording: 'Recording',
  reject: 'Reject',
  rejectAll: 'Reject all',
  reducedMotion: 'Reduced motion',
  resolve: 'Resolve',
  reviewChanges: 'Review changes',
  removeBookmark: 'Remove bookmark',
  removeBookmarkShort: 'No mark',
  removeLink: 'Remove link',
  readingMinutes: '{count} min',
  readingTime: 'Reading time',
  right: 'Right',
  rowsShort: 'R',
  save: 'Save',
  saveAs: 'Save As...',
  saveSettings: 'Save Settings',
  saved: 'Saved',
  savedLocation: 'Saved location',
  savedStatus: 'Saved status',
  settings: 'Settings',
  settingsUpdated: 'Settings updated',
  selectionWords: 'Selection words',
  showRulers: 'Rulers',
  smartDashes: 'Convert -- to em dash',
  smartListTriggers: 'Turn - and 1. starters into lists',
  smartQuotes: 'Use smart quotes',
  smartTyping: 'Smart typing',
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
  statsEstimateNote: 'Page count and reading time are estimates, not deterministic pagination.',
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
  trackChangesPrivacyWarning: 'Tracked changes can reveal edit history and deleted text in saved files.',
  trackChangesRecordingOff: 'Track changes recording off',
  trackChangesRecordingOn: 'Track changes recording on',
  trackChangesStatus: 'Track changes status',
  trackedChanges: 'Tracked changes',
  trackedChangeAccepted: 'Tracked change accepted',
  trackedChangeRejected: 'Tracked change rejected',
  trackedChangeSelected: 'Tracked change selected',
  typoReplacements: 'Replace common typos',
  tableColumns: 'Table columns',
  tableDeleted: 'Table deleted',
  tableInserted: 'Table inserted',
  tableOfContents: 'Table of contents',
  tableOfContentsUpdated: 'Contents updated from Heading 1-3',
  tableRows: 'Table rows',
  tableUpdated: 'Table updated',
  tables: 'Tables',
  templateLoaded: 'Template loaded',
  templates: 'Templates',
  textFormatting: 'Text formatting',
  textColor: 'Text color',
  top: 'Top',
  txt: 'TXT',
  uiLocale: 'UI language',
  underline: 'Underline',
  unlink: 'Unlink',
  unresolvedComments: 'Unresolved comments',
  undo: 'Undo',
  undoApplied: 'Undo applied',
  updateStyle: 'Update',
  updateStyleFromSelection: 'Update style from selection',
  useTemplate: 'Use Template',
  userDictionarySuffix: 'user',
  version: 'Version',
  viewMode: 'View mode',
  width: 'Width',
  words: 'Words',
  workspaceViews: 'Workspace views',
  zoom: 'Zoom',
  customZoom: 'Custom zoom',
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
  accessibilityAndPerformance: 'Accesibilidad y rendimiento',
  blocks: 'Bloques',
  bold: 'Negrita',
  bottom: 'Inferior',
  capitalizeSentences: 'Poner mayuscula al iniciar oraciones',
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
  exportPdfPathRequired: 'Ingrese una ruta de exportacion PDF en Archivo > Exportar.',
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
  imageAlignment: 'Alineacion de imagen',
  imageAltText: 'Texto alternativo',
  imageCaption: 'Pie',
  imageControls: 'Controles de imagen',
  imageInserted: 'Imagen insertada',
  imageTooLarge: 'La imagen es demasiado grande. Comprímala o cambie su tamaño antes de insertarla.',
  imageScale: 'Escala',
  imageUpdated: 'Imagen actualizada',
  insertImage: 'Insertar imagen',
  italic: 'Cursiva',
  language: 'Idioma',
  largeToolbar: 'Barra y controles grandes',
  left: 'Izquierda',
  license: 'Licencia',
  lowResourceMode: 'Modo de bajo consumo',
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
  reducedMotion: 'Movimiento reducido',
  replace: 'Reemplazar',
  right: 'Derecha',
  save: 'Guardar',
  saveAs: 'Guardar como...',
  saveSettings: 'Guardar ajustes',
  saved: 'Guardado',
  settings: 'Ajustes',
  settingsUpdated: 'Ajustes guardados',
  smartDashes: 'Convertir -- en raya',
  smartListTriggers: 'Convertir - y 1. en listas',
  smartQuotes: 'Usar comillas tipograficas',
  smartTyping: 'Escritura inteligente',
  spelling: 'Ortografia',
  starting: 'Iniciando...',
  startWriting: 'Empieza a escribir...',
  statusNoSpellingIssues: 'No se encontraron problemas de ortografia',
  stats: 'Estadisticas',
  telemetry: 'Telemetria',
  typoReplacements: 'Corregir errores comunes',
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
