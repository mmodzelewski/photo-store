<script lang="ts">
    import { open } from "@tauri-apps/api/dialog";
    import Button from "../components/Button.svelte";
    import { invoke, convertFileSrc } from "@tauri-apps/api/tauri";
    import { pictureDir } from "@tauri-apps/api/path";
    import { listen } from "@tauri-apps/api/event";
    import { navigation } from "../navigation";
    import { onDestroy, onMount } from "svelte";

    const unlistenFunctions = [];
    let dialog: HTMLDialogElement;
    let filesIndexed: number;
    let thumbnailsGenerated: { done: number; total: number };
    onMount(async () => {
        const unlisten1 = await listen("thumbnails-generated", (event) => {
            console.log(event.payload);
            thumbnailsGenerated = event.payload;
        });
        unlistenFunctions.push(unlisten1);

        const unlisten2 = await listen("files-indexed", (event) => {
            console.log(event.payload);
            filesIndexed = event.payload.total;
            dialog.showModal();
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

    function getImgSrc(status) {
        return convertFileSrc(status.latest);
    }
</script>

<div>
    <Button on:click={selectDir}>Select directory</Button>
</div>
<dialog
    bind:this={dialog}
    class="h-48 w-96 rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700 text-white"
>
    {#if filesIndexed === undefined}
        <div>Indexing images</div>
    {:else if thumbnailsGenerated === undefined}
        <div>Generating thumbnails</div>
    {:else}
        <div>
            Generating thumbnails {thumbnailsGenerated.done} / {thumbnailsGenerated.total}
            <img src={getImgSrc(thumbnailsGenerated)} />
        </div>
    {/if}
</dialog>
