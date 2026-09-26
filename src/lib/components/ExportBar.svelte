<script lang="ts">
  import type { AudioTrack, ClipOutcome, Encoder, Format, SizeAdvice } from "$lib/api";
  import { bytesToMb, fmtMb } from "$lib/format";
  import { posToValue, valueToPos } from "$lib/slider";

  export type Phase = "idle" | "exporting" | "done";
  export type Progress = { clip: number; count: number; fraction: number; eta: number | null };

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

  const fmtFormat = (f: Format) => `${f.height}p · ${Math.round(f.fps)}`;
  const trackName = (t: AudioTrack) => t.title ?? `Track ${t.index + 1}`;
  const done = $derived(outcomes.filter((o) => o.kind === "done") as Extract<ClipOutcome, { kind: "done" }>[]);
  const failed = $derived(outcomes.filter((o) => o.kind === "failed") as Extract<ClipOutcome, { kind: "failed" }>[]);

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
</script>

<div class="dock">
  {#if phase === "idle"}
    <div class="row">
      <div class="seg" role="radiogroup" aria-label="Export mode">
        <button class:on={mode === "original"} onclick={() => (mode = "original")}>Original</button>
        <button class:on={mode === "shrink"} onclick={() => (mode = "shrink")}>Shrink</button>
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
        <div class="size note">Lossless and instant. Each clip starts at the keyframe before your mark.</div>
      {/if}

      {#if tracks.length >= 2}
        <div class="chips" aria-label="Audio tracks">
          {#each tracks as t}
            <button class="chip" class:on={selectedTracks.includes(t.index)} onclick={() => toggleTrack(t.index)}>{trackName(t)}</button>
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
        <span>Exporting at <b>{fmtFormat(chosenFormat)} fps</b> <span class="muted">to fit {showMb(targetMb)} MB cleanly.</span></span>
        <span class="grow"></span>
        <button class="fix alt" onclick={() => (chosenFormat = null)}>Keep {fmtFormat(source)}</button>
      </div>
    {:else if inZone && advice}
      <div class="line">
        <span class="dot">!</span>
        <span><b>Tight for {fmtFormat(source)} fps</b> <span class="muted">— fast movement will look blocky.</span></span>
        <span class="grow"></span>
        {#each advice.suggestions as s}
          <button class="fix" class:alt={!s.suggested} onclick={() => (chosenFormat = s.format)}>
            {s.suggested ? "Use " : ""}{fmtFormat(s.format)}{s.clean ? " (clean)" : ""}
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
        <span>Clip <b>{progress.clip} of {progress.count}</b> · {mode === "shrink" ? `shrinking to ${showMb(targetMb)} MB` : "copying losslessly"}</span>
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
          <span class="muted">· {done.map((d) => fmtMb(d.sizeBytes).replace(" MB", "")).join(" · ")} MB{copiedToClipboard ? " · copied to clipboard" : ""}</span>
        {:else}
          <b>Nothing was saved</b>
        {/if}
        {#if failed.length > 0}
          <span class="err">· {failed.length} failed: {failed[0].error}</span>
        {/if}
      </span>
      {#if failed.length > 0}
        <button class="btn ghost" onclick={onretry}>Try again</button>
      {/if}
      {#if done.length > 0}
        <button class="btn ghost" onclick={onreveal}>Show in folder</button>
      {/if}
      <button class="btn" onclick={ondone}>Done</button>
    </div>
  {/if}
</div>

<style>
  .dock {
    margin: 12px 14px 14px;
    padding: 12px 14px;
    border-radius: 12px;
    background: var(--panel);
    border: 1px solid var(--line);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    min-height: 36px;
  }
  .grow {
    flex: 1;
  }
  .seg {
    display: flex;
    background: var(--panel-2);
    border-radius: 999px;
    padding: 3px;
  }
  .seg button {
    border: none;
    background: transparent;
    padding: 6px 14px;
    border-radius: 999px;
    color: var(--muted);
  }
  .seg button.on {
    background: var(--seg-on);
    color: var(--text);
    font-weight: 600;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.3);
  }
  .size {
    flex: 1;
    min-width: 200px;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .size.note {
    color: var(--muted);
    font-size: 12px;
  }
  .slider {
    position: relative;
    flex: 1;
    height: 22px;
  }
  .slider .zone,
  .slider .fill {
    position: absolute;
    left: 0;
    top: 8px;
    height: 6px;
    border-radius: 3px;
    pointer-events: none;
  }
  .slider::before {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 8px;
    height: 6px;
    border-radius: 3px;
    background: var(--panel-2);
  }
  .slider .zone {
    background: rgba(242, 182, 64, 0.32);
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
    height: 22px;
    background: transparent;
  }
  .slider input::-webkit-slider-thumb {
    appearance: none;
    width: 16px;
    height: 16px;
    margin-top: 3px;
    border-radius: 50%;
    background: var(--text);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.45);
  }
  .slider.warn input::-webkit-slider-thumb {
    box-shadow: 0 0 0 3px rgba(242, 182, 64, 0.45);
  }
  .val {
    border: none;
    background: transparent;
    font-weight: 700;
    font-size: 13px;
    white-space: nowrap;
    padding: 4px 6px;
    border-radius: 6px;
    min-width: 72px;
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
    font-size: 10.5px;
    margin-left: 4px;
    font-weight: 400;
  }
  .typed {
    width: 72px;
    padding: 4px 6px;
    border-radius: 6px;
    border: 1px solid var(--accent);
    background: var(--panel-2);
    color: var(--text);
    font: inherit;
    font-weight: 700;
    text-align: right;
  }
  .chips {
    display: flex;
    gap: 6px;
  }
  .chip {
    font-size: 11.5px;
    padding: 5px 11px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--muted);
  }
  .chip.on {
    border-color: var(--accent);
    color: var(--text);
    background: var(--accent-soft);
  }
  .line {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    margin-top: 10px;
    padding-top: 10px;
    border-top: 1px dashed var(--line);
  }
  .line.quiet {
    border-top: none;
    margin-top: 4px;
    padding-top: 0;
    font-size: 11.5px;
  }
  .dot {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--warn);
    color: #2b2622;
    display: grid;
    place-items: center;
    font-weight: 800;
    font-size: 12px;
    flex: none;
  }
  .dot.ok {
    background: var(--ok);
  }
  .fix {
    font-size: 11.5px;
    padding: 4px 11px;
    border-radius: 999px;
    border: 1px solid var(--warn);
    color: var(--warn);
    font-weight: 600;
    background: transparent;
  }
  .fix.alt {
    border-color: var(--line);
    color: var(--muted);
    font-weight: 500;
  }
  .fix:hover {
    background: var(--warn-soft);
  }
  .prog-top {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: 12px;
    margin-bottom: 7px;
  }
  .prog-top b {
    color: var(--text);
    font-weight: 600;
  }
  .bar {
    height: 6px;
    border-radius: 3px;
    background: var(--panel-2);
    overflow: hidden;
  }
  .bar i {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 3px;
    transition: width 0.2s linear;
  }
  .prog-actions {
    text-align: right;
    margin-top: 8px;
  }
  .check {
    color: var(--ok);
    font-weight: 800;
    font-size: 15px;
  }
  .result {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .err {
    color: var(--warn);
  }
</style>
