<script lang="ts">
    import { open } from "@tauri-apps/api/dialog";
    import Button from "../components/Button.svelte";
    import { invoke } from "@tauri-apps/api/tauri";

    async function selectDir() {
        const dirs = await open({
            directory: true,
            recursive: true,
            multiple: true,
        });
        if (dirs) {
            invoke("save_images_dirs", { dirs });
        }
    }
</script>

<div>
    <Button on:click={selectDir}>Select directory</Button>
</div>
