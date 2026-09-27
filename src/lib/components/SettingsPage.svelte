<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { getVersion } from "@tauri-apps/api/app";
  import type { AudioDefault, SaveTo, Settings } from "$lib/api";

  let { settings, onchange }: { settings: Settings; onchange: (s: Settings) => void } = $props();

  let version = $state("");
  onMount(() => {
    getVersion().then((v) => (version = v));
  });

  // radios are bound to this, not directly to settings.saveTo: a cancelled
  // folder pick needs to snap the checked radio back even when the saveTo
  // value itself never changed (Svelte skips a DOM update when the bound
  // value is unchanged, so the click would otherwise stick to "folder").
  let choice = $state(untrack(() => settings.saveTo));
  $effect(() => {
    choice = settings.saveTo;
  });

  const set = (patch: Partial<Settings>) => onchange({ ...settings, ...patch });

  async function chooseFolder(): Promise<string | null> {
    const picked = await open({ directory: true, title: "Always save clips to…" });
    return typeof picked === "string" ? picked : null;
  }

  async function pickSaveTo(v: SaveTo) {
    if (v === "folder" && !settings.folder) {
      const f = await chooseFolder();
      if (!f) {
        choice = settings.saveTo; // cancelled: snap the radio back to the real setting
        return;
      }
      set({ saveTo: v, folder: f });
    } else {
      set({ saveTo: v });
    }
  }

  async function changeFolder() {
    const f = await chooseFolder();
    if (f) set({ saveTo: "folder", folder: f });
  }

  function onSize(e: Event) {
    const v = Number((e.currentTarget as HTMLInputElement).value.replace(",", "."));
    if (Number.isFinite(v) && v > 0) set({ targetMb: Math.min(1000, Math.max(1, v)) });
  }

  const saveOptions: [SaveTo, string][] = [
    ["ask", "Ask every time"],
    ["nextToOriginal", "Next to the original recording"],
    ["folder", "Always this folder"],
  ];
  const audioOptions: [AudioDefault, string][] = [
    ["first", "First track"],
    ["all", "All tracks"],
    ["last", "Same as last time"],
  ];
</script>

<div class="page">
  <section class="card" aria-labelledby="save-h">
    <h2 id="save-h">Save clips to</h2>
    <p class="help">Where exported clips go.</p>
    <div role="radiogroup" aria-labelledby="save-h">
      {#each saveOptions as [value, label]}
        <label class="radio">
          <input type="radio" name="saveTo" bind:group={choice} {value} onchange={() => pickSaveTo(value)} />
          {label}
        </label>
      {/each}
    </div>
    {#if settings.folder}
      <div class="path"><code>{settings.folder}</code><button class="btn ghost small" onclick={changeFolder}>Change…</button></div>
    {/if}
  </section>

  <section class="card row">
    <div>
      <h2 id="clip-h">Copy clips to clipboard</h2>
      <p class="help flush">After export, so you can paste them straight into a chat.</p>
    </div>
    <button class="toggle" role="switch" aria-checked={settings.copyToClipboard} aria-labelledby="clip-h"
      onclick={() => set({ copyToClipboard: !settings.copyToClipboard })}><i></i></button>
  </section>

  <section class="card" aria-labelledby="open-h">
    <h2 id="open-h">When a clip opens</h2>
    <p class="help">Starting point for every clip. You can still change it in the export bar.</p>
    <div class="field">
      <span id="audio-l">Audio</span>
      <div class="seg" role="radiogroup" aria-labelledby="audio-l">
        {#each audioOptions as [value, label]}
          <button role="radio" aria-checked={settings.audio === value} class:on={settings.audio === value} onclick={() => set({ audio: value })}>{label}</button>
        {/each}
      </div>
    </div>
    <div class="field">
      <span id="mode-l">Export as</span>
      <div class="seg" role="radiogroup" aria-labelledby="mode-l">
        <button role="radio" aria-checked={settings.startMode === "original"} class:on={settings.startMode === "original"} onclick={() => set({ startMode: "original" })}>Original quality</button>
        <button role="radio" aria-checked={settings.startMode === "shrink"} class:on={settings.startMode === "shrink"} onclick={() => set({ startMode: "shrink" })}>Shrink to size</button>
      </div>
    </div>
    <div class="field">
      <label for="size">Target size</label>
      <span class="num"><input id="size" type="number" min="1" max="1000" step="0.5" value={settings.targetMb} onchange={onSize} /> MB</span>
    </div>
  </section>

  <footer class="foot">
    shrink {version} ·
    <button class="link" onclick={() => openUrl(`https://github.com/asunoaim/shrink/releases/tag/v${version}`)}>What's new</button> ·
    <button class="link" onclick={() => openUrl("https://github.com/asunoaim/shrink")}>GitHub</button>
  </footer>
</div>

<style>
  .page {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    width: 100%;
    max-width: 560px; /* geometry */
    margin: 0 auto;
    padding: var(--s5) var(--s4);
  }
  .card {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--r-md);
    padding: var(--s3) var(--s4);
  }
  .card.row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
  }
  h2 {
    margin: 0 0 2px;
    font-size: var(--t-lg);
    font-weight: var(--w-bold);
  }
  .help {
    margin: 0 0 var(--s3);
    color: var(--muted);
    font-size: var(--t-sm);
  }
  .help.flush {
    margin: 0;
  }
  .radio {
    display: flex;
    align-items: center;
    gap: var(--s2);
    padding: var(--s1) 0;
  }
  .radio input {
    accent-color: var(--accent);
    margin: 0;
  }
  .path {
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin: var(--s1) 0 0 var(--s5);
    font-size: var(--t-sm);
  }
  .path code {
    background: var(--bg);
    padding: var(--s1) var(--s2);
    border-radius: var(--r-sm);
    user-select: text;
  }
  .toggle {
    flex: none;
    width: 36px; /* geometry */
    height: 20px; /* geometry */
    border: none;
    border-radius: var(--r-pill);
    background: var(--panel-2);
    position: relative;
    padding: 0;
  }
  .toggle[aria-checked="true"] {
    background: var(--accent);
  }
  .toggle i {
    position: absolute;
    top: 3px; /* geometry */
    left: 3px; /* geometry */
    width: 14px; /* geometry */
    height: 14px; /* geometry */
    border-radius: 50%;
    background: var(--text);
    transition: transform 0.15s;
  }
  .toggle[aria-checked="true"] i {
    transform: translateX(16px); /* geometry */
    background: var(--on-accent);
  }
  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--s4);
    margin-top: var(--s3);
  }
  .seg {
    display: flex;
    background: var(--panel-2);
    border-radius: var(--r-pill);
    padding: var(--s1);
  }
  .seg button {
    border: none;
    background: transparent;
    padding: var(--s1) var(--s3);
    border-radius: var(--r-pill);
    color: var(--muted-hi);
  }
  .seg button.on {
    background: var(--bg);
    color: var(--text);
    font-weight: var(--w-medium);
  }
  .num input {
    width: 72px; /* geometry */
    padding: var(--s1) var(--s2);
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    text-align: right;
  }
  .foot {
    text-align: center;
    color: var(--muted);
    font-size: var(--t-sm);
    padding: var(--s2) 0 var(--s4);
  }
  .link {
    border: none;
    background: none;
    padding: 0;
    color: var(--muted);
    text-decoration: underline;
  }
</style>
