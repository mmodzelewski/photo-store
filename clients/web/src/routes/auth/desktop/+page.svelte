<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { BACKEND_URL } from '$lib/config';

	onMount(async () => {
		const token: string = localStorage.getItem('authToken')!;
		const response = await fetch(BACKEND_URL + '/auth/desktop' + $page.url.search, {
			headers: {
				"Authorization": token
			}
		});
		const data = await response.json();
        window.location.replace(data.redirect_uri);
	});
</script>

<div>Authenticating...</div>
