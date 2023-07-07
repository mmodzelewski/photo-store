<script lang="ts">
    import { onDestroy, createEventDispatcher } from "svelte";

    export let hasMore = true;

    const threshold = 500;
    const dispatch = createEventDispatcher();

    let isLoadMore = false;

    const scrollCallback = (e: any) => {
        const element = e.target;
        const offset =
            element.scrollingElement.scrollHeight -
            element.scrollingElement.clientHeight -
            element.scrollingElement.scrollTop;

        if (offset <= threshold) {
            if (!isLoadMore && hasMore) {
                dispatch("loadMore");
            }
            isLoadMore = true;
        } else {
            isLoadMore = false;
        }
    };

    document.addEventListener("scroll", scrollCallback);
    document.addEventListener("resize", scrollCallback);

    onDestroy(() => {
        document.removeEventListener("scroll", null);
        document.removeEventListener("resize", null);
    });
</script>
