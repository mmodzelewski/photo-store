<svelte:options immutable={true} />

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
    import { onDestroy, onMount } from "svelte";

    export let path: string;

    let show = false;
    let ref: HTMLElement;

    onMount(() => {
        observer.observe(ref);
    });

    onDestroy(() => {
        observer.unobserve(ref);
    });
</script>

<div
    bind:this={ref}
    on:view_entered={() => (show = true)}
    on:view_left={() => (show = false)}
    class="flex h-80 w-80 items-center border-2 border-solid border-slate-700"
>
    {#if show}
        <img src={path} />
    {/if}
</div>
