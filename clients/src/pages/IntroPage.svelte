<script lang="ts">
    import { open } from "@tauri-apps/plugin-dialog";
    import { invoke, convertFileSrc } from "@tauri-apps/api/core";
    import { pictureDir } from "@tauri-apps/api/path";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import { Navigation } from "../navigation.svelte";
    import { onDestroy, onMount } from "svelte";
    import Button from "@components/Button.svelte";

    type ThumbnailsGeneratedPayload = {
        done: number;
        total: number;
        latest: string;
    };
    type FilesIndexedPayload = { total: number };

    const unlistenFunctions: UnlistenFn[] = [];
    let dialog: HTMLDialogElement;
    let filesIndexed: number;
    let thumbnailsGenerated: ThumbnailsGeneratedPayload;
    let latestThumbnail: string;
    let thumbnailToDisplay: string;
    let interval: NodeJS.Timeout;

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
            },
        );
        unlistenFunctions.push(unlisten1);

        const unlisten2 = await listen<FilesIndexedPayload>(
            "files-indexed",
            (event) => {
                filesIndexed = event.payload.total;
                dialog.showModal();
            },
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
            Navigation.goToGallery()
        }
    }
</script>

<div>
    <Button onclick={selectDir}>Select directory</Button>
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
            <img
                class="h-36 w-36 object-cover"
                src={thumbnailToDisplay}
                alt="Currently processed item"
            />
        </div>
    {/if}
</dialog>
