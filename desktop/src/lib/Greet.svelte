<script lang="ts">
    import { readDir, BaseDirectory, type FileEntry } from "@tauri-apps/api/fs";
    import { convertFileSrc } from "@tauri-apps/api/tauri";
    let entries: FileEntry[] = [];
    let urls: string[] = [];

    async function getImages() {
        entries = await readDir("images", {
            dir: BaseDirectory.Picture,
            recursive: true,
        });
        urls = entries.map((entry) => convertFileSrc(entry.path));
    }
</script>

<button on:click={getImages}>Get Images</button>
<div>
    {#each urls as entry}
        <img src={entry} width="500" alt="" />
    {/each}
</div>
