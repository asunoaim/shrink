<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import {
    cancelExport,
    copyToClipboard,
    fileName,
    fileUrl,
    initialFile,
    makeProxy,
    onExportDone,
    onExportEvent,
    onProxyProgress,
    openClip,
    sizeAdvice,
    startExport,
    type ClipOutcome,
    type ClipView,
    type Format,
    type SizeAdvice,
  } from "$lib/api";
  import { bytesToMb, fmtMb, mbToBytes } from "$lib/format";
  import { clampTarget, doneOutputs, mergeRetry } from "$lib/exporting";
  import { longest, markIn, markOut, numbered, removeSection, type Section } from "$lib/sections";
  import Player from "$lib/components/Player.svelte";
  import Timeline from "$lib/components/Timeline.svelte";
  import ExportBar, { type Phase, type Progress } from "$lib/components/ExportBar.svelte";
  import UpdateNotice from "$lib/components/UpdateNotice.svelte";

  const VIDEO_EXT = ["mp4", "mkv", "mov", "m4v", "webm", "avi", "ts", "flv"];

  // clip
  let view = $state<ClipView | null>(null);
  let src = $state("");
  let loading = $state(false);
  let error = $state("");
  let dragOver = $state(false);
  let proxyProgress: number | null = $state(null);

  // playback + sections
  let player: Player | undefined = $state();
  let currentTime = $state(0);
  let paused = $state(true);
  let sections: Section[] = $state([]);
  let selected: number | null = $state(null);
  let pendingIn: number | null = $state(null);

  // export settings (size and mode remembered between sessions)
  let mode: "original" | "shrink" = $state(load("mode", "original"));
  let targetMb = $state(load("targetMb", 25));
  let chosenFormat: Format | null = $state(null);
  let selectedTracks: number[] = $state([]);
  let advice: SizeAdvice | null = $state(null);

  // export run
  let phase: Phase = $state("idle");
  let progress: Progress | null = $state(null);
  let outcomes: ClipOutcome[] = $state([]);
  let copiedToClipboard = $state(false);
  let lastRun: { sections: (Section & { number: number })[]; outDir: string } | null = null;
  let runNumbers: number[] = []; // clip numbers in the current run, in order
  let retrying = false;
  let picking = false; // folder dialog open: a second Export click must not start another run
  let startedAt = 0;

  const info = $derived(view?.info ?? null);
  const source: Format = $derived(info ? { width: info.width, height: info.height, fps: info.fps } : { width: 0, height: 0, fps: 0 });

  function load<T>(key: string, fallback: T): T {
    try {
      const v = localStorage.getItem(`shrink.${key}`);
      return v === null ? fallback : (JSON.parse(v) as T);
    } catch {
      return fallback;
    }
  }
  function save(key: string, v: unknown) {
    try {
      localStorage.setItem(`shrink.${key}`, JSON.stringify(v));
    } catch {
      /* storage unavailable: settings just aren't remembered */
    }
  }

  $effect(() => save("mode", mode));
  $effect(() => save("targetMb", targetMb));

  // size advice follows the longest section, the target and the audio choice
  $effect(() => {
    const len = longest(sections);
    const hasAudio = selectedTracks.length > 0;
    const bytes = mbToBytes(targetMb);
    if (!info || len <= 0) {
      advice = null;
      return;
    }
    sizeAdvice(len, bytes, hasAudio)
      .then((a) => (advice = a))
      .catch(() => (advice = null));
  });

  // a remembered or earlier target can be bigger than these sections can use
  $effect(() => {
    if (!advice) return;
    const c = clampTarget(targetMb, bytesToMb(advice.sliderMinBytes), bytesToMb(advice.sliderMaxBytes));
    if (c !== targetMb) targetMb = c;
  });

  async function openPath(path: string) {
    if (phase === "exporting") return;
    loading = true;
    error = "";
    try {
      const v = await openClip(path);
      view = v;
      src = fileUrl(v.info.path);
      sections = [];
      selected = null;
      pendingIn = null;
      chosenFormat = null;
      currentTime = 0;
      phase = "idle";
      selectedTracks = v.info.audioTracks.length > 0 ? [0] : [];
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function pickFile() {
    const f = await open({ multiple: false, directory: false, filters: [{ name: "Videos", extensions: VIDEO_EXT }] });
    if (typeof f === "string") openPath(f);
  }

  // the built-in player can't decode this file: make a playable copy
  async function onPlayerError() {
    if (!view || proxyProgress !== null) return;
    proxyProgress = 0;
    try {
      src = fileUrl(await makeProxy());
    } catch (e) {
      error = `Can't preview this clip: ${e}`;
    } finally {
      proxyProgress = null;
    }
  }

  async function runExport(ordered: (Section & { number: number })[], outDir?: string) {
    if (!info || ordered.length === 0 || picking || phase === "exporting") return;
    let dir = outDir;
    if (!dir) {
      picking = true;
      try {
        const picked = await open({ directory: true, title: "Save clips to…" });
        if (typeof picked !== "string") return;
        dir = picked;
      } finally {
        picking = false;
      }
    }
    retrying = outDir !== undefined;
    if (!retrying) {
      lastRun = { sections: ordered, outDir: dir };
      outcomes = [];
    }
    runNumbers = ordered.map((s) => s.number);
    progress = { clip: 1, count: ordered.length, fraction: 0, eta: null };
    startedAt = performance.now();
    phase = "exporting";
    try {
      await startExport({
        sections: ordered.map((s) => ({ start: s.start, end: s.end, number: s.number })),
        mode: mode === "original" ? { kind: "original" } : { kind: "shrink", targetBytes: mbToBytes(targetMb), format: chosenFormat },
        audioTracks: selectedTracks,
        outDir: dir,
      });
    } catch (e) {
      phase = "done";
      const err: ClipOutcome = { kind: "failed", error: String(e) };
      outcomes = retrying ? mergeRetry(outcomes, runNumbers, runNumbers.map(() => err)) : ordered.map(() => err);
      copiedToClipboard = false;
    }
  }

  // "Try again": the failed clips keep their numbers and folder
  function retryFailed() {
    if (!lastRun) return;
    const failed = lastRun.sections.filter((s) => outcomes[s.number - 1]?.kind !== "done");
    runExport(failed, lastRun.outDir);
  }

  function reveal() {
    const first = outcomes.find((o) => o.kind === "done");
    if (first && first.kind === "done") revealItemInDir(first.output);
  }

  function step(seconds: number) {
    player?.seek(currentTime + seconds);
  }

  function onKey(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    if (!info || target.tagName === "INPUT" || phase === "exporting") return;
    const frame = 1 / (info.fps || 60);
    switch (e.key) {
      case " ":
        e.preventDefault();
        player?.toggle();
        break;
      case "ArrowLeft":
        e.preventDefault();
        step(e.shiftKey ? -1 : -frame);
        break;
      case "ArrowRight":
        e.preventDefault();
        step(e.shiftKey ? 1 : frame);
        break;
      case "i":
      case "I": {
        const st = markIn({ sections, pendingIn, selected }, currentTime, info.duration);
        ({ sections, pendingIn, selected } = st);
        break;
      }
      case "o":
      case "O": {
        const st = markOut({ sections, pendingIn, selected }, currentTime, info.duration);
        ({ sections, pendingIn, selected } = st);
        break;
      }
      case "Delete":
      case "Backspace":
        if (selected !== null) {
          sections = removeSection(sections, selected);
          selected = null;
        }
        break;
      case "Escape":
        selected = null;
        pendingIn = null;
        break;
    }
  }

  onMount(() => {
    const unlisten: Promise<() => void>[] = [
      onExportEvent((e) => {
        if (!progress) return;
        if (e.kind === "clipStarted") {
          progress = { ...progress, clip: runNumbers.indexOf(e.clipNumber) + 1, count: e.clipCount, fraction: 0 };
        }
        if (e.kind === "progress") {
          const overall = (progress.clip - 1 + e.fraction) / progress.count;
          const elapsed = (performance.now() - startedAt) / 1000;
          const eta = overall > 0.03 && elapsed > 1 ? (elapsed * (1 - overall)) / overall : null;
          progress = { ...progress, fraction: e.fraction, eta };
        }
      }),
      onExportDone(async (d) => {
        if (retrying) {
          outcomes = mergeRetry(outcomes, runNumbers, d.outcomes);
          const all = doneOutputs(outcomes);
          copiedToClipboard = all.length > 0 && (await copyToClipboard(all).then(() => true, () => false));
        } else {
          outcomes = d.outcomes;
          copiedToClipboard = d.copiedToClipboard;
        }
        phase = outcomes.every((o) => o.kind === "cancelled") ? "idle" : "done";
      }),
      onProxyProgress((f) => (proxyProgress = f)),
      getCurrentWebview().onDragDropEvent((e) => {
        const p = e.payload;
        if (p.type === "enter" || p.type === "over") dragOver = true;
        else if (p.type === "leave") dragOver = false;
        else if (p.type === "drop") {
          dragOver = false;
          if (p.paths.length > 0) openPath(p.paths[0]);
        }
      }),
    ];
    initialFile().then((f) => {
      if (f) openPath(f);
    });
    return () => unlisten.forEach((u) => u.then((f) => f()));
  });
</script>

<svelte:window onkeydown={onKey} />

<div class="app">
  <header>
    <div class="logo"><span class="mark"></span>shrink</div>
    <div class="file">
      {#if info}<b>{fileName(info.path)}</b>{info.height}p · {Math.round(info.fps)} fps · {fmtMb(info.sizeBytes)}{/if}
    </div>
    <UpdateNotice busy={phase === "exporting"} />
    {#if info}
      <button class="btn ghost small" onclick={pickFile} disabled={phase === "exporting"}>Open…</button>
    {/if}
  </header>

  {#if view && info}
    <main>
      <Player bind:this={player} {src} duration={info.duration} bind:currentTime bind:paused onerror={onPlayerError} />
      <Timeline
        duration={info.duration}
        {currentTime}
        onseek={(t) => player?.seek(t)}
        thumbs={view.thumbs ? fileUrl(view.thumbs) : null}
        keyframes={info.keyframes}
        showActualStart={mode === "original"}
        {pendingIn}
        bind:sections
        bind:selected
      />
      <div class="hint muted">
        Drag on the timeline to mark a highlight, or press <kbd>I</kbd> and <kbd>O</kbd>. <kbd>Space</kbd> plays, <kbd>←</kbd><kbd>→</kbd> step a frame, <kbd>Shift</kbd> for a second, <kbd>Del</kbd> removes.
      </div>
    </main>
    <ExportBar
      bind:mode
      bind:targetMb
      bind:chosenFormat
      bind:selectedTracks
      {advice}
      {source}
      tracks={info.audioTracks}
      count={sections.length}
      encoder={view.encoder}
      {phase}
      {progress}
      {outcomes}
      {copiedToClipboard}
      onexport={() => runExport(numbered(sections))}
      oncancel={() => cancelExport()}
      onreveal={reveal}
      ondone={() => (phase = "idle")}
      onretry={retryFailed}
    />
  {:else}
    <div class="drop" class:over={dragOver}>
      {#if loading}
        <div class="big">Opening…</div>
      {:else}
        <div class="big">Drop a clip here</div>
        <div class="muted">or right-click any video → Open in shrink</div>
        <button class="btn ghost" onclick={pickFile}>Open file…</button>
      {/if}
      {#if error}
        <div class="error">{error}</div>
      {/if}
    </div>
  {/if}

  {#if view && (dragOver || loading)}
    <div class="overlay">{loading ? "Opening…" : "Drop to open"}</div>
  {/if}
  {#if proxyProgress !== null}
    <div class="overlay">Preparing preview… {Math.round(proxyProgress * 100)}%</div>
  {/if}
  {#if view && error}
    <div class="toast" role="alert">{error} <button onclick={() => (error = "")}>✕</button></div>
  {/if}
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
    position: relative;
  }
  header {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 16px 10px;
  }
  .logo {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-logo);
    font-weight: 700;
    font-size: 20px;
    letter-spacing: -0.01em;
  }
  .mark {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 14px var(--accent-glow);
  }
  .file {
    flex: 1;
    min-width: 0;
    font-size: 12.5px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .file b {
    color: var(--text);
    font-weight: 500;
    margin-right: 10px;
  }
  .small {
    padding: 6px 14px;
    font-size: 12px;
  }
  main {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 0 16px;
  }
  .hint {
    font-size: 11px;
    text-align: center;
    margin-top: 12px;
  }
  kbd {
    font-family: var(--font);
    font-size: 10px;
    padding: 1px 5px;
    border-radius: 4px;
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--text);
    margin: 0 1px;
  }
  .drop {
    flex: 1;
    margin: 4px 16px 16px;
    border: 2px dashed var(--line);
    border-radius: 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    text-align: center;
    transition: border-color 0.15s, background 0.15s;
  }
  .drop.over {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .big {
    font-family: var(--font-logo);
    font-size: 30px;
    font-weight: 700;
  }
  .error {
    color: var(--warn);
    max-width: 520px;
    font-size: 12px;
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(20, 16, 13, 0.7);
    backdrop-filter: blur(3px);
    font-family: var(--font-logo);
    font-size: 26px;
    font-weight: 700;
    z-index: 10;
  }
  .toast {
    position: absolute;
    top: 56px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--panel);
    border: 1px solid var(--warn);
    color: var(--text);
    padding: 8px 12px;
    border-radius: 10px;
    font-size: 12px;
    z-index: 11;
    max-width: 80%;
  }
  .toast button {
    border: none;
    background: transparent;
    color: var(--muted);
    margin-left: 8px;
  }
</style>
