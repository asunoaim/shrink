<script lang="ts">
  import { fmtTime } from "$lib/format";
  import {
    addSection,
    moveSection,
    numbered,
    resizeSection,
    type Edge,
    type Section,
  } from "$lib/sections";
  import { actualStart } from "$lib/slider";

  let {
    duration,
    currentTime,
    onseek,
    thumbs,
    keyframes,
    showActualStart,
    pendingIn,
    sections = $bindable([]),
    selected = $bindable(null),
  }: {
    duration: number;
    currentTime: number;
    onseek: (t: number) => void;
    thumbs: string | null;
    keyframes: number[];
    showActualStart: boolean;
    pendingIn: number | null;
    sections?: Section[];
    selected?: number | null;
  } = $props();

  type Drag =
    | { kind: "scrub" }
    | { kind: "draw"; from: number; to: number; x0: number; moved: boolean }
    | { kind: "move"; id: number; t0: number; base: Section[]; x0: number; moved: boolean }
    | { kind: "resize"; id: number; edge: Edge };

  let track: HTMLDivElement | undefined = $state();
  let drag: Drag | null = $state(null);

  const pct = (t: number) => `${(t / duration) * 100}%`;
  const ticks = $derived(tickTimes(duration));
  const list = $derived(numbered(sections));

  function tickTimes(d: number): number[] {
    const steps = [1, 2, 5, 10, 15, 30, 60, 120, 300, 600];
    const step = steps.find((s) => d / s <= 10) ?? 1200;
    const out: number[] = [];
    for (let t = 0; t <= d + 1e-6; t += step) out.push(t);
    return out;
  }

  function timeAt(e: PointerEvent): number {
    const r = track!.getBoundingClientRect();
    return Math.min(duration, Math.max(0, ((e.clientX - r.left) / r.width) * duration));
  }

  function begin(e: PointerEvent, d: Drag) {
    if (e.button !== 0) return;
    e.stopPropagation();
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = d;
  }

  function onScrubDown(e: PointerEvent) {
    begin(e, { kind: "scrub" });
    onseek(timeAt(e));
  }

  function onTrackDown(e: PointerEvent) {
    const t = timeAt(e);
    begin(e, { kind: "draw", from: t, to: t, x0: e.clientX, moved: false });
  }

  function onSectionDown(e: PointerEvent, s: Section) {
    selected = s.id;
    begin(e, { kind: "move", id: s.id, t0: timeAt(e), base: sections, x0: e.clientX, moved: false });
  }

  function onEdgeDown(e: PointerEvent, s: Section, edge: Edge) {
    selected = s.id;
    begin(e, { kind: "resize", id: s.id, edge });
  }

  function onMove(e: PointerEvent) {
    if (!drag) return;
    const t = timeAt(e);
    switch (drag.kind) {
      case "scrub":
        onseek(t);
        break;
      case "draw":
        drag.to = t;
        drag.moved ||= Math.abs(e.clientX - drag.x0) > 4;
        break;
      case "move":
        drag.moved ||= Math.abs(e.clientX - drag.x0) > 3;
        if (drag.moved) sections = moveSection(drag.base, drag.id, t - drag.t0, duration);
        break;
      case "resize":
        sections = resizeSection(sections, drag.id, drag.edge, t, duration);
        onseek(t);
        break;
    }
  }

  function onUp(e: PointerEvent) {
    if (!drag) return;
    if (drag.kind === "draw") {
      if (drag.moved) {
        const before = sections.length;
        sections = addSection(sections, drag.from, drag.to, duration);
        if (sections.length > before) selected = sections[sections.length - 1].id;
      } else {
        selected = null;
        onseek(timeAt(e));
      }
    } else if (drag.kind === "move" && !drag.moved) {
      onseek(timeAt(e));
    }
    drag = null;
  }
</script>

