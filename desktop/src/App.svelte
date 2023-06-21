<script lang="ts">
    import { open } from "@tauri-apps/api/dialog";

    async function selectDir() {
        await open({
            directory: true,
            recursive: true,
        });
    }
</script>

<div class="container mx-auto p-4 text-white">
    <h1>Photo store</h1>

    <div>
        <button
            on:click={selectDir}
            class="border-solid border-2 border-zinc-500 p-4 rounded-lg bg-zinc-700"
        >
            Select directory
        </button>
    </div>
</div>
