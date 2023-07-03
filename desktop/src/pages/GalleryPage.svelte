<script lang="ts">
    import { convertFileSrc, invoke } from "@tauri-apps/api/tauri";
    import { onMount } from "svelte";
    let files: { path: string; asset: string }[] = [];
    onMount(async () => {
        const paths: string[] = await invoke("get_indexed_images");
        console.log(paths);
        files = paths.map((path) => {
            return { asset: convertFileSrc(path), path: path };
        });
        console.log(files);
    });
</script>

<h2>Gallery page</h2>

<div class="gap-4 grid grid-flow-row-dense grid-cols-4">
    {#each files as file, index (file.path)}
        <div class="flex items-center border-solid border-2 border-slate-700">
            <img src={file.asset} loading={index > 5 ? "lazy" : "eager"} />
        </div>
    {/each}
</div>

