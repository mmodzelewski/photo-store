<svelte:options immutable={true} />

<script lang="ts">
    import { convertFileSrc, invoke } from "@tauri-apps/api/tauri";
    import Photo from "../components/Photo.svelte";
    import { onMount } from "svelte";
    let images: { path: string; asset: string }[] = [];

    async function getImages() {
        const paths: string[] = await invoke("get_indexed_images");
        images = paths.map((path) => {
            return { asset: convertFileSrc(path), path: path };
        });
    }

    onMount(() => {
        getImages();
    });
</script>

<h2>Gallery page</h2>

<div class="grid grid-flow-row-dense grid-cols-4 gap-4">
    {#each images as image (image.path)}
        <Photo path={image.asset} />
    {/each}
</div>
