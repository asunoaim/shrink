<script lang="ts">
  import { fmtTime } from "$lib/format";

  let {
    src,
    duration,
    currentTime = $bindable(0),
    paused = $bindable(true),
    onerror,
  }: {
    src: string;
    duration: number;
    currentTime?: number;
    paused?: boolean;
    onerror?: () => void;
  } = $props();

  let video: HTMLVideoElement | undefined = $state();

  export function seek(t: number) {
    if (!video) return;
    const clamped = Math.min(Math.max(0, t), duration);
    video.currentTime = clamped;
    currentTime = clamped;
  }

  export function toggle() {
    if (!video) return;
    if (video.paused) video.play();
    else video.pause();
  }

  export function pause() {
    video?.pause();
  }
</script>

<div class="stage">
  <!-- svelte-ignore a11y_media_has_caption -->
  <video
    bind:this={video}
    {src}
    bind:currentTime
    bind:paused
    preload="auto"
    onerror={() => onerror?.()}
    onclick={toggle}
  ></video>
  <div class="hud">{fmtTime(currentTime)} / {fmtTime(duration)}</div>
  <button class="play" onclick={toggle} aria-label={paused ? "Play" : "Pause"}>
    {#if paused}
      <svg viewBox="0 0 16 16" width="12" height="12"><path d="M4 2.5v11l9-5.5z" fill="currentColor" /></svg>
    {:else}
      <svg viewBox="0 0 16 16" width="12" height="12"><path d="M4 2h3v12H4zM9 2h3v12H9z" fill="currentColor" /></svg>
    {/if}
  </button>
</div>

<style>
  .stage {
    position: relative;
    flex: 1;
    min-height: 0; /* geometry */
    border-radius: var(--r-md);
    overflow: hidden;
    background: var(--stage);
  }
  video {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }
  .hud {
    position: absolute;
    left: var(--s3);
    bottom: var(--s3);
    font-size: var(--t-sm);
    font-variant-numeric: tabular-nums;
    color: var(--hud-text);
    background: var(--hud-bg);
    padding: var(--s1) var(--s2);
    border-radius: var(--r-sm);
    pointer-events: none;
  }
  .play {
    position: absolute;
    right: var(--s3);
    bottom: var(--s3);
    width: 32px; /* geometry */
    height: 32px; /* geometry */
    border-radius: 50%;
    border: none;
    background: var(--hud-bg);
    color: var(--hud-text);
    display: grid;
    place-items: center;
  }
  .play:hover {
    background: var(--hud-bg-hover);
  }
</style>