<div class="timeline" role="presentation" onpointermove={onMove} onpointerup={onUp} onpointercancel={onUp}>
  <div class="ruler" role="presentation" onpointerdown={onScrubDown}>
    {#each ticks as t}
      <span class="tick" style:left={pct(t)}>{fmtTime(t).replace(/\.\d$/, "")}</span>
    {/each}
  </div>

  <div class="track" bind:this={track} role="presentation" onpointerdown={onTrackDown}>
    <div class="strip" style:background-image={thumbs ? `url("${thumbs}")` : "none"}></div>

    {#each list as s (s.id)}
      {@const real = showActualStart ? actualStart(keyframes, s.start) : s.start}
      {#if showActualStart && s.start - real > 0.05}
        <div class="lead" style:left={pct(real)} style:width={pct(s.start - real)} title="Lossless clips start at the keyframe before your mark">
          <span class="lead-label">starts {fmtTime(real)}</span>
        </div>
      {/if}
      <div
        class="section"
        class:selected={selected === s.id}
        style:left={pct(s.start)}
        style:width={pct(s.end - s.start)}
        role="presentation"
        onpointerdown={(e) => onSectionDown(e, s)}
      >
        <i class="badge">{s.number}</i>
        <span class="edge start" role="presentation" onpointerdown={(e) => onEdgeDown(e, s, "start")}></span>
        <span class="edge end" role="presentation" onpointerdown={(e) => onEdgeDown(e, s, "end")}></span>
      </div>
    {/each}

    {#if drag?.kind === "draw" && drag.moved}
      <div class="section drawing" style:left={pct(Math.min(drag.from, drag.to))} style:width={pct(Math.abs(drag.to - drag.from))}></div>
    {/if}

    {#if pendingIn !== null}
      <div class="pending" style:left={pct(pendingIn)} title="Start marked. Press O to set the end."></div>
    {/if}
  </div>

  <div class="playhead" style:left={pct(currentTime)}></div>
</div>

<style>
  .timeline {
    position: relative;
    margin: 0 2px;
  }
  .ruler {
    position: relative;
    height: 20px;
    cursor: ew-resize;
  }
  .tick {
    position: absolute;
    top: 3px;
    transform: translateX(-50%);
    font-size: 10.5px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    pointer-events: none;
  }
  .tick:first-child {
    transform: none;
  }
  .track {
    position: relative;
    height: 58px;
    cursor: crosshair;
  }
  .strip {
    position: absolute;
    inset: 6px 0;
    border-radius: 8px;
    background-color: var(--panel-2);
    background-size: 100% 100%;
    opacity: 0.8;
  }
  .section {
    position: absolute;
    top: 2px;
    bottom: 2px;
    border-radius: 8px;
    border: 2px solid var(--accent);
    background: var(--accent-soft);
    cursor: grab;
    min-width: 6px;
  }
  .section.selected {
    background: rgba(239, 106, 63, 0.3);
    box-shadow: 0 0 0 3px rgba(239, 106, 63, 0.25), 0 0 16px var(--accent-glow);
  }
  .section.drawing {
    border-style: dashed;
    pointer-events: none;
  }
  .badge {
    position: absolute;
    top: -10px;
    left: 6px;
    font-style: normal;
    font-size: 10px;
    font-weight: 700;
    background: var(--accent);
    color: var(--on-accent);
    padding: 1px 7px;
    border-radius: 999px;
    pointer-events: none;
  }
  .edge {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 10px;
    cursor: ew-resize;
  }
  .edge.start {
    left: -6px;
  }
  .edge.end {
    right: -6px;
  }
  .lead {
    position: absolute;
    top: 10px;
    bottom: 10px;
    border-top: 2px dashed var(--accent);
    border-bottom: 2px dashed var(--accent);
    border-left: 2px dashed var(--accent);
    border-radius: 6px 0 0 6px;
    opacity: 0.75;
    pointer-events: none;
  }
  .lead-label {
    position: absolute;
    bottom: -24px;
    left: 0;
    font-size: 10px;
    color: var(--muted);
    white-space: nowrap;
  }
  .pending {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 2px;
    background: var(--accent);
    box-shadow: 0 0 10px var(--accent-glow);
    pointer-events: none;
  }
  .playhead {
    position: absolute;
    top: 16px;
    bottom: -2px;
    width: 2px;
    margin-left: -1px;
    background: var(--text);
    pointer-events: none;
  }
  .playhead::before {
    content: "";
    position: absolute;
    top: -2px;
    left: -4px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--text);
  }
</style>
