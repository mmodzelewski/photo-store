<script lang="ts">
    import { navigation } from "./navigation";
    import GalleryPage from "./pages/GalleryPage.svelte";
    import Button from "./components/Button.svelte";
</script>

<div class="container mx-auto p-4 text-white">
    <h1>Photo store</h1>
    <Button on:click={() => navigation.set(GalleryPage)}>Go to gallery</Button>

    <svelte:component this={$navigation} />
</div>
