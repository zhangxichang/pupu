<script lang="ts">
  import iconUrl from "../../icon.svg";
  import ResourceLoading from "./ResourceLoading.svelte";
</script>

<svelte:head>
  <link rel="icon" href={iconUrl} />
  <title>Pupu</title>
</svelte:head>

<canvas id="canvas" class="fixed inset-0 h-dvh! w-dvw! outline-none"></canvas>
<ResourceLoading />
