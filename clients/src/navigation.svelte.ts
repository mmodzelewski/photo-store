import LoginPage from "@pages/LoginPage.svelte";
import GalleryPage from "@pages/GalleryPage.svelte"
import IntroPage from "@pages/IntroPage.svelte"
import { invoke } from "@tauri-apps/api/core";

async function createNavigation() {

    const status = await invoke("get_status");

    let initialState = LoginPage;
    switch (status) {
        case 'directories_selected':
            initialState = GalleryPage;
            break;
        case 'after_login':
            initialState = IntroPage;
            break;
    }
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

