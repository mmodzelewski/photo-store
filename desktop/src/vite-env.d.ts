/// <reference types="svelte" />
/// <reference types="vite/client" />

declare namespace svelteHTML {
    interface HTMLAttributes<T> {
        "on:view_entered"?: (event: CustomEvent<void>) => void;
        "on:view_left"?: (event: CustomEvent<void>) => void;
    }
}
