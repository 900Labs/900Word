<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { onMount, tick } from 'svelte';
  import {
    addEditorCommentToSelection,
    createEditor,
    createEditorBookmarkId,
    createEditorCommentId,
    createEditorNoteId,
    editorTopLevelInsertionIndex,
    editTableStructure,
    findEditorTextMatches,
    insertEditorNoteReference,
    insertTable as insertEditorTable,
    replaceAllEditorText,
    replaceEditorTextRange,
    removeEditorNoteReferenceFromDocument,
    removeEditorLink,
    removeEditorBlockBookmark,
    removeEditorCommentFromDocument,
    restoreEditorSelection,
    selectEditorCommentRange,
    selectEditorTrackedChangeRange,
    selectEditorTopLevelBlock,
    selectEditorTextRange,
    adjustSelectedListLevel,
    clearEditorDirectFormatting,
    setEditorBlockType,
    setEditorBlockBookmark,
    setEditorLink,
    setEditorParagraphFormat,
    setSelectedImageAttrs,
    setSelectedTableCellAttrs,
    setEditorSpellIssues,
    setEditorTextStyle,
    selectedEditorText,
    snapshotEditorFormatting,
    snapshotEditorDomSelection,
    snapshotEditorSelection,
    toggleEditorList,
    toggleEditorMark,
    type EditorFindMatch,
    type EditorFormattingSnapshot,
    type EditorSelectionSnapshot,
    type EditorSpellIssueRange,
    type SupportedImageAttrs,
    type SupportedTableCellAttrs,
    type SupportedListName,
    type SupportedMarkName,
    type SupportedParagraphAttrs,
    type SupportedTableEditAction,
    type SupportedTextStyleAttrs
  } from './lib/editor';
  import {
    buildEditorSyncCommands,
    canEditProjectedDocument,
    documentLinkTargets,
    documentLinkTargetsFromEditableBlocks,
    documentOutline,
    documentOutlineFromEditableBlocks,
    documentProjectionWarnings,
    documentToText,
    nextNoteLabelInDocument,
    noteSummariesInDocument,
    pageFieldTokens,
    pageRegionIsReadOnly,
    pageRegionTextToBlocks,
    pageRegionToText,
    trackedChangesInDocument,
    type DocumentStyle,
    type DocumentCommand,
    type DocumentState,
    type EditorProjectedChange,
    type CommentThread,
    type DocumentLinkTarget,
    type DocumentOutlineEntry,
    type NoteSummary,
    type TrackedChangeSummary,
    type NoteKind,
    type PageField,
    type PageRegionKind,
    type PageSetup
  } from './lib/documentProjection';
  import { localeDirection, translate, uiLocales, type UiStringKey } from './lib/i18n';
  import {
    identifyGlobalShortcut,
    shortcutLabel,
    shortcutLabels,
    shortcutPlatformFromNavigator,
    type GlobalShortcutCommand
  } from './lib/keyboardShortcuts';
  import { defaultSmartTypingSettings, type SmartTypingSettings } from './lib/smartTyping';
  import {
    buildExpandedDocumentStats,
    type CoreDocumentStats,
    type ExpandedDocumentStats
  } from './lib/documentStats';
  import {
    buildDocumentInspectorSummary,
    type DocumentInspectorLocationStatus,
    type DocumentInspectorPrivacyWarningKind,
    type DocumentInspectorSavedStatus,
    type DocumentInspectorSummary
  } from './lib/documentInspector';
  import {
    clampEditorZoom,
    editorViewportStyle,
    editorZoomStep,
    fitWidthZoomPercent,
    maxEditorZoom,
    minEditorZoom,
    type EditorViewMode,
    type EditorZoomChoice
  } from './lib/editorViewport';
  import {
    handleToolbarClickActivation,
    handleToolbarMouseActivation,
    handleToolbarPointerActivation,
    type ToolbarActivationState
  } from './lib/toolbarActivation';

  interface SpellIssue {
    word: string;
    byte_start: number;
    byte_end: number;
    suggestions?: string[];
  }

  interface SpellCheckResult {
    language_tag: string;
    dictionary_display_name: string;
    issues: SpellIssue[];
    warnings: string[];
  }

  interface ExportFileResult {
    format: string;
    byte_len: number;
  }

  interface PdfExportOptions {
    pageStart?: number | null;
    pageEnd?: number | null;
  }

  interface DictionaryInfo {
    language_tag: string;
    display_name: string;
    bundled: boolean;
    user: boolean;
    license: string;
    source: string;
  }

  interface Settings {
    telemetry_enabled: boolean;
    language_tag: string;
    ui_locale: string;
    high_contrast: boolean;
    large_toolbar: boolean;
    reduced_motion: boolean;
    low_resource: boolean;
    smart_typing: SmartTypingSettings;
  }

  interface RecentDocumentSummary {
    token: string;
    label: string;
    is_current: boolean;
  }

  interface RecoveryDocumentSummary {
    token: string;
    label: string;
    modified_unix_seconds: number;
    byte_len: number;
  }

  interface DocumentFileState {
    has_current_path: boolean;
    dirty: boolean;
    recent_documents: RecentDocumentSummary[];
    recovery_documents: RecoveryDocumentSummary[];
  }

  interface TemplateSummary {
    id: string;
    name: string;
    description: string;
  }

  interface PageFormat {
    id: string;
    label: string;
    width_mm: number;
    height_mm: number;
  }

  type ViewId = 'editor' | 'settings' | 'about';
  const viewOrder: ViewId[] = ['editor', 'settings', 'about'];
  const odtFileFilters = [{ name: 'OpenDocument Text', extensions: ['odt'] }];
  const documentFileFilters = [
    { name: 'Documents', extensions: ['odt', 'docx'] },
    ...odtFileFilters,
    { name: 'Word Document', extensions: ['docx'] }
  ];
  const imageFileFilters = [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp'] }];
  const pageFormats: PageFormat[] = [
    { id: 'a4', label: 'A4 (210 x 297 mm)', width_mm: 210, height_mm: 297 },
    { id: 'a3', label: 'A3 (297 x 420 mm)', width_mm: 297, height_mm: 420 },
    { id: 'a5', label: 'A5 (148 x 210 mm)', width_mm: 148, height_mm: 210 },
    { id: 'letter', label: 'US Letter (216 x 279 mm)', width_mm: 216, height_mm: 279 },
    { id: 'legal', label: 'US Legal (216 x 356 mm)', width_mm: 216, height_mm: 356 },
    { id: 'tabloid', label: 'Tabloid (279 x 432 mm)', width_mm: 279, height_mm: 432 }
  ];
  const paragraphStyles = [
    { id: 'body', label: 'Normal' },
    { id: 'title', label: 'Title' },
    { id: 'subtitle', label: 'Subtitle' },
    { id: 'heading-1', label: 'Heading 1' },
    { id: 'heading-2', label: 'Heading 2' },
    { id: 'heading-3', label: 'Heading 3' },
    { id: 'quote', label: 'Quote' },
    { id: 'code', label: 'Code' },
    { id: 'caption', label: 'Caption' }
  ];
  const fontFamilies = [
    { id: 'system-ui', label: 'System' },
    { id: 'serif', label: 'Serif' },
    { id: 'sans-serif', label: 'Sans' },
    { id: 'monospace', label: 'Mono' }
  ];
  const fontSizes = [9, 10, 11, 12, 14, 16, 18, 24, 32];
  const tableDimensions = [1, 2, 3, 4, 5, 6, 7, 8];
  const lineSpacings = [
    { id: 1000, label: '1.0' },
    { id: 1150, label: '1.15' },
    { id: 1500, label: '1.5' },
    { id: 2000, label: '2.0' }
  ];
  const imageAlignments: Array<{ id: NonNullable<SupportedImageAttrs['alignment']>; label: string }> = [
    { id: 'inline', label: 'Inline' },
    { id: 'left', label: 'Left' },
    { id: 'center', label: 'Center' },
    { id: 'right', label: 'Right' }
  ];
  const tableCellBackgrounds: Array<{ id: NonNullable<SupportedTableCellAttrs['backgroundColor']> | 'none'; labelKey: UiStringKey }> = [
    { id: 'none', labelKey: 'tableCellBackgroundNone' },
    { id: '#f1f5f9', labelKey: 'tableCellBackgroundLightGray' },
    { id: '#fff3bf', labelKey: 'tableCellBackgroundLightYellow' },
    { id: '#dbeafe', labelKey: 'tableCellBackgroundLightBlue' },
    { id: '#dcfce7', labelKey: 'tableCellBackgroundLightGreen' }
  ];
  const tableCellAlignments: Array<{ id: NonNullable<SupportedTableCellAttrs['align']> | 'inherit'; labelKey: UiStringKey }> = [
    { id: 'inherit', labelKey: 'tableCellAlignInherit' },
    { id: 'left', labelKey: 'alignLeft' },
    { id: 'center', labelKey: 'alignCenter' },
    { id: 'right', labelKey: 'alignRight' },
    { id: 'justify', labelKey: 'alignJustify' }
  ];
  const localCommentAuthor = 'Local User';
  const localTrackedChangeAuthor = 'Local User';
  const maxCommentBodyChars = 2000;
  const maxNoteBodyChars = 4000;

  let title = $state('900Word');
  let status = $state(translate('en-US', 'starting'));
  let activeView = $state<ViewId>('editor');
  let editorViewMode = $state<EditorViewMode>('page-layout');
  let zoomChoice = $state<EditorZoomChoice>('fit-width');
  let customZoomPercent = $state(100);
  let showRulers = $state(false);
  let editorViewportWidth = $state(0);
  let plainText = $state('');
  let stats = $state<CoreDocumentStats>({ word_count: 0, character_count: 0, block_count: 0 });
  let spellIssues = $state<SpellIssue[]>([]);
  let projectionWarnings = $state<string[]>([]);
  let exportPathInput = $state('');
  let pdfPageRangeMode = $state<'all' | 'range'>('all');
  let pdfPageStartInput = $state('1');
  let pdfPageEndInput = $state('1');
  let fileState = $state<DocumentFileState>({
    has_current_path: false,
    dirty: false,
    recent_documents: [],
    recovery_documents: []
  });
  let templates = $state<TemplateSummary[]>([]);
  let selectedTemplateId = $state('blank');
  let selectedStyleId = $state('body');
  let selectedFontFamily = $state('system-ui');
  let selectedFontSize = $state(12);
  let selectedTextColor = $state('#20242c');
  let selectedHighlightColor = $state('#fff3bf');
  let selectedLineSpacing = $state(1150);
  let spacingBefore = $state(0);
  let spacingAfter = $state(3);
  let firstLineIndent = $state(0);
  let activeFormatting = $state<EditorFormattingSnapshot>(emptyFormattingSnapshot());
  let imageAltText = $state('Image');
  let imageCaption = $state('');
  let imageAlignment = $state<NonNullable<SupportedImageAttrs['alignment']>>('inline');
  let imageScalePercent = $state(100);
  let tableCellBackground = $state<NonNullable<SupportedTableCellAttrs['backgroundColor']> | 'none'>('none');
  let tableCellAlignment = $state<NonNullable<SupportedTableCellAttrs['align']> | 'inherit'>('inherit');
  let tableCellBorder = $state<NonNullable<SupportedTableCellAttrs['border']>>('visible');
  let spellIssueRanges = $state<EditorSpellIssueRange[]>([]);
  let spellPopover = $state<{
    issue: EditorSpellIssueRange;
    x: number;
    y: number;
  } | null>(null);
  let statsPanelOpen = $state(false);
  let inspectorPanelOpen = $state(false);
  let ignoredSpellWords = new Set<string>();
  let ignoredSpellInstances = new Set<string>();
  let pageSetup = $state<PageSetup>(defaultPageSetup());
  let headerText = $state('');
  let footerText = $state('');
  let firstHeaderText = $state('');
  let firstFooterText = $state('');
  let differentFirstPage = $state(false);
  let findQuery = $state('');
  let replaceText = $state('');
  let findCaseSensitive = $state(false);
  let findRanges = $state<EditorFindMatch[]>([]);
  let activeFindIndex = $state(-1);
  let findPanelOpen = $state(false);
  let linkPanelOpen = $state(false);
  let linkHrefInput = $state('');
  let selectedLinkTargetId = $state('');
  let commentsPanelOpen = $state(false);
  let reviewPanelOpen = $state(false);
  let notesPanelOpen = $state(false);
  let newCommentBody = $state('');
  let selectedCommentText = $state('');
  let activeCommentId = $state<string | null>(null);
  let fileMenuOpen = $state(false);
  let exportMenuOpen = $state(false);
  let navigatorHeadings = $state<DocumentOutlineEntry[]>([]);
  let linkTargets = $state<DocumentLinkTarget[]>([]);
  let dictionaries = $state<DictionaryInfo[]>([]);
  let personalDictionaryWords = $state<string[]>([]);
  let personalDictionaryError = $state<string | null>(null);
  let installDictionaryLanguageTag = $state('en-US');
  let installDictionaryAffPath = $state<string | null>(null);
  let installDictionaryDicPath = $state<string | null>(null);
  let removingDictionaryLanguageTag = $state<string | null>(null);
  let settings = $state<Settings>({
    telemetry_enabled: false,
    language_tag: 'en-US',
    ui_locale: 'en-US',
    high_contrast: false,
    large_toolbar: false,
    reduced_motion: false,
    low_resource: false,
    smart_typing: defaultSmartTypingSettings()
  });
  let uiDirection = $derived(localeDirection(settings.ui_locale));
  let normalizedDictionaryLanguageTag = $derived(normalizeDictionaryLanguageTag(settings.language_tag));
  let selectedDictionary = $derived(
    dictionaries.find((dictionary) => dictionaryMatchesSelectedLanguage(dictionary, normalizedDictionaryLanguageTag))
  );
  let personalDictionaryLanguageTag = $derived(selectedDictionary?.language_tag ?? normalizedDictionaryLanguageTag);
  let canInstallDictionary = $derived(
    installDictionaryLanguageTag.trim().length > 0 && Boolean(installDictionaryAffPath && installDictionaryDicPath)
  );
  const shortcutPlatform = shortcutPlatformFromNavigator();
  let documentState: DocumentState | undefined;
  let editorEditable = $derived(documentState ? canEditProjectedDocument(documentState) : false);
  let commentThreads = $derived(sortedCommentThreads(documentState));
  let noteSummaries = $derived(noteSummariesInDocument(documentState));
  let trackedChanges = $derived(trackedChangesInDocument(documentState));
  let trackChangesRecording = $derived(Boolean(documentState?.track_changes?.recording));
  let unresolvedCommentCount = $derived(commentThreads.filter((comment) => !comment.resolved).length);
  let trackedChangeCount = $derived(trackedChanges.length);
  let selectionWordCount = $derived(activeFormatting.selectionWordCount);
  let expandedStats = $derived<ExpandedDocumentStats>(
    buildExpandedDocumentStats({
      coreStats: stats,
      document: documentState,
      plainText,
      selectionWordCount,
      pageSetup
    })
  );
  let inspectorSummary = $derived<DocumentInspectorSummary>(
    buildDocumentInspectorSummary({
      coreStats: stats,
      document: documentState,
      fileState,
      plainText,
      selectionWordCount
    })
  );
  let fitWidthZoom = $derived(
    editorViewMode === 'page-layout' ? fitWidthZoomPercent(pageSetup, editorViewportWidth, showRulers ? 22 : 0) : 100
  );
  let effectiveZoomPercent = $derived(zoomChoice === 'fit-width' ? fitWidthZoom : customZoomPercent);
  let editorSurfaceStyle = $derived(editorViewportStyle(pageSetup, effectiveZoomPercent));
  let showAutomaticSidebarContent = $derived(
    !settings.low_resource && (navigatorHeadings.length > 0 || fileState.recent_documents.length > 0)
  );
  let showWorkspaceSidebar = $derived(
    commentsPanelOpen ||
    notesPanelOpen ||
    reviewPanelOpen ||
      showAutomaticSidebarContent ||
      fileState.recovery_documents.length > 0 ||
      projectionWarnings.length > 0
  );
  let editorSyncQueue = Promise.resolve();
  let editorSyncError: string | null = null;
  let tableRows = $state(2);
  let tableColumns = $state(2);
  let lastEditorSelection = $state<EditorSelectionSnapshot | undefined>();
  let editorHasStarted = $state(false);
  let editorIsEmpty = $state(true);
  let editorHost: HTMLDivElement;
  let printFrame: HTMLIFrameElement;
  let findInput = $state<HTMLInputElement | undefined>();
  let replaceInput = $state<HTMLInputElement | undefined>();
  let commentBodyInput = $state<HTMLTextAreaElement | undefined>();
  let exportPathInputElement = $state<HTMLInputElement | undefined>();
  let linkInput = $state<HTMLInputElement | undefined>();
  let fileMenuRoot = $state<HTMLDivElement | undefined>();
  let linkToolsRoot = $state<HTMLDivElement | undefined>();
  let view: ReturnType<typeof createEditor> | undefined;
  const toolbarActivationState: ToolbarActivationState = { pointerCommandHandled: false };

  async function newDocument() {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('new_document');
    await loadDocumentIntoEditor(document, '');
    await refreshFileState();
  }

  async function newDocumentFromTemplate() {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('new_document_from_template', {
      templateId: selectedTemplateId
    });
    await loadDocumentIntoEditor(document, tr('templateLoaded'));
    await refreshFileState();
  }

  async function loadDocumentIntoEditor(document: DocumentState, nextStatus: string) {
    editorSyncError = null;
    documentState = document;
    if (activeCommentId && !document.comments?.[activeCommentId]) {
      activeCommentId = null;
    }
    if (!settings.low_resource && Object.keys(document.comments ?? {}).length > 0) {
      commentsPanelOpen = true;
    }
    if (!settings.low_resource && noteSummariesInDocument(document).length > 0) {
      notesPanelOpen = true;
    }
    if (!settings.low_resource && (trackedChangesInDocument(document).length > 0 || document.track_changes?.recording)) {
      reviewPanelOpen = true;
    }
    title = document.meta.title;
    plainText = documentToText(document);
    navigatorHeadings = documentOutline(document);
    linkTargets = documentLinkTargets(document);
    editorIsEmpty = plainText.trim().length === 0;
    editorHasStarted = !editorIsEmpty;
    pageSetup = document.sections[0]?.page ?? defaultPageSetup();
    const pageRegions = document.sections[0]?.page_regions ?? {};
    headerText = pageRegionToText(pageRegions.header);
    footerText = pageRegionToText(pageRegions.footer);
    firstHeaderText = pageRegionToText(pageRegions.first_header);
    firstFooterText = pageRegionToText(pageRegions.first_footer);
    differentFirstPage = Boolean(pageRegions.different_first_page);
    stats = await invoke<CoreDocumentStats>('get_document_stats');
    projectionWarnings = collectDocumentWarnings(document);
    spellIssues = [];
    spellIssueRanges = [];
    spellPopover = null;
    const editable = canEditProjectedDocument(document);
    status = editable ? nextStatus : tr('editorReadOnly');
    view?.destroy();
    view = createEditor(editorHost, document, handleEditorChange, {
      editable,
      trackChanges: {
        recording: Boolean(document.track_changes?.recording),
        author: localTrackedChangeAuthor
      },
      smartTyping: () => settings.smart_typing ?? defaultSmartTypingSettings(),
      onInteraction: markEditorStarted,
      onSelectionChange: (selection) => {
        lastEditorSelection = selection;
        refreshSelectionFormatting(selection);
      }
    });
    refreshSelectionFormatting();
    refreshFindState();
  }

  function handleEditorChange(change: EditorProjectedChange) {
    plainText = change.text;
    navigatorHeadings = documentOutlineFromEditableBlocks(change.blocks);
    linkTargets = documentLinkTargetsFromEditableBlocks(change.blocks);
    editorHasStarted = true;
    editorIsEmpty = change.text.trim().length === 0;
    refreshFindState();
    clearSpellDecorations();
    refreshSelectionFormatting();
    editorSyncError = null;
    editorSyncQueue = editorSyncQueue
      .then(() => syncEditorChange(change))
      .then(() => {
        editorSyncError = null;
      })
      .catch((error: unknown) => {
        editorSyncError = error instanceof Error ? error.message : String(error);
        status = editorSyncError;
      });
  }

  async function syncEditorChange(change: EditorProjectedChange) {
    if (!documentState || !canEditProjectedDocument(documentState)) {
      return;
    }

    const commands = buildEditorSyncCommands(documentState, change.blocks);
    if (commands.length === 0) {
      return;
    }

    let nextDocument = documentState;
    for (const command of commands) {
      nextDocument = await invoke<DocumentState>('apply_document_command', {
        command
      });
    }
    documentState = nextDocument;
    navigatorHeadings = documentOutline(nextDocument);
    linkTargets = documentLinkTargets(nextDocument);
    projectionWarnings = collectDocumentWarnings(nextDocument);
    fileState = { ...fileState, dirty: true };
    stats = await invoke<CoreDocumentStats>('get_document_stats');
  }

  async function refreshFileState() {
    fileState = await invoke<DocumentFileState>('get_document_file_state');
  }

  async function waitForEditorSync() {
    await editorSyncQueue;
    if (editorSyncError) {
      throw new Error(editorSyncError);
    }
  }

  async function openDocumentAtPath(path: string) {
    await waitForEditorSync();
    const isDocx = path.trim().toLocaleLowerCase().endsWith('.docx');
    const document = await invoke<DocumentState>(isDocx ? 'open_docx_document' : 'open_document', {
      path
    });
    await loadDocumentIntoEditor(document, isDocx ? tr('docxImported') : tr('documentOpened'));
    await refreshFileState();
  }

  async function openDocumentWithDialog() {
    const selected = await openDialog({
      multiple: false,
      filters: documentFileFilters
    });
    if (typeof selected !== 'string') {
      return;
    }
    await openDocumentAtPath(selected);
  }

  async function openRecentDocument(token: string) {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('open_recent_document', {
      token
    });
    await loadDocumentIntoEditor(document, tr('documentOpened'));
    await refreshFileState();
  }

  async function saveCurrentDocument() {
    await waitForEditorSync();
    fileState = await invoke<DocumentFileState>('save_document');
    status = tr('documentSaved');
  }

  async function saveDocumentAsPath(path: string) {
    await waitForEditorSync();
    fileState = await invoke<DocumentFileState>('save_document_as', {
      path
    });
    status = tr('documentSavedAs');
  }

  async function saveDocumentAsWithDialog() {
    const selected = await saveDialog({
      defaultPath: defaultDocumentFileName(),
      filters: odtFileFilters
    });
    if (!selected) {
      return;
    }
    await saveDocumentAsPath(selected);
  }

  async function autosaveDocument() {
    await waitForEditorSync();
    await invoke<RecoveryDocumentSummary>('autosave_document');
    await refreshFileState();
    status = tr('autosaveUpdated');
  }

  async function insertImageWithDialog() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    const selected = await openDialog({
      multiple: false,
      filters: imageFileFilters
    });
    if (typeof selected !== 'string') {
      return;
    }
    await waitForEditorSync();
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    const blockIndex = editorTopLevelInsertionIndex(view, lastEditorSelection);
    try {
      const document = await invoke<DocumentState>('import_image', {
        path: selected,
        sectionIndex: 0,
        blockIndex
      });
      await loadDocumentIntoEditor(document, tr('imageInserted'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function recoverDocument(token: string) {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('recover_document', {
      token
    });
    await loadDocumentIntoEditor(document, tr('recoveryOpened'));
    await refreshFileState();
  }

  async function discardRecovery(token: string) {
    await invoke('discard_recovery', {
      token
    });
    await refreshFileState();
    status = tr('recoveryDiscarded');
  }

  async function loadShellState() {
    settings = await invoke<Settings>('get_settings');
    installDictionaryLanguageTag = normalizeDictionaryLanguageTag(settings.language_tag);
    dictionaries = await invoke<DictionaryInfo[]>('list_dictionaries');
    await refreshPersonalDictionaryWords(false);
    templates = await invoke<TemplateSummary[]>('list_templates');
  }

  async function refreshDictionaries() {
    try {
      dictionaries = await invoke<DictionaryInfo[]>('list_dictionaries');
      await refreshPersonalDictionaryWords(false);
      status = tr('dictionaryListRefreshed');
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function chooseDictionaryAffFile() {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      filters: [{ name: tr('dictionaryAffFile'), extensions: ['aff'] }]
    });
    if (typeof selected === 'string') {
      installDictionaryAffPath = selected;
    }
  }

  async function chooseDictionaryDicFile() {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      filters: [{ name: tr('dictionaryDicFile'), extensions: ['dic'] }]
    });
    if (typeof selected === 'string') {
      installDictionaryDicPath = selected;
    }
  }

  async function installDictionary() {
    const languageTag = normalizeDictionaryLanguageTag(installDictionaryLanguageTag);
    if (!languageTag || !installDictionaryAffPath || !installDictionaryDicPath) {
      return;
    }
    try {
      const installed = await invoke<DictionaryInfo>('install_user_dictionary', {
        languageTag,
        affPath: installDictionaryAffPath,
        dicPath: installDictionaryDicPath
      });
      dictionaries = await invoke<DictionaryInfo[]>('list_dictionaries');
      settings.language_tag = installed.language_tag;
      installDictionaryLanguageTag = installed.language_tag;
      installDictionaryAffPath = null;
      installDictionaryDicPath = null;
      await refreshPersonalDictionaryWords(false);
      await checkSpelling();
      status = tr('dictionaryInstallSuccess');
    } catch (error) {
      status = dictionaryInstallErrorStatus(error);
    }
  }

  async function refreshPersonalDictionaryWords(announce = true) {
    if (personalDictionaryLanguageTag.trim().length === 0) {
      personalDictionaryWords = [];
      personalDictionaryError = tr('personalDictionaryUnavailable');
      if (announce) {
        status = tr('personalDictionaryUnavailable');
      }
      return;
    }
    try {
      personalDictionaryWords = await invoke<string[]>('list_personal_dictionary_words', {
        languageTag: personalDictionaryLanguageTag
      });
      personalDictionaryError = null;
      if (announce) {
        status = tr('personalDictionaryRefreshed');
      }
    } catch {
      personalDictionaryWords = [];
      personalDictionaryError = tr('personalDictionaryUnavailable');
      if (announce) {
        status = tr('personalDictionaryUnavailable');
      }
    }
  }

  function dictionaryInstallErrorStatus(error: unknown) {
    const message = typeof error === 'string' ? error : '';
    if (message === 'invalid language') {
      return tr('dictionaryInstallInvalidLanguage');
    }
    if (message === 'unsupported file') {
      return tr('dictionaryInstallUnsupportedFile');
    }
    return tr('dictionaryInstallFailed');
  }

  function dictionaryRemovalErrorStatus(error: unknown) {
    const message = typeof error === 'string' ? error : '';
    if (message === 'invalid language') {
      return tr('dictionaryRemoveInvalidLanguage');
    }
    return tr('dictionaryRemoveFailed');
  }

  async function handleDictionarySelectionChanged() {
    await tick();
    await refreshPersonalDictionaryWords(false);
  }

  async function removeInstalledUserDictionary(dictionary: DictionaryInfo) {
    if (!dictionary.user || dictionary.bundled) {
      return;
    }
    removingDictionaryLanguageTag = dictionary.language_tag;
    const removedActiveDictionary = dictionaryMatchesSelectedLanguage(
      dictionary,
      normalizeDictionaryLanguageTag(settings.language_tag)
    );
    try {
      await invoke('remove_user_dictionary', {
        languageTag: dictionary.language_tag
      });
      dictionaries = await invoke<DictionaryInfo[]>('list_dictionaries');
      if (removedActiveDictionary) {
        const fallbackDictionary =
          dictionaries.find((candidate) => candidate.language_tag === 'en-US' && candidate.bundled) ??
          dictionaries[0];
        settings.language_tag = fallbackDictionary?.language_tag ?? 'en-US';
      }
      await refreshPersonalDictionaryWords(false);
      await checkSpelling();
      status = tr('dictionaryRemoveSuccess');
    } catch (error) {
      status = dictionaryRemovalErrorStatus(error);
    } finally {
      removingDictionaryLanguageTag = null;
    }
  }

  async function removePersonalDictionaryWord(word: string) {
    try {
      personalDictionaryWords = await invoke<string[]>('remove_from_personal_dictionary', {
        word,
        languageTag: personalDictionaryLanguageTag
      });
      personalDictionaryError = null;
      ignoredSpellWords.delete(normalizeSpellWord(word));
      await checkSpelling();
      status = tr('personalDictionaryWordRemoved');
    } catch {
      personalDictionaryError = tr('personalDictionaryUnavailable');
      status = tr('personalDictionaryUnavailable');
    }
  }

  async function saveSettings() {
    try {
      settings = await invoke<Settings>('update_settings', {
        settings
      });
      await refreshPersonalDictionaryWords(false);
      status = tr('settingsUpdated');
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function resetSettings() {
    try {
      settings = await invoke<Settings>('reset_settings');
      await refreshPersonalDictionaryWords(false);
      status = tr('settingsReset');
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function exportText() {
    try {
      await waitForEditorSync();
      const result = await invoke<ExportFileResult>('export_txt_to_path', {
        path: exportPathInput
      });
      status = tr('exportTxtSaved', { bytes: result.byte_len });
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function exportHtml() {
    try {
      await waitForEditorSync();
      const result = await invoke<ExportFileResult>('export_html_to_path', {
        path: exportPathInput
      });
      status = tr('exportHtmlSaved', { bytes: result.byte_len });
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function exportPdf() {
    try {
      await waitForEditorSync();
      const options = pdfExportOptions();
      if (!options) {
        status = tr('exportPdfInvalidRange');
        return;
      }
      const result = await invoke<ExportFileResult>('export_pdf_to_path', {
        path: exportPathInput,
        options
      });
      status = tr('exportPdfSaved', { bytes: result.byte_len });
    } catch (error) {
      setStatusFromError(error);
    }
  }

  function pdfExportOptions(): PdfExportOptions | null {
    if (pdfPageRangeMode === 'all') {
      return {};
    }
    const pageStart = parsePositiveIntegerInput(pdfPageStartInput);
    const pageEnd = parsePositiveIntegerInput(pdfPageEndInput);
    if (pageStart === null || pageEnd === null || pageEnd < pageStart) {
      return null;
    }
    return { pageStart, pageEnd };
  }

  function parsePositiveIntegerInput(value: string): number | null {
    const trimmed = value.trim();
    if (!/^[1-9]\d*$/.test(trimmed)) {
      return null;
    }
    return Number.parseInt(trimmed, 10);
  }

  async function exportDocx() {
    try {
      await waitForEditorSync();
      const result = await invoke<ExportFileResult>('export_docx_to_path', {
        path: exportPathInput
      });
      status = tr('exportDocxSaved', { bytes: result.byte_len });
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function runExportPdfShortcut() {
    if (exportPathInput.trim().toLocaleLowerCase().endsWith('.pdf')) {
      await exportPdf();
      return;
    }
    fileMenuOpen = true;
    exportMenuOpen = true;
    status = tr('exportPdfPathRequired');
    await tick();
    exportPathInputElement?.focus();
  }

  async function printDocument() {
    try {
      await waitForEditorSync();
      const html = await invoke<string>('prepare_print_html');
      printFrame.srcdoc = html;
      await new Promise((resolve) => window.setTimeout(resolve, 75));
      printFrame.contentWindow?.focus();
      printFrame.contentWindow?.print();
      status = tr('printPrepared');
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function checkSpelling() {
    const result = await invoke<SpellCheckResult>('check_spelling', {
      text: plainText,
      languageTag: settings.language_tag
    });
    spellIssues = result.issues.filter((issue) => !spellIssueIgnored(issue));
    spellIssueRanges = setEditorSpellIssues(view, spellIssues, plainText);
    spellPopover = null;
    if (result.warnings.length > 0) {
      status = `${tr('offlineDictionaryFallback')}: ${result.dictionary_display_name}`;
    } else {
      status =
        spellIssues.length === 0
          ? tr('statusNoSpellingIssues')
          : tr('spellIssueCount', { count: spellIssues.length });
    }
  }

  function clearSpellDecorations() {
    spellIssues = [];
    spellIssueRanges = setEditorSpellIssues(view, [], plainText);
    spellPopover = null;
    ignoredSpellInstances = new Set();
  }

  function handleEditorContextMenu(event: MouseEvent) {
    if (!view || spellIssueRanges.length === 0) {
      return;
    }
    const pos = view.posAtCoords({ left: event.clientX, top: event.clientY })?.pos;
    if (pos === undefined) {
      return;
    }
    const issue = spellIssueRanges.find((candidate) => pos >= candidate.from && pos <= candidate.to);
    if (!issue) {
      spellPopover = null;
      return;
    }
    event.preventDefault();
    spellPopover = {
      issue,
      x: Math.min(event.clientX, window.innerWidth - 240),
      y: Math.min(event.clientY, window.innerHeight - 220)
    };
  }

  function replaceSpellIssue(suggestion: string) {
    if (!spellPopover) {
      return;
    }
    const issue = spellPopover.issue;
    if (replaceEditorTextRange(view, issue.from, issue.to, suggestion)) {
      spellPopover = null;
      status = tr('spellingSuggestionApplied');
    }
  }

  function ignoreSpellIssueOnce() {
    if (!spellPopover) {
      return;
    }
    ignoredSpellInstances.add(spellInstanceKey(spellPopover.issue));
    refreshVisibleSpellIssues();
    status = tr('spellIgnoredOnce');
  }

  function ignoreSpellIssueAll() {
    if (!spellPopover) {
      return;
    }
    ignoredSpellWords.add(normalizeSpellWord(spellPopover.issue.word));
    refreshVisibleSpellIssues();
    status = tr('spellIgnoredAll');
  }

  async function addSpellIssueToPersonalDictionary() {
    if (!spellPopover) {
      return;
    }
    const word = spellPopover.issue.word;
    try {
      await invoke('add_to_personal_dictionary', {
        word,
        languageTag: personalDictionaryLanguageTag
      });
      ignoredSpellWords.add(normalizeSpellWord(word));
      refreshVisibleSpellIssues();
      dictionaries = await invoke<DictionaryInfo[]>('list_dictionaries');
      await refreshPersonalDictionaryWords(false);
      status = tr('spellAddedToDictionary');
    } catch (error) {
      setStatusFromError(error);
    }
  }

  function refreshVisibleSpellIssues() {
    spellIssues = spellIssues.filter((issue) => !spellIssueIgnored(issue));
    spellIssueRanges = setEditorSpellIssues(view, spellIssues, plainText);
    spellPopover = null;
  }

  function spellIssueIgnored(issue: SpellIssue) {
    return ignoredSpellWords.has(normalizeSpellWord(issue.word)) || ignoredSpellInstances.has(spellInstanceKey(issue));
  }

  function spellInstanceKey(issue: Pick<SpellIssue, 'word' | 'byte_start' | 'byte_end'>) {
    return `${normalizeSpellWord(issue.word)}:${issue.byte_start}:${issue.byte_end}`;
  }

  function normalizeSpellWord(word: string) {
    return word.trim().replace(/^'+|'+$/g, '').toLocaleLowerCase();
  }

  function normalizeDictionaryLanguageTag(languageTag: string) {
    return languageTag.trim().replaceAll('_', '-');
  }

  function dictionaryMatchesSelectedLanguage(dictionary: DictionaryInfo, normalizedLanguageTag: string) {
    return (
      dictionary.language_tag === normalizedLanguageTag ||
      (normalizedLanguageTag === 'en' && dictionary.language_tag === 'en-US')
    );
  }

  function dictionarySourceTypeLabel(dictionary: DictionaryInfo) {
    if (dictionary.bundled && dictionary.user) {
      return tr('dictionarySourceBundledUser');
    }
    if (dictionary.user) {
      return tr('dictionarySourceUser');
    }
    if (dictionary.bundled) {
      return tr('dictionarySourceBundled');
    }
    return tr('dictionarySourceLocal');
  }

  function dictionarySourceLabel(dictionary: DictionaryInfo) {
    if (dictionary.bundled && dictionary.user) {
      return tr('dictionarySourceLocal');
    }
    if (dictionary.user) {
      return tr('dictionarySourceUserFolder');
    }
    if (dictionary.bundled) {
      return tr('dictionarySourceAppBundle');
    }
    return tr('dictionarySourceLocal');
  }

  function applyInlineMark(mark: SupportedMarkName, label: string) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = toggleEditorMark(view, mark, lastEditorSelection)
      ? tr('markToggled', { label })
      : tr('markUnavailable', { label });
  }

  function runToolbarPointerCommand(event: PointerEvent, command: () => void) {
    if (handleToolbarPointerActivation(event, toolbarActivationState, (callback) => window.setTimeout(callback, 0))) {
      runToolbarCommand(command);
    }
  }

  function runToolbarMouseCommand(event: MouseEvent, command: () => void) {
    if (handleToolbarMouseActivation(event, toolbarActivationState)) {
      runToolbarCommand(command);
    }
  }

  function runToolbarKeyboardCommand(event: MouseEvent, command: () => void) {
    if (handleToolbarClickActivation(event)) {
      runToolbarCommand(command);
    }
  }

  function runToolbarCommand(command: () => void) {
    captureToolbarSelection(true);
    restoreEditorSelection(view, lastEditorSelection);
    command();
  }

  function captureToolbarSelection(includeEmpty = false) {
    const selection = snapshotEditorDomSelection(view) ?? (view ? snapshotEditorSelection(view) : undefined);
    if (selection && (includeEmpty || !selection.empty)) {
      lastEditorSelection = selection;
    }
  }

  function refreshSelectionFormatting(selection = lastEditorSelection) {
    activeFormatting = snapshotEditorFormatting(view, selection);
    selectedStyleId = paragraphStyles.some((style) => style.id === activeFormatting.styleId)
      ? activeFormatting.styleId
      : 'body';
    selectedFontFamily = activeFormatting.textStyle.fontFamily ?? 'system-ui';
    selectedFontSize = activeFormatting.textStyle.fontSizePt ?? 12;
    selectedTextColor = activeFormatting.textStyle.textColor ?? '#20242c';
    selectedHighlightColor = activeFormatting.textStyle.highlightColor ?? '#fff3bf';
    selectedLineSpacing = activeFormatting.paragraphFormat.lineSpacing ?? 1150;
    spacingBefore = activeFormatting.paragraphFormat.spacingBefore ?? 0;
    spacingAfter = activeFormatting.paragraphFormat.spacingAfter ?? 3;
    firstLineIndent = activeFormatting.paragraphFormat.firstLineIndent ?? 0;
    imageAltText = activeFormatting.image?.altText ?? 'Image';
    imageCaption = activeFormatting.image?.caption ?? '';
    imageAlignment = activeFormatting.image?.alignment ?? 'inline';
    imageScalePercent = activeFormatting.image?.scalePercent ?? 100;
    tableCellBackground = activeFormatting.table?.cell.backgroundColor ?? 'none';
    tableCellAlignment = activeFormatting.table?.cell.align ?? 'inherit';
    tableCellBorder = activeFormatting.table?.cell.border ?? 'visible';
    selectedCommentText = selectedEditorText(view, selection).trim();
  }

  function emptyFormattingSnapshot(): EditorFormattingSnapshot {
    return {
      blockType: null,
      styleId: 'body',
      paragraphFormat: {},
      textStyle: {},
      marks: {
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
        superscript: false,
        subscript: false
      },
      linkHref: null,
      blockBookmarkId: null,
      list: null,
      table: null,
      image: null,
      selectionWordCount: 0
    };
  }

  function applyImageAttrs(attrs: SupportedImageAttrs) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    const changed = setSelectedImageAttrs(view, attrs, lastEditorSelection);
    refreshSelectionFormatting();
    status = changed ? tr('imageUpdated') : tr('paragraphUnchanged');
  }

  function setImageScale(value: number) {
    imageScalePercent = Math.min(200, Math.max(25, Math.round(value)));
    applyImageAttrs({ scalePercent: imageScalePercent });
  }

  function setImageAlignment(value: string) {
    if (value === 'inline' || value === 'left' || value === 'center' || value === 'right') {
      imageAlignment = value;
      applyImageAttrs({ alignment: value });
    }
  }

  function applyTableCellAttrs(attrs: SupportedTableCellAttrs) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    const changed = setSelectedTableCellAttrs(view, attrs, lastEditorSelection);
    refreshSelectionFormatting();
    status = changed ? tr('tableCellUpdated') : tr('paragraphUnchanged');
  }

  function setTableCellBackground(value: string) {
    if (
      value === 'none' ||
      value === '#f1f5f9' ||
      value === '#fff3bf' ||
      value === '#dbeafe' ||
      value === '#dcfce7'
    ) {
      tableCellBackground = value;
      applyTableCellAttrs({ backgroundColor: value === 'none' ? null : value });
    }
  }

  function setTableCellAlignment(value: string) {
    if (value === 'inherit' || value === 'left' || value === 'center' || value === 'right' || value === 'justify') {
      tableCellAlignment = value;
      applyTableCellAttrs({ align: value === 'inherit' ? null : value });
    }
  }

  function setTableCellBorder(value: string) {
    if (value === 'visible' || value === 'hidden') {
      tableCellBorder = value;
      applyTableCellAttrs({ border: value });
    }
  }

  function markEditorStarted() {
    editorHasStarted = true;
  }

  async function updateStyleFromSelection() {
    if (!editorEditable || !documentState) {
      status = tr('editorReadOnly');
      return;
    }

    captureToolbarSelection(true);
    refreshSelectionFormatting();
    if (activeFormatting.blockType !== 'paragraph') {
      status = tr('styleUpdateNeedsParagraph');
      return;
    }

    try {
      await waitForEditorSync();
      const styleId = activeFormatting.styleId || 'body';
      const existing = documentState.styles?.[styleId];
      const style: DocumentStyle = {
        id: styleId,
        name: existing?.name ?? styleLabel(styleId),
        kind: 'Paragraph',
        parent: existing?.parent ?? null,
        properties: {
          ...(existing?.properties ?? {}),
          paragraph: paragraphAttrsToWordCoreFormat(activeFormatting.paragraphFormat)
        }
      };
      const document = await invoke<DocumentState>('apply_document_command', {
        command: {
          type: 'update_style',
          style
        }
      });
      await loadDocumentIntoEditor(document, tr('styleUpdated', { style: style.name }));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  function paragraphAttrsToWordCoreFormat(attrs: SupportedParagraphAttrs) {
    const format: Record<string, string | number | null> = {};
    if (attrs.align !== undefined && attrs.align !== null) format.alignment = attrs.align;
    if (attrs.lineSpacing !== undefined && attrs.lineSpacing !== null) {
      format.line_spacing_per_mille = attrs.lineSpacing;
    }
    if (attrs.spacingBefore !== undefined && attrs.spacingBefore !== null) {
      format.spacing_before_mm = attrs.spacingBefore;
    }
    if (attrs.spacingAfter !== undefined && attrs.spacingAfter !== null) {
      format.spacing_after_mm = attrs.spacingAfter;
    }
    if (attrs.indentStart !== undefined && attrs.indentStart !== null) {
      format.indent_start_mm = attrs.indentStart;
    }
    if (attrs.indentEnd !== undefined && attrs.indentEnd !== null) {
      format.indent_end_mm = attrs.indentEnd;
    }
    if (attrs.firstLineIndent !== undefined && attrs.firstLineIndent !== null) {
      format.first_line_indent_mm = attrs.firstLineIndent;
    }
    return Object.keys(format).length > 0 ? format : null;
  }

  function styleLabel(styleId: string) {
    return paragraphStyles.find((style) => style.id === styleId)?.label ?? styleId;
  }

  function applyParagraph() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = setEditorBlockType(view, 'paragraph', { style: 'body' }, lastEditorSelection)
      ? tr('paragraphApplied')
      : tr('paragraphUnchanged');
  }

  function applyParagraphStyle(styleId: string) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    selectedStyleId = styleId;
    if (styleId === 'heading-1' || styleId === 'heading-2' || styleId === 'heading-3') {
      applyHeading(Number(styleId.slice(-1)));
      return;
    }
    status = setEditorBlockType(view, 'paragraph', { style: styleId }, lastEditorSelection)
      ? tr('styleApplied', { style: paragraphStyles.find((style) => style.id === styleId)?.label ?? styleId })
      : tr('styleUnchanged');
  }

  function applyHeading(level: number) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = setEditorBlockType(view, 'heading', { level }, lastEditorSelection)
      ? tr('headingApplied', { level })
      : tr('headingUnchanged', { level });
  }

  function applyTextStyle(attrs: SupportedTextStyleAttrs, label: string) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = setEditorTextStyle(view, attrs, lastEditorSelection)
      ? tr('styleApplied', { style: label })
      : tr('styleUnchanged');
  }

  function applyParagraphFormat(attrs: SupportedParagraphAttrs, label: string) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = setEditorParagraphFormat(view, attrs, lastEditorSelection)
      ? tr('paragraphFormatApplied', { label })
      : tr('paragraphUnchanged');
  }

  function clearFormatting() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = clearEditorDirectFormatting(view, lastEditorSelection)
      ? tr('formattingCleared')
      : tr('paragraphUnchanged');
  }

  function applyList(listName: SupportedListName) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = toggleEditorList(view, listName, lastEditorSelection)
      ? listName === 'ordered_list'
        ? tr('numberedListApplied')
        : tr('bulletListApplied')
      : tr('paragraphUnchanged');
  }

  function adjustListLevel(delta: number) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = adjustSelectedListLevel(view, delta, lastEditorSelection)
      ? tr('listLevelChanged')
      : tr('paragraphUnchanged');
  }

  function insertTable() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = insertEditorTable(view, tableRows, tableColumns, lastEditorSelection)
      ? tr('tableInserted')
      : tr('paragraphUnchanged');
  }

  async function insertOrUpdateTableOfContents() {
    if (!documentState) {
      return;
    }
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    try {
      await waitForEditorSync();
      const existingIndex = tableOfContentsBlockIndex(documentState);
      const insertionIndex =
        existingIndex >= 0 ? existingIndex : editorTopLevelInsertionIndex(view, lastEditorSelection) ?? 0;
      const document = await invoke<DocumentState>('apply_document_command', {
        command: {
          type: 'insert_or_update_table_of_contents',
          section_index: 0,
          block_index: insertionIndex
        }
      });
      await loadDocumentIntoEditor(document, tr('tableOfContentsUpdated'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  function tableOfContentsBlockIndex(document: DocumentState): number {
    return document.sections[0]?.blocks.findIndex((block) => block.type === 'TableOfContents') ?? -1;
  }

  function editTable(action: SupportedTableEditAction) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    const changed = editTableStructure(view, action, lastEditorSelection);
    refreshSelectionFormatting();
    status = changed ? tr(action === 'delete_table' ? 'tableDeleted' : 'tableUpdated') : tr('paragraphUnchanged');
  }

  async function openLinkPanel() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    captureToolbarSelection(true);
    restoreEditorSelection(view, lastEditorSelection);
    refreshSelectionFormatting();
    linkHrefInput = activeFormatting.linkHref ?? '';
    selectedLinkTargetId = linkHrefInput.startsWith('#') ? linkHrefInput.slice(1) : '';
    linkPanelOpen = true;
    await tick();
    linkInput?.focus();
    linkInput?.select();
  }

  function applyLinkFromPanel() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    if (linkHrefInput.trim().startsWith('#') && !linkTargets.some((target) => `#${target.id}` === linkHrefInput.trim())) {
      status = tr('linkInvalid');
      return;
    }
    const changed = setEditorLink(view, linkHrefInput, lastEditorSelection);
    if (changed) {
      linkPanelOpen = false;
      refreshSelectionFormatting();
      status = tr('linkApplied');
    } else {
      status = tr('linkInvalid');
    }
  }

  function applyLinkTarget(targetId: string) {
    selectedLinkTargetId = targetId;
    linkHrefInput = targetId ? `#${targetId}` : '';
  }

  function addBookmarkToSelection() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    const bookmarkId = activeFormatting.blockBookmarkId ?? createEditorBookmarkId();
    const changed = setEditorBlockBookmark(view, bookmarkId, lastEditorSelection);
    refreshSelectionFormatting();
    status = changed ? tr('bookmarkAdded') : tr('paragraphUnchanged');
  }

  function removeBookmarkFromSelection() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    const changed = removeEditorBlockBookmark(view, lastEditorSelection);
    refreshSelectionFormatting();
    status = changed ? tr('bookmarkRemoved') : tr('paragraphUnchanged');
  }

  function removeLinkFromSelection() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    const changed = removeEditorLink(view, lastEditorSelection);
    linkPanelOpen = false;
    refreshSelectionFormatting();
    status = changed ? tr('linkRemoved') : tr('linkUnchanged');
  }

  async function insertNote(kind: NoteKind) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    captureToolbarSelection(false);
    const promptLabel = kind === 'footnote' ? tr('footnotePrompt') : tr('endnotePrompt');
    const body = globalThis.prompt?.(promptLabel, '')?.trim();
    if (body === undefined) {
      return;
    }
    if (body.length === 0) {
      status = tr('noteBodyRequired');
      return;
    }
    if (Array.from(body).length > maxNoteBodyChars) {
      status = tr('noteBodyTooLong', { max: maxNoteBodyChars });
      return;
    }

    const id = createEditorNoteId();
    const label = nextNoteLabel(kind);
    if (!insertEditorNoteReference(view, id, kind, label, lastEditorSelection)) {
      status = tr('noteInsertionUnavailable');
      return;
    }

    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('apply_document_command', {
        command: {
          type: 'add_note',
          id,
          kind,
          body
        }
      });
      await loadDocumentIntoEditor(document, kind === 'footnote' ? tr('footnoteInserted') : tr('endnoteInserted'));
      await refreshFileState();
    } catch (error) {
      if (removeEditorNoteReferenceFromDocument(view, id)) {
        await waitForEditorSync().catch(() => undefined);
      }
      setStatusFromError(error);
    }
  }

  function nextNoteLabel(kind: NoteKind) {
    return nextNoteLabelInDocument(documentState, kind);
  }

  function noteKindLabel(note: NoteSummary) {
    return note.kind === 'footnote' ? tr('insertFootnote') : tr('insertEndnote');
  }

  async function addCommentToSelection() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    captureToolbarSelection(false);
    const selectedText = selectedEditorText(view, lastEditorSelection).trim();
    const body = newCommentBody.trim();
    if (selectedText.length === 0) {
      status = tr('commentSelectionRequired');
      return;
    }
    if (body.length === 0) {
      status = tr('commentBodyRequired');
      return;
    }
    if (Array.from(body).length > maxCommentBodyChars) {
      status = tr('commentBodyTooLong', { max: maxCommentBodyChars });
      return;
    }

    const id = createEditorCommentId();
    if (!addEditorCommentToSelection(view, id, lastEditorSelection)) {
      status = tr('commentSelectionRequired');
      return;
    }

    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('apply_document_command', {
        command: {
          type: 'add_comment',
          id,
          author: localCommentAuthor,
          body
        }
      });
      newCommentBody = '';
      activeCommentId = id;
      commentsPanelOpen = true;
      await loadDocumentIntoEditor(document, tr('commentAdded'));
      await refreshFileState();
    } catch (error) {
      removeEditorCommentFromDocument(view, id);
      await waitForEditorSync().catch(() => undefined);
      setStatusFromError(error);
    }
  }

  async function openCommentsPanelForShortcut() {
    if (!editorEditable && commentThreads.length === 0) {
      status = tr('editorReadOnly');
      return;
    }
    activeView = 'editor';
    commentsPanelOpen = true;
    captureToolbarSelection(true);
    refreshSelectionFormatting(lastEditorSelection);
    await tick();
    if (editorEditable) {
      commentBodyInput?.focus();
      if (selectedCommentText.length === 0) {
        status = tr('commentSelectionRequired');
      }
    }
  }

  async function setCommentResolved(comment: CommentThread, resolved: boolean) {
    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('apply_document_command', {
        command: {
          type: 'set_comment_resolved',
          id: comment.id,
          resolved
        }
      });
      activeCommentId = comment.id;
      await loadDocumentIntoEditor(document, resolved ? tr('commentResolved') : tr('commentReopened'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function deleteComment(comment: CommentThread) {
    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('apply_document_command', {
        command: {
          type: 'delete_comment',
          id: comment.id
        }
      });
      activeCommentId = null;
      await loadDocumentIntoEditor(document, tr('commentDeleted'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  function jumpToComment(comment: CommentThread) {
    activeView = 'editor';
    commentsPanelOpen = true;
    activeCommentId = comment.id;
    status = selectEditorCommentRange(view, comment.id) ? tr('commentSelected') : tr('noMatches');
  }

  async function setTrackChangesRecording(enabled: boolean) {
    if (!documentState || !editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('apply_document_command', {
        command: {
          type: 'set_track_changes_recording',
          enabled
        }
      });
      reviewPanelOpen = true;
      await loadDocumentIntoEditor(document, enabled ? tr('trackChangesRecordingOn') : tr('trackChangesRecordingOff'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function acceptTrackedChange(change: TrackedChangeSummary) {
    await resolveTrackedChangeCommand({ type: 'accept_tracked_change', id: change.id }, tr('trackedChangeAccepted'));
  }

  async function rejectTrackedChange(change: TrackedChangeSummary) {
    await resolveTrackedChangeCommand({ type: 'reject_tracked_change', id: change.id }, tr('trackedChangeRejected'));
  }

  async function acceptAllTrackedChanges() {
    await resolveTrackedChangeCommand({ type: 'accept_all_tracked_changes' }, tr('allTrackedChangesAccepted'));
  }

  async function rejectAllTrackedChanges() {
    await resolveTrackedChangeCommand({ type: 'reject_all_tracked_changes' }, tr('allTrackedChangesRejected'));
  }

  async function resolveTrackedChangeCommand(command: DocumentCommand, nextStatus: string) {
    if (!documentState || !editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('apply_document_command', { command });
      await loadDocumentIntoEditor(document, nextStatus);
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  function jumpToTrackedChange(change: TrackedChangeSummary) {
    activeView = 'editor';
    reviewPanelOpen = true;
    status = selectEditorTrackedChangeRange(view, change.id) ? tr('trackedChangeSelected') : tr('noMatches');
  }

  function trackedChangeTimestamp(change: TrackedChangeSummary) {
    const parsed = Date.parse(change.created_at);
    if (!Number.isFinite(parsed)) {
      return '';
    }
    return new Intl.DateTimeFormat(settings.ui_locale, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(parsed);
  }

  function trackedChangePreview(change: TrackedChangeSummary) {
    const text = change.text.replace(/\s+/g, ' ').trim();
    return text.length > 80 ? `${text.slice(0, 77)}...` : text || tr('emptyChangeText');
  }

  function jumpToHeading(entry: DocumentOutlineEntry) {
    activeView = 'editor';
    status = selectEditorTopLevelBlock(view, entry.editorBlockIndex)
      ? tr('headingJumped', { heading: entry.text })
      : tr('noMatches');
  }

  function setSpacingField(field: 'spacingBefore' | 'spacingAfter' | 'firstLineIndent', value: number) {
    if (!Number.isFinite(value)) {
      return;
    }
    const normalized = Math.trunc(value);
    if (field === 'spacingBefore') {
      spacingBefore = normalized;
      applyParagraphFormat({ spacingBefore: normalized }, tr('spacingBefore'));
    } else if (field === 'spacingAfter') {
      spacingAfter = normalized;
      applyParagraphFormat({ spacingAfter: normalized }, tr('spacingAfter'));
    } else {
      firstLineIndent = normalized;
      applyParagraphFormat({ firstLineIndent: normalized }, tr('firstLineIndent'));
    }
  }

  async function undoDocument() {
    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('undo');
      await loadDocumentIntoEditor(document, tr('undoApplied'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function redoDocument() {
    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('redo');
      await loadDocumentIntoEditor(document, tr('redoApplied'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  function refreshFindState() {
    findRanges = findEditorTextMatches(view, findQuery, findCaseSensitive);
    if (findRanges.length === 0) {
      activeFindIndex = -1;
    } else if (activeFindIndex < 0 || activeFindIndex >= findRanges.length) {
      activeFindIndex = 0;
    }
  }

  function selectFindMatch(index: number) {
    refreshFindState();
    if (findRanges.length === 0) {
      status = tr('noMatches');
      return;
    }
    activeFindIndex = (index + findRanges.length) % findRanges.length;
    const range = findRanges[activeFindIndex];
    selectEditorTextRange(view, range.from, range.to);
    status = tr('matchCount', { current: activeFindIndex + 1, total: findRanges.length });
  }

  function findNext() {
    selectFindMatch(activeFindIndex + 1);
  }

  function findPrevious() {
    selectFindMatch(activeFindIndex - 1);
  }

  async function openFindPanel(focusTarget: 'find' | 'replace' = 'find') {
    findPanelOpen = true;
    await tick();
    const input = focusTarget === 'replace' ? replaceInput : findInput;
    input?.focus();
    input?.select();
  }

  function toggleFindPanel() {
    if (findPanelOpen) {
      findPanelOpen = false;
      view?.focus();
    } else {
      openFindPanel('find');
    }
  }

  function closeFileMenu() {
    fileMenuOpen = false;
    exportMenuOpen = false;
  }

  function toggleDocumentInspector() {
    inspectorPanelOpen = !inspectorPanelOpen;
    if (inspectorPanelOpen) {
      statsPanelOpen = false;
    }
  }

  function openDocumentInspector() {
    inspectorPanelOpen = true;
    statsPanelOpen = false;
  }

  function toggleStatsPanel() {
    statsPanelOpen = !statsPanelOpen;
    if (statsPanelOpen) {
      inspectorPanelOpen = false;
    }
  }

  function toggleFileMenu() {
    fileMenuOpen = !fileMenuOpen;
    if (!fileMenuOpen) {
      exportMenuOpen = false;
    }
  }

  function runFileMenuAction(action: () => void | Promise<void>) {
    closeFileMenu();
    Promise.resolve(action()).catch(setStatusFromError);
  }

  function selectView(viewId: ViewId) {
    activeView = viewId;
    closeFileMenu();
  }

  function inspectorSavedStatusLabel(status: DocumentInspectorSavedStatus) {
    switch (status) {
      case 'saved':
        return tr('inspectorSaved');
      case 'saved_with_unsaved_changes':
        return tr('inspectorSavedWithUnsavedChanges');
      case 'unsaved':
        return tr('inspectorUnsaved');
    }
  }

  function inspectorLocationStatusLabel(status: DocumentInspectorLocationStatus) {
    return status === 'backend_only' ? tr('inspectorBackendOnlyLocation') : tr('inspectorNoSavedLocation');
  }

  function inspectorPrivacyWarningLabel(kind: DocumentInspectorPrivacyWarningKind) {
    switch (kind) {
      case 'comments':
        return tr('inspectorPrivacyComments');
      case 'tracked_changes':
        return tr('inspectorPrivacyTrackedChanges');
      case 'metadata':
        return tr('inspectorPrivacyMetadata');
      case 'recovery':
        return tr('inspectorPrivacyRecovery');
      case 'unsaved':
        return tr('inspectorPrivacyUnsaved');
    }
  }

  function setEditorViewMode(mode: EditorViewMode) {
    editorViewMode = mode;
  }

  function setZoomChoice(value: string) {
    if (value === 'fit-width') {
      zoomChoice = 'fit-width';
    } else if (value === '100') {
      customZoomPercent = 100;
      zoomChoice = '100';
    } else {
      zoomChoice = 'custom';
    }
  }

  function setCustomZoom(value: number) {
    if (!Number.isFinite(value)) {
      return;
    }
    customZoomPercent = clampEditorZoom(value);
    zoomChoice = 'custom';
  }

  function handleWindowClick(event: MouseEvent) {
    const target = event.target;
    if (!(target instanceof Node)) {
      return;
    }
    if (fileMenuOpen && !fileMenuRoot?.contains(target)) {
      closeFileMenu();
    }
    if (linkPanelOpen && !linkToolsRoot?.contains(target)) {
      linkPanelOpen = false;
    }
    if (spellPopover && target instanceof Element && !target.closest('.spell-popover')) {
      spellPopover = null;
    }
  }

  function replaceCurrentMatch() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    refreshFindState();
    if (findRanges.length === 0 || activeFindIndex < 0) {
      status = tr('noMatches');
      return;
    }
    const range = findRanges[activeFindIndex];
    if (replaceEditorTextRange(view, range.from, range.to, replaceText)) {
      refreshFindState();
      status = tr('matchReplaced');
    }
  }

  function replaceAllMatches() {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    refreshFindState();
    const ranges = [...findRanges];
    if (replaceAllEditorText(view, ranges, replaceText)) {
      refreshFindState();
      status = tr('matchesReplaced', { count: ranges.length });
    } else {
      status = tr('noMatches');
    }
  }

  async function applyPageSetup() {
    try {
      await waitForEditorSync();
      const document = await invoke<DocumentState>('apply_document_command', {
        command: {
          type: 'update_page_setup',
          section_index: 0,
          page: pageSetup
        }
      });
      await loadDocumentIntoEditor(document, tr('pageSetupUpdated'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  async function applyPageRegions() {
    if (!documentState) {
      return;
    }
    if (pageRegionsReadOnly()) {
      status = tr('pageRegionsReadOnly');
      return;
    }

    try {
      await waitForEditorSync();
      let document = documentState;
      const commands: DocumentCommand[] = [
        {
          type: 'update_page_region',
          section_index: 0,
          region: 'header',
          blocks: pageRegionTextToBlocks(headerText)
        },
        {
          type: 'update_page_region',
          section_index: 0,
          region: 'footer',
          blocks: pageRegionTextToBlocks(footerText)
        },
        {
          type: 'update_page_region',
          section_index: 0,
          region: 'first_header',
          blocks: pageRegionTextToBlocks(firstHeaderText)
        },
        {
          type: 'update_page_region',
          section_index: 0,
          region: 'first_footer',
          blocks: pageRegionTextToBlocks(firstFooterText)
        },
        {
          type: 'set_different_first_page',
          section_index: 0,
          enabled: differentFirstPage
        }
      ];
      for (const command of commands) {
        document = await invoke<DocumentState>('apply_document_command', {
          command
        });
      }
      await loadDocumentIntoEditor(document, tr('pageRegionsUpdated'));
      await refreshFileState();
    } catch (error) {
      setStatusFromError(error);
    }
  }

  function insertPageFieldToken(region: PageRegionKind, field: PageField) {
    const token = pageFieldTokens[field];
    if (region === 'header') {
      headerText += token;
    } else if (region === 'footer') {
      footerText += token;
    } else if (region === 'first_header') {
      firstHeaderText += token;
    } else {
      firstFooterText += token;
    }
  }

  function pageRegionsReadOnly() {
    const regions = documentState?.sections[0]?.page_regions;
    return (
      pageRegionIsReadOnly(regions?.header) ||
      pageRegionIsReadOnly(regions?.footer) ||
      pageRegionIsReadOnly(regions?.first_header) ||
      pageRegionIsReadOnly(regions?.first_footer)
    );
  }

  function updatePageSetupField(field: keyof PageSetup, value: number) {
    if (!Number.isFinite(value)) {
      return;
    }
    pageSetup = { ...pageSetup, [field]: Math.trunc(value) };
  }

  function updatePageFormat(formatId: string) {
    const format = pageFormats.find((candidate) => candidate.id === formatId);
    if (!format) {
      return;
    }
    pageSetup = {
      ...pageSetup,
      width_mm: format.width_mm,
      height_mm: format.height_mm
    };
  }

  function currentPageFormatId() {
    return (
      pageFormats.find(
        (format) => format.width_mm === pageSetup.width_mm && format.height_mm === pageSetup.height_mm
      )?.id ?? 'custom'
    );
  }

  function sortedCommentThreads(document: DocumentState | undefined): CommentThread[] {
    return Object.values(document?.comments ?? {}).sort((left, right) => {
      if (left.resolved !== right.resolved) {
        return left.resolved ? 1 : -1;
      }
      return left.created_at.localeCompare(right.created_at);
    });
  }

  function commentTimestamp(comment: CommentThread) {
    const parsed = Date.parse(comment.created_at);
    if (!Number.isFinite(parsed)) {
      return '';
    }
    return new Intl.DateTimeFormat(settings.ui_locale, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    }).format(parsed);
  }

  function defaultDocumentFileName() {
    const cleaned = title
      .replace(/[\\/:*?"<>|]+/g, ' ')
      .replace(/\s+/g, ' ')
      .trim();
    const baseName = cleaned.length > 0 ? cleaned : 'Untitled Document';
    return baseName.toLowerCase().endsWith('.odt') ? baseName : `${baseName}.odt`;
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && linkPanelOpen) {
      event.preventDefault();
      linkPanelOpen = false;
      view?.focus();
      return;
    }

    if (event.key === 'Escape' && fileMenuOpen) {
      event.preventDefault();
      closeFileMenu();
      return;
    }

    const command = identifyGlobalShortcut(event);
    if (!command) {
      return;
    }

    event.preventDefault();
    switch (command) {
      case 'newDocument':
        newDocument().catch(setStatusFromError);
        break;
      case 'openDocument':
        openDocumentWithDialog().catch(setStatusFromError);
        break;
      case 'saveDocument':
        saveCurrentDocument().catch(setStatusFromError);
        break;
      case 'saveDocumentAs':
        saveDocumentAsWithDialog().catch(setStatusFromError);
        break;
      case 'printDocument':
        printDocument().catch(setStatusFromError);
        break;
      case 'undo':
        void undoDocument();
        break;
      case 'redo':
        void redoDocument();
        break;
      case 'bold':
        applyInlineMark('bold', tr('bold'));
        break;
      case 'italic':
        applyInlineMark('italic', tr('italic'));
        break;
      case 'underline':
        applyInlineMark('underline', tr('underline'));
        break;
      case 'find':
        activeView = 'editor';
        void openFindPanel('find');
        break;
      case 'replace':
        activeView = 'editor';
        void openFindPanel('replace');
        break;
      case 'heading1':
        applyHeading(1);
        break;
      case 'heading2':
        applyHeading(2);
        break;
      case 'heading3':
        applyHeading(3);
        break;
      case 'insertLink':
        void openLinkPanel();
        break;
      case 'insertComment':
        openCommentsPanelForShortcut().catch(setStatusFromError);
        break;
      case 'bulletList':
        applyList('bullet_list');
        break;
      case 'numberedList':
        applyList('ordered_list');
        break;
      case 'increaseIndent':
        adjustListLevel(1);
        break;
      case 'decreaseIndent':
        adjustListLevel(-1);
        break;
      case 'exportPdf':
        runExportPdfShortcut().catch(setStatusFromError);
        break;
    }
  }

  function handleViewTabKeydown(event: KeyboardEvent, viewId: ViewId) {
    const current = viewOrder.indexOf(viewId);
    let next = current;
    if (event.key === 'ArrowRight') {
      next = (current + 1) % viewOrder.length;
    } else if (event.key === 'ArrowLeft') {
      next = (current - 1 + viewOrder.length) % viewOrder.length;
    } else if (event.key === 'Home') {
      next = 0;
    } else if (event.key === 'End') {
      next = viewOrder.length - 1;
    } else {
      return;
    }

    event.preventDefault();
    selectView(viewOrder[next]);
    queueMicrotask(() => document.getElementById(`${activeView}-tab`)?.focus());
  }

  function defaultPageSetup(): PageSetup {
    return {
      width_mm: 210,
      height_mm: 297,
      margin_top_mm: 25,
      margin_right_mm: 25,
      margin_bottom_mm: 25,
      margin_left_mm: 25
    };
  }

  function setStatusFromError(error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    status = message === 'image file is too large' ? tr('imageTooLarge') : message;
  }

  function tr(key: UiStringKey, values: Record<string, string | number> = {}) {
    return translate(settings.ui_locale, key, values);
  }

  function shortcut(command: GlobalShortcutCommand) {
    return shortcutLabel(command, shortcutPlatform);
  }

  function shortcutList(command: GlobalShortcutCommand) {
    return shortcutLabels(command, shortcutPlatform).join(', ');
  }

  function shortcutTitle(label: string, command: GlobalShortcutCommand) {
    return `${label} (${shortcutList(command)})`;
  }

  onMount(() => {
    window.addEventListener('keydown', handleGlobalKeydown);
    window.addEventListener('click', handleWindowClick);
    Promise.all([newDocument(), loadShellState()]).catch((error: unknown) => {
      status = error instanceof Error ? error.message : String(error);
    });

    return () => {
      window.removeEventListener('keydown', handleGlobalKeydown);
      window.removeEventListener('click', handleWindowClick);
      view?.destroy();
    };
  });

  function collectDocumentWarnings(document: DocumentState): string[] {
    return [...(document.warnings ?? []).map((warning) => warning.message), ...documentProjectionWarnings(document)];
  }
</script>

<main
  class:high-contrast={settings.high_contrast}
  class="app-shell"
  data-large-toolbar={settings.large_toolbar ? 'true' : 'false'}
  data-low-resource={settings.low_resource ? 'true' : 'false'}
  data-reduced-motion={settings.reduced_motion ? 'true' : 'false'}
  dir={uiDirection}
  lang={settings.ui_locale}
>
  <header class="topbar">
    <div>
      <h1>{title}</h1>
    </div>
  </header>

  <nav class="menu-strip" aria-label={tr('mainMenu')}>
    <div bind:this={fileMenuRoot} class="file-menu">
      <button
        aria-expanded={fileMenuOpen}
        aria-haspopup="menu"
        class="menu-button"
        type="button"
        onclick={toggleFileMenu}
      >
        {tr('file')}
      </button>
      {#if fileMenuOpen}
        <div class="file-menu-popover" aria-label={tr('file')}>
          <div class="menu-popover-header" aria-hidden="true">
            <span>{tr('file')}</span>
          </div>

          <button class="menu-command primary-command" type="button" onclick={() => runFileMenuAction(newDocument)}>
            <span class="menu-glyph glyph-new" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('new')}</span>
            </span>
            <span class="menu-shortcut">{shortcut('newDocument')}</span>
          </button>

          <button class="menu-command" type="button" onclick={() => runFileMenuAction(openDocumentWithDialog)}>
            <span class="menu-glyph glyph-open" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('open')}</span>
            </span>
            <span class="menu-shortcut">{shortcut('openDocument')}</span>
          </button>

          <div class="menu-separator" role="separator"></div>

          <button
            class="menu-command"
            disabled={!fileState.has_current_path}
            type="button"
            onclick={() => runFileMenuAction(saveCurrentDocument)}
          >
            <span class="menu-glyph glyph-save" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('save')}</span>
            </span>
            <span class="menu-shortcut">{shortcut('saveDocument')}</span>
          </button>
          <button class="menu-command" type="button" onclick={() => runFileMenuAction(saveDocumentAsWithDialog)}>
            <span class="menu-glyph glyph-save-as" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('saveAs')}</span>
            </span>
            <span class="menu-shortcut">{shortcut('saveDocumentAs')}</span>
          </button>
          <button class="menu-command" type="button" onclick={() => runFileMenuAction(autosaveDocument)}>
            <span class="menu-glyph glyph-autosave" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('autosave')}</span>
            </span>
          </button>
          <button class="menu-command" type="button" onclick={() => runFileMenuAction(openDocumentInspector)}>
            <span class="menu-glyph glyph-inspector" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('documentInspector')}</span>
            </span>
          </button>

          <button
            class="menu-command"
            disabled={!editorEditable}
            type="button"
            onclick={() => runFileMenuAction(insertImageWithDialog)}
          >
            <span class="menu-glyph glyph-image" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('insertImage')}</span>
            </span>
          </button>
          <button
            class="menu-command"
            disabled={!editorEditable}
            title={tr('tableOfContents')}
            type="button"
            onclick={() => runFileMenuAction(insertOrUpdateTableOfContents)}
          >
            <span class="menu-glyph glyph-toc" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('insertOrUpdateTableOfContents')}</span>
            </span>
          </button>

          <div class="menu-separator" role="separator"></div>

          <button
            aria-expanded={exportMenuOpen}
            aria-haspopup="true"
            class="menu-command submenu-trigger"
            type="button"
            onclick={() => (exportMenuOpen = !exportMenuOpen)}
          >
            <span class="menu-glyph glyph-export" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('export')}</span>
            </span>
          </button>
          {#if exportMenuOpen}
            <div class="file-submenu-panel" aria-label={tr('export')}>
              <label for="file-menu-export-path">{tr('exportPath')}</label>
              <input
                id="file-menu-export-path"
                aria-label={tr('exportPath')}
                bind:this={exportPathInputElement}
                bind:value={exportPathInput}
                placeholder={tr('exportPathPlaceholder')}
                type="text"
              />
              <fieldset class="menu-field" aria-label={tr('pdfPageRange')}>
                <legend>{tr('pdfPageRange')}</legend>
                <label>
                  <input
                    checked={pdfPageRangeMode === 'all'}
                    name="pdf-page-range-mode"
                    type="radio"
                    value="all"
                    onchange={() => (pdfPageRangeMode = 'all')}
                  />
                  {tr('pdfAllPages')}
                </label>
                <div class="menu-input-row">
                  <label>
                    <input
                      checked={pdfPageRangeMode === 'range'}
                      name="pdf-page-range-mode"
                      type="radio"
                      value="range"
                      onchange={() => (pdfPageRangeMode = 'range')}
                    />
                    {tr('pdfPageRangeFrom')}
                  </label>
                  <input
                    aria-label={tr('pdfPageRangeFrom')}
                    bind:value={pdfPageStartInput}
                    disabled={pdfPageRangeMode !== 'range'}
                    inputmode="numeric"
                    min="1"
                    type="number"
                  />
                </div>
                <label for="file-menu-pdf-page-end">{tr('pdfPageRangeTo')}</label>
                <input
                  id="file-menu-pdf-page-end"
                  aria-label={tr('pdfPageRangeTo')}
                  bind:value={pdfPageEndInput}
                  disabled={pdfPageRangeMode !== 'range'}
                  inputmode="numeric"
                  min="1"
                  type="number"
                />
              </fieldset>
              <div class="export-format-grid">
                <button class="format-option" type="button" onclick={() => runFileMenuAction(exportText)}>
                  <span>{tr('txt')}</span>
                </button>
                <button class="format-option" type="button" onclick={() => runFileMenuAction(exportHtml)}>
                  <span>{tr('html')}</span>
                </button>
                <button
                  class="format-option"
                  title={shortcutTitle(tr('exportPdf'), 'exportPdf')}
                  type="button"
                  onclick={() => runFileMenuAction(exportPdf)}
                >
                  <span>{tr('pdf')}</span>
                  <span class="format-shortcut">{shortcut('exportPdf')}</span>
                </button>
                <button class="format-option" type="button" onclick={() => runFileMenuAction(exportDocx)}>
                  <span>{tr('docx')}</span>
                </button>
              </div>
            </div>
          {/if}

          <button class="menu-command" type="button" onclick={() => runFileMenuAction(printDocument)}>
            <span class="menu-glyph glyph-print" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('print')}</span>
            </span>
            <span class="menu-shortcut">{shortcut('printDocument')}</span>
          </button>
        </div>
      {/if}
    </div>

    <div class="view-tabs" role="tablist" aria-label={tr('workspaceViews')}>
      <button
        aria-controls="editor-view"
        aria-selected={activeView === 'editor'}
        id="editor-tab"
        onkeydown={(event) => handleViewTabKeydown(event, 'editor')}
        role="tab"
        type="button"
        onclick={() => selectView('editor')}
      >
        {tr('editor')}
      </button>
      <button
        aria-controls="settings-view"
        aria-selected={activeView === 'settings'}
        id="settings-tab"
        onkeydown={(event) => handleViewTabKeydown(event, 'settings')}
        role="tab"
        type="button"
        onclick={() => selectView('settings')}
      >
        {tr('settings')}
      </button>
      <button
        aria-controls="about-view"
        aria-selected={activeView === 'about'}
        id="about-tab"
        onkeydown={(event) => handleViewTabKeydown(event, 'about')}
        role="tab"
        type="button"
        onclick={() => selectView('about')}
      >
        {tr('about')}
      </button>
    </div>
  </nav>

  <section class="command-bar" aria-label={tr('editingToolbar')}>
    <div class="tool-group" role="group" aria-label={tr('history')}>
      <button title={shortcutTitle(tr('undo'), 'undo')} type="button" onclick={undoDocument}>{tr('undo')}</button>
      <button title={shortcutTitle(tr('redo'), 'redo')} type="button" onclick={redoDocument}>{tr('redo')}</button>
    </div>

    <div class="tool-group view-mode-tools" role="group" aria-label={tr('viewMode')}>
      <button
        aria-pressed={editorViewMode === 'draft'}
        class:active-format={editorViewMode === 'draft'}
        type="button"
        onclick={() => setEditorViewMode('draft')}
      >
        {tr('draftView')}
      </button>
      <button
        aria-pressed={editorViewMode === 'page-layout'}
        class:active-format={editorViewMode === 'page-layout'}
        type="button"
        onclick={() => setEditorViewMode('page-layout')}
      >
        {tr('pageLayoutView')}
      </button>
      <select
        aria-label={tr('zoom')}
        value={zoomChoice}
        onchange={(event) => setZoomChoice(event.currentTarget.value)}
        title={tr('zoom')}
      >
        <option value="fit-width">{tr('fitWidth')}</option>
        <option value="100">100%</option>
        <option value="custom">{tr('custom')}</option>
      </select>
      <label class="compact-number zoom-slider" title={tr('customZoom')}>
        {tr('zoom')}
        <input
          aria-label={tr('customZoom')}
          disabled={zoomChoice === 'fit-width'}
          min={minEditorZoom}
          max={maxEditorZoom}
          step={editorZoomStep}
          type="range"
          value={customZoomPercent}
          oninput={(event) => setCustomZoom(Number(event.currentTarget.value))}
        />
        <span class="zoom-value">{effectiveZoomPercent}%</span>
      </label>
      <label class="check-row compact ruler-toggle" title={tr('showRulers')}>
        <input bind:checked={showRulers} type="checkbox" />
        {tr('showRulers')}
      </label>
    </div>

    <div class="tool-group style-tools" role="group" aria-label={tr('styles')}>
      <select
        aria-label={tr('styles')}
        disabled={!editorEditable}
        bind:value={selectedStyleId}
        onpointerdown={() => captureToolbarSelection(true)}
        onchange={(event) => applyParagraphStyle(event.currentTarget.value)}
        title={tr('styles')}
      >
        {#each paragraphStyles as style}
          <option value={style.id}>{style.label}</option>
        {/each}
      </select>
      <button
        disabled={!editorEditable || activeFormatting.blockType !== 'paragraph'}
        title={tr('updateStyleFromSelection')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, updateStyleFromSelection)}
        onclick={(event) => runToolbarKeyboardCommand(event, updateStyleFromSelection)}
      >
        {tr('updateStyle')}
      </button>
      <button
        disabled={!editorEditable}
        title={tr('clearFormatting')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, clearFormatting)}
        onclick={(event) => runToolbarKeyboardCommand(event, clearFormatting)}
      >
        {tr('clear')}
      </button>
    </div>

    <div class="tool-group formatting-tools" role="group" aria-label={tr('textFormatting')}>
      <button
        aria-label={tr('bold')}
        aria-pressed={activeFormatting.marks.bold}
        class:active-format={activeFormatting.marks.bold}
        class="format-button strong"
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyInlineMark('bold', tr('bold')))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('bold', tr('bold')))}
        title={shortcutTitle(tr('bold'), 'bold')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('bold', tr('bold')))}
      >
        B
      </button>
      <button
        aria-label={tr('italic')}
        aria-pressed={activeFormatting.marks.italic}
        class:active-format={activeFormatting.marks.italic}
        class="format-button italic"
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyInlineMark('italic', tr('italic')))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('italic', tr('italic')))}
        title={shortcutTitle(tr('italic'), 'italic')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('italic', tr('italic')))}
      >
        I
      </button>
      <button
        aria-label={tr('underline')}
        aria-pressed={activeFormatting.marks.underline}
        class:active-format={activeFormatting.marks.underline}
        class="format-button underline"
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyInlineMark('underline', tr('underline')))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('underline', tr('underline')))}
        title={shortcutTitle(tr('underline'), 'underline')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('underline', tr('underline')))}
      >
        U
      </button>
      <button
        aria-label={tr('strikethrough')}
        aria-pressed={activeFormatting.marks.strikethrough}
        class:active-format={activeFormatting.marks.strikethrough}
        class="format-button strike"
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyInlineMark('strikethrough', tr('strikethrough')))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('strikethrough', tr('strikethrough')))}
        title={tr('strikethrough')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('strikethrough', tr('strikethrough')))}
      >
        S
      </button>
      <button
        aria-label={tr('superscript')}
        aria-pressed={activeFormatting.marks.superscript}
        class:active-format={activeFormatting.marks.superscript}
        class="format-button script"
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyInlineMark('superscript', tr('superscript')))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('superscript', tr('superscript')))}
        title={tr('superscript')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('superscript', tr('superscript')))}
      >
        x<sup>2</sup>
      </button>
      <button
        aria-label={tr('subscript')}
        aria-pressed={activeFormatting.marks.subscript}
        class:active-format={activeFormatting.marks.subscript}
        class="format-button script"
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyInlineMark('subscript', tr('subscript')))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('subscript', tr('subscript')))}
        title={tr('subscript')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('subscript', tr('subscript')))}
      >
        x<sub>2</sub>
      </button>
    </div>

    <div bind:this={linkToolsRoot} class="tool-group link-tools" role="group" aria-label={tr('linkTools')}>
      <button
        aria-expanded={linkPanelOpen}
        aria-label={tr('openLinkPanel')}
        aria-pressed={Boolean(activeFormatting.linkHref)}
        class:active-format={Boolean(activeFormatting.linkHref)}
        disabled={!editorEditable}
        title={shortcutTitle(tr('link'), 'insertLink')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => void openLinkPanel())}
        onclick={(event) => runToolbarKeyboardCommand(event, () => void openLinkPanel())}
      >
        {tr('link')}
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.linkHref}
        title={tr('removeLink')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, removeLinkFromSelection)}
        onclick={(event) => runToolbarKeyboardCommand(event, removeLinkFromSelection)}
      >
        {tr('unlink')}
      </button>
      <button
        aria-pressed={Boolean(activeFormatting.blockBookmarkId)}
        class:active-format={Boolean(activeFormatting.blockBookmarkId)}
        disabled={!editorEditable || !activeFormatting.blockType}
        title={tr('bookmark')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, addBookmarkToSelection)}
        onclick={(event) => runToolbarKeyboardCommand(event, addBookmarkToSelection)}
      >
        {tr('bookmark')}
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.blockBookmarkId}
        title={tr('removeBookmark')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, removeBookmarkFromSelection)}
        onclick={(event) => runToolbarKeyboardCommand(event, removeBookmarkFromSelection)}
      >
        {tr('removeBookmarkShort')}
      </button>
      {#if linkPanelOpen}
        <div class="link-popover" role="dialog" aria-label={tr('linkTools')}>
          {#if linkTargets.length > 0}
            <select
              aria-label={tr('linkTarget')}
              bind:value={selectedLinkTargetId}
              onchange={(event) => applyLinkTarget(event.currentTarget.value)}
            >
              <option value="">{tr('externalLink')}</option>
              {#each linkTargets as target}
                <option value={target.id}>
                  {target.kind === 'heading' && target.level ? `H${target.level} ` : ''}{target.label}
                </option>
              {/each}
            </select>
          {/if}
          <input
            aria-label={tr('linkHref')}
            bind:this={linkInput}
            bind:value={linkHrefInput}
            onkeydown={(event) => {
              if (event.key === 'Enter') {
                event.preventDefault();
                applyLinkFromPanel();
              }
            }}
            placeholder="https://example.invalid or #bookmark"
            type="text"
          />
          <button type="button" onpointerdown={(event) => event.preventDefault()} onclick={applyLinkFromPanel}>
            {tr('apply')}
          </button>
          <button type="button" onpointerdown={(event) => event.preventDefault()} onclick={removeLinkFromSelection}>
            {tr('removeLink')}
          </button>
        </div>
      {/if}
    </div>

    <div class="tool-group comment-tools" role="group" aria-label={tr('comments')}>
      <label class="check-row compact review-toggle" title={tr('recordChanges')}>
        <input
          checked={trackChangesRecording}
          disabled={!editorEditable}
          type="checkbox"
          onchange={(event) => setTrackChangesRecording(event.currentTarget.checked)}
        />
        {tr('recordChanges')}
      </label>
      <button
        aria-expanded={reviewPanelOpen}
        class:active-format={reviewPanelOpen}
        disabled={!editorEditable && trackedChangeCount === 0}
        title={tr('reviewChanges')}
        type="button"
        onclick={() => (reviewPanelOpen = !reviewPanelOpen)}
      >
        {tr('reviewChanges')}{trackedChangeCount > 0 ? ` ${trackedChangeCount}` : ''}
      </button>
      <button
        aria-expanded={commentsPanelOpen}
        class:active-format={commentsPanelOpen}
        disabled={!editorEditable && commentThreads.length === 0}
        title={shortcutTitle(tr('comments'), 'insertComment')}
        type="button"
        onclick={() => (commentsPanelOpen = !commentsPanelOpen)}
      >
        {tr('comments')}{unresolvedCommentCount > 0 ? ` ${unresolvedCommentCount}` : ''}
      </button>
      <button
        aria-expanded={notesPanelOpen}
        class:active-format={notesPanelOpen}
        disabled={noteSummaries.length === 0}
        title={tr('notes')}
        type="button"
        onclick={() => (notesPanelOpen = !notesPanelOpen)}
      >
        {tr('notes')}{noteSummaries.length > 0 ? ` ${noteSummaries.length}` : ''}
      </button>
      <button
        disabled={!editorEditable}
        title={tr('insertFootnote')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => void insertNote('footnote'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => void insertNote('footnote'))}
      >
        {tr('insertFootnote')}
      </button>
      <button
        disabled={!editorEditable}
        title={tr('insertEndnote')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => void insertNote('endnote'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => void insertNote('endnote'))}
      >
        {tr('insertEndnote')}
      </button>
    </div>

    <div class="tool-group font-tools" role="group" aria-label={tr('fontControls')}>
      <select
        aria-label={tr('fontFamily')}
        disabled={!editorEditable}
        bind:value={selectedFontFamily}
        onpointerdown={() => captureToolbarSelection(true)}
        onchange={(event) => applyTextStyle({ fontFamily: event.currentTarget.value }, tr('fontFamily'))}
        title={tr('fontFamily')}
      >
        {#each fontFamilies as font}
          <option value={font.id}>{font.label}</option>
        {/each}
      </select>
      <select
        aria-label={tr('fontSize')}
        disabled={!editorEditable}
        bind:value={selectedFontSize}
        onpointerdown={() => captureToolbarSelection(true)}
        onchange={(event) => applyTextStyle({ fontSizePt: Number(event.currentTarget.value) }, tr('fontSize'))}
        title={tr('fontSize')}
      >
        {#each fontSizes as size}
          <option value={size}>{size} pt</option>
        {/each}
      </select>
      <input
        aria-label={tr('textColor')}
        disabled={!editorEditable}
        bind:value={selectedTextColor}
        class="color-input"
        onpointerdown={() => captureToolbarSelection(true)}
        onchange={(event) => applyTextStyle({ textColor: event.currentTarget.value }, tr('textColor'))}
        title={tr('textColor')}
        type="color"
      />
      <input
        aria-label={tr('highlightColor')}
        disabled={!editorEditable}
        bind:value={selectedHighlightColor}
        class="color-input"
        onpointerdown={() => captureToolbarSelection(true)}
        onchange={(event) => applyTextStyle({ highlightColor: event.currentTarget.value }, tr('highlightColor'))}
        title={tr('highlightColor')}
        type="color"
      />
    </div>

    <div class="tool-group" role="group" aria-label={tr('blockFormatting')}>
      <button
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, applyParagraph)}
        onmousedown={(event) => runToolbarMouseCommand(event, applyParagraph)}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, applyParagraph)}
      >
        {tr('paragraph')}
      </button>
      <button
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyHeading(1))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyHeading(1))}
        title={shortcutTitle(tr('heading1'), 'heading1')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyHeading(1))}
      >
        {tr('heading1')}
      </button>
      <button
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyHeading(2))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyHeading(2))}
        title={shortcutTitle(tr('heading2'), 'heading2')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyHeading(2))}
      >
        {tr('heading2')}
      </button>
      <button
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyHeading(3))}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyHeading(3))}
        title={shortcutTitle(tr('heading3'), 'heading3')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyHeading(3))}
      >
        {tr('heading3')}
      </button>
    </div>

    <div class="tool-group paragraph-tools" role="group" aria-label={tr('paragraphControls')}>
      <button
        aria-pressed={activeFormatting.paragraphFormat.align === 'left'}
        class:active-format={activeFormatting.paragraphFormat.align === 'left'}
        disabled={!editorEditable}
        title={tr('alignLeft')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyParagraphFormat({ align: 'left' }, tr('alignLeft')))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyParagraphFormat({ align: 'left' }, tr('alignLeft')))}
      >
        L
      </button>
      <button
        aria-pressed={activeFormatting.paragraphFormat.align === 'center'}
        class:active-format={activeFormatting.paragraphFormat.align === 'center'}
        disabled={!editorEditable}
        title={tr('alignCenter')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyParagraphFormat({ align: 'center' }, tr('alignCenter')))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyParagraphFormat({ align: 'center' }, tr('alignCenter')))}
      >
        C
      </button>
      <button
        aria-pressed={activeFormatting.paragraphFormat.align === 'right'}
        class:active-format={activeFormatting.paragraphFormat.align === 'right'}
        disabled={!editorEditable}
        title={tr('alignRight')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyParagraphFormat({ align: 'right' }, tr('alignRight')))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyParagraphFormat({ align: 'right' }, tr('alignRight')))}
      >
        R
      </button>
      <button
        aria-pressed={activeFormatting.paragraphFormat.align === 'justify'}
        class:active-format={activeFormatting.paragraphFormat.align === 'justify'}
        disabled={!editorEditable}
        title={tr('alignJustify')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyParagraphFormat({ align: 'justify' }, tr('alignJustify')))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyParagraphFormat({ align: 'justify' }, tr('alignJustify')))}
      >
        J
      </button>
      <select
        aria-label={tr('lineSpacing')}
        disabled={!editorEditable}
        bind:value={selectedLineSpacing}
        onpointerdown={() => captureToolbarSelection(true)}
        onchange={(event) => applyParagraphFormat({ lineSpacing: Number(event.currentTarget.value) }, tr('lineSpacing'))}
        title={tr('lineSpacing')}
      >
        {#each lineSpacings as lineSpacing}
          <option value={lineSpacing.id}>{lineSpacing.label}</option>
        {/each}
      </select>
      <label class="compact-number" title={tr('spacingBefore')}>
        {tr('before')}
        <input
          disabled={!editorEditable}
          min="0"
          max="50"
          type="number"
          value={spacingBefore}
          onpointerdown={() => captureToolbarSelection(true)}
          onchange={(event) => setSpacingField('spacingBefore', event.currentTarget.valueAsNumber)}
        />
      </label>
      <label class="compact-number" title={tr('spacingAfter')}>
        {tr('after')}
        <input
          disabled={!editorEditable}
          min="0"
          max="50"
          type="number"
          value={spacingAfter}
          onpointerdown={() => captureToolbarSelection(true)}
          onchange={(event) => setSpacingField('spacingAfter', event.currentTarget.valueAsNumber)}
        />
      </label>
      <label class="compact-number" title={tr('firstLineIndent')}>
        {tr('firstLine')}
        <input
          disabled={!editorEditable}
          min="-50"
          max="50"
          type="number"
          value={firstLineIndent}
          onpointerdown={() => captureToolbarSelection(true)}
          onchange={(event) => setSpacingField('firstLineIndent', event.currentTarget.valueAsNumber)}
        />
      </label>
    </div>

    <div class="tool-group list-tools" role="group" aria-label={tr('lists')}>
      <button
        aria-pressed={activeFormatting.list?.type === 'bullet_list'}
        class:active-format={activeFormatting.list?.type === 'bullet_list'}
        disabled={!editorEditable}
        title={shortcutTitle(tr('bulletList'), 'bulletList')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyList('bullet_list'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyList('bullet_list'))}
      >
        UL
      </button>
      <button
        aria-pressed={activeFormatting.list?.type === 'ordered_list'}
        class:active-format={activeFormatting.list?.type === 'ordered_list'}
        disabled={!editorEditable}
        title={shortcutTitle(tr('numberedList'), 'numberedList')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyList('ordered_list'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyList('ordered_list'))}
      >
        1.
      </button>
      <button
        disabled={!editorEditable}
        title={shortcutTitle(tr('decreaseIndent'), 'decreaseIndent')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => adjustListLevel(-1))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => adjustListLevel(-1))}
      >
        &lt;
      </button>
      <button
        disabled={!editorEditable}
        title={shortcutTitle(tr('increaseIndent'), 'increaseIndent')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => adjustListLevel(1))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => adjustListLevel(1))}
      >
        &gt;
      </button>
    </div>

    <div class="tool-group table-tools" role="group" aria-label={tr('tables')}>
      <label class="table-size-field" title={tr('tableRows')}>
        {tr('rowsShort')}
        <select
          aria-label={tr('tableRows')}
          disabled={!editorEditable}
          bind:value={tableRows}
          onpointerdown={() => captureToolbarSelection(true)}
        >
          {#each tableDimensions as size}
            <option value={size}>{size}</option>
          {/each}
        </select>
      </label>
      <label class="table-size-field" title={tr('tableColumns')}>
        {tr('columnsShort')}
        <select
          aria-label={tr('tableColumns')}
          disabled={!editorEditable}
          bind:value={tableColumns}
          onpointerdown={() => captureToolbarSelection(true)}
        >
          {#each tableDimensions as size}
            <option value={size}>{size}</option>
          {/each}
        </select>
      </label>
      <button
        class="table-insert-button"
        disabled={!editorEditable}
        title={tr('insertTable')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, insertTable)}
        onclick={(event) => runToolbarKeyboardCommand(event, insertTable)}
      >
        {tr('insertTableShort')}
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.table?.canAddRow}
        title={tr('addRowAbove')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => editTable('add_row_above'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => editTable('add_row_above'))}
      >
        +R^
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.table?.canAddRow}
        title={tr('addRowBelow')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => editTable('add_row_below'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => editTable('add_row_below'))}
      >
        +Rv
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.table?.canDeleteRow}
        title={tr('deleteRow')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => editTable('delete_row'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => editTable('delete_row'))}
      >
        -R
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.table?.canAddColumn}
        title={tr('addColumnLeft')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => editTable('add_column_left'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => editTable('add_column_left'))}
      >
        +C&lt;
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.table?.canAddColumn}
        title={tr('addColumnRight')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => editTable('add_column_right'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => editTable('add_column_right'))}
      >
        +C&gt;
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.table?.canDeleteColumn}
        title={tr('deleteColumn')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => editTable('delete_column'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => editTable('delete_column'))}
      >
        -C
      </button>
      <button
        disabled={!editorEditable || !activeFormatting.table?.canDeleteTable}
        title={tr('deleteTable')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => editTable('delete_table'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => editTable('delete_table'))}
      >
        Del
      </button>
      {#if activeFormatting.table}
        <label class="table-cell-field" title={tr('tableCellBackground')}>
          {tr('tableCellBackground')}
          <select
            aria-label={tr('tableCellBackground')}
            disabled={!editorEditable}
            bind:value={tableCellBackground}
            onpointerdown={() => captureToolbarSelection(true)}
            onchange={(event) => setTableCellBackground(event.currentTarget.value)}
          >
            {#each tableCellBackgrounds as background}
              <option value={background.id}>{tr(background.labelKey)}</option>
            {/each}
          </select>
        </label>
        <label class="table-cell-field" title={tr('tableCellAlignment')}>
          {tr('tableCellAlignment')}
          <select
            aria-label={tr('tableCellAlignment')}
            disabled={!editorEditable}
            bind:value={tableCellAlignment}
            onpointerdown={() => captureToolbarSelection(true)}
            onchange={(event) => setTableCellAlignment(event.currentTarget.value)}
          >
            {#each tableCellAlignments as alignment}
              <option value={alignment.id}>{tr(alignment.labelKey)}</option>
            {/each}
          </select>
        </label>
        <label class="table-cell-field" title={tr('tableCellBorder')}>
          {tr('tableCellBorder')}
          <select
            aria-label={tr('tableCellBorder')}
            disabled={!editorEditable}
            bind:value={tableCellBorder}
            onpointerdown={() => captureToolbarSelection(true)}
            onchange={(event) => setTableCellBorder(event.currentTarget.value)}
          >
            <option value="visible">{tr('tableCellBorderVisible')}</option>
            <option value="hidden">{tr('tableCellBorderHidden')}</option>
          </select>
        </label>
      {/if}
    </div>

    {#if activeFormatting.image}
      <div class="tool-group image-tools" role="group" aria-label={tr('imageControls')}>
        <label class="image-text-field">
          {tr('imageAltText')}
          <input
            disabled={!editorEditable}
            bind:value={imageAltText}
            onpointerdown={() => captureToolbarSelection(true)}
            onchange={() => applyImageAttrs({ altText: imageAltText })}
            type="text"
          />
        </label>
        <label class="image-text-field">
          {tr('imageCaption')}
          <input
            disabled={!editorEditable}
            bind:value={imageCaption}
            onpointerdown={() => captureToolbarSelection(true)}
            onchange={() => applyImageAttrs({ caption: imageCaption })}
            type="text"
          />
        </label>
        <select
          aria-label={tr('imageAlignment')}
          disabled={!editorEditable}
          bind:value={imageAlignment}
          onpointerdown={() => captureToolbarSelection(true)}
          onchange={(event) => setImageAlignment(event.currentTarget.value)}
          title={tr('imageAlignment')}
        >
          {#each imageAlignments as alignment}
            <option value={alignment.id}>{alignment.label}</option>
          {/each}
        </select>
        <label class="compact-number image-scale" title={tr('imageScale')}>
          {tr('imageScale')}
          <input
            disabled={!editorEditable}
            min="25"
            max="200"
            step="5"
            type="range"
            bind:value={imageScalePercent}
            onpointerdown={() => captureToolbarSelection(true)}
            onchange={(event) => setImageScale(Number(event.currentTarget.value))}
          />
          <input
            disabled={!editorEditable}
            min="25"
            max="200"
            type="number"
            value={imageScalePercent}
            onpointerdown={() => captureToolbarSelection(true)}
            onchange={(event) => setImageScale(event.currentTarget.valueAsNumber)}
          />
        </label>
      </div>
    {/if}

    <div class="tool-group template-tools" role="group" aria-label={tr('templates')}>
      <select aria-label={tr('templates')} bind:value={selectedTemplateId}>
        {#each templates as template}
          <option value={template.id}>{template.name}</option>
        {/each}
      </select>
      <button type="button" onclick={newDocumentFromTemplate}>{tr('useTemplate')}</button>
    </div>

    <div class="tool-group search-tools" role="search" aria-label={tr('findAndReplace')}>
      <button
        aria-controls="find-panel"
        aria-expanded={findPanelOpen}
        aria-label={tr('findAndReplace')}
        class="search-toggle"
        title={`${tr('findAndReplace')} (${shortcut('find')}, ${shortcut('replace')})`}
        type="button"
        onclick={toggleFindPanel}
      >
        <span aria-hidden="true" class="search-icon"></span>
      </button>
      {#if findPanelOpen}
        <div class="find-popover" id="find-panel">
          <input
            aria-label={tr('find')}
            bind:this={findInput}
            bind:value={findQuery}
            oninput={refreshFindState}
            placeholder={tr('find')}
            type="search"
          />
          <label class="check-row compact">
            <input bind:checked={findCaseSensitive} onchange={refreshFindState} type="checkbox" />
            {tr('case')}
          </label>
          <button disabled={findRanges.length === 0} type="button" onclick={findPrevious}>{tr('previous')}</button>
          <button disabled={findRanges.length === 0} type="button" onclick={findNext}>{tr('next')}</button>
          <span class="match-count">{findRanges.length === 0 ? '0' : tr('matchCount', { current: activeFindIndex + 1, total: findRanges.length })}</span>
          <input
            aria-label={tr('replace')}
            bind:this={replaceInput}
            bind:value={replaceText}
            placeholder={tr('replace')}
            type="text"
          />
          <button disabled={!editorEditable || findRanges.length === 0} type="button" onclick={replaceCurrentMatch}>{tr('replace')}</button>
          <button disabled={!editorEditable || findRanges.length === 0} type="button" onclick={replaceAllMatches}>{tr('all')}</button>
        </div>
      {/if}
    </div>
  </section>

  <section
    class={showWorkspaceSidebar ? 'workspace has-sidebar' : 'workspace'}
    aria-label={tr('documentWorkspace')}
  >
    {#if showWorkspaceSidebar}
      <aside class="sidebar" aria-label={tr('documentStatistics')}>
        {#if !settings.low_resource && navigatorHeadings.length > 0}
          <h2>{tr('navigator')}</h2>
          <ul class="navigator-list">
            {#each navigatorHeadings as heading}
              <li class={`navigator-level-${heading.level}`}>
                <button type="button" onclick={() => jumpToHeading(heading)}>
                  <span class="navigator-marker">H{heading.level}</span>
                  <span>{heading.text}</span>
                </button>
              </li>
            {/each}
          </ul>
        {/if}

        {#if reviewPanelOpen}
          <section class="review-panel" aria-label={tr('reviewChanges')}>
            <div class="sidebar-section-heading">
              <h2>{tr('reviewChanges')}</h2>
              <span>{trackedChangeCount}</span>
            </div>
            <p class="privacy-note">{tr('trackChangesPrivacyWarning')}</p>
            <div class="review-bulk-actions">
              <button disabled={!editorEditable || trackedChangeCount === 0} type="button" onclick={acceptAllTrackedChanges}>
                {tr('acceptAll')}
              </button>
              <button disabled={!editorEditable || trackedChangeCount === 0} type="button" onclick={rejectAllTrackedChanges}>
                {tr('rejectAll')}
              </button>
            </div>
            {#if trackedChanges.length > 0}
              <ul class="changes-list">
                {#each trackedChanges as change}
                  <li class={`review-${change.kind}`}>
                    <button class="change-jump" type="button" onclick={() => jumpToTrackedChange(change)}>
                      <span class="change-kind">{change.kind === 'insertion' ? tr('insertion') : tr('deletion')}</span>
                      <span class="change-author">{change.author || localTrackedChangeAuthor}</span>
                      {#if trackedChangeTimestamp(change)}
                        <span class="change-time">{trackedChangeTimestamp(change)}</span>
                      {/if}
                      <span class="change-text">{trackedChangePreview(change)}</span>
                    </button>
                    <div class="change-actions">
                      <button disabled={!editorEditable} type="button" onclick={() => acceptTrackedChange(change)}>{tr('accept')}</button>
                      <button disabled={!editorEditable} type="button" onclick={() => rejectTrackedChange(change)}>{tr('reject')}</button>
                    </div>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="empty-sidebar-note">{tr('noTrackedChanges')}</p>
            {/if}
          </section>
        {/if}

        {#if commentsPanelOpen}
          <section class="comments-panel" aria-label={tr('comments')}>
            <div class="sidebar-section-heading">
              <h2>{tr('comments')}</h2>
              <span>{unresolvedCommentCount}/{commentThreads.length}</span>
            </div>
            <div class="comment-add-box">
              <label>
                {tr('commentBody')}
                <textarea
                  bind:this={commentBodyInput}
                  bind:value={newCommentBody}
                  disabled={!editorEditable}
                  maxlength={maxCommentBodyChars}
                  rows="3"
                ></textarea>
              </label>
              <button
                disabled={!editorEditable || selectedCommentText.length === 0 || newCommentBody.trim().length === 0}
                type="button"
                onclick={addCommentToSelection}
              >
                {tr('addComment')}
              </button>
            </div>
            {#if commentThreads.length > 0}
              <ul class="comments-list">
                {#each commentThreads as comment}
                  <li class:resolved-comment={comment.resolved} class:active-comment={activeCommentId === comment.id}>
                    <button class="comment-jump" type="button" onclick={() => jumpToComment(comment)}>
                      <span class="comment-author">{comment.author || localCommentAuthor}</span>
                      {#if commentTimestamp(comment)}
                        <span class="comment-time">{commentTimestamp(comment)}</span>
                      {/if}
                      <span class="comment-body">{comment.body}</span>
                    </button>
                    <div class="comment-actions">
                      <button type="button" onclick={() => setCommentResolved(comment, !comment.resolved)}>
                        {comment.resolved ? tr('reopen') : tr('resolve')}
                      </button>
                      <button type="button" onclick={() => deleteComment(comment)}>{tr('deleteComment')}</button>
                    </div>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="empty-sidebar-note">{tr('noComments')}</p>
            {/if}
          </section>
        {/if}

        {#if notesPanelOpen}
          <section class="notes-panel" aria-label={tr('notes')}>
            <div class="sidebar-section-heading">
              <h2>{tr('notes')}</h2>
              <span>{noteSummaries.length}</span>
            </div>
            {#if noteSummaries.length > 0}
              <ul class="notes-list">
                {#each noteSummaries as note}
                  <li>
                    <span class="note-heading">
                      <span class="note-kind">{noteKindLabel(note)}</span>
                      <span class="note-label">[{note.label}]</span>
                    </span>
                    <span class="note-body">{note.body}</span>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="empty-sidebar-note">{tr('noNotes')}</p>
            {/if}
          </section>
        {/if}

        {#if !settings.low_resource && fileState.recent_documents.length > 0}
          <h2>{tr('recent')}</h2>
          <ul class="action-list">
            {#each fileState.recent_documents as recent}
              <li>
                <button type="button" onclick={() => openRecentDocument(recent.token)}>
                  {recent.label}{recent.is_current ? ' *' : ''}
                </button>
              </li>
            {/each}
          </ul>
        {/if}

        {#if fileState.recovery_documents.length > 0}
          <h2>{tr('recovery')}</h2>
          <ul class="action-list">
            {#each fileState.recovery_documents as recovery}
              <li>
                <span>{recovery.label}</span>
                <button type="button" onclick={() => recoverDocument(recovery.token)}>{tr('recover')}</button>
                <button type="button" onclick={() => discardRecovery(recovery.token)}>{tr('discard')}</button>
              </li>
            {/each}
          </ul>
        {/if}

        {#if projectionWarnings.length > 0}
          <h2>{tr('projection')}</h2>
          <ul>
            {#each projectionWarnings as warning}
              <li>{warning}</li>
            {/each}
          </ul>
        {/if}
      </aside>
    {/if}

    <div
      aria-label={tr('editor')}
      class:editor-view-draft={editorViewMode === 'draft'}
      class:editor-view-page-layout={editorViewMode === 'page-layout'}
      class:hidden-view={activeView !== 'editor'}
      class:zoom-fit={zoomChoice === 'fit-width'}
      class:zoom-fixed={zoomChoice !== 'fit-width'}
      class="editor-panel"
      id="editor-view"
      oncontextmenu={handleEditorContextMenu}
      role="tabpanel"
      tabindex={activeView === 'editor' ? 0 : -1}
    >
      <div
        bind:clientWidth={editorViewportWidth}
        class:rulers-enabled={showRulers}
        class="editor-viewport"
        style={editorSurfaceStyle}
      >
        <div class:rulers-enabled={showRulers} class="editor-surface">
          {#if showRulers}
            <div class="ruler ruler-top" aria-hidden="true"><span></span></div>
            <div class="ruler ruler-left" aria-hidden="true"><span></span></div>
          {/if}
          <div
            bind:this={editorHost}
            class:empty-document={!editorHasStarted && editorIsEmpty}
            class="editor-host"
            data-placeholder={tr('startWriting')}
          ></div>
        </div>
      </div>
      {#if spellPopover}
        <div
          aria-label={tr('spelling')}
          class="spell-popover"
          role="dialog"
          style={`left: ${spellPopover.x}px; top: ${spellPopover.y}px;`}
        >
          <div class="spell-popover-title">{spellPopover.issue.word}</div>
          {#if spellPopover.issue.suggestions && spellPopover.issue.suggestions.length > 0}
            {#each spellPopover.issue.suggestions as suggestion}
              <button type="button" onclick={() => replaceSpellIssue(suggestion)}>{suggestion}</button>
            {/each}
          {:else}
            <span class="spell-muted">{tr('noSuggestions')}</span>
          {/if}
          <div class="spell-actions">
            <button type="button" onclick={ignoreSpellIssueOnce}>{tr('ignoreOnce')}</button>
            <button type="button" onclick={ignoreSpellIssueAll}>{tr('ignoreAll')}</button>
            <button type="button" onclick={addSpellIssueToPersonalDictionary}>{tr('addToDictionary')}</button>
          </div>
        </div>
      {/if}
    </div>

    <div
      aria-label={tr('settings')}
      class:hidden-view={activeView !== 'settings'}
      class="panel-view"
      id="settings-view"
      role="tabpanel"
    >
      <div class="form-surface">
        <h2>{tr('settings')}</h2>

        <section class="settings-group" aria-labelledby="page-settings-heading">
          <h3 id="page-settings-heading">{tr('page')}</h3>
          <div class="page-setup">
            <label class="page-format-field">
              {tr('pageSize')}
              <select
                aria-label={tr('pageSize')}
                value={currentPageFormatId()}
                onchange={(event) => updatePageFormat(event.currentTarget.value)}
              >
                {#each pageFormats as format}
                  <option value={format.id}>{format.label}</option>
                {/each}
                <option value="custom">{tr('custom')}</option>
              </select>
            </label>
            <label>
              {tr('width')}
              <input
                min="50"
                max="500"
                type="number"
                value={pageSetup.width_mm}
                oninput={(event) => updatePageSetupField('width_mm', event.currentTarget.valueAsNumber)}
              />
            </label>
            <label>
              {tr('height')}
              <input
                min="50"
                max="500"
                type="number"
                value={pageSetup.height_mm}
                oninput={(event) => updatePageSetupField('height_mm', event.currentTarget.valueAsNumber)}
              />
            </label>
            <label>
              {tr('top')}
              <input
                min="0"
                max="100"
                type="number"
                value={pageSetup.margin_top_mm}
                oninput={(event) => updatePageSetupField('margin_top_mm', event.currentTarget.valueAsNumber)}
              />
            </label>
            <label>
              {tr('right')}
              <input
                min="0"
                max="100"
                type="number"
                value={pageSetup.margin_right_mm}
                oninput={(event) => updatePageSetupField('margin_right_mm', event.currentTarget.valueAsNumber)}
              />
            </label>
            <label>
              {tr('bottom')}
              <input
                min="0"
                max="100"
                type="number"
                value={pageSetup.margin_bottom_mm}
                oninput={(event) => updatePageSetupField('margin_bottom_mm', event.currentTarget.valueAsNumber)}
              />
            </label>
            <label>
              {tr('left')}
              <input
                min="0"
                max="100"
                type="number"
                value={pageSetup.margin_left_mm}
                oninput={(event) => updatePageSetupField('margin_left_mm', event.currentTarget.valueAsNumber)}
              />
            </label>
            <button type="button" onclick={applyPageSetup}>{tr('applyPage')}</button>
          </div>
        </section>

        <section class="settings-group" aria-labelledby="page-regions-heading">
          <h3 id="page-regions-heading">{tr('headersFooters')}</h3>
          <label class="check-row">
            <input bind:checked={differentFirstPage} disabled={pageRegionsReadOnly()} type="checkbox" />
            {tr('differentFirstPage')}
          </label>
          <div class="page-regions-grid">
            <label>
              {tr('header')}
              <textarea
                bind:value={headerText}
                disabled={pageRegionsReadOnly()}
                rows="2"
              ></textarea>
            </label>
            <div class="field-button-row" role="group" aria-label={tr('headerFields')}>
              <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('header', 'page_number')}>{tr('pageNumber')}</button>
              <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('header', 'page_count')}>{tr('pageCount')}</button>
              <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('header', 'date')}>{tr('dateField')}</button>
            </div>

            <label>
              {tr('footer')}
              <textarea
                bind:value={footerText}
                disabled={pageRegionsReadOnly()}
                rows="2"
              ></textarea>
            </label>
            <div class="field-button-row" role="group" aria-label={tr('footerFields')}>
              <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('footer', 'page_number')}>{tr('pageNumber')}</button>
              <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('footer', 'page_count')}>{tr('pageCount')}</button>
              <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('footer', 'date')}>{tr('dateField')}</button>
            </div>

            {#if differentFirstPage}
              <label>
                {tr('firstHeader')}
                <textarea
                  bind:value={firstHeaderText}
                  disabled={pageRegionsReadOnly()}
                  rows="2"
                ></textarea>
              </label>
              <div class="field-button-row" role="group" aria-label={tr('firstHeaderFields')}>
                <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('first_header', 'page_number')}>{tr('pageNumber')}</button>
                <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('first_header', 'page_count')}>{tr('pageCount')}</button>
                <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('first_header', 'date')}>{tr('dateField')}</button>
              </div>

              <label>
                {tr('firstFooter')}
                <textarea
                  bind:value={firstFooterText}
                  disabled={pageRegionsReadOnly()}
                  rows="2"
                ></textarea>
              </label>
              <div class="field-button-row" role="group" aria-label={tr('firstFooterFields')}>
                <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('first_footer', 'page_number')}>{tr('pageNumber')}</button>
                <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('first_footer', 'page_count')}>{tr('pageCount')}</button>
                <button disabled={pageRegionsReadOnly()} type="button" onclick={() => insertPageFieldToken('first_footer', 'date')}>{tr('dateField')}</button>
              </div>
            {/if}
          </div>
          <button disabled={pageRegionsReadOnly()} type="button" onclick={applyPageRegions}>{tr('applyHeadersFooters')}</button>
        </section>

        <section class="settings-group" aria-labelledby="dictionary-manager-heading">
          <div class="field-button-row">
            <h3 id="dictionary-manager-heading">{tr('dictionaryManager')}</h3>
            <button type="button" onclick={refreshDictionaries}>{tr('dictionaryRefresh')}</button>
          </div>

          <label>
            {tr('activeDictionary')}
            <select bind:value={settings.language_tag} onchange={handleDictionarySelectionChanged}>
              {#if !selectedDictionary && settings.language_tag.trim().length > 0}
                <option value={settings.language_tag}>
                  {tr('dictionaryUnavailableOption', { languageTag: settings.language_tag })}
                </option>
              {/if}
              {#each dictionaries as dictionary}
                <option value={dictionary.language_tag}>
                  {dictionary.display_name}{dictionary.user ? ` (${tr('userDictionarySuffix')})` : ''}
                </option>
              {/each}
            </select>
          </label>

          <p class="compact muted">{tr('dictionaryOfflineState')}</p>

          {#if !selectedDictionary}
            <p class="compact">{tr('dictionaryUnavailable')}</p>
          {/if}

          <div class="personal-dictionary-panel">
            <p class="compact"><strong>{tr('dictionaryInstallLocal')}</strong></p>
            <label>
              {tr('dictionaryLanguageTag')}
              <input bind:value={installDictionaryLanguageTag} autocomplete="off" spellcheck="false" />
            </label>
            <div class="field-button-row" role="group" aria-label={tr('dictionaryInstallLocal')}>
              <button type="button" onclick={chooseDictionaryAffFile}>{tr('dictionaryChooseAff')}</button>
              <span class="compact muted">
                {installDictionaryAffPath ? tr('dictionaryAffSelected') : tr('dictionaryAffNotSelected')}
              </span>
              <button type="button" onclick={chooseDictionaryDicFile}>{tr('dictionaryChooseDic')}</button>
              <span class="compact muted">
                {installDictionaryDicPath ? tr('dictionaryDicSelected') : tr('dictionaryDicNotSelected')}
              </span>
              <button type="button" disabled={!canInstallDictionary} onclick={installDictionary}>
                {tr('dictionaryInstall')}
              </button>
            </div>
          </div>

          <div class="personal-dictionary-panel">
            <div class="field-button-row">
              <p class="compact"><strong>{tr('personalDictionary')}</strong></p>
              <button type="button" onclick={() => refreshPersonalDictionaryWords()}>
                {tr('personalDictionaryRefresh')}
              </button>
            </div>

            {#if personalDictionaryError}
              <p class="compact">{personalDictionaryError}</p>
            {:else if personalDictionaryWords.length === 0}
              <p class="compact muted">{tr('personalDictionaryEmpty')}</p>
            {:else}
              <div class="page-regions-grid" role="list" aria-label={tr('personalDictionaryWords')}>
                {#each personalDictionaryWords as word (word)}
                  <article role="listitem">
                    <p class="compact">
                      <strong>{word}</strong>
                    </p>
                    <button
                      aria-label={`${tr('personalDictionaryRemove')} ${word}`}
                      type="button"
                      onclick={() => removePersonalDictionaryWord(word)}
                    >
                      {tr('personalDictionaryRemove')}
                    </button>
                  </article>
                {/each}
              </div>
            {/if}
          </div>

          <div class="page-regions-grid" role="list" aria-label={tr('installedDictionaries')}>
            {#if dictionaries.length === 0}
              <article role="listitem">
                <p class="compact muted">{tr('dictionaryNoneInstalled')}</p>
              </article>
            {:else}
              {#each dictionaries as dictionary}
                <article
                  aria-current={dictionaryMatchesSelectedLanguage(dictionary, normalizedDictionaryLanguageTag)
                    ? 'true'
                    : undefined}
                  role="listitem"
                >
                  <p class="compact">
                    <strong>{dictionary.display_name}</strong>
                    <span class="muted">{tr('dictionaryLanguageTag')}: {dictionary.language_tag}</span>
                  </p>
                  <dl>
                    <div>
                      <dt>{tr('dictionarySourceType')}</dt>
                      <dd>{dictionarySourceTypeLabel(dictionary)}</dd>
                    </div>
                    <div>
                      <dt>{tr('dictionarySourceLabel')}</dt>
                      <dd>{dictionarySourceLabel(dictionary)}</dd>
                    </div>
                    <div>
                      <dt>{tr('dictionaryLicense')}</dt>
                      <dd>{dictionary.license}</dd>
                    </div>
                  </dl>
                  {#if dictionary.user && !dictionary.bundled}
                    <button
                      aria-label={`${tr('dictionaryRemove')} ${dictionary.display_name}`}
                      disabled={removingDictionaryLanguageTag === dictionary.language_tag}
                      type="button"
                      onclick={() => removeInstalledUserDictionary(dictionary)}
                    >
                      {tr('dictionaryRemove')}
                    </button>
                  {/if}
                </article>
              {/each}
            {/if}
          </div>
        </section>

        <label>
          {tr('uiLocale')}
          <select bind:value={settings.ui_locale}>
            {#each uiLocales as locale}
              <option value={locale.tag}>{locale.display_name}</option>
            {/each}
          </select>
        </label>

        <label class="check-row">
          <input bind:checked={settings.high_contrast} type="checkbox" />
          {tr('highContrast')}
        </label>

        <section class="settings-group" aria-labelledby="accessibility-performance-settings-heading">
          <h3 id="accessibility-performance-settings-heading">{tr('accessibilityAndPerformance')}</h3>
          <label class="check-row">
            <input bind:checked={settings.large_toolbar} type="checkbox" />
            {tr('largeToolbar')}
          </label>
          <label class="check-row">
            <input bind:checked={settings.reduced_motion} type="checkbox" />
            {tr('reducedMotion')}
          </label>
          <label class="check-row">
            <input bind:checked={settings.low_resource} type="checkbox" />
            {tr('lowResourceMode')}
          </label>
        </section>

        <section class="settings-group" aria-labelledby="smart-typing-settings-heading">
          <h3 id="smart-typing-settings-heading">{tr('smartTyping')}</h3>
          <label class="check-row">
            <input bind:checked={settings.smart_typing.capitalize_sentences} type="checkbox" />
            {tr('capitalizeSentences')}
          </label>
          <label class="check-row">
            <input bind:checked={settings.smart_typing.smart_quotes} type="checkbox" />
            {tr('smartQuotes')}
          </label>
          <label class="check-row">
            <input bind:checked={settings.smart_typing.smart_dashes} type="checkbox" />
            {tr('smartDashes')}
          </label>
          <label class="check-row">
            <input bind:checked={settings.smart_typing.typo_replacements} type="checkbox" />
            {tr('typoReplacements')}
          </label>
          <label class="check-row">
            <input bind:checked={settings.smart_typing.list_triggers} type="checkbox" />
            {tr('smartListTriggers')}
          </label>
        </section>

        <label class="check-row muted">
          <input checked={settings.telemetry_enabled} disabled type="checkbox" />
          {tr('telemetry')}
        </label>

        <div class="field-button-row">
          <button type="button" onclick={saveSettings}>{tr('saveSettings')}</button>
          <button type="button" onclick={resetSettings}>{tr('resetSettings')}</button>
        </div>
      </div>
    </div>

    <div
      aria-label={tr('about900Word')}
      class:hidden-view={activeView !== 'about'}
      class="panel-view"
      id="about-view"
      role="tabpanel"
    >
      <div class="form-surface">
        <h2>900Word</h2>
        <dl>
          <div><dt>{tr('version')}</dt><dd>0.1.0</dd></div>
          <div><dt>{tr('license')}</dt><dd>GPL-3.0-or-later</dd></div>
          <div><dt>{tr('documentFormat')}</dt><dd>OpenDocument Text</dd></div>
          <div><dt>{tr('telemetry')}</dt><dd>{tr('off')}</dd></div>
        </dl>
      </div>
    </div>
  </section>

  <footer class="bottom-toolbar" aria-label={tr('documentStatistics')}>
    <div class="bottom-group">
      <span class="bottom-label">{tr('file')}</span>
      <span>{tr('saved')}: <strong>{fileState.has_current_path ? tr('yes') : tr('no')}</strong></span>
      <span>{tr('dirty')}: <strong>{fileState.dirty ? tr('yes') : tr('no')}</strong></span>
    </div>
    <div class="bottom-group">
      <button
        aria-expanded={statsPanelOpen}
        class="bottom-action"
        type="button"
        onclick={toggleStatsPanel}
      >
        {tr('stats')}
      </button>
      <button
        aria-expanded={inspectorPanelOpen}
        class="bottom-action"
        type="button"
        onclick={toggleDocumentInspector}
      >
        {tr('documentInspector')}
      </button>
      <span>{tr('words')}: <strong>{expandedStats.wordCount}</strong></span>
      <span>{tr('selectionWords')}: <strong>{expandedStats.selectionWordCount}</strong></span>
      <span>{tr('estimatedPages')}: <strong>{expandedStats.estimatedPageCount}</strong></span>
      {#if inspectorPanelOpen}
        <div class="status-panel inspector-panel" role="dialog" aria-label={tr('documentInspector')}>
          <div class="status-panel-heading">
            <h2>{tr('documentInspector')}</h2>
            <span>{inspectorSummary.format}</span>
          </div>
          <dl>
            <div><dt>{tr('documentFormat')}</dt><dd>{inspectorSummary.format}</dd></div>
            <div><dt>{tr('savedStatus')}</dt><dd>{inspectorSavedStatusLabel(inspectorSummary.savedStatus)}</dd></div>
            <div><dt>{tr('savedLocation')}</dt><dd>{inspectorLocationStatusLabel(inspectorSummary.locationStatus)}</dd></div>
            <div><dt>{tr('created')}</dt><dd>{inspectorSummary.createdAt || tr('notAvailable')}</dd></div>
            <div><dt>{tr('modified')}</dt><dd>{inspectorSummary.modifiedAt || tr('notAvailable')}</dd></div>
            <div><dt>{tr('pageSize')}</dt><dd>{inspectorSummary.pageSize || tr('notAvailable')}</dd></div>
            <div><dt>{tr('words')}</dt><dd>{inspectorSummary.wordCount}</dd></div>
            <div><dt>{tr('charactersWithSpaces')}</dt><dd>{inspectorSummary.characterCount}</dd></div>
            <div><dt>{tr('charactersNoSpaces')}</dt><dd>{inspectorSummary.characterCountWithoutSpaces}</dd></div>
            <div><dt>{tr('paragraphs')}</dt><dd>{inspectorSummary.paragraphCount}</dd></div>
            <div><dt>{tr('blocks')}</dt><dd>{inspectorSummary.blockCount}</dd></div>
            <div><dt>{tr('estimatedPages')}</dt><dd>{inspectorSummary.estimatedPageCount}</dd></div>
            <div><dt>{tr('selectionWords')}</dt><dd>{inspectorSummary.selectionWordCount}</dd></div>
            <div><dt>{tr('embeddedImages')}</dt><dd>{inspectorSummary.embeddedImageCount}</dd></div>
            <div><dt>{tr('embeddedImageBytes')}</dt><dd>{inspectorSummary.embeddedImageBytesLabel}</dd></div>
            <div><dt>{tr('comments')}</dt><dd>{inspectorSummary.commentCount}</dd></div>
            <div><dt>{tr('unresolvedComments')}</dt><dd>{inspectorSummary.unresolvedCommentCount}</dd></div>
            <div><dt>{tr('trackChangesStatus')}</dt><dd>{inspectorSummary.trackChangesRecording ? tr('recording') : tr('notRecording')}</dd></div>
            <div><dt>{tr('trackedChanges')}</dt><dd>{inspectorSummary.trackedChangeCount}</dd></div>
            <div><dt>{tr('footnotes')}</dt><dd>{inspectorSummary.footnoteCount}</dd></div>
            <div><dt>{tr('endnotes')}</dt><dd>{inspectorSummary.endnoteCount}</dd></div>
          </dl>
          <section class="inspector-warning-section" aria-label={tr('privacyWarnings')}>
            <h3>{tr('privacyWarnings')}</h3>
            {#if inspectorSummary.privacyWarnings.length > 0}
              <ul class="privacy-warning-list">
                {#each inspectorSummary.privacyWarnings as warning}
                  <li>{inspectorPrivacyWarningLabel(warning)}</li>
                {/each}
              </ul>
            {:else}
              <p class="status-panel-note">{tr('noPrivacyWarnings')}</p>
            {/if}
          </section>
        </div>
      {/if}
      {#if statsPanelOpen}
        <div class="status-panel stats-panel" role="dialog" aria-label={tr('documentStatistics')}>
          <p class="status-panel-note">{tr('statsEstimateNote')}</p>
          <dl>
            <div><dt>{tr('words')}</dt><dd>{expandedStats.wordCount}</dd></div>
            <div><dt>{tr('selectionWords')}</dt><dd>{expandedStats.selectionWordCount}</dd></div>
            <div><dt>{tr('charactersWithSpaces')}</dt><dd>{expandedStats.characterCountWithSpaces}</dd></div>
            <div><dt>{tr('charactersNoSpaces')}</dt><dd>{expandedStats.characterCountWithoutSpaces}</dd></div>
            <div><dt>{tr('paragraphs')}</dt><dd>{expandedStats.paragraphCount}</dd></div>
            <div><dt>{tr('blocks')}</dt><dd>{expandedStats.blockCount}</dd></div>
            <div><dt>{tr('estimatedPages')}</dt><dd>{expandedStats.estimatedPageCount}</dd></div>
            <div><dt>{tr('estimatedReadingTime')}</dt><dd>{tr('readingMinutes', { count: expandedStats.estimatedReadingMinutes })}</dd></div>
            <div><dt>{tr('comments')}</dt><dd>{expandedStats.commentCount}</dd></div>
            <div><dt>{tr('unresolvedComments')}</dt><dd>{expandedStats.unresolvedCommentCount}</dd></div>
            <div><dt>{tr('trackChangesStatus')}</dt><dd>{expandedStats.trackChangesRecording ? tr('recording') : tr('notRecording')}</dd></div>
            <div><dt>{tr('trackedChanges')}</dt><dd>{expandedStats.trackedChangeCount}</dd></div>
            <div><dt>{tr('images')}</dt><dd>{expandedStats.imageCount}</dd></div>
            <div><dt>{tr('embeddedAssets')}</dt><dd>{expandedStats.assetCount}</dd></div>
            <div><dt>{tr('footnotes')}</dt><dd>{expandedStats.footnoteCount}</dd></div>
            <div><dt>{tr('endnotes')}</dt><dd>{expandedStats.endnoteCount}</dd></div>
            <div><dt>{tr('pageSize')}</dt><dd>{expandedStats.pageSize}</dd></div>
          </dl>
        </div>
      {/if}
    </div>
    <div class="bottom-group">
      <span class="bottom-label">{tr('spelling')}</span>
      <button class="bottom-action" type="button" onclick={checkSpelling}>{tr('checkSpelling')}</button>
      {#if spellIssues.length === 0}
        <span>{tr('noIssues')}</span>
      {:else}
        <span>{tr('spellIssueCount', { count: spellIssues.length })}</span>
      {/if}
    </div>
  </footer>

  <iframe bind:this={printFrame} class="print-frame" title={tr('printFrame')}></iframe>
</main>
