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
    min-height: 0;
    border-radius: 10px;
    overflow: hidden;
    background: #141110;
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
    left: 12px;
    bottom: 12px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    color: #fff;
    background: rgba(0, 0, 0, 0.5);
    padding: 4px 9px;
    border-radius: 7px;
    backdrop-filter: blur(6px);
    pointer-events: none;
  }
  .play {
    position: absolute;
    right: 12px;
    bottom: 12px;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: none;
    background: rgba(0, 0, 0, 0.5);
    color: #fff;
    display: grid;
    place-items: center;
    backdrop-filter: blur(6px);
  }
  .play:hover {
    background: rgba(0, 0, 0, 0.7);
  }
</style>
