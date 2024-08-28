import IntroPage from "@pages/IntroPage.svelte";
import GalleryPage from "@pages/GalleryPage.svelte"
import { invoke } from "@tauri-apps/api/core";

async function createNavigation() {

    const hasDirs = await invoke("has_images_dirs");
    const initialState = hasDirs ? GalleryPage : IntroPage;
    let component = $state(initialState);

    function goToGallery() {
        component = GalleryPage;
    }

    return {
        get component() {
            return component;
        },
        goToGallery,
    }
}

export const Navigation = await createNavigation();

