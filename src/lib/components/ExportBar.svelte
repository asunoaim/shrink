<script lang="ts">
  import type { AudioTrack, ClipOutcome, Encoder, Format, SizeAdvice } from "$lib/api";
  import { bytesToMb, fmtFormat, fmtMb } from "$lib/format";
  import { posToValue, valueToPos } from "$lib/slider";

  export type Phase = "idle" | "exporting" | "done";
  export type Progress = { clip: number; count: number; fraction: number; eta: number | null; preparing: boolean };

  let {
    mode = $bindable("original"),
    targetMb = $bindable(25),
    chosenFormat = $bindable(null),
    selectedTracks = $bindable([]),
    advice,
    source,
    tracks,
    count,
    encoder,
    phase,
    progress,
    outcomes,
    copiedToClipboard,
    onexport,
    oncancel,
    onreveal,
    ondone,
    onretry,
  }: {
    mode?: "original" | "shrink";
    targetMb?: number;
    chosenFormat?: Format | null;
    selectedTracks?: number[];
    advice: SizeAdvice | null;
    source: Format;
    tracks: AudioTrack[];
    count: number;
    encoder: Encoder;
    phase: Phase;
    progress: Progress | null;
    outcomes: ClipOutcome[];
    copiedToClipboard: boolean;
    onexport: () => void;
    oncancel: () => void;
    onreveal: () => void;
    ondone: () => void;
    onretry: () => void;
  } = $props();

  const minMb = $derived(advice ? bytesToMb(advice.sliderMinBytes) : 1);
  const maxMb = $derived(advice ? Math.max(bytesToMb(advice.sliderMaxBytes), minMb) : 100);
  const warnMb = $derived(advice ? bytesToMb(advice.warnMaxBytes) : 0);
  const pos = $derived(valueToPos(targetMb, minMb, maxMb));
  const zone = $derived(valueToPos(warnMb, minMb, maxMb));
  const inZone = $derived(mode === "shrink" && !!advice?.tooSmall && !chosenFormat);

  let editing = $state(false);
  let typed = $state("");
  let showDetails = $state(false);

  const trackName = (t: AudioTrack) => t.title ?? `Audio ${t.index + 1}`;
  const done = $derived(outcomes.filter((o) => o.kind === "done") as Extract<ClipOutcome, { kind: "done" }>[]);
  const failed = $derived(outcomes.filter((o) => o.kind === "failed") as Extract<ClipOutcome, { kind: "failed" }>[]);
  const chosenClean = $derived(!!chosenFormat && !!advice?.suggestions.find((s) => s.format.height === chosenFormat!.height && Math.round(s.format.fps) === Math.round(chosenFormat!.fps))?.clean);

  function showMb(mb: number) {
    return mb < 10 ? mb.toFixed(1) : Math.round(mb).toString();
  }

  function onSlide(e: Event) {
    const p = Number((e.currentTarget as HTMLInputElement).value) / 1000;
    const v = posToValue(p, minMb, maxMb);
    targetMb = v < 10 ? Math.round(v * 10) / 10 : Math.round(v);
    chosenFormat = null;
  }

  function startEdit() {
    typed = showMb(targetMb);
    editing = true;
  }

  function commitEdit() {
    const v = Number(typed.replace(",", "."));
    if (Number.isFinite(v) && v > 0) {
      targetMb = Math.min(maxMb, Math.max(minMb, v));
      chosenFormat = null;
    }
    editing = false;
  }

  function toggleTrack(i: number) {
    selectedTracks = selectedTracks.includes(i)
      ? selectedTracks.filter((t) => t !== i)
      : [...selectedTracks, i].sort((a, b) => a - b);
  }

  function focusSelect(node: HTMLInputElement) {
    // after the click that opened it has finished, or focus stays on the old button
    requestAnimationFrame(() => {
      node.focus();
      node.select();
    });
  }

  $effect(() => {
    if (phase !== "done") showDetails = false;
  });
</script>

