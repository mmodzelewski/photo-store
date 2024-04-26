<script lang="ts">
    export let disabled = false;
</script>

<button
    {disabled}
    on:click
    class="rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700 p-4"
>
    <slot />
</button>
