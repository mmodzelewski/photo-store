import { writable } from "svelte/store";
import IntroPage from "./pages/IntroPage.svelte";

export const navigation = writable(IntroPage);
