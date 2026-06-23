<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount, tick } from 'svelte';
  import {
    createEditor,
    findEditorTextMatches,
    replaceAllEditorText,
    replaceEditorTextRange,
    selectEditorTextRange,
    setEditorBlockType,
    toggleEditorMark,
    type EditorFindMatch,
    type EditorSelectionSnapshot,
    type SupportedMarkName
  } from './lib/editor';
  import {
    buildEditorSyncCommands,
    canEditProjectedDocument,
    documentProjectionWarnings,
    documentToText,
    type DocumentState,
    type EditorProjectedChange,
    type PageSetup
  } from './lib/documentProjection';
  import { localeDirection, translate, uiLocales, type UiStringKey } from './lib/i18n';

  interface DocumentStats {
    word_count: number;
    character_count: number;
    block_count: number;
  }

  interface SpellIssue {
    word: string;
    byte_start: number;
    byte_end: number;
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

  type ViewId = 'editor' | 'settings' | 'about';
  const viewOrder: ViewId[] = ['editor', 'settings', 'about'];

  let title = $state('900Word');
  let status = $state(translate('en-US', 'starting'));
  let activeView = $state<ViewId>('editor');
  let plainText = $state('');
  let stats = $state<DocumentStats>({ word_count: 0, character_count: 0, block_count: 0 });
  let spellIssues = $state<SpellIssue[]>([]);
  let projectionWarnings = $state<string[]>([]);
  let filePathInput = $state('');
  let exportPathInput = $state('');
  let fileState = $state<DocumentFileState>({
    has_current_path: false,
    dirty: false,
    recent_documents: [],
    recovery_documents: []
  });
  let templates = $state<TemplateSummary[]>([]);
  let selectedTemplateId = $state('blank');
  let pageSetup = $state<PageSetup>(defaultPageSetup());
  let findQuery = $state('');
  let replaceText = $state('');
  let findCaseSensitive = $state(false);
  let findRanges = $state<EditorFindMatch[]>([]);
  let activeFindIndex = $state(-1);
  let findPanelOpen = $state(false);
  let fileMenuOpen = $state(false);
  let exportMenuOpen = $state(false);
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
  let showWorkspaceSidebar = $derived(
    fileState.recent_documents.length > 0 ||
      fileState.recovery_documents.length > 0 ||
      projectionWarnings.length > 0
  );
  let editorSyncQueue = Promise.resolve();
  let editorSyncError: string | null = null;
  let lastEditorSelection = $state<EditorSelectionSnapshot | undefined>();
  let editorHost: HTMLDivElement;
  let printFrame: HTMLIFrameElement;
  let findInput = $state<HTMLInputElement | undefined>();
  let fileMenuRoot = $state<HTMLDivElement | undefined>();
  let view: ReturnType<typeof createEditor> | undefined;

  async function newDocument() {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('new_document');
    await loadDocumentIntoEditor(document, tr('ready'));
    filePathInput = '';
    await refreshFileState();
  }

  async function newDocumentFromTemplate() {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('new_document_from_template', {
      templateId: selectedTemplateId
    });
    await loadDocumentIntoEditor(document, tr('templateLoaded'));
    filePathInput = '';
    await refreshFileState();
  }

  async function loadDocumentIntoEditor(document: DocumentState, nextStatus: string) {
    editorSyncError = null;
    documentState = document;
    title = document.meta.title;
    plainText = documentToText(document);
    pageSetup = document.sections[0]?.page ?? defaultPageSetup();
    stats = await invoke<DocumentStats>('get_document_stats');
    projectionWarnings = collectDocumentWarnings(document);
    const editable = canEditProjectedDocument(document);
    status = editable ? nextStatus : tr('editorReadOnly');
    view?.destroy();
    view = createEditor(editorHost, document, handleEditorChange, {
      editable,
      onSelectionChange: (selection) => {
        lastEditorSelection = selection;
      }
    });
    refreshFindState();
  }

  function handleEditorChange(change: EditorProjectedChange) {
    plainText = change.text;
    refreshFindState();
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

  async function openDocumentFromPath() {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('open_document', {
      path: filePathInput
    });
    await loadDocumentIntoEditor(document, tr('documentOpened'));
    await refreshFileState();
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

  async function saveDocumentAsPath() {
    await waitForEditorSync();
    fileState = await invoke<DocumentFileState>('save_document_as', {
      path: filePathInput
    });
    status = tr('documentSavedAs');
  }

  async function autosaveDocument() {
    await waitForEditorSync();
    await invoke<RecoveryDocumentSummary>('autosave_document');
    await refreshFileState();
    status = tr('autosaveUpdated');
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
    spellIssues = result.issues;
    if (result.warnings.length > 0) {
      status = `${tr('offlineDictionaryFallback')}: ${result.dictionary_display_name}`;
    } else {
      status =
        spellIssues.length === 0
          ? tr('statusNoSpellingIssues')
          : tr('spellIssueCount', { count: spellIssues.length });
    }
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

  function runToolbarMouseCommand(event: MouseEvent, command: () => void) {
    event.preventDefault();
    command();
  }

  function runToolbarKeyboardCommand(event: MouseEvent, command: () => void) {
    if (event.detail === 0) {
      command();
    }
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

  function applyHeading(level: number) {
    if (!editorEditable) {
      status = tr('editorReadOnly');
      return;
    }
    status = setEditorBlockType(view, 'heading', { level }, lastEditorSelection)
      ? tr('headingApplied', { level })
      : tr('headingUnchanged', { level });
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

  function handleWindowClick(event: MouseEvent) {
    if (!fileMenuOpen || !(event.target instanceof Node) || fileMenuRoot?.contains(event.target)) {
      return;
    }
    closeFileMenu();
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

  function updatePageSetupField(field: keyof PageSetup, value: number) {
    if (!Number.isFinite(value)) {
      return;
    }
    pageSetup = { ...pageSetup, [field]: Math.trunc(value) };
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
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
    } else if (key === 'f') {
      event.preventDefault();
      activeView = 'editor';
      openFindPanel();
    } else if (key === 's') {
      event.preventDefault();
      saveCurrentDocument().catch(setStatusFromError);
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
    status = error instanceof Error ? error.message : String(error);
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
      <p>{status}</p>
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
        <div class="file-menu-popover" role="menu" aria-label={tr('file')}>
          <button role="menuitem" type="button" onclick={() => runFileMenuAction(newDocument)}>{tr('new')}</button>

          <div class="menu-field" role="none">
            <label for="file-menu-odt-path">{tr('odtPath')}</label>
            <input
              id="file-menu-odt-path"
              aria-label={tr('odtPath')}
              bind:value={filePathInput}
              placeholder={tr('odtPathPlaceholder')}
              type="text"
            />
            <button role="menuitem" type="button" onclick={() => runFileMenuAction(openDocumentFromPath)}>
              {tr('open')}
            </button>
          </div>

          <div class="menu-separator" role="separator"></div>

          <button
            disabled={!fileState.has_current_path}
            role="menuitem"
            type="button"
            onclick={() => runFileMenuAction(saveCurrentDocument)}
          >
            {tr('save')}
          </button>
          <button role="menuitem" type="button" onclick={() => runFileMenuAction(saveDocumentAsPath)}>
            {tr('saveAs')}
          </button>
          <button role="menuitem" type="button" onclick={() => runFileMenuAction(autosaveDocument)}>
            {tr('autosave')}
          </button>

          <div class="menu-separator" role="separator"></div>

          <button
            aria-expanded={exportMenuOpen}
            aria-haspopup="menu"
            class="submenu-trigger"
            role="menuitem"
            type="button"
            onclick={() => (exportMenuOpen = !exportMenuOpen)}
          >
            {tr('export')}
          </button>
          {#if exportMenuOpen}
            <div class="file-submenu-panel" role="menu" aria-label={tr('export')}>
              <label for="file-menu-export-path">{tr('exportPath')}</label>
              <input
                id="file-menu-export-path"
                aria-label={tr('exportPath')}
                bind:value={exportPathInput}
                placeholder={tr('exportPathPlaceholder')}
                type="text"
              />
              <button role="menuitem" type="button" onclick={() => runFileMenuAction(exportText)}>
                {tr('txt')}
              </button>
              <button role="menuitem" type="button" onclick={() => runFileMenuAction(exportHtml)}>
                {tr('html')}
              </button>
              <button role="menuitem" type="button" onclick={() => runFileMenuAction(exportPdf)}>
                {tr('pdf')}
              </button>
            </div>
          {/if}

          <button role="menuitem" type="button" onclick={() => runFileMenuAction(printDocument)}>
            {tr('print')}
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

    <div class="tool-group formatting-tools" role="group" aria-label={tr('textFormatting')}>
      <button
        aria-label={tr('bold')}
        class="format-button strong"
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('bold', tr('bold')))}
        title={tr('bold')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('bold', tr('bold')))}
      >
        B
      </button>
      <button
        aria-label={tr('italic')}
        class="format-button italic"
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('italic', tr('italic')))}
        title={tr('italic')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('italic', tr('italic')))}
      >
        I
      </button>
      <button
        aria-label={tr('underline')}
        class="format-button underline"
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('underline', tr('underline')))}
        title={tr('underline')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('underline', tr('underline')))}
      >
        U
      </button>
      <button
        aria-label={tr('strikethrough')}
        class="format-button strike"
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('strikethrough', tr('strikethrough')))}
        title={tr('strikethrough')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('strikethrough', tr('strikethrough')))}
      >
        S
      </button>
      <button
        aria-label={tr('superscript')}
        class="format-button script"
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('superscript', tr('superscript')))}
        title={tr('superscript')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('superscript', tr('superscript')))}
      >
        x<sup>2</sup>
      </button>
      <button
        aria-label={tr('subscript')}
        class="format-button script"
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyInlineMark('subscript', tr('subscript')))}
        title={tr('subscript')}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyInlineMark('subscript', tr('subscript')))}
      >
        x<sub>2</sub>
      </button>
    </div>

    <div class="tool-group" role="group" aria-label={tr('blockFormatting')}>
      <button
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, applyParagraph)}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, applyParagraph)}
      >
        {tr('paragraph')}
      </button>
      <button
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyHeading(1))}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyHeading(1))}
      >
        {tr('heading1')}
      </button>
      <button
        disabled={!editorEditable}
        onmousedown={(event) => runToolbarMouseCommand(event, () => applyHeading(2))}
        type="button"
        onclick={(event) => runToolbarKeyboardCommand(event, () => applyHeading(2))}
      >
        {tr('heading2')}
      </button>
    </div>

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
      class:hidden-view={activeView !== 'editor'}
      class="editor-panel"
      id="editor-view"
      role="tabpanel"
    >
      <div
        bind:this={editorHost}
        class:empty-document={plainText.trim().length === 0}
        class="editor-host"
        data-placeholder={tr('startWriting')}
      ></div>
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
      <span class="bottom-label">{tr('stats')}</span>
      <span>{tr('words')}: <strong>{stats.word_count}</strong></span>
      <span>{tr('characters')}: <strong>{stats.character_count}</strong></span>
      <span>{tr('blocks')}: <strong>{stats.block_count}</strong></span>
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
