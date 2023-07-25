<script lang="ts">
    import { invoke } from "@tauri-apps/api/tauri";
    import { onMount } from "svelte";
    import ImageThumbnail from "@components/ImageThumbnail.svelte";
    import ImagePreviewDialog from "@components/ImagePreviewDialog.svelte";
    import type { Image } from "src/lib/image";

    let images: Image[] = [];
    let dialog: ImagePreviewDialog;

    async function getImages() {
        images = await invoke("get_images");
    }

    onMount(() => {
        getImages();
    });
</script>

<h2>Gallery page</h2>

<div class="grid grid-flow-row-dense grid-cols-4 gap-4">
    {#each images as image, index (image.id)}
        <ImageThumbnail {image} on:click={() => dialog.open(index)} />
    {/each}
</div>
<ImagePreviewDialog bind:this={dialog} {images} />
