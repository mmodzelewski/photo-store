<script lang="ts">
    import { invoke } from "@tauri-apps/api/tauri";
    import { onMount } from "svelte";
    import Photo from "@components/Photo.svelte";
    import ImagePreviewDialog from "@components/ImagePreviewDialog.svelte";

    type Image = {
        id: string;
        path: string;
        thumbnail_small: string;
        thumbnail_big: string;
    };

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
        <Photo
            path={image.thumbnail_small}
            on:click={() => dialog.open(index)}
        />
    {/each}
</div>
<ImagePreviewDialog bind:this={dialog} {images} />
