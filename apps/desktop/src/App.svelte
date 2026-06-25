<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
  import { onMount, tick } from 'svelte';
  import {
    createEditor,
    createEditorBookmarkId,
    editorTopLevelInsertionIndex,
    editTableStructure,
    findEditorTextMatches,
    insertTable as insertEditorTable,
    replaceAllEditorText,
    replaceEditorTextRange,
    removeEditorLink,
    removeEditorBlockBookmark,
    restoreEditorSelection,
    selectEditorTopLevelBlock,
    selectEditorTextRange,
    adjustSelectedListLevel,
    clearEditorDirectFormatting,
    setEditorBlockType,
    setEditorBlockBookmark,
    setEditorLink,
    setEditorParagraphFormat,
    setSelectedImageAttrs,
    setEditorSpellIssues,
    setEditorTextStyle,
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
    pageFieldTokens,
    pageRegionIsReadOnly,
    pageRegionTextToBlocks,
    pageRegionToText,
    type DocumentStyle,
    type DocumentCommand,
    type DocumentState,
    type EditorProjectedChange,
    type DocumentLinkTarget,
    type DocumentOutlineEntry,
    type ListBlock,
    type PageField,
    type PageRegionKind,
    type PageSetup
  } from './lib/documentProjection';
  import { localeDirection, translate, uiLocales, type UiStringKey } from './lib/i18n';
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

  interface DocumentStats {
    word_count: number;
    character_count: number;
    block_count: number;
  }

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

  let title = $state('900Word');
  let status = $state(translate('en-US', 'starting'));
  let activeView = $state<ViewId>('editor');
  let editorViewMode = $state<EditorViewMode>('page-layout');
  let zoomChoice = $state<EditorZoomChoice>('fit-width');
  let customZoomPercent = $state(100);
  let showRulers = $state(false);
  let editorViewportWidth = $state(0);
  let plainText = $state('');
  let stats = $state<DocumentStats>({ word_count: 0, character_count: 0, block_count: 0 });
  let spellIssues = $state<SpellIssue[]>([]);
  let projectionWarnings = $state<string[]>([]);
  let exportPathInput = $state('');
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
  let spellIssueRanges = $state<EditorSpellIssueRange[]>([]);
  let spellPopover = $state<{
    issue: EditorSpellIssueRange;
    x: number;
    y: number;
  } | null>(null);
  let statsPanelOpen = $state(false);
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
  let fileMenuOpen = $state(false);
  let exportMenuOpen = $state(false);
  let navigatorHeadings = $state<DocumentOutlineEntry[]>([]);
  let linkTargets = $state<DocumentLinkTarget[]>([]);
  let dictionaries = $state<DictionaryInfo[]>([]);
  let settings = $state<Settings>({
    telemetry_enabled: false,
    language_tag: 'en-US',
    ui_locale: 'en-US',
    high_contrast: false
  });
  let uiDirection = $derived(localeDirection(settings.ui_locale));
  let documentState: DocumentState | undefined;
  let editorEditable = $derived(documentState ? canEditProjectedDocument(documentState) : false);
  let selectionWordCount = $derived(activeFormatting.selectionWordCount);
  let characterCountNoSpaces = $derived(countNonWhitespaceCharacters(plainText));
  let paragraphCount = $derived(countProjectedParagraphs(documentState));
  let readingMinutes = $derived(stats.word_count === 0 ? 0 : Math.max(1, Math.ceil(stats.word_count / 200)));
  let fitWidthZoom = $derived(
    editorViewMode === 'page-layout' ? fitWidthZoomPercent(pageSetup, editorViewportWidth, showRulers ? 22 : 0) : 100
  );
  let effectiveZoomPercent = $derived(zoomChoice === 'fit-width' ? fitWidthZoom : customZoomPercent);
  let editorSurfaceStyle = $derived(editorViewportStyle(pageSetup, effectiveZoomPercent));
  let showWorkspaceSidebar = $derived(
    navigatorHeadings.length > 0 ||
    fileState.recent_documents.length > 0 ||
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
  let linkInput = $state<HTMLInputElement | undefined>();
  let fileMenuRoot = $state<HTMLDivElement | undefined>();
  let linkToolsRoot = $state<HTMLDivElement | undefined>();
  let view: ReturnType<typeof createEditor> | undefined;

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
    stats = await invoke<DocumentStats>('get_document_stats');
    projectionWarnings = collectDocumentWarnings(document);
    spellIssues = [];
    spellIssueRanges = [];
    spellPopover = null;
    const editable = canEditProjectedDocument(document);
    status = editable ? nextStatus : tr('editorReadOnly');
    view?.destroy();
    view = createEditor(editorHost, document, handleEditorChange, {
      editable,
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
    stats = await invoke<DocumentStats>('get_document_stats');
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
    const document = await invoke<DocumentState>('open_document', {
      path
    });
    await loadDocumentIntoEditor(document, tr('documentOpened'));
    await refreshFileState();
  }

  async function openDocumentWithDialog() {
    const selected = await openDialog({
      multiple: false,
      filters: odtFileFilters
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
    dictionaries = await invoke<DictionaryInfo[]>('list_dictionaries');
    templates = await invoke<TemplateSummary[]>('list_templates');
  }

  async function saveSettings() {
    settings = await invoke<Settings>('update_settings', {
      settings
    });
    status = tr('settingsUpdated');
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
      const result = await invoke<ExportFileResult>('export_pdf_to_path', {
        path: exportPathInput
      });
      status = tr('exportPdfSaved', { bytes: result.byte_len });
    } catch (error) {
      setStatusFromError(error);
    }
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
        languageTag: settings.language_tag
      });
      ignoredSpellWords.add(normalizeSpellWord(word));
      refreshVisibleSpellIssues();
      dictionaries = await invoke<DictionaryInfo[]>('list_dictionaries');
      status = tr('spellAddedToDictionary', { word });
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
    if (event.button !== 0) {
      return;
    }
    captureToolbarSelection(true);
    event.preventDefault();
    restoreEditorSelection(view, lastEditorSelection);
    command();
  }

  function runToolbarKeyboardCommand(event: MouseEvent, command: () => void) {
    event.preventDefault();
    if (event.detail !== 0) {
      return;
    }
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

  async function openFindPanel() {
    findPanelOpen = true;
    await tick();
    findInput?.focus();
    findInput?.select();
  }

  function toggleFindPanel() {
    if (findPanelOpen) {
      findPanelOpen = false;
      view?.focus();
    } else {
      openFindPanel();
    }
  }

  function closeFileMenu() {
    fileMenuOpen = false;
    exportMenuOpen = false;
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

  function countNonWhitespaceCharacters(text: string) {
    return Array.from(text).filter((char) => !/\s/.test(char)).length;
  }

  function countProjectedParagraphs(document: DocumentState | undefined) {
    if (!document) {
      return 0;
    }
    return document.sections.reduce(
      (total, section) =>
        total +
        section.blocks.reduce((blockTotal, block) => {
          if (block.type === 'Paragraph' || block.type === 'Heading') {
            return blockTotal + 1;
          }
          if (isProjectedListBlock(block)) {
            return blockTotal + block.value.items.length;
          }
          return blockTotal;
        }, 0),
      0
    );
  }

  function isProjectedListBlock(
    block: DocumentState['sections'][number]['blocks'][number]
  ): block is ListBlock {
    return (
      block.type === 'List' &&
      typeof block.value === 'object' &&
      block.value !== null &&
      'items' in block.value &&
      Array.isArray(block.value.items)
    );
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

    const mod = event.metaKey || event.ctrlKey;
    if (!mod) {
      return;
    }

    const key = event.key.toLowerCase();
    const target = event.target instanceof HTMLElement ? event.target : undefined;
    const targetIsInput = target?.tagName === 'INPUT' || target?.tagName === 'SELECT' || target?.tagName === 'TEXTAREA';
    if (targetIsInput && !['f', 'p', 's'].includes(key)) {
      return;
    }

    if (key === 'b') {
      event.preventDefault();
      applyInlineMark('bold', tr('bold'));
    } else if (key === 'i') {
      event.preventDefault();
      applyInlineMark('italic', tr('italic'));
    } else if (key === 'u') {
      event.preventDefault();
      applyInlineMark('underline', tr('underline'));
    } else if (event.altKey && ['1', '2', '3'].includes(key)) {
      event.preventDefault();
      applyHeading(Number(key));
    } else if (event.shiftKey && event.code === 'Digit7') {
      event.preventDefault();
      applyList('ordered_list');
    } else if (event.shiftKey && event.code === 'Digit8') {
      event.preventDefault();
      applyList('bullet_list');
    } else if (key === ']') {
      event.preventDefault();
      adjustListLevel(1);
    } else if (key === '[') {
      event.preventDefault();
      adjustListLevel(-1);
    } else if (key === 'k') {
      event.preventDefault();
      void openLinkPanel();
    } else if (key === 'f') {
      event.preventDefault();
      activeView = 'editor';
      openFindPanel();
    } else if (key === 'n') {
      event.preventDefault();
      newDocument().catch(setStatusFromError);
    } else if (key === 'o') {
      event.preventDefault();
      openDocumentWithDialog().catch(setStatusFromError);
    } else if (key === 's') {
      event.preventDefault();
      if (event.shiftKey) {
        saveDocumentAsWithDialog().catch(setStatusFromError);
      } else {
        saveCurrentDocument().catch(setStatusFromError);
      }
    } else if (key === 'p') {
      event.preventDefault();
      printDocument().catch(setStatusFromError);
    } else if (key === 'z') {
      event.preventDefault();
      if (event.shiftKey) {
        redoDocument();
      } else {
        undoDocument();
      }
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

<main class:high-contrast={settings.high_contrast} class="app-shell" dir={uiDirection} lang={settings.ui_locale}>
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
            <span class="menu-shortcut">Cmd+N</span>
          </button>

          <button class="menu-command" type="button" onclick={() => runFileMenuAction(openDocumentWithDialog)}>
            <span class="menu-glyph glyph-open" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('open')}</span>
            </span>
            <span class="menu-shortcut">Cmd+O</span>
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
            <span class="menu-shortcut">Cmd+S</span>
          </button>
          <button class="menu-command" type="button" onclick={() => runFileMenuAction(saveDocumentAsWithDialog)}>
            <span class="menu-glyph glyph-save-as" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('saveAs')}</span>
            </span>
            <span class="menu-shortcut">Cmd+Shift+S</span>
          </button>
          <button class="menu-command" type="button" onclick={() => runFileMenuAction(autosaveDocument)}>
            <span class="menu-glyph glyph-autosave" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('autosave')}</span>
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
                bind:value={exportPathInput}
                placeholder={tr('exportPathPlaceholder')}
                type="text"
              />
              <div class="export-format-grid">
                <button class="format-option" type="button" onclick={() => runFileMenuAction(exportText)}>
                  <span>{tr('txt')}</span>
                </button>
                <button class="format-option" type="button" onclick={() => runFileMenuAction(exportHtml)}>
                  <span>{tr('html')}</span>
                </button>
                <button class="format-option" type="button" onclick={() => runFileMenuAction(exportPdf)}>
                  <span>{tr('pdf')}</span>
                </button>
              </div>
            </div>
          {/if}

          <button class="menu-command" type="button" onclick={() => runFileMenuAction(printDocument)}>
            <span class="menu-glyph glyph-print" aria-hidden="true"></span>
            <span class="menu-command-main">
              <span class="menu-command-label">{tr('print')}</span>
            </span>
            <span class="menu-shortcut">Cmd+P</span>
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
      <button type="button" onclick={undoDocument}>{tr('undo')}</button>
      <button type="button" onclick={redoDocument}>{tr('redo')}</button>
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
        title={`${tr('bold')} (Cmd+B)`}
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
        title={`${tr('italic')} (Cmd+I)`}
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
        title={`${tr('underline')} (Cmd+U)`}
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
        title={`${tr('link')} (Cmd+K)`}
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
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, applyParagraph)}
      >
        {tr('paragraph')}
      </button>
      <button
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyHeading(1))}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyHeading(1))}
      >
        {tr('heading1')}
      </button>
      <button
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyHeading(2))}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyHeading(2))}
      >
        {tr('heading2')}
      </button>
      <button
        disabled={!editorEditable}
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyHeading(3))}
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
        title={tr('bulletList')}
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
        title={tr('numberedList')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => applyList('ordered_list'))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyList('ordered_list'))}
      >
        1.
      </button>
      <button
        disabled={!editorEditable}
        title={tr('decreaseIndent')}
        type="button"
        onpointerdown={(event) => runToolbarPointerCommand(event, () => adjustListLevel(-1))}
        onclick={(event) => runToolbarKeyboardCommand(event, () => adjustListLevel(-1))}
      >
        &lt;
      </button>
      <button
        disabled={!editorEditable}
        title={tr('increaseIndent')}
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
        title={tr('findAndReplace')}
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
          <input aria-label={tr('replace')} bind:value={replaceText} placeholder={tr('replace')} type="text" />
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
        {#if navigatorHeadings.length > 0}
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

        {#if fileState.recent_documents.length > 0}
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

        <label>
          {tr('dictionary')}
          <select bind:value={settings.language_tag}>
            {#each dictionaries as dictionary}
              <option value={dictionary.language_tag}>
                {dictionary.display_name}{dictionary.user ? ` (${tr('userDictionarySuffix')})` : ''}
              </option>
            {/each}
          </select>
        </label>

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

        <label class="check-row muted">
          <input checked={settings.telemetry_enabled} disabled type="checkbox" />
          {tr('telemetry')}
        </label>

        <button type="button" onclick={saveSettings}>{tr('saveSettings')}</button>
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
        onclick={() => (statsPanelOpen = !statsPanelOpen)}
      >
        {tr('stats')}
      </button>
      <span>{tr('words')}: <strong>{stats.word_count}</strong></span>
      <span>{tr('selectionWords')}: <strong>{selectionWordCount}</strong></span>
      <span>{tr('characters')}: <strong>{stats.character_count}</strong></span>
      <span>{tr('blocks')}: <strong>{stats.block_count}</strong></span>
      {#if statsPanelOpen}
        <div class="status-panel" role="dialog" aria-label={tr('documentStatistics')}>
          <dl>
            <div><dt>{tr('words')}</dt><dd>{stats.word_count}</dd></div>
            <div><dt>{tr('selectionWords')}</dt><dd>{selectionWordCount}</dd></div>
            <div><dt>{tr('characters')}</dt><dd>{stats.character_count}</dd></div>
            <div><dt>{tr('charactersNoSpaces')}</dt><dd>{characterCountNoSpaces}</dd></div>
            <div><dt>{tr('paragraphs')}</dt><dd>{paragraphCount}</dd></div>
            <div><dt>{tr('blocks')}</dt><dd>{stats.block_count}</dd></div>
            <div><dt>{tr('readingTime')}</dt><dd>{tr('readingMinutes', { count: readingMinutes })}</dd></div>
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
