<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { createEditor } from './lib/editor';
  import {
    buildEditorSyncCommands,
    canEditProjectedDocument,
    documentProjectionWarnings,
    documentToText,
    type DocumentState,
    type EditorProjectedChange
  } from './lib/documentProjection';

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

  interface DictionaryInfo {
    language_tag: string;
    display_name: string;
    license: string;
  }

  interface Settings {
    telemetry_enabled: boolean;
    language_tag: string;
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

  type ViewId = 'editor' | 'settings' | 'about';

  let title = $state('900Word');
  let status = $state('Starting...');
  let activeView = $state<ViewId>('editor');
  let plainText = $state('');
  let stats = $state<DocumentStats>({ word_count: 0, character_count: 0, block_count: 0 });
  let spellIssues = $state<SpellIssue[]>([]);
  let projectionWarnings = $state<string[]>([]);
  let filePathInput = $state('');
  let fileState = $state<DocumentFileState>({
    has_current_path: false,
    dirty: false,
    recent_documents: [],
    recovery_documents: []
  });
  let dictionaries = $state<DictionaryInfo[]>([]);
  let settings = $state<Settings>({
    telemetry_enabled: false,
    language_tag: 'en',
    high_contrast: false
  });
  let documentState: DocumentState | undefined;
  let editorSyncQueue = Promise.resolve();
  let editorSyncError: string | null = null;
  let editorHost: HTMLDivElement;
  let view: ReturnType<typeof createEditor> | undefined;

  async function newDocument() {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('new_document');
    await loadDocumentIntoEditor(document, 'Ready');
    filePathInput = '';
    await refreshFileState();
  }

  async function loadDocumentIntoEditor(document: DocumentState, nextStatus: string) {
    editorSyncError = null;
    documentState = document;
    title = document.meta.title;
    plainText = documentToText(document);
    stats = await invoke<DocumentStats>('get_document_stats');
    projectionWarnings = collectDocumentWarnings(document);
    const editable = canEditProjectedDocument(document);
    status = editable ? nextStatus : 'Read-only projection warning';
    view?.destroy();
    view = createEditor(editorHost, document, handleEditorChange, { editable });
  }

  function handleEditorChange(change: EditorProjectedChange) {
    plainText = change.text;
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
    await loadDocumentIntoEditor(document, 'Document opened');
    await refreshFileState();
  }

  async function openRecentDocument(token: string) {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('open_recent_document', {
      token
    });
    await loadDocumentIntoEditor(document, 'Recent document opened');
    await refreshFileState();
  }

  async function saveCurrentDocument() {
    await waitForEditorSync();
    fileState = await invoke<DocumentFileState>('save_document');
    status = 'Document saved';
  }

  async function saveDocumentAsPath() {
    await waitForEditorSync();
    fileState = await invoke<DocumentFileState>('save_document_as', {
      path: filePathInput
    });
    status = 'Document saved';
  }

  async function autosaveDocument() {
    await waitForEditorSync();
    await invoke<RecoveryDocumentSummary>('autosave_document');
    await refreshFileState();
    status = 'Recovery draft updated';
  }

  async function recoverDocument(token: string) {
    await waitForEditorSync();
    const document = await invoke<DocumentState>('recover_document', {
      token
    });
    await loadDocumentIntoEditor(document, 'Recovery draft opened');
    await refreshFileState();
  }

  async function discardRecovery(token: string) {
    await invoke('discard_recovery', {
      token
    });
    await refreshFileState();
    status = 'Recovery draft discarded';
  }

  async function loadShellState() {
    settings = await invoke<Settings>('get_settings');
    dictionaries = await invoke<DictionaryInfo[]>('list_dictionaries');
  }

  async function saveSettings() {
    settings = await invoke<Settings>('update_settings', {
      settings
    });
    status = 'Settings updated';
  }

  async function exportText() {
    const text = await invoke<string>('export_txt');
    status = `TXT export prepared (${text.length} characters)`;
  }

  async function exportHtml() {
    const html = await invoke<string>('export_html');
    status = `HTML export prepared (${html.length} characters)`;
  }

  async function exportPdf() {
    const pdf = await invoke<number[]>('export_pdf');
    status = `PDF export prepared (${pdf.length} bytes)`;
  }

  async function checkSpelling() {
    spellIssues = await invoke<SpellIssue[]>('check_spelling', {
      text: plainText,
      languageTag: 'en'
    });
    status = spellIssues.length === 0 ? 'No spelling issues found' : `${spellIssues.length} spelling issue(s)`;
  }

  onMount(() => {
    Promise.all([newDocument(), loadShellState()]).catch((error: unknown) => {
      status = error instanceof Error ? error.message : String(error);
    });

    return () => {
      view?.destroy();
    };
  });

  function collectDocumentWarnings(document: DocumentState): string[] {
    return [...(document.warnings ?? []).map((warning) => warning.message), ...documentProjectionWarnings(document)];
  }
</script>

<main class:high-contrast={settings.high_contrast} class="app-shell">
  <header class="topbar">
    <div>
      <h1>{title}</h1>
      <p>{status}</p>
    </div>
    <nav aria-label="Document actions">
      <button type="button" onclick={newDocument}>New</button>
      <input
        aria-label="ODT path"
        bind:value={filePathInput}
        class="path-input"
        placeholder="Document .odt path"
        type="text"
      />
      <button type="button" onclick={openDocumentFromPath}>Open</button>
      <button disabled={!fileState.has_current_path} type="button" onclick={saveCurrentDocument}>Save</button>
      <button type="button" onclick={saveDocumentAsPath}>Save As</button>
      <button type="button" onclick={autosaveDocument}>Autosave</button>
      <button type="button" onclick={exportText}>TXT</button>
      <button type="button" onclick={exportHtml}>HTML</button>
      <button type="button" onclick={exportPdf}>PDF</button>
      <button type="button" onclick={checkSpelling}>Spell</button>
    </nav>
  </header>

  <div class="view-tabs" role="tablist" aria-label="Workspace views">
    <button
      aria-controls="editor-view"
      aria-selected={activeView === 'editor'}
      role="tab"
      type="button"
      onclick={() => (activeView = 'editor')}
    >
      Editor
    </button>
    <button
      aria-controls="settings-view"
      aria-selected={activeView === 'settings'}
      role="tab"
      type="button"
      onclick={() => (activeView = 'settings')}
    >
      Settings
    </button>
    <button
      aria-controls="about-view"
      aria-selected={activeView === 'about'}
      role="tab"
      type="button"
      onclick={() => (activeView = 'about')}
    >
      About
    </button>
  </div>

  <section class="workspace" aria-label="Document workspace">
    <aside class="sidebar" aria-label="Document statistics">
      <h2>File</h2>
      <dl>
        <div><dt>Saved</dt><dd>{fileState.has_current_path ? 'Yes' : 'No'}</dd></div>
        <div><dt>Dirty</dt><dd>{fileState.dirty ? 'Yes' : 'No'}</dd></div>
      </dl>

      {#if fileState.recent_documents.length > 0}
        <h2>Recent</h2>
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
        <h2>Recovery</h2>
        <ul class="action-list">
          {#each fileState.recovery_documents as recovery}
            <li>
              <span>{recovery.label}</span>
              <button type="button" onclick={() => recoverDocument(recovery.token)}>Open</button>
              <button type="button" onclick={() => discardRecovery(recovery.token)}>Discard</button>
            </li>
          {/each}
        </ul>
      {/if}

      <h2>Stats</h2>
      <dl>
        <div><dt>Words</dt><dd>{stats.word_count}</dd></div>
        <div><dt>Characters</dt><dd>{stats.character_count}</dd></div>
        <div><dt>Blocks</dt><dd>{stats.block_count}</dd></div>
      </dl>

      <h2>Spelling</h2>
      {#if spellIssues.length === 0}
        <p>No issues.</p>
      {:else}
        <ul>
          {#each spellIssues as issue}
            <li>{issue.word}</li>
          {/each}
        </ul>
      {/if}

      {#if projectionWarnings.length > 0}
        <h2>Projection</h2>
        <ul>
          {#each projectionWarnings as warning}
            <li>{warning}</li>
          {/each}
        </ul>
      {/if}
    </aside>

    <div
      aria-label="Editor"
      class:hidden-view={activeView !== 'editor'}
      class="editor-panel"
      id="editor-view"
      role="tabpanel"
    >
      <div bind:this={editorHost} class="editor-host"></div>
    </div>

    <div
      aria-label="Settings"
      class:hidden-view={activeView !== 'settings'}
      class="panel-view"
      id="settings-view"
      role="tabpanel"
    >
      <div class="form-surface">
        <h2>Settings</h2>

        <label>
          Language
          <select bind:value={settings.language_tag}>
            {#each dictionaries as dictionary}
              <option value={dictionary.language_tag}>
                {dictionary.display_name}
              </option>
            {/each}
          </select>
        </label>

        <label class="check-row">
          <input bind:checked={settings.high_contrast} type="checkbox" />
          High contrast
        </label>

        <label class="check-row muted">
          <input checked={settings.telemetry_enabled} disabled type="checkbox" />
          Telemetry
        </label>

        <button type="button" onclick={saveSettings}>Save Settings</button>
      </div>
    </div>

    <div
      aria-label="About 900Word"
      class:hidden-view={activeView !== 'about'}
      class="panel-view"
      id="about-view"
      role="tabpanel"
    >
      <div class="form-surface">
        <h2>900Word</h2>
        <dl>
          <div><dt>Version</dt><dd>0.1.0</dd></div>
          <div><dt>License</dt><dd>GPL-3.0-or-later</dd></div>
          <div><dt>Document format</dt><dd>OpenDocument Text</dd></div>
          <div><dt>Telemetry</dt><dd>Off</dd></div>
        </dl>
      </div>
    </div>
  </section>
</main>
