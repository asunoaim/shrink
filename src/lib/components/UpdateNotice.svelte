<script lang="ts">
  import { onMount } from "svelte";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";

  let { busy }: { busy: boolean } = $props();

  let update: Update | null = $state(null);
  let status: "idle" | "installing" | "failed" = $state("idle");
  let percent = $state(0);

  onMount(() => {
    // quiet when offline or when there's nothing new
    check()
      .then((u) => (update = u))
      .catch(() => {});
  });

  async function install() {
    if (!update) return;
    status = "installing";
    let total = 0;
    let got = 0;
    try {
      await update.downloadAndInstall((e) => {
        if (e.event === "Started") total = e.data.contentLength ?? 0;
        if (e.event === "Progress") {
          got += e.data.chunkLength;
          percent = total ? Math.round((got / total) * 100) : 0;
        }
      });
      await relaunch();
    } catch {
      status = "failed";
    }
  }
</script>

{#if update}
  <div class="notice" title={update.body ?? ""}>
    {#if status === "installing"}
      <span>Updating… {percent}%</span>
    {:else if status === "failed"}
      <span>Update failed.</span>
      <button onclick={install} disabled={busy}>Retry</button>
    {:else}
      <span>Version {update.version} is out</span>
      <button onclick={install} disabled={busy} title={busy ? "Finish the export first" : ""}>Update &amp; restart</button>
    {/if}
  </div>
{/if}

<style>
  .notice {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    padding: 4px 4px 4px 12px;
    border-radius: 999px;
    background: var(--accent-soft);
    border: 1px solid var(--accent);
    white-space: nowrap;
  }
  button {
    border: none;
    border-radius: 999px;
    padding: 4px 10px;
    font-weight: 700;
    font-size: 11.5px;
    background: var(--accent);
    color: var(--on-accent);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
