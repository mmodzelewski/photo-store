<script context="module" lang="ts">
    export type Image = { id: string };
    const observer = new IntersectionObserver(
        (entries) => {
            entries.forEach((entry) => {
                if (entry.isIntersecting) {
                    entry.target.dispatchEvent(new CustomEvent("view_entered"));
                } else {
                    entry.target.dispatchEvent(new CustomEvent("view_left"));
                }
            });
        },
        {
            rootMargin: "150px",
        }
    );
</script>

<script lang="ts">
    import { toCoverUri } from "src/lib/image";
    import { onDestroy, onMount } from "svelte";
    //import { fetch } from '@tauri-apps/api/http';

    export let image: Image;

    $: convertedPath = toCoverUri(image);

    let inView = false;
    let show = false;
    let ref: HTMLElement;
    async function fetchImg(path: string) {
        await fetch(path, {
            mode: "no-cors",
        });
    }
    $: {
        if (inView) {
            //show = true;
            fetchImg(convertedPath).then(() => {
                show = true;
            });
        }
    }

    onMount(() => {
        observer.observe(ref);
    });

    onDestroy(() => {
        observer.unobserve(ref);
    });
</script>

<button
    bind:this={ref}
    on:view_entered={() => (inView = true)}
    on:view_left={() => (inView = false)}
    class="flex h-80 w-80 items-center border-2 border-solid border-slate-700"
    on:click
>
    {@html show}
    {#if show}
        <img
            class="h-80 w-80 object-cover"
            src={convertedPath}
            alt="Description"
        />
    {/if}
</button>
