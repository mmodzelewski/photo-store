<svelte:options immutable={true} />

<script lang="ts">
    type Image = {
        id: string;
        path: string;
        thumbnail_path: string;
    };
    import { convertFileSrc, invoke } from "@tauri-apps/api/tauri";
    import { onMount } from "svelte";
    import Photo from "@components/Photo.svelte";

    let images: Image[] = [];

    async function getImages() {
        images = await invoke("get_images");
    }

    onMount(() => {
        getImages();
    });
    let dialog: HTMLDialogElement;
    let selectedImagePath: string;

    function showImage(image: Image) {
        selectedImagePath = convertFileSrc(image.path);
        dialog.showModal();
    }
</script>

<h2>Gallery page</h2>

<div class="grid grid-flow-row-dense grid-cols-4 gap-4">
    {#each images as image (image.id)}
        <Photo
            path={image.thumbnail_path}
            on:click={() => {
                showImage(image);
            }}
        />
    {/each}
</div>
<dialog
    bind:this={dialog}
    on:click={() => dialog.close()}
    class="aspect-auto w-9/12 rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700"
>
    <div on:click|stopPropagation>
        <img src={selectedImagePath} />
    </div>
</dialog>

<style>
    ::backdrop {
        background: black;
        opacity: 50%;
    }
</style>
