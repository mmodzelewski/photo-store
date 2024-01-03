<script lang="ts">
    import Button from "@components/Button.svelte";
    import { navigation } from "./navigation";
    import { invoke } from "@tauri-apps/api";

    let dialog: HTMLDialogElement;
    let accessToken: string = "";

    function saveToken() {
        invoke("save_token", { token: accessToken });
        dialog.close();
    }
</script>

<div class="container mx-auto p-4 text-white">
    <div class="flex items-center justify-between">
        <h1>Photo store</h1>
        <Button on:click={() => dialog.showModal()}>Settings</Button>
    </div>
    <svelte:component this={$navigation} />
</div>

<dialog
    bind:this={dialog}
    class="h-44 w-96 rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700 text-white backdrop:bg-black backdrop:opacity-50"
>
    <div class="flex h-full w-full flex-col justify-center">
        <label for="access-token">Access token</label>
        <input
            name="access-token"
            class="text-black"
            bind:value={accessToken}
        />
        <Button on:click={() => saveToken()}>Save</Button>
    </div>
</dialog>
