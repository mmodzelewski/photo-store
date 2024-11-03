<script lang="ts">
    import type { HTMLButtonAttributes } from "svelte/elements";

    let {
        disabled = false,
        onclick = () => {},
        children,
    }: HTMLButtonAttributes = $props();
</script>

<button
    {disabled}
    {onclick}
    class="rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700 p-4"
>
    {#if children}
        {@render children()}
    {:else}
        Button
    {/if}
</button>
