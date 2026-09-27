<script lang="ts">
  import { onMount, tick } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import {
    cancelExport,
    clipKeyframes,
    clipThumbs,
    copyToClipboard,
    fileName,
    fileUrl,
    folderExists,
    getSettings,
    initialFile,
    makeProxy,
    onExportDone,
    onExportEvent,
    onProxyProgress,
    openClip,
    setSettings,
    sizeAdvice,
    startExport,
    type ClipOutcome,
    type ClipView,
    type Format,
    type Settings,
    type SizeAdvice,
  } from "$lib/api";
  import { bytesToMb, fmtMb, mbToBytes } from "$lib/format";
  import { clampTarget, doneOutputs, mergeRetry } from "$lib/exporting";
  import { keyAction } from "$lib/keys";
  import { DEFAULT_SETTINGS, initialTracks, migrateTargetMb, resolveOutDir } from "$lib/settings";
  import { longest, markIn, markOut, neighbour, numbered, removeSection, type Section } from "$lib/sections";
  import Player from "$lib/components/Player.svelte";
  import Timeline from "$lib/components/Timeline.svelte";
  import ExportBar, { type Phase, type Progress } from "$lib/components/ExportBar.svelte";
  import SettingsPage from "$lib/components/SettingsPage.svelte";
  import UpdateNotice from "$lib/components/UpdateNotice.svelte";

  const VIDEO_EXT = ["mp4", "mkv", "mov", "m4v", "webm", "avi", "ts", "flv"];

  let settings: Settings = $state({ ...DEFAULT_SETTINGS });
  let screen: "editor" | "settings" = $state("editor");

  function updateSettings(s: Settings) {
    settings = s;
    setSettings(s).catch((e) => (error = `Couldn't save settings: ${e}`));
  }

  function openSettings() {
    if (phase === "exporting") return;
    player?.pause();
    screen = "settings";
  }

  let gearButton: HTMLButtonElement | undefined = $state();

  async function closeSettings() {
    // a size typed but not yet committed saves on blur ("change"), before the page goes
    (document.activeElement as HTMLElement | null)?.blur();
    screen = "editor";
    await tick();
    // how focus arrived stays as before settings opened: a clicked gear keeps Space on play
    gearButton?.focus();
  }

  // clip
  let view = $state<ClipView | null>(null);
  let src = $state("");
  let loading = $state(false);
  let error = $state("");
  let dragOver = $state(false);
  let proxyProgress: number | null = $state(null);
  let keyframesReady = $state(false);
  let loadingName = $state("");

  // playback + sections
  let player: Player | undefined = $state();
  let currentTime = $state(0);
  let paused = $state(true);
  let sections: Section[] = $state([]);
  let selected: number | null = $state(null);
  let pendingIn: number | null = $state(null);

  // export settings: every clip starts in Original; the size is remembered
  let mode: "original" | "shrink" = $state("original");
  let targetMb = $state(DEFAULT_SETTINGS.targetMb);
  let chosenFormat: Format | null = $state(null);
  let selectedTracks: number[] = $state([]);
  let advice: SizeAdvice | null = $state(null);
  let adviceTicket = 0;

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

  // size advice follows the longest section, the target and the audio choice
  $effect(() => {
    const len = longest(sections);
    const hasAudio = selectedTracks.length > 0;
    const bytes = mbToBytes(targetMb);
    if (!info || len <= 0) {
      advice = null;
      return;
    }
    // while dragging the slider, only the newest answer counts
    const ticket = ++adviceTicket;
    sizeAdvice(len, bytes, hasAudio)
      .then((a) => ticket === adviceTicket && (advice = a))
      .catch(() => ticket === adviceTicket && (advice = null));
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
    loadingName = fileName(path);
    error = "";
    try {
      const v = await openClip(path);
      view = v;
      src = fileUrl(v.info.path);
      sections = [];
      selected = null;
      pendingIn = null;
      chosenFormat = null;
      mode = settings.startMode;
      targetMb = settings.targetMb;
      selectedTracks = initialTracks(settings.audio, settings.lastAudio, v.info.audioTracks.length);
      currentTime = 0;
      phase = "idle";
      keyframesReady = false;
      screen = "editor"; // dropped or opened while the settings page was up
      const opened = v.info.path;
      // both arrive later; drop them if another clip was opened meanwhile
      clipKeyframes(opened)
        .then((k) => {
          if (view?.info.path !== opened) return;
          view = { ...view, info: { ...view.info, keyframes: k } };
          keyframesReady = true;
        })
        .catch(() => {});
      if (!v.thumbs) {
        clipThumbs(opened)
          .then((t) => {
            if (view?.info.path === opened) view = { ...view, thumbs: t };
          })
          .catch(() => {});
      }
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
      // set before the first await, so a second Export click meanwhile is ignored
      picking = true;
      try {
        // an unreachable folder counts as missing: ask instead
        const hasFolder =
          settings.saveTo === "folder" && !!settings.folder && (await folderExists(settings.folder).catch(() => false));
        const target = resolveOutDir(settings, info.path, hasFolder);
        if (target.kind === "dir") {
          dir = target.dir;
        } else {
          const picked = await open({ directory: true, title: target.title });
          if (typeof picked !== "string") return;
          dir = picked;
        }
      } finally {
        picking = false;
      }
    }
    retrying = outDir !== undefined;
    if (!retrying) {
      lastRun = { sections: ordered, outDir: dir };
      outcomes = [];
      updateSettings({ ...settings, lastAudio: selectedTracks });
    }
    runNumbers = ordered.map((s) => s.number);
    progress = { clip: 1, count: ordered.length, fraction: 0, eta: null, preparing: true };
    startedAt = performance.now();
    phase = "exporting";
    try {
      await startExport({
        sections: ordered.map((s) => ({ start: s.start, end: s.end, number: s.number })),
        mode: mode === "original" ? { kind: "original" } : { kind: "shrink", targetBytes: mbToBytes(targetMb), format: chosenFormat },
        audioTracks: selectedTracks,
        outDir: dir,
      }, settings.copyToClipboard);
    } catch (e) {
      // Cancel pressed while it was still preparing: nothing ran, nothing failed
      if (String(e) === "Cancelled.") {
        phase = "idle";
        progress = null;
        return;
      }
      phase = "done";
      const err: ClipOutcome = { kind: "failed", error: String(e), detail: "" };
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

  // How focus last moved. WebView2 marks a mouse-focused button :focus-visible
  // on the first key press, so we track it ourselves: a click means Space
  // should still play, a Tab means Space should press the focused button.
  let focusByPointer = false;

  function onKey(e: KeyboardEvent) {
    if (screen === "settings") {
      if (e.key === "Escape") closeSettings();
      return;
    }
    const target = e.target as HTMLElement;
    if (!info || screen !== "editor") return;
    const action = keyAction(e.key, {
      typing: target.tagName === "INPUT" || target.tagName === "TEXTAREA",
      buttonFocusedByKeyboard: target.tagName === "BUTTON" && !focusByPointer,
      exporting: phase === "exporting",
      shift: e.shiftKey,
    });
    if (!action) return;
    e.preventDefault();
    const frame = 1 / (info.fps || 60);
    switch (action.kind) {
      case "jump": {
        const next = neighbour(sections, selected, action.dir);
        if (next) {
          selected = next.id;
          player?.seek(next.start);
        }
        break;
      }
      case "toggle":
        player?.toggle();
        break;
      case "step":
        step(action.dir * (action.unit === "second" ? 1 : frame));
        break;
      case "markIn":
        ({ sections, pendingIn, selected } = markIn({ sections, pendingIn, selected }, currentTime, info.duration));
        break;
      case "markOut":
        ({ sections, pendingIn, selected } = markOut({ sections, pendingIn, selected }, currentTime, info.duration));
        break;
      case "remove":
        if (selected !== null) {
          sections = removeSection(sections, selected);
          selected = null;
        }
        break;
      case "clear":
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
          progress = { ...progress, clip: runNumbers.indexOf(e.clipNumber) + 1, count: e.clipCount, fraction: 0, preparing: false };
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
          copiedToClipboard = settings.copyToClipboard && all.length > 0 && (await copyToClipboard(all).then(() => true, () => false));
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
    getSettings()
      .then((s) => {
        let stored: string | null = null;
        try {
          stored = localStorage.getItem("shrink.targetMb");
        } catch {
          /* no storage: nothing to take over */
        }
        const migrated = migrateTargetMb(s, stored);
        if (migrated) {
          settings = migrated;
          // the old value goes only once the new file has it; a failed save retries next launch
          setSettings(migrated)
            .then(() => {
              try {
                localStorage.removeItem("shrink.targetMb");
              } catch {
                /* ignore */
              }
            })
            .catch((e) => (error = `Couldn't save settings: ${e}`));
        } else {
          settings = s;
        }
        targetMb = settings.targetMb;
      })
      .catch(() => {})
      // "Open in shrink": open the clip once the settings are in, so it starts with them
      .then(() => initialFile())
      .then((f) => {
        if (f) openPath(f);
      });
    return () => unlisten.forEach((u) => u.then((f) => f()));
  });
</script>

<svelte:window
  onpointerdowncapture={() => (focusByPointer = true)}
  onkeydowncapture={(e) => e.key === "Tab" && (focusByPointer = false)}
  onkeydown={onKey}
/>

<div class="app">
  <header>
    <div class="logo"><span class="mark"></span>shrink</div>
    <div class="file">
      {#if screen === "settings"}<b>Settings</b>{:else if info}<b>{fileName(info.path)}</b>{info.height}p · {Math.round(info.fps)} fps · {fmtMb(info.sizeBytes)}{/if}
    </div>
    <UpdateNotice busy={phase === "exporting"} />
    {#if screen === "settings"}
      <button class="btn ghost small" onclick={closeSettings}>← Back</button>
    {:else}
      <button class="icon" bind:this={gearButton} aria-label="Settings" title="Settings" onclick={openSettings} disabled={phase === "exporting"}>⚙</button>
    {/if}
    {#if screen === "editor" && info}
      <button class="btn ghost small" onclick={pickFile} disabled={phase === "exporting"}>Open…</button>
    {/if}
  </header>

  <div class="screen" hidden={screen === "settings"}>
    {#if view && info}
      <main>
        <Player bind:this={player} {src} duration={info.duration} bind:currentTime bind:paused onerror={onPlayerError} />
        <Timeline
          duration={info.duration}
          {currentTime}
          onseek={(t) => player?.seek(t)}
          thumbs={view.thumbs ? fileUrl(view.thumbs) : null}
          keyframes={info.keyframes}
          showActualStart={mode === "original" && keyframesReady}
          {pendingIn}
          bind:sections
          bind:selected
        />
        <div class="hint muted">
          Drag on the timeline or press <kbd>I</kbd> <kbd>O</kbd> to mark a highlight · click it or press <kbd>Q</kbd> <kbd>E</kbd> to jump there · <kbd>Space</kbd> plays · <kbd>←</kbd><kbd>→</kbd> frame, <kbd>Shift</kbd> second · wheel skips · <kbd>Del</kbd> removes
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
          <div class="big">Opening {loadingName}…</div>
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
  </div>
  {#if screen === "settings"}<SettingsPage {settings} onchange={updateSettings} />{/if}

  {#if view && (dragOver || loading)}
    <div class="overlay">{loading ? `Opening ${loadingName}…` : "Drop to open"}</div>
  {/if}
  {#if proxyProgress !== null}
    <div class="overlay">Preparing preview… {Math.round(proxyProgress * 100)}%</div>
  {/if}
  {#if view && error}
    <div class="toast" role="alert">{error} <button aria-label="Dismiss" onclick={() => (error = "")}>✕</button></div>
  {/if}
</div>

<style>
  .app {
    height: 100vh; /* geometry */
    display: flex;
    flex-direction: column;
    position: relative;
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--s4);
    padding: var(--s3) var(--s4) var(--s3);
  }
  .logo {
    display: flex;
    align-items: center;
    gap: var(--s2);
    font-family: var(--font-logo);
    font-weight: 800; /* Sora wordmark */
    font-size: var(--t-xl);
    letter-spacing: -0.02em;
  }
  .mark {
    width: 14px; /* geometry */
    height: 14px; /* geometry */
    border-radius: 50%;
    background: var(--accent);
  }
  .file {
    flex: 1;
    min-width: 0; /* geometry */
    font-size: var(--t-sm);
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .file b {
    color: var(--text);
    font-weight: var(--w-medium);
    margin-right: var(--s3);
  }
  .screen {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .screen[hidden] {
    display: none;
  }
  .icon {
    width: 28px; /* geometry */
    height: 28px; /* geometry */
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: transparent;
    color: var(--muted-hi);
    display: grid;
    place-items: center;
  }
  .icon:hover {
    background: var(--panel-2);
  }
  .icon:disabled {
    opacity: 0.45;
    cursor: default;
  }
  main {
    flex: 1;
    min-height: 0; /* geometry */
    display: flex;
    flex-direction: column;
    gap: var(--s3);
    padding: 0 var(--s4);
  }
  .hint {
    font-size: var(--t-xs);
    text-align: center;
    margin-top: var(--s3);
  }
  kbd {
    font-family: var(--font);
    font-size: var(--t-xs);
    padding: var(--s1) var(--s1);
    border-radius: var(--r-sm);
    border: 1px solid var(--line);
    background: var(--panel);
    color: var(--text);
    margin: 0 var(--s1);
  }
  .drop {
    flex: 1;
    margin: var(--s1) var(--s4) var(--s4);
    border: 2px dashed var(--line);
    border-radius: var(--r-md);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--s3);
    text-align: center;
    transition: border-color 0.15s, background 0.15s;
  }
  .drop.over {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .big {
    font-family: var(--font-logo);
    font-size: var(--t-2xl);
    font-weight: 800; /* Sora wordmark */
  }
  .error {
    color: var(--warn);
    max-width: 520px; /* geometry */
    font-size: var(--t-sm);
  }
  .overlay {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: var(--scrim);
    font-family: var(--font-logo);
    font-size: var(--t-2xl);
    font-weight: 800; /* Sora wordmark */
    z-index: 10;
  }
  .toast {
    position: absolute;
    top: 56px; /* geometry */
    left: 50%;
    transform: translateX(-50%);
    background: var(--panel);
    border: 1px solid var(--warn);
    color: var(--text);
    padding: var(--s2) var(--s3);
    border-radius: var(--r-md);
    font-size: var(--t-sm);
    z-index: 11;
    max-width: 80%;
  }
  .toast button {
    border: none;
    background: transparent;
    color: var(--muted);
    margin-left: var(--s2);
  }
</style>
