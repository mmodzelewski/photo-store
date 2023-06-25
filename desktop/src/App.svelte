<script lang="ts">
    import { navigation } from "./navigation";
    import Button from "./components/Button.svelte";
</script>

<div class="container mx-auto p-4 text-white">
    <h1>Photo store</h1>
    <Button on:click={() => navigation.goToGallery()}>Go to gallery</Button>

    <svelte:component this={$navigation} />
</div>
