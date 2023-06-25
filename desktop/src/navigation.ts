import { writable } from "svelte/store";
import IntroPage from "./pages/IntroPage.svelte";
import GalleryPage from "./pages/GalleryPage.svelte";

function createNavigation() {
    const { subscribe, set } = writable(IntroPage);

    return {
        subscribe,
        goToGallery: () => set(GalleryPage),
    };
}

export const navigation = createNavigation();

