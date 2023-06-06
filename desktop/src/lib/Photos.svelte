<script lang="ts">
    import { readDir, BaseDirectory } from "@tauri-apps/api/fs";
    import { convertFileSrc } from "@tauri-apps/api/tauri";
    import { onMount } from "svelte";

    let urls: string[] = [];
    onMount(async () => {
        const entries = await readDir("images", {
            dir: BaseDirectory.Picture,
            recursive: true,
        });
        urls = entries.map((entry) => convertFileSrc(entry.path));
    });
</script>

<div>
    {#each urls as url}
        <img src={url} height="500" alt=""/>
    {/each}
</div>