<div class="dock">
  {#if phase === "idle"}
    <div class="row">
      <div class="seg" role="radiogroup" aria-label="Export mode">
        <button role="radio" aria-checked={mode === "original"} class:on={mode === "original"} onclick={() => (mode = "original")}>Original quality</button>
        <button role="radio" aria-checked={mode === "shrink"} class:on={mode === "shrink"} onclick={() => (mode = "shrink")}>Shrink to size</button>
      </div>

      {#if mode === "shrink"}
        <div class="size">
          <div class="slider" class:warn={inZone}>
            <div class="zone" style:width={`${zone * 100}%`}></div>
            <div class="fill" style:width={`${pos * 100}%`}></div>
            <input type="range" min="0" max="1000" step="1" value={Math.round(pos * 1000)} oninput={onSlide} aria-label="Target size per clip" />
          </div>
          {#if editing}
            <input class="typed" use:focusSelect bind:value={typed} onblur={commitEdit}
              onkeydown={(e) => { if (e.key === "Enter") commitEdit(); if (e.key === "Escape") editing = false; e.stopPropagation(); }} />
          {:else}
            <button class="val" class:warn={inZone} onclick={startEdit} title="Click to type a size">
              {showMb(targetMb)} MB<small>{count > 1 ? "each" : ""}</small>
            </button>
          {/if}
        </div>
      {:else}
        <div class="size note">Same quality as the recording. Clips can start up to a second early, the closest point where a cut stays lossless.</div>
      {/if}

      {#if tracks.length >= 2}
        <div class="chips" aria-label="Audio tracks">
          {#each tracks as t}
            <button class="chip" class:on={selectedTracks.includes(t.index)} aria-pressed={selectedTracks.includes(t.index)} onclick={() => toggleTrack(t.index)}>{trackName(t)}</button>
          {/each}
        </div>
      {/if}

      <button class="btn" disabled={count === 0} onclick={onexport}>
        {count === 0 ? "Mark a section" : count === 1 ? "Export clip" : `Export ${count} clips`}
      </button>
    </div>

    {#if mode === "shrink" && chosenFormat}
      <div class="line">
        <span class="dot ok">✓</span>
        <span>Exporting at <b>{fmtFormat(chosenFormat)}</b>
          <span class="muted">{chosenClean ? `to fit ${showMb(targetMb)} MB cleanly.` : `to fit ${showMb(targetMb)} MB. Fast movement may still look blocky.`}</span></span>
        <span class="grow"></span>
        <button class="fix alt" onclick={() => (chosenFormat = null)}>Keep {fmtFormat(source)}</button>
      </div>
    {:else if inZone && advice}
      <div class="line">
        <span class="dot">!</span>
        <span><b>Tight for {fmtFormat(source)}</b> <span class="muted">— fast movement will look blocky.</span></span>
        <span class="grow"></span>
        {#each advice.suggestions as s}
          <button class="fix" class:alt={!s.suggested} onclick={() => (chosenFormat = s.format)}>
            {s.suggested ? "Use " : ""}{fmtFormat(s.format)}{s.clean ? " · looks clean" : ""}
          </button>
        {/each}
      </div>
    {/if}
    {#if mode === "shrink" && encoder === "x264"}
      <div class="line quiet"><span class="muted">No graphics-card encoder found. Shrinking uses the CPU (slower).</span></div>
    {/if}
  {:else if phase === "exporting" && progress}
    {@const overall = (progress.clip - 1 + progress.fraction) / progress.count}
    <div class="prog">
      <div class="prog-top">
        <span>
          {#if progress.preparing}
            Preparing…
          {:else}
            Clip <b>{progress.clip} of {progress.count}</b> · {mode === "shrink" ? `shrinking to ${showMb(targetMb)} MB` : "copying losslessly"}
          {/if}
        </span>
        <span><b>{Math.round(overall * 100)}%</b>{progress.eta !== null ? ` · ~${Math.ceil(progress.eta)} s left` : ""}</span>
      </div>
      <div class="bar"><i style:width={`${overall * 100}%`}></i></div>
      <div class="prog-actions"><button class="btn ghost" onclick={oncancel}>Cancel</button></div>
    </div>
  {:else if phase === "done"}
    <div class="row">
      {#if done.length > 0}
        <span class="check">✓</span>
      {:else}
        <span class="dot">!</span>
      {/if}
      <span class="grow result">
        {#if done.length > 0}
          <b>{done.length === 1 ? "1 clip saved" : `${done.length} clips saved`}</b>
          <span class="muted">· {done.map((d) => fmtMb(d.sizeBytes)).join(" · ")}{copiedToClipboard ? " · copied to clipboard" : ""}</span>
        {:else}
          <b>Nothing was saved</b>
        {/if}
        {#if failed.length > 0}
          <span class="err">· {failed.length} failed · {failed[0].error}</span>
        {/if}
      </span>
      {#if failed.length > 0}
        <button class="btn ghost small" aria-expanded={showDetails} onclick={() => (showDetails = !showDetails)}>Details</button>
        <button class="btn ghost" onclick={onretry}>Try again</button>
      {/if}
      {#if done.length > 0}
        <button class="btn ghost" onclick={onreveal}>Show in folder</button>
      {/if}
      <button class="btn" onclick={ondone}>Done</button>
    </div>
    {#if showDetails && failed.length > 0}
      <ul class="details">
        {#each outcomes as o, i}
          {#if o.kind === "failed"}
            <li><b>Clip {i + 1}:</b> {o.error} <code>{o.detail}</code></li>
          {/if}
        {/each}
      </ul>
    {/if}
  {/if}
</div>

<style>
  .dock {
    margin: var(--s3) var(--s4) var(--s4);
    padding: var(--s3) var(--s4);
    border-radius: var(--r-md);
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--s4);
    min-height: 36px; /* geometry */
  }
  .grow {
    flex: 1;
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
    padding: var(--s2) var(--s4);
    border-radius: var(--r-pill);
    color: var(--muted-hi);
  }
  .seg button.on {
    background: var(--bg);
    color: var(--text);
    font-weight: var(--w-bold);
  }
  .size {
    flex: 1;
    min-width: 200px; /* geometry */
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  .size.note {
    color: var(--muted);
    font-size: var(--t-sm);
  }
  .slider {
    position: relative;
    flex: 1;
    height: 22px; /* geometry */
  }
  .slider .zone,
  .slider .fill {
    position: absolute;
    left: 0;
    top: 8px; /* geometry */
    height: 6px; /* geometry */
    border-radius: var(--r-sm);
    pointer-events: none;
  }
  .slider::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 8px; /* geometry */
    height: 6px; /* geometry */
    border-radius: var(--r-sm);
    background: var(--panel-2);
  }
  .slider .zone {
    background: var(--warn-zone);
  }
  .slider .fill {
    background: var(--accent);
  }
  .slider.warn .fill {
    background: var(--warn);
  }
  .slider input {
    position: absolute;
    inset: 0;
    width: 100%;
    margin: 0;
    background: transparent;
    appearance: none;
    cursor: pointer;
  }
  .slider input::-webkit-slider-runnable-track {
    height: 22px; /* geometry */
    background: transparent;
  }
  .slider input::-webkit-slider-thumb {
    appearance: none;
    width: 16px; /* geometry */
    height: 16px; /* geometry */
    margin-top: 3px; /* geometry */
    border-radius: 50%;
    background: var(--text);
    box-shadow: var(--shadow);
  }
  .slider.warn input::-webkit-slider-thumb {
    box-shadow: 0 0 0 3px var(--warn-ring); /* geometry */
  }
  .val {
    border: none;
    background: transparent;
    font-weight: var(--w-bold);
    font-size: var(--t-md);
    white-space: nowrap;
    padding: var(--s1) var(--s2);
    border-radius: var(--r-sm);
    min-width: 72px; /* geometry */
    text-align: right;
  }
  .val:hover {
    background: var(--panel-2);
  }
  .val.warn {
    color: var(--warn);
  }
  .val small {
    color: var(--muted);
    font-size: var(--t-xs);
    margin-left: var(--s1);
    font-weight: var(--w-regular);
  }
  .typed {
    width: 72px; /* geometry */
    padding: var(--s1) var(--s2);
    border-radius: var(--r-sm);
    border: 1px solid var(--accent);
    background: var(--panel-2);
    color: var(--text);
    font: inherit;
    font-weight: var(--w-bold);
    text-align: right;
  }
  .chips {
    display: flex;
    gap: var(--s2);
  }
  .chip {
    font-size: var(--t-sm);
    padding: var(--s1) var(--s3);
    border-radius: var(--r-pill);
    border: 1px solid var(--line);
    background: transparent;
    color: var(--muted-hi);
  }
  .chip.on {
    border-color: var(--accent);
    color: var(--text);
    background: var(--accent-soft);
  }
  .line {
    display: flex;
    align-items: center;
    gap: var(--s3);
    flex-wrap: wrap;
    margin-top: var(--s3);
    padding-top: var(--s3);
    border-top: 1px dashed var(--line);
  }
  .line.quiet {
    border-top: none;
    margin-top: var(--s1);
    padding-top: 0;
    font-size: var(--t-sm);
  }
  .dot {
    width: 18px; /* geometry */
    height: 18px; /* geometry */
    border-radius: 50%;
    background: var(--warn);
    color: var(--on-warn);
    display: grid;
    place-items: center;
    font-weight: var(--w-bold);
    font-size: var(--t-sm);
    flex: none;
  }
  .dot.ok {
    background: var(--ok);
  }
  .fix {
    font-size: var(--t-sm);
    padding: var(--s1) var(--s3);
    border-radius: var(--r-pill);
    border: 1px solid var(--warn);
    color: var(--warn);
    font-weight: var(--w-bold);
    background: transparent;
  }
  .fix.alt {
    border-color: var(--line);
    color: var(--muted);
    font-weight: var(--w-medium);
  }
  .fix:hover {
    background: var(--warn-soft);
  }
  .prog {
    display: flex;
    flex-direction: column;
  }
  .prog-top {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: var(--t-sm);
    margin-bottom: var(--s2);
  }
  .prog-top b {
    color: var(--text);
    font-weight: var(--w-bold);
  }
  .bar {
    height: 6px; /* geometry */
    border-radius: var(--r-sm);
    background: var(--panel-2);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: var(--r-sm);
    transition: width 0.2s linear;
  }
  .prog-actions {
    text-align: right;
    margin-top: var(--s2);
  }
  .check {
    color: var(--ok);
    font-weight: var(--w-bold);
    font-size: var(--t-lg);
  }
  .result {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .err {
    color: var(--warn);
  }
  .details {
    margin: var(--s2) 0 0;
    padding: var(--s2) var(--s3);
    border-top: 1px solid var(--line);
    font-size: var(--t-sm);
    list-style: none;
    user-select: text;
    max-height: 160px; /* geometry */
    overflow: auto;
  }
  .details code {
    display: block;
    color: var(--muted);
    white-space: pre-wrap;
    margin: var(--s1) 0 var(--s2);
  }
</style>
