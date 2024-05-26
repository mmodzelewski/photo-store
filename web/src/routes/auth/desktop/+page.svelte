<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';

	onMount(async () => {
		const token: string = localStorage.getItem('authToken')!;
		const response = await fetch('http://localhost:3000/auth/desktop' + $page.url.search, {
			headers: {
				"Authorization": token
			}
		});
		let data = await response.json();
        window.location.replace(data.redirect_uri);
	});
</script>
