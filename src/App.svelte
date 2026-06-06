<script>
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { LogicalSize } from "@tauri-apps/api/dpi";
  import { onMount } from "svelte";

  let text = $state("");
  let status = $state("");
  let notes = $state([]);
  let selectedFile = $state("inbox.md");
  let expanded = $state(false);
  let expandedContent = $state("");
  let expandedName = $state("");

  const COMPACT_H = 248;
  const EXPANDED_H = 420;
  const WIDTH = 428;

  onMount(() => {
    loadNotes();
    const unlisten = getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      if (focused) loadNotes();
    });
    return () => { unlisten.then(fn => fn()); };
  });

  async function loadNotes() {
    try {
      notes = await invoke("get_recent_notes");
    } catch (e) {
      console.error("loadNotes error:", e);
      notes = [];
    }
  }

  async function submit() {
    const trimmed = text.trim();
    if (!trimmed) return;
    try {
      const result = await invoke("append_to_note", { filename: selectedFile, content: trimmed });
      console.log("append result:", result);
      status = "✓";
      text = "";
      setTimeout(() => (status = ""), 1500);
      loadNotes();
    } catch (e) {
      console.error("append error:", e);
      status = "✗ " + e;
      setTimeout(() => (status = ""), 3000);
    }
  }

  function selectNote(note) {
    selectedFile = note.name + ".md";
    openExpanded(note);
  }

  async function openExpanded(note) {
    try {
      expandedContent = await invoke("read_note", { filename: note.name + ".md" });
      expandedName = note.name;
      expanded = true;
      await getCurrentWindow().setSize(new LogicalSize(WIDTH, EXPANDED_H));
    } catch (e) {
      expandedContent = "Error: " + e;
    }
  }

  async function closeExpanded() {
    expanded = false;
    expandedContent = "";
    expandedName = "";
    await getCurrentWindow().setSize(new LogicalSize(WIDTH, COMPACT_H));
  }

  async function launchTui() {
    try { await invoke("open_tui"); } catch (e) { console.error("open_tui:", e); }
  }

  function relativeTime(ts) {
    const now = Math.floor(Date.now() / 1000);
    const diff = now - ts;
    if (diff < 60) return "ahora";
    if (diff < 3600) return `hace ${Math.floor(diff / 60)}m`;
    if (diff < 86400) return `hace ${Math.floor(diff / 3600)}h`;
    if (diff < 172800) return "ayer";
    return `hace ${Math.floor(diff / 86400)}d`;
  }

  function handleInputKey(e) {
    if (e.type === "keyup" && e.key === "Enter") {
      e.preventDefault();
      submit();
    } else if (e.type === "keydown" && e.key === "Escape") {
      text = "";
      status = "";
      selectedFile = "inbox.md";
    }
  }

  function handleGlobalKey(e) {
    if (expanded && e.key === "Escape") {
      e.preventDefault();
      closeExpanded();
    }
  }

  let fileLabel = $derived(
    selectedFile === "inbox.md" ? "" : " → " + selectedFile.replace(".md", "")
  );
</script>

<svelte:window onkeydown={handleGlobalKey} />

