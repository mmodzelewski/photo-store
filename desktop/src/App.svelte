<script lang="ts">
    import { open } from "@tauri-apps/api/dialog";

    async function selectDir() {
        await open({
            directory: true,
            recursive: true,
        });
    }
</script>

<main>
    <h1>Photo store</h1>

    <div>
        <button on:click={selectDir}>Select directory</button>
    </div>
</main>
