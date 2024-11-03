import LoginPage from "@pages/LoginPage.svelte";
import GalleryPage from "@pages/GalleryPage.svelte"
import IntroPage from "@pages/IntroPage.svelte"
import { invoke } from "@tauri-apps/api/core";

async function createNavigation() {

    const hasDirs = await invoke("has_images_dirs");
    // todo: add LoginPage check
    const initialState = hasDirs ? GalleryPage : IntroPage;
    let component = $state(initialState);

    function goToIntro() {
        component = IntroPage;
    }
    function goToGallery() {
        component = GalleryPage;
    }

    return {
        get component() {
            return component;
        },
        goToIntro,
        goToGallery,
    }
}

export const Navigation = await createNavigation();

