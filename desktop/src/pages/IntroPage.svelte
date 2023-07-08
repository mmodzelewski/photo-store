<script lang="ts">
    import { open } from "@tauri-apps/api/dialog";
    import Button from "../components/Button.svelte";
    import { invoke } from "@tauri-apps/api/tauri";
    import { pictureDir } from "@tauri-apps/api/path";
    import { navigation } from "../navigation";

    async function selectDir() {
        const picsDir = await pictureDir();
        const dirs = await open({
            defaultPath: picsDir,
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
