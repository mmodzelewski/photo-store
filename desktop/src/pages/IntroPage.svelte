<script lang="ts">
    import { open } from "@tauri-apps/api/dialog";
    import Button from "../components/Button.svelte";
    import { invoke } from "@tauri-apps/api/tauri";
    import { pictureDir } from "@tauri-apps/api/path";
    import { listen } from "@tauri-apps/api/event";
    import { navigation } from "../navigation";
    import { onDestroy, onMount } from "svelte";

    const unlistenFunctions = [];
    onMount(async () => {
        const unlisten1 = await listen("thumbnails-generated", (event) => {
            console.log(event.payload);
        });
        unlistenFunctions.push(unlisten1);

        const unlisten2 = await listen("files-indexed", (event) => {
            console.log(event.payload);
        });
        unlistenFunctions.push(unlisten2);
    });

    onDestroy(() => {
        unlistenFunctions.forEach((unlisten) => unlisten());
    });

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
