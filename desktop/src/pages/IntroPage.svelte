<script lang="ts">
    import { open } from "@tauri-apps/api/dialog";
    import { invoke, convertFileSrc } from "@tauri-apps/api/tauri";
    import { pictureDir } from "@tauri-apps/api/path";
    import { listen } from "@tauri-apps/api/event";
    import { navigation } from "../navigation";
    import { onDestroy, onMount } from "svelte";
    import Button from "@components/Button.svelte";

    type ThumbnailsGeneratedPayload = {
        done: number;
        total: number;
        latest: string;
    };
    type FilesIndexedPayload = { total: number };

    const unlistenFunctions = [];
    let dialog: HTMLDialogElement;
    let filesIndexed: number;
    let thumbnailsGenerated: ThumbnailsGeneratedPayload;
    let latestThumbnail: string;
    let thumbnailToDisplay: string;
    let interval: NodeJS.Timer;

    onMount(async () => {
        const unlisten1 = await listen<ThumbnailsGeneratedPayload>(
            "thumbnails-generated",
            (event) => {
                thumbnailsGenerated = event.payload;
                latestThumbnail = convertFileSrc(thumbnailsGenerated.latest);
                if (!interval) {
                    thumbnailToDisplay = latestThumbnail;
                    interval = setInterval(() => {
                        thumbnailToDisplay = latestThumbnail;
                    }, 2000);
                }
            }
        );
        unlistenFunctions.push(unlisten1);

        const unlisten2 = await listen<FilesIndexedPayload>(
            "files-indexed",
            (event) => {
                filesIndexed = event.payload.total;
                dialog.showModal();
            }
        );
        unlistenFunctions.push(unlisten2);
    });

    onDestroy(() => {
        unlistenFunctions.forEach((unlisten) => unlisten());
        if (interval) {
            clearInterval(interval);
        }
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
<dialog
    bind:this={dialog}
    class="h-44 w-96 rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700 text-white"
>
    {#if filesIndexed === undefined}
        <div>Indexing images</div>
    {:else if thumbnailsGenerated === undefined}
        <div>Generating thumbnails</div>
    {:else}
        <div class="flex h-full items-center justify-around align-middle">
            <p>
                {thumbnailsGenerated.done} / {thumbnailsGenerated.total}
            </p>
            <div class="h-36 w-36 overflow-hidden">
                <img src={thumbnailToDisplay} alt="Currently processed item" />
            </div>
        </div>
    {/if}
</dialog>
