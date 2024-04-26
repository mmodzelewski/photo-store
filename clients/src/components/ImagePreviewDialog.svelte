<script lang="ts">
    import Button from "@components/Button.svelte";
    import { toContainUri, type Image } from "src/lib/image";

    export let images: Image[] = [];
    export function open(image_index: number) {
        dialog.showModal();
        index = image_index;
    }

    function handleKeyDown(event: KeyboardEvent) {
        switch (event.key) {
            case "ArrowLeft":
                if (hasPrev) {
                    index -= 1;
                }
                break;
            case "ArrowRight":
                if (hasNext) {
                    index += 1;
                }
                break;
        }
    }

    let index = 0;
    let dialog: HTMLDialogElement;
    let imagePath: string;
    $: hasNext = index < images.length - 1;
    $: hasPrev = index > 0;
    $: {
        const image = images[index];
        if (image) {
            imagePath = toContainUri(image);
        }
    }
</script>

<svelte:window on:keydown={handleKeyDown} />

<dialog
    bind:this={dialog}
    class="rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700 text-white"
>
    <div>
        <Button on:click={() => (index -= 1)} disabled={!hasPrev}>Prev</Button>
        <Button on:click={() => dialog.close()}>Close</Button>
        <img
            src={imagePath}
            alt="Full-size preview"
            class="max-h-[80vh] max-w-[90vw]"
        />
        <Button disabled={!hasNext} on:click={() => (index += 1)}>Next</Button>
    </div>
</dialog>

<style>
    ::backdrop {
        background: black;
        opacity: 50%;
    }
</style>
