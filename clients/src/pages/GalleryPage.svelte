<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import ImageThumbnail from "@components/ImageThumbnail.svelte";
    import ImagePreviewDialog from "@components/ImagePreviewDialog.svelte";
    import type { Image } from "src/lib/image";
    import Button from "@components/Button.svelte";

    let images: Image[] = [];
    let dialog: ImagePreviewDialog;

    async function getImages() {
        images = await invoke("get_images");
    }

    onMount(() => {
        getImages();
    });

    function syncImages() {
        invoke("sync_images");
    }
</script>

<h2>Gallery page</h2>
<Button on:click={syncImages}>Sync images</Button>

<div class="grid grid-flow-row-dense grid-cols-4 gap-4">
    {#each images as image, index (image.id)}
        <ImageThumbnail {image} on:click={() => dialog.open(index)} />
    {/each}
</div>
<ImagePreviewDialog bind:this={dialog} {images} />
