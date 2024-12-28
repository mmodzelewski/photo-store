<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { onDestroy, onMount } from "svelte";
    import ImageThumbnail from "@components/ImageThumbnail.svelte";
    import ImagePreviewDialog from "@components/ImagePreviewDialog.svelte";
    import type { Image } from "src/lib/image";
    import Button from "@components/Button.svelte";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";

    let images: Image[] = [];
    let dialog: ImagePreviewDialog;
    let indexUpdatedUnlisten: UnlistenFn;

    async function getImages() {
        images = await invoke("get_images");
    }

    onMount(async () => {
        getImages();
        indexUpdatedUnlisten = await listen("index-updated", () => getImages());
    });

    function syncImages() {
        invoke("sync_images");
    }

    onDestroy(() => {
        indexUpdatedUnlisten();
    });
</script>

<h2>Gallery page</h2>
<Button onclick={syncImages}>Sync images</Button>

<div class="grid grid-flow-row-dense grid-cols-4 gap-4">
    {#each images as image, index (image.id)}
        <ImageThumbnail {image} on:click={() => dialog.open(index)}/>
    {/each}
</div>
<ImagePreviewDialog bind:this={dialog} {images}/>
