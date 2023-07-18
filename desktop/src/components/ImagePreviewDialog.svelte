<script lang="ts">
    import Button from "@components/Button.svelte";
    import { convertFileSrc } from "@tauri-apps/api/tauri";

    type Image = {
        id: string;
        path: string;
        thumbnail_path: string;
    };

    export let images: Image[] = [];
    export function open(index: number) {
        dialog.showModal();
        this.index = index;
    }

    let index = 0;
    let dialog: HTMLDialogElement;
    let imagePath: string;
    $: hasNext = index < images.length - 1;
    $: hasPrev = index > 0;
    $: {
        const path = images[index]?.path;
        if (path) {
            imagePath = convertFileSrc(path);
        }
    }
</script>

<dialog
    bind:this={dialog}
    on:click={() => dialog.close()}
    class="aspect-auto w-9/12 rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700 text-white"
>
    <div on:click|stopPropagation>
        <Button on:click={() => (index -= 1)} disabled={!hasPrev}>Prev</Button>
        <img src={imagePath} />
        <Button disabled={!hasNext} on:click={() => (index += 1)}>Next</Button>
    </div>
</dialog>

<style>
    ::backdrop {
        background: black;
        opacity: 50%;
    }
</style>
