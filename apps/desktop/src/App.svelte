<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import { createEditor } from './lib/editor';
  import { documentToText, type DocumentState } from './lib/documentProjection';

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

  let title = $state('900Word');
  let status = $state('Starting...');
  let plainText = $state('');
  let stats = $state<DocumentStats>({ word_count: 0, character_count: 0, block_count: 0 });
  let spellIssues = $state<SpellIssue[]>([]);
  let editorHost: HTMLDivElement;
  let view: ReturnType<typeof createEditor> | undefined;

  async function loadDocument() {
    const document = await invoke<DocumentState>('new_document');
    title = document.meta.title;
    plainText = documentToText(document);
    stats = await invoke<DocumentStats>('get_document_stats');
    status = 'Ready';
    view?.destroy();
    view = createEditor(editorHost, plainText, (text) => {
      plainText = text;
    });
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
    loadDocument().catch((error: unknown) => {
      status = error instanceof Error ? error.message : String(error);
    });

    return () => {
      view?.destroy();
    };
  });
</script>

<main class="app-shell">
  <header class="topbar">
    <div>
      <h1>{title}</h1>
      <p>{status}</p>
    </div>
    <nav aria-label="Document actions">
      <button type="button" onclick={exportText}>TXT</button>
      <button type="button" onclick={exportHtml}>HTML</button>
      <button type="button" onclick={exportPdf}>PDF</button>
      <button type="button" onclick={checkSpelling}>Spell</button>
    </nav>
  </header>

  <section class="workspace" aria-label="Document workspace">
    <aside class="sidebar" aria-label="Document statistics">
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
    </aside>

    <section class="editor-panel" aria-label="Editor">
      <div bind:this={editorHost} class="editor-host"></div>
    </section>
  </section>
</main>
