<script lang="ts">
    import { convertFileSrc, invoke } from "@tauri-apps/api/tauri";
    import InfiniteScroll from "../components/InfiniteScroll.svelte";
    import { onMount } from "svelte";
    let images: { path: string; asset: string }[] = [];
    let page = 0;
    let newImages: { path: string; asset: string }[] = [];

    async function getImages() {
        const paths: string[] = await invoke("get_indexed_images_paged", {
            page,
        });
        newImages = paths.map((path) => {
            return { asset: convertFileSrc(path), path: path };
        });
    }
    onMount(() => {
        getImages();
    });

    $: images = [...images, ...newImages];
</script>

<h2>Gallery page</h2>

<div class="gap-4 grid grid-flow-row-dense grid-cols-4">
    {#each images as image, index (image.path)}
        <div class="flex items-center border-solid border-2 border-slate-700">
            <img src={image.asset} loading={index > 5 ? "lazy" : "eager"} />
        </div>
    {/each}
    <InfiniteScroll
        hasMore={newImages.length > 0}
        on:loadMore={() => {
            page++;
            getImages();
        }}
    />
</div>