{#if expanded}
  <div class="expanded-view">
    <div class="expanded-header">
      <span class="expanded-title">{expandedName}</span>
      <button class="close-btn" onclick={closeExpanded}>✕</button>
    </div>
    <pre class="expanded-content">{expandedContent}</pre>
    <div class="hint-bar">
      <span class="hint"><kbd>Esc</kbd> cerrar</span>
    </div>
  </div>
{:else}
  <div class="container">
    <div class="input-row">
      <span class="prompt">❯</span>
      <input
        type="text"
        bind:value={text}
        onkeydown={handleInputKey}
        onkeyup={handleInputKey}
        placeholder="Escribe y presiona Enter..."
      />
      <button class="send-btn" onclick={submit}>↵</button>
      {#if status}
        <span class="status">{status}</span>
      {/if}
      {#if fileLabel}
        <span class="file-label">{fileLabel}</span>
      {/if}
    </div>

    <div class="grid">
      {#each notes as note}
        <button class="cell" onclick={() => selectNote(note)}>
          <span class="cell-name">{note.name}</span>
          {#if note.task_count > 0}
            <span class="cell-tasks">✓ {note.tasks_done}/{note.task_count}</span>
          {:else}
            <span class="cell-time">{relativeTime(note.modified)}</span>
          {/if}
        </button>
      {/each}
      <button class="cell cell-tui" onclick={launchTui}>
        <span class="cell-name">▶ Abrir TUI</span>
        <span class="cell-time">terminal</span>
      </button>
    </div>

    <div class="hint-bar">
      <span class="hint"><kbd>Enter</kbd> enviar</span>
      <span class="hint"><kbd>Esc</kbd> limpiar</span>
    </div>
  </div>
{/if}

<style>
  :global(html) {
    width: 100%;
    height: 100%;
    overflow: hidden;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    width: 100%;
    height: 100%;
    background: transparent;
    overflow: hidden;
    font-family: "JetBrains Mono", "Fira Code", monospace;
    color: #cdd6f4;
  }

  .container {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 8px;
    box-sizing: border-box;
    gap: 6px;
  }

  .input-row {
    display: flex;
    align-items: center;
    min-height: 36px;
    background: rgba(30, 30, 46, 0.92);
    border: 1px solid rgba(137, 180, 250, 0.3);
    border-radius: 8px;
    padding: 6px 12px;
    gap: 8px;
    backdrop-filter: blur(12px);
    flex-shrink: 0;
  }

  .prompt {
    color: #a6e3a1;
    font-size: 14px;
    flex-shrink: 0;
  }

  input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: #cdd6f4;
    font-family: inherit;
    font-size: 13px;
    min-width: 0;
  }

  input::placeholder {
    color: rgba(205, 214, 244, 0.4);
  }

  .send-btn {
    background: rgba(166, 227, 161, 0.15);
    border: 1px solid rgba(166, 227, 161, 0.3);
    color: #a6e3a1;
    border-radius: 4px;
    padding: 2px 8px;
    cursor: pointer;
    font-size: 13px;
    flex-shrink: 0;
  }

  .send-btn:hover {
    background: rgba(166, 227, 161, 0.25);
  }

  .status {
    color: #a6e3a1;
    font-size: 12px;
    flex-shrink: 0;
  }

  .file-label {
    color: #89b4fa;
    font-size: 11px;
    flex-shrink: 0;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    grid-template-rows: repeat(2, 1fr);
    gap: 6px;
    flex: 1;
    min-height: 0;
  }

  .cell {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    background: rgba(30, 30, 46, 0.85);
    border: 1px solid rgba(137, 180, 250, 0.15);
    border-radius: 8px;
    padding: 6px;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    backdrop-filter: blur(8px);
    color: #cdd6f4;
    font-family: inherit;
    font-size: 12px;
    text-align: center;
    word-break: break-all;
  }

  .cell:hover {
    background: rgba(49, 50, 68, 0.95);
    border-color: rgba(137, 180, 250, 0.4);
  }

  .cell-name {
    font-size: 12px;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: #cdd6f4;
  }

  .cell-tasks {
    font-size: 10px;
    color: #a6e3a1;
  }

  .cell-time {
    font-size: 10px;
    color: rgba(205, 214, 244, 0.5);
  }

  .cell-tui {
    border-color: rgba(166, 227, 161, 0.25);
  }

  .cell-tui .cell-name {
    color: #a6e3a1;
  }

  .hint-bar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 16px;
    min-height: 28px;
    flex-shrink: 0;
    font-size: 11px;
  }

  .hint {
    color: rgba(205, 214, 244, 0.45);
  }

  kbd {
    display: inline-block;
    background: rgba(49, 50, 68, 0.8);
    border: 1px solid rgba(137, 180, 250, 0.2);
    border-radius: 3px;
    padding: 1px 5px;
    font-family: inherit;
    font-size: 10px;
    color: rgba(205, 214, 244, 0.6);
  }

  .expanded-view {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 8px;
    box-sizing: border-box;
    gap: 6px;
  }

  .expanded-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 32px;
    padding: 0 8px;
    flex-shrink: 0;
  }

  .expanded-title {
    font-size: 14px;
    font-weight: bold;
    color: #89b4fa;
  }

  .close-btn {
    background: none;
    border: none;
    color: rgba(205, 214, 244, 0.5);
    font-size: 16px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 4px;
  }

  .close-btn:hover {
    background: rgba(49, 50, 68, 0.8);
    color: #f38ba8;
  }

  .expanded-content {
    flex: 1;
    margin: 0;
    padding: 12px;
    background: rgba(30, 30, 46, 0.85);
    border: 1px solid rgba(137, 180, 250, 0.15);
    border-radius: 8px;
    overflow-y: auto;
    font-family: inherit;
    font-size: 12px;
    line-height: 1.5;
    color: #cdd6f4;
    white-space: pre-wrap;
    word-wrap: break-word;
  }
</style>
