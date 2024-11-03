<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import { onDestroy, onMount } from "svelte";
    import { Navigation } from "src/navigation.svelte";
    import Button from "@components/Button.svelte";

    let initPrivateKeyDialog: HTMLDialogElement;
    let passphrase: string = "";

    const unlistenFunctions: UnlistenFn[] = [];

    async function authenticate() {
        invoke("authenticate");
    }

    async function initPrivateKey() {
        try {
            await invoke("get_private_key", { passphrase });
            initPrivateKeyDialog.close();
            Navigation.goToIntro();
        } catch (e) {
            console.log(e);
        }
    }

    onMount(async () => {
        const unlisten = await listen<void>("authenticated", () => {
            initPrivateKeyDialog.showModal();
        });
        unlistenFunctions.push(unlisten);
    });

    onDestroy(() => {
        unlistenFunctions.forEach((unlisten) => unlisten());
    });
</script>

<Button onclick={() => authenticate()}>Login</Button>

<dialog
    bind:this={initPrivateKeyDialog}
    class="h-44 w-96 rounded-lg border-2 border-solid border-zinc-500 bg-zinc-700 text-white backdrop:bg-black backdrop:opacity-50"
>
    <div class="flex h-full w-full flex-col justify-center">
        <input class="text-black" bind:value={passphrase} type="password" />
        <Button onclick={() => initPrivateKey()}>Init encryption</Button>
    </div>
</dialog>
