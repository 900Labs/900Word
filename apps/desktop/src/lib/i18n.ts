export type UiLocaleTag = 'en-US' | 'es-ES' | 'ar';

export type UiStringKey =
  | 'about'
  | 'about900Word'
  | 'accept'
  | 'acceptAll'
  | 'accessibilityAndPerformance'
  | 'activeObject'
  | 'all'
  | 'allTrackedChangesAccepted'
  | 'allTrackedChangesRejected'
  | 'addColumnLeft'
  | 'addColumnRight'
  | 'addComment'
  | 'addRowAbove'
  | 'addRowBelow'
  | 'after'
  | 'activeDictionary'
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
  | 'blankDocument'
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
  | 'chooseExportLocation'
  | 'clear'
  | 'clearFormatting'
  | 'clearSpelling'
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
  | 'dictionaryAffFile'
  | 'dictionaryAffNotSelected'
  | 'dictionaryAffSelected'
  | 'dictionaryChooseAff'
  | 'dictionaryChooseDic'
  | 'dictionaryDicFile'
  | 'dictionaryDicNotSelected'
  | 'dictionaryDicSelected'
  | 'dictionaryInstall'
  | 'dictionaryInstallFailed'
  | 'dictionaryInstallInvalidLanguage'
  | 'dictionaryInstallLocal'
  | 'dictionaryInstallSuccess'
  | 'dictionaryInstallUnsupportedFile'
  | 'dictionaryLanguageTag'
  | 'dictionaryLicense'
  | 'dictionaryListRefreshed'
  | 'dictionaryManager'
  | 'dictionaryNoneInstalled'
  | 'dictionaryOfflineState'
  | 'dictionaryRefresh'
  | 'dictionaryRemove'
  | 'dictionaryRemoveFailed'
  | 'dictionaryRemoveInvalidLanguage'
  | 'dictionaryRemoveSuccess'
  | 'dictionarySourceAppBundle'
  | 'dictionarySourceBundled'
  | 'dictionarySourceBundledUser'
  | 'dictionarySourceLabel'
  | 'dictionarySourceLocal'
  | 'dictionarySourceType'
  | 'dictionarySourceUser'
  | 'dictionarySourceUserFolder'
  | 'dictionaryUnavailable'
  | 'dictionaryUnavailableOption'
  | 'differentFirstPage'
  | 'documentActions'
  | 'documentFormat'
  | 'documentInspector'
  | 'documentNavigation'
  | 'documentStructure'
  | 'docx'
  | 'docxImported'
  | 'documentOpened'
  | 'documentSaved'
  | 'documentSavedAs'
  | 'dirty'
  | 'discard'
  | 'deletion'
  | 'draftView'
  | 'edit'
  | 'editor'
  | 'editorReadOnly'
  | 'editingToolbar'
  | 'emptyChangeText'
  | 'export'
  | 'externalLink'
  | 'exportHtml'
  | 'exportHtmlDescription'
  | 'exportHtmlPrepared'
  | 'exportHtmlSaved'
  | 'exportDocx'
  | 'exportDocxDescription'
  | 'exportDocxSaved'
  | 'exportPdf'
  | 'exportPdfDescription'
  | 'exportPdfInvalidRange'
  | 'exportPdfPrepared'
  | 'exportPdfSaved'
  | 'exportTxt'
  | 'exportTxtDescription'
  | 'exportTxtPrepared'
  | 'exportTxtSaved'
  | 'file'
  | 'find'
  | 'findAndReplace'
  | 'findMatches'
  | 'fitWidth'
  | 'firstLine'
  | 'firstLineIndent'
  | 'fontControls'
  | 'fontFamily'
  | 'fontSize'
  | 'footer'
  | 'footerFields'
  | 'footnotes'
  | 'format'
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
  | 'headings'
  | 'headingJumped'
  | 'headingUnchanged'
  | 'help'
  | 'height'
  | 'headersFooters'
  | 'highContrast'
  | 'highlightColor'
  | 'home'
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
  | 'insert'
  | 'insertImage'
  | 'insertFootnote'
  | 'insertEndnote'
  | 'insertOrUpdateTableOfContents'
  | 'insertTable'
  | 'insertTableShort'
  | 'installedDictionaries'
  | 'ignoreAll'
  | 'ignoreOnce'
  | 'italic'
  | 'language'
  | 'languageTools'
  | 'largeToolbar'
  | 'layout'
  | 'layoutStructure'
  | 'left'
  | 'license'
  | 'lineSpacing'
  | 'link'
  | 'linkApplied'
  | 'linkHref'
  | 'linkInvalid'
  | 'linkRemoved'
  | 'linkTarget'
  | 'linkTargetJumped'
  | 'linkTargets'
  | 'linkTools'
  | 'linkUnchanged'
  | 'linksAndBookmarks'
  | 'listLevelChanged'
  | 'lists'
  | 'lowResourceMode'
  | 'matchCount'
  | 'matchReplaced'
  | 'matchesReplaced'
  | 'markToggled'
  | 'markUnavailable'
  | 'mainMenu'
  | 'marginModerate'
  | 'marginNarrow'
  | 'marginNormal'
  | 'marginPresets'
  | 'margins'
  | 'marginWide'
  | 'modified'
  | 'navigator'
  | 'new'
  | 'newFromTemplate'
  | 'next'
  | 'nextChange'
  | 'nextComment'
  | 'nextIssue'
  | 'noIssues'
  | 'noMatches'
  | 'noSuggestions'
  | 'no'
  | 'noComments'
  | 'noHeadings'
  | 'noImageSelected'
  | 'noLinkTargets'
  | 'noObjectSelected'
  | 'noPrivacyWarnings'
  | 'noTableSelected'
  | 'noTrackedChanges'
  | 'notAvailable'
  | 'notRecording'
  | 'notes'
  | 'noNotes'
  | 'noRecentDocuments'
  | 'noRecoveryDrafts'
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
  | 'objects'
  | 'objectsWorkspace'
  | 'off'
  | 'offlineDictionaryFallback'
  | 'open'
  | 'openDictionarySettings'
  | 'openLinkPanel'
  | 'openRecent'
  | 'page'
  | 'pageCount'
  | 'pageLayoutView'
  | 'pageNumber'
  | 'pageRegionsReadOnly'
  | 'pageRegionsUpdated'
  | 'pageSetup'
  | 'paragraph'
  | 'paragraphApplied'
  | 'paragraphControls'
  | 'paragraphFormatApplied'
  | 'paragraphs'
  | 'paragraphUnchanged'
  | 'pageSetupUpdated'
  | 'pageSize'
  | 'personalDictionary'
  | 'personalDictionaryEmpty'
  | 'personalDictionaryRefresh'
  | 'personalDictionaryRefreshed'
  | 'personalDictionaryRemove'
  | 'personalDictionaryUnavailable'
  | 'personalDictionaryWordRemoved'
  | 'personalDictionaryWords'
  | 'pdf'
  | 'pdfAllPages'
  | 'pdfPageRange'
  | 'pdfPageRangeHint'
  | 'pdfPageRangeModeRange'
  | 'pdfPageRangeFrom'
  | 'pdfPageRangeTo'
  | 'previous'
  | 'previousChange'
  | 'previousComment'
  | 'previousIssue'
  | 'print'
  | 'printDescription'
  | 'printFrame'
  | 'printPrepared'
  | 'properties'
  | 'privacyWarnings'
  | 'projection'
  | 'quickAccess'
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
  | 'resolved'
  | 'review'
  | 'reviewChanges'
  | 'reviewSummary'
  | 'removeBookmark'
  | 'removeBookmarkShort'
  | 'removeLink'
  | 'right'
  | 'ribbonTabs'
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
  | 'resetSettings'
  | 'settingsReset'
  | 'settingsUpdated'
  | 'selectionWords'
  | 'selectedImage'
  | 'selectedTable'
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
  | 'spellingCleared'
  | 'spellingIssue'
  | 'spellingIssueSelected'
  | 'spellingIssues'
  | 'spellingSuggestions'
  | 'spellingSuggestionApplied'
  | 'starting'
  | 'startCenter'
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
  | 'tableColumnWidth'
  | 'tableCellAlignInherit'
  | 'tableCellAlignment'
  | 'tableCellBackground'
  | 'tableCellBackgroundLightBlue'
  | 'tableCellBackgroundLightGray'
  | 'tableCellBackgroundLightGreen'
  | 'tableCellBackgroundLightYellow'
  | 'tableCellBackgroundNone'
  | 'tableCellBorder'
  | 'tableCellBorderHidden'
  | 'tableCellBorderVisible'
  | 'tableCellUpdated'
  | 'tableDeleted'
  | 'tableInserted'
  | 'tableOfContents'
  | 'tableOfContentsUpdated'
  | 'tableRows'
  | 'tableUpdated'
  | 'tables'
  | 'deleteTemplate'
  | 'templateLoaded'
  | 'templateDeleted'
  | 'templateDeleteFailed'
  | 'templateGallery'
  | 'templateName'
  | 'templateSaved'
  | 'templateSaveFailed'
  | 'templates'
  | 'textFormatting'
  | 'textColor'
  | 'top'
  | 'tools'
  | 'txt'
  | 'uiLocale'
  | 'underline'
  | 'unlink'
  | 'unresolved'
  | 'unresolvedComments'
  | 'undo'
  | 'undoApplied'
  | 'updateStyle'
  | 'updateStyleFromSelection'
  | 'saveTemplate'
  | 'useTemplate'
  | 'userDictionarySuffix'
  | 'version'
  | 'view'
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
  activeObject: 'Active object',
  all: 'All',
  allTrackedChangesAccepted: 'All tracked changes accepted',
  allTrackedChangesRejected: 'All tracked changes rejected',
  addColumnLeft: 'Add column left',
  addColumnRight: 'Add column right',
  addComment: 'Add comment',
  addRowAbove: 'Add row above',
  addRowBelow: 'Add row below',
  after: 'After',
  activeDictionary: 'Active dictionary',
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
  blankDocument: 'Blank document',
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
  chooseExportLocation: 'Choose an export format and save location.',
  clear: 'Clear',
  clearFormatting: 'Clear formatting',
  clearSpelling: 'Clear spelling',
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
  dictionaryAffFile: 'AFF file',
  dictionaryAffNotSelected: 'No AFF selected',
  dictionaryAffSelected: 'AFF selected',
  dictionaryChooseAff: 'Choose AFF',
  dictionaryChooseDic: 'Choose DIC',
  dictionaryDicFile: 'DIC file',
  dictionaryDicNotSelected: 'No DIC selected',
  dictionaryDicSelected: 'DIC selected',
  dictionaryInstall: 'Install',
  dictionaryInstallFailed: 'Dictionary could not be installed',
  dictionaryInstallInvalidLanguage: 'Language tag is invalid',
  dictionaryInstallLocal: 'Install local Hunspell dictionary',
  dictionaryInstallSuccess: 'Dictionary installed',
  dictionaryInstallUnsupportedFile: 'Dictionary files are unsupported',
  dictionaryLanguageTag: 'Language tag',
  dictionaryLicense: 'License',
  dictionaryListRefreshed: 'Dictionary list refreshed',
  dictionaryManager: 'Dictionary manager',
  dictionaryNoneInstalled: 'No installed dictionaries found.',
  dictionaryOfflineState: 'Offline/local only. Dictionaries are read from the app bundle and local user dictionary folder.',
  dictionaryRefresh: 'Refresh',
  dictionaryRemove: 'Remove',
  dictionaryRemoveFailed: 'Dictionary could not be removed',
  dictionaryRemoveInvalidLanguage: 'Language tag is invalid',
  dictionaryRemoveSuccess: 'Dictionary removed',
  dictionarySourceAppBundle: 'App bundle',
  dictionarySourceBundled: 'Bundled',
  dictionarySourceBundledUser: 'Bundled + user',
  dictionarySourceLabel: 'Source',
  dictionarySourceLocal: 'Local dictionary',
  dictionarySourceType: 'Source type',
  dictionarySourceUser: 'User',
  dictionarySourceUserFolder: 'Local user dictionary folder',
  dictionaryUnavailable:
    'The selected dictionary is not installed. Spell checks fall back to the bundled English bootstrap dictionary when needed.',
  dictionaryUnavailableOption: 'Unavailable ({languageTag})',
  differentFirstPage: 'Different first page',
  documentActions: 'Document actions',
  documentFormat: 'Document format',
  documentInspector: 'Document Inspector',
  documentNavigation: 'Navigation',
  documentStructure: 'Document structure',
  docx: 'DOCX',
  docxImported: 'DOCX imported as an unsaved ODT document',
  documentOpened: 'Document opened',
  documentSaved: 'Document saved',
  documentSavedAs: 'Document saved as',
  dirty: 'Dirty',
  discard: 'Discard',
  deletion: 'Deletion',
  draftView: 'Draft',
  edit: 'Edit',
  editor: 'Editor',
  editorReadOnly: 'Editor is read-only',
  editingToolbar: 'Editing toolbar',
  emptyChangeText: 'Empty text change',
  export: 'Export',
  externalLink: 'External link',
  exportHtml: 'Export HTML',
  exportHtmlDescription: 'Sanitized offline HTML',
  exportHtmlPrepared: 'HTML export prepared ({characters} characters)',
  exportHtmlSaved: 'HTML export written ({bytes} bytes)',
  exportDocx: 'Export DOCX',
  exportDocxDescription: 'Bounded DOCX conversion',
  exportDocxSaved: 'DOCX export written ({bytes} bytes)',
  exportPdf: 'Export PDF',
  exportPdfDescription: 'Lightweight PDF snapshot',
  exportPdfInvalidRange: 'Enter a valid PDF page range.',
  exportPdfPrepared: 'PDF export prepared ({bytes} bytes)',
  exportPdfSaved: 'PDF export written ({bytes} bytes)',
  exportTxt: 'Export TXT',
  exportTxtDescription: 'Plain document text',
  exportTxtPrepared: 'TXT export prepared ({characters} characters)',
  exportTxtSaved: 'TXT export written ({bytes} bytes)',
  file: 'File',
  find: 'Find',
  findAndReplace: 'Find and replace',
  findMatches: 'Matches',
  fitWidth: 'Fit Width',
  firstLine: 'First',
  firstLineIndent: 'First-line indent',
  fontControls: 'Font controls',
  fontFamily: 'Font family',
  fontSize: 'Font size',
  footer: 'Footer',
  footerFields: 'Footer fields',
  footnotes: 'Footnotes',
  format: 'Format',
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
  headings: 'Headings',
  headingJumped: 'Jumped to {heading}',
  headingUnchanged: 'Heading {level} unchanged',
  help: 'Help',
  height: 'Height',
  headersFooters: 'Headers and footers',
  highContrast: 'High contrast',
  highlightColor: 'Highlight color',
  home: 'Home',
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
  insert: 'Insert',
  insertFootnote: 'Footnote',
  insertEndnote: 'Endnote',
  insertImage: 'Insert image',
  insertOrUpdateTableOfContents: 'Insert/update contents',
  insertTable: 'Insert table',
  insertTableShort: 'Insert',
  installedDictionaries: 'Installed dictionaries',
  ignoreAll: 'Ignore all',
  ignoreOnce: 'Ignore once',
  italic: 'Italic',
  language: 'Language',
  languageTools: 'Language tools',
  largeToolbar: 'Large toolbar and controls',
  layout: 'Layout',
  layoutStructure: 'Layout & structure',
  left: 'Left',
  license: 'License',
  lineSpacing: 'Line spacing',
  link: 'Link',
  linkApplied: 'Link applied',
  linkHref: 'Link address',
  linkInvalid: 'Use a safe http, https, mailto, or #bookmark link',
  linkRemoved: 'Link removed',
  linkTarget: 'Link target',
  linkTargetJumped: 'Jumped to {target}',
  linkTargets: 'Targets',
  linkTools: 'Link tools',
  linkUnchanged: 'Link unchanged',
  linksAndBookmarks: 'Links and bookmarks',
  listLevelChanged: 'List level changed',
  lists: 'Lists',
  lowResourceMode: 'Low-resource mode',
  matchCount: '{current}/{total}',
  matchReplaced: 'Match replaced',
  matchesReplaced: '{count} match(es) replaced',
  markToggled: '{label} toggled',
  markUnavailable: '{label} unavailable',
  mainMenu: 'Main menu',
  marginModerate: 'Moderate',
  marginNarrow: 'Narrow',
  marginNormal: 'Normal',
  marginPresets: 'Margin presets',
  margins: 'Margins',
  marginWide: 'Wide',
  modified: 'Modified',
  navigator: 'Navigator',
  new: 'New',
  newFromTemplate: 'New from template',
  next: 'Next',
  nextChange: 'Next change',
  nextComment: 'Next comment',
  nextIssue: 'Next issue',
  no: 'No',
  noComments: 'No comments',
  noHeadings: 'No headings yet',
  noImageSelected: 'Select an image to edit image placement and metadata.',
  noLinkTargets: 'No headings or bookmarks with link targets',
  noObjectSelected: 'No object selected',
  noPrivacyWarnings: 'No privacy warnings.',
  noTableSelected: 'Select a table cell to edit table structure and cell formatting.',
  noTrackedChanges: 'No tracked changes',
  notAvailable: 'Not available',
  notRecording: 'Not recording',
  notes: 'Notes',
  noNotes: 'No footnotes or endnotes',
  noRecentDocuments: 'No recent documents',
  noRecoveryDrafts: 'No recovery drafts',
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
  objects: 'Objects',
  objectsWorkspace: 'Objects workspace',
  off: 'Off',
  offlineDictionaryFallback: 'Dictionary fallback used',
  open: 'Open',
  openDictionarySettings: 'Dictionary settings',
  openLinkPanel: 'Insert or edit link',
  openRecent: 'Open',
  page: 'Page',
  pageCount: 'Page count',
  pageLayoutView: 'Page Layout',
  pageNumber: 'Page number',
  pageRegionsReadOnly: 'Imported header/footer content is read-only',
  pageRegionsUpdated: 'Headers and footers updated',
  pageSetup: 'Page setup',
  paragraph: 'P',
  paragraphApplied: 'Paragraph applied',
  paragraphControls: 'Paragraph controls',
  paragraphFormatApplied: '{label} applied',
  paragraphUnchanged: 'Paragraph unchanged',
  paragraphs: 'Paragraphs',
  pageSetupUpdated: 'Page setup updated',
  pageSize: 'Page size',
  personalDictionary: 'Personal dictionary',
  personalDictionaryEmpty: 'No personal words for this dictionary.',
  personalDictionaryRefresh: 'Refresh words',
  personalDictionaryRefreshed: 'Personal dictionary refreshed',
  personalDictionaryRemove: 'Remove',
  personalDictionaryUnavailable: 'Personal dictionary is unavailable.',
  personalDictionaryWordRemoved: 'Removed word from the personal dictionary',
  personalDictionaryWords: 'Personal dictionary words',
  pdf: 'PDF',
  pdfAllPages: 'All pages',
  pdfPageRange: 'PDF pages',
  pdfPageRangeHint: 'Range needs positive whole pages, end at or after start.',
  pdfPageRangeModeRange: 'Page range',
  pdfPageRangeFrom: 'From page',
  pdfPageRangeTo: 'To page',
  previous: 'Prev',
  previousChange: 'Previous change',
  previousComment: 'Previous comment',
  previousIssue: 'Previous issue',
  print: 'Prepare print',
  printDescription: 'Sanitized WebView/browser print prep',
  printFrame: 'Sanitized print view',
  printPrepared: 'Sanitized print view prepared',
  properties: 'Properties',
  privacyWarnings: 'Privacy warnings',
  projection: 'Projection',
  quickAccess: 'Quick access',
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
  resolved: 'Resolved',
  review: 'Review',
  reviewChanges: 'Review changes',
  reviewSummary: 'Review summary',
  removeBookmark: 'Remove bookmark',
  removeBookmarkShort: 'No mark',
  removeLink: 'Remove link',
  readingMinutes: '{count} min',
  readingTime: 'Reading time',
  right: 'Right',
  ribbonTabs: 'Toolbar tabs',
  rowsShort: 'R',
  save: 'Save',
  saveAs: 'Save As...',
  saveSettings: 'Save Settings',
  saved: 'Saved',
  savedLocation: 'Saved location',
  savedStatus: 'Saved status',
  settings: 'Settings',
  resetSettings: 'Reset Settings',
  settingsReset: 'Settings reset to defaults',
  settingsUpdated: 'Settings updated',
  selectionWords: 'Selection words',
  selectedImage: 'Selected image',
  selectedTable: 'Selected table',
  showRulers: 'Rulers',
  smartDashes: 'Convert -- to em dash',
  smartListTriggers: 'Turn - and 1. starters into lists',
  smartQuotes: 'Use smart quotes',
  smartTyping: 'Smart typing',
  spellAddedToDictionary: 'Added word to the personal dictionary',
  spellIgnoredAll: 'Word ignored for this session',
  spellIgnoredOnce: 'Spelling issue ignored once',
  spellIssueCount: '{count} spelling issue(s)',
  spelling: 'Spelling',
  spellingCleared: 'Spelling marks cleared',
  spellingIssue: 'Spelling issue',
  spellingIssueSelected: 'Spelling issue selected',
  spellingIssues: 'Spelling issues',
  spellingSuggestions: 'Spelling suggestions',
  spellingSuggestionApplied: 'Spelling suggestion applied',
  starting: 'Starting...',
  startCenter: 'Start Center',
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
  tableColumnWidth: 'Column width',
  tableCellAlignInherit: 'Inherit',
  tableCellAlignment: 'Cell align',
  tableCellBackground: 'Cell fill',
  tableCellBackgroundLightBlue: 'Blue',
  tableCellBackgroundLightGray: 'Gray',
  tableCellBackgroundLightGreen: 'Green',
  tableCellBackgroundLightYellow: 'Yellow',
  tableCellBackgroundNone: 'None',
  tableCellBorder: 'Cell border',
  tableCellBorderHidden: 'Hidden',
  tableCellBorderVisible: 'Visible',
  tableCellUpdated: 'Table cell updated',
  tableDeleted: 'Table deleted',
  tableInserted: 'Table inserted',
  tableOfContents: 'Table of contents',
  tableOfContentsUpdated: 'Contents updated from Heading 1-3',
  tableRows: 'Table rows',
  tableUpdated: 'Table updated',
  tables: 'Tables',
  deleteTemplate: 'Delete Template',
  templateLoaded: 'Template loaded',
  templateDeleted: 'Template deleted',
  templateDeleteFailed: 'Template could not be deleted',
  templateGallery: 'Template gallery',
  templateName: 'Template name',
  templateSaved: 'Template saved',
  templateSaveFailed: 'Template could not be saved',
  templates: 'Templates',
  textFormatting: 'Text formatting',
  textColor: 'Text color',
  top: 'Top',
  tools: 'Tools',
  txt: 'TXT',
  uiLocale: 'UI language',
  underline: 'Underline',
  unlink: 'Unlink',
  unresolved: 'Unresolved',
  unresolvedComments: 'Unresolved comments',
  undo: 'Undo',
  undoApplied: 'Undo applied',
  updateStyle: 'Update',
  updateStyleFromSelection: 'Update style from selection',
  saveTemplate: 'Save Template',
  useTemplate: 'Use Template',
  userDictionarySuffix: 'user',
  version: 'Version',
  view: 'View',
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
  activeDictionary: 'Diccionario activo',
  activeObject: 'Objeto activo',
  blankDocument: 'Documento en blanco',
  blocks: 'Bloques',
  bold: 'Negrita',
  bottom: 'Inferior',
  capitalizeSentences: 'Poner mayuscula al iniciar oraciones',
  case: 'Mayusculas',
  characters: 'Caracteres',
  checkSpelling: 'Revisar ortografia',
  chooseExportLocation: 'Elija formato y ubicacion de exportacion.',
  clearSpelling: 'Limpiar ortografia',
  custom: 'Personalizado',
  dictionary: 'Diccionario',
  dictionaryAffFile: 'Archivo AFF',
  dictionaryAffNotSelected: 'AFF no seleccionado',
  dictionaryAffSelected: 'AFF seleccionado',
  dictionaryChooseAff: 'Elegir AFF',
  dictionaryChooseDic: 'Elegir DIC',
  dictionaryDicFile: 'Archivo DIC',
  dictionaryDicNotSelected: 'DIC no seleccionado',
  dictionaryDicSelected: 'DIC seleccionado',
  dictionaryInstall: 'Instalar',
  dictionaryInstallFailed: 'No se pudo instalar el diccionario',
  dictionaryInstallInvalidLanguage: 'La etiqueta de idioma no es valida',
  dictionaryInstallLocal: 'Instalar diccionario Hunspell local',
  dictionaryInstallSuccess: 'Diccionario instalado',
  dictionaryInstallUnsupportedFile: 'Los archivos de diccionario no son compatibles',
  dictionaryLanguageTag: 'Etiqueta de idioma',
  dictionaryLicense: 'Licencia',
  dictionaryListRefreshed: 'Lista de diccionarios actualizada',
  dictionaryManager: 'Administrador de diccionarios',
  dictionaryNoneInstalled: 'No hay diccionarios instalados.',
  dictionaryOfflineState: 'Los diccionarios son solo locales; no hay descargas, nube ni red.',
  dictionaryRefresh: 'Actualizar',
  dictionaryRemove: 'Quitar',
  dictionaryRemoveFailed: 'No se pudo quitar el diccionario',
  dictionaryRemoveInvalidLanguage: 'La etiqueta de idioma no es valida',
  dictionaryRemoveSuccess: 'Diccionario quitado',
  dictionarySourceAppBundle: 'Paquete de la app',
  dictionarySourceBundled: 'Incluido',
  dictionarySourceBundledUser: 'Incluido y usuario',
  dictionarySourceLabel: 'Origen',
  dictionarySourceLocal: 'Diccionario local',
  dictionarySourceType: 'Tipo de origen',
  dictionarySourceUser: 'Usuario',
  dictionarySourceUserFolder: 'Carpeta local de diccionarios del usuario',
  dictionaryUnavailable:
    'El diccionario seleccionado no esta instalado. La revision ortografica usara el diccionario ingles incluido cuando sea necesario.',
  dictionaryUnavailableOption: 'No disponible ({languageTag})',
  documentActions: 'Acciones del documento',
  documentFormat: 'Formato de documento',
  documentNavigation: 'Navegacion',
  documentStructure: 'Estructura del documento',
  documentOpened: 'Documento abierto',
  documentSaved: 'Documento guardado',
  documentSavedAs: 'Documento guardado como',
  dirty: 'Cambios',
  discard: 'Descartar',
  edit: 'Editar',
  editor: 'Editor',
  editorReadOnly: 'El editor es de solo lectura',
  editingToolbar: 'Barra de edicion',
  export: 'Exportar',
  exportDocxDescription: 'Conversion DOCX acotada',
  exportHtmlDescription: 'HTML sin conexion saneado',
  exportHtmlPrepared: 'Exportacion HTML preparada ({characters} caracteres)',
  exportHtmlSaved: 'Exportacion HTML escrita ({bytes} bytes)',
  exportPdfDescription: 'Instantanea PDF ligera',
  exportPdfInvalidRange: 'Ingrese un rango de paginas PDF valido.',
  exportPdfPrepared: 'Exportacion PDF preparada ({bytes} bytes)',
  exportPdfSaved: 'Exportacion PDF escrita ({bytes} bytes)',
  exportTxtDescription: 'Texto plano del documento',
  exportTxtPrepared: 'Exportacion TXT preparada ({characters} caracteres)',
  exportTxtSaved: 'Exportacion TXT escrita ({bytes} bytes)',
  file: 'Archivo',
  find: 'Buscar',
  findAndReplace: 'Buscar y reemplazar',
  findMatches: 'Coincidencias',
  format: 'Formato',
  height: 'Alto',
  help: 'Ayuda',
  headings: 'Titulos',
  highContrast: 'Alto contraste',
  home: 'Inicio',
  html: 'HTML',
  imageAlignment: 'Alineacion de imagen',
  imageAltText: 'Texto alternativo',
  imageCaption: 'Pie',
  imageControls: 'Controles de imagen',
  imageInserted: 'Imagen insertada',
  imageTooLarge: 'La imagen es demasiado grande. Comprímala o cambie su tamaño antes de insertarla.',
  imageScale: 'Escala',
  imageUpdated: 'Imagen actualizada',
  insert: 'Insertar',
  insertImage: 'Insertar imagen',
  installedDictionaries: 'Diccionarios instalados',
  italic: 'Cursiva',
  language: 'Idioma',
  languageTools: 'Herramientas de idioma',
  largeToolbar: 'Barra y controles grandes',
  layout: 'Diseno',
  layoutStructure: 'Diseno y estructura',
  left: 'Izquierda',
  linkTargetJumped: 'Saltado a {target}',
  linkTargets: 'Destinos',
  linksAndBookmarks: 'Enlaces y marcadores',
  license: 'Licencia',
  lowResourceMode: 'Modo de bajo consumo',
  mainMenu: 'Menu principal',
  marginModerate: 'Moderado',
  marginNarrow: 'Estrecho',
  marginNormal: 'Normal',
  marginPresets: 'Preajustes de margenes',
  margins: 'Margenes',
  marginWide: 'Amplio',
  new: 'Nuevo',
  newFromTemplate: 'Nuevo desde plantilla',
  next: 'Siguiente',
  nextChange: 'Cambio siguiente',
  nextComment: 'Comentario siguiente',
  nextIssue: 'Problema siguiente',
  noIssues: 'Sin problemas.',
  noHeadings: 'Aun no hay titulos',
  noImageSelected: 'Seleccione una imagen para editar ubicacion y metadatos.',
  noLinkTargets: 'No hay titulos ni marcadores con destino',
  noObjectSelected: 'No hay objeto seleccionado',
  noMatches: 'Sin coincidencias',
  noRecentDocuments: 'No hay documentos recientes',
  noRecoveryDrafts: 'No hay borradores de recuperacion',
  noTableSelected: 'Seleccione una celda de tabla para editar estructura y formato.',
  no: 'No',
  objects: 'Objetos',
  objectsWorkspace: 'Espacio de objetos',
  off: 'Apagado',
  open: 'Abrir',
  openDictionarySettings: 'Ajustes de diccionario',
  page: 'Pagina',
  pageSetupUpdated: 'Pagina actualizada',
  pageSetup: 'Configuracion de pagina',
  pageSize: 'Tamano de pagina',
  personalDictionary: 'Diccionario personal',
  personalDictionaryEmpty: 'No hay palabras personales para este diccionario.',
  personalDictionaryRefresh: 'Actualizar palabras',
  personalDictionaryRefreshed: 'Diccionario personal actualizado',
  personalDictionaryRemove: 'Quitar',
  personalDictionaryUnavailable: 'El diccionario personal no esta disponible.',
  personalDictionaryWordRemoved: 'Se quito la palabra del diccionario personal',
  personalDictionaryWords: 'Palabras del diccionario personal',
  pdf: 'PDF',
  pdfAllPages: 'Todas las paginas',
  pdfPageRange: 'Paginas PDF',
  pdfPageRangeHint: 'El rango requiere paginas enteras positivas y fin igual o despues del inicio.',
  pdfPageRangeModeRange: 'Rango de paginas',
  pdfPageRangeFrom: 'Desde pagina',
  pdfPageRangeTo: 'Hasta pagina',
  previousChange: 'Cambio anterior',
  previousComment: 'Comentario anterior',
  previousIssue: 'Problema anterior',
  print: 'Preparar impresion',
  printDescription: 'Preparacion de impresion WebView/navegador saneada',
  printFrame: 'Vista de impresion saneada',
  printPrepared: 'Vista de impresion saneada preparada',
  properties: 'Propiedades',
  quickAccess: 'Acceso rapido',
  recent: 'Recientes',
  recovery: 'Recuperacion',
  recoveryDiscarded: 'Borrador de recuperacion descartado',
  recoveryOpened: 'Borrador de recuperacion abierto',
  redo: 'Rehacer',
  reducedMotion: 'Movimiento reducido',
  replace: 'Reemplazar',
  resolved: 'Resuelto',
  review: 'Revision',
  reviewSummary: 'Resumen de revision',
  right: 'Derecha',
  ribbonTabs: 'Pestanas de barra',
  save: 'Guardar',
  saveAs: 'Guardar como...',
  saveSettings: 'Guardar ajustes',
  saveTemplate: 'Guardar plantilla',
  saved: 'Guardado',
  settings: 'Ajustes',
  resetSettings: 'Restablecer ajustes',
  settingsReset: 'Ajustes restablecidos',
  settingsUpdated: 'Ajustes guardados',
  selectedImage: 'Imagen seleccionada',
  selectedTable: 'Tabla seleccionada',
  smartDashes: 'Convertir -- en raya',
  smartListTriggers: 'Convertir - y 1. en listas',
  smartQuotes: 'Usar comillas tipograficas',
  smartTyping: 'Escritura inteligente',
  spelling: 'Ortografia',
  spellingCleared: 'Marcas de ortografia limpiadas',
  spellingIssue: 'Problema de ortografia',
  spellingIssueSelected: 'Problema de ortografia seleccionado',
  spellingIssues: 'Problemas de ortografia',
  spellingSuggestions: 'Sugerencias de ortografia',
  starting: 'Iniciando...',
  startCenter: 'Centro de inicio',
  startWriting: 'Empieza a escribir...',
  statusNoSpellingIssues: 'No se encontraron problemas de ortografia',
  stats: 'Estadisticas',
  telemetry: 'Telemetria',
  typoReplacements: 'Corregir errores comunes',
  tableCellAlignment: 'Alinear celda',
  tableCellBackground: 'Relleno de celda',
  tableCellBorder: 'Borde de celda',
  tableColumnWidth: 'Ancho de columna',
  tableCellUpdated: 'Celda actualizada',
  deleteTemplate: 'Eliminar plantilla',
  templateLoaded: 'Plantilla cargada',
  templateDeleted: 'Plantilla eliminada',
  templateDeleteFailed: 'No se pudo eliminar la plantilla',
  templateGallery: 'Galeria de plantillas',
  templateName: 'Nombre de plantilla',
  templateSaved: 'Plantilla guardada',
  templateSaveFailed: 'No se pudo guardar la plantilla',
  templates: 'Plantillas',
  top: 'Superior',
  tools: 'Herramientas',
  txt: 'TXT',
  uiLocale: 'Idioma de interfaz',
  underline: 'Subrayado',
  undo: 'Deshacer',
  unresolved: 'Sin resolver',
  useTemplate: 'Usar plantilla',
  userDictionarySuffix: 'usuario',
  version: 'Version',
  view: 'Vista',
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
