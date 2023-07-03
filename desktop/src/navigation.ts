import { writable } from "svelte/store";
import IntroPage from "./pages/IntroPage.svelte";
import GalleryPage from "./pages/GalleryPage.svelte";
import { invoke } from "@tauri-apps/api";

async function createNavigation() {
    const hasDirs = await invoke("has_images_dirs")
    const { subscribe, set } = writable(hasDirs ? GalleryPage : IntroPage);

    return {
        subscribe,
        goToGallery: () => set(GalleryPage),
    };
}

export const navigation = await createNavigation();
