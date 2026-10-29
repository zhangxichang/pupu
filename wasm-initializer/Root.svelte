<script lang="ts">
  import { loadingState } from "./state.svelte.ts";

  let percent = $derived(
    loadingState.total > 0
      ? Math.min(100, (loadingState.current / loadingState.total) * 100)
      : 0,
  );
</script>

{#if loadingState.visible}
  <div class="overlay">
    {#if loadingState.status === "error"}
      <p>
        {loadingState.error instanceof Error
          ? loadingState.error.message
          : String(loadingState.error)}
      </p>
    {:else}
      <div class="bar">
        <div class="fill" style:width={`${percent}%`}></div>
      </div>
      <span>{Math.round(percent)}%</span>
    {/if}
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 9999;
    display: grid;
    place-content: center;
    gap: 8px;
    background: #fff;
    color: #333;
    font-family: system-ui, sans-serif;
  }

  .bar {
    width: 240px;
    height: 4px;
    overflow: hidden;
    background: #eee;
    border-radius: 2px;
  }

  .fill {
    height: 100%;
    background: #333;
  }
</style>
