<script context="module" lang="ts">
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
    import { convertFileSrc } from "@tauri-apps/api/tauri";
    import { onDestroy, onMount } from "svelte";

    export let path: string;
    $: convertedPath = convertFileSrc(path);

    let show = false;
    let ref: HTMLElement;

    onMount(() => {
        observer.observe(ref);
    });

    onDestroy(() => {
        observer.unobserve(ref);
    });
</script>

<button
    bind:this={ref}
    on:view_entered={() => (show = true)}
    on:view_left={() => (show = false)}
    class="flex h-80 w-80 items-center border-2 border-solid border-slate-700"
    on:click
>
    {#if show}
        <img
            class="h-80 w-80 object-cover"
            src={convertedPath}
            alt="Description"
        />
    {/if}
</button>
