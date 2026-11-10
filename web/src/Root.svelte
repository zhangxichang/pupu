<script lang="ts">
  import iconUrl from "../../icon.svg";
  import ResourceLoading from "./ResourceLoading.svelte";
</script>

<svelte:head>
  <link rel="icon" href={iconUrl} />
  <title>Pupu</title>
</svelte:head>

<canvas id="canvas" class="fixed inset-0 w-dvw! h-dvh! outline-none"></canvas>
<ResourceLoading />
