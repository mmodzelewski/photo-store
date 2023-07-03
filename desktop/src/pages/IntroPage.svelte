<script lang="ts">
    import { open } from "@tauri-apps/api/dialog";
    import Button from "../components/Button.svelte";
    import { invoke } from "@tauri-apps/api/tauri";
    import { navigation } from "../navigation";

    async function selectDir() {
        const dirs = await open({
            directory: true,
            recursive: true,
            multiple: true,
        });
        if (dirs) {
            await invoke("save_images_dirs", { dirs });
            navigation.goToGallery();
        }
    }
</script>

<div>
    <Button on:click={selectDir}>Select directory</Button>
</div>
