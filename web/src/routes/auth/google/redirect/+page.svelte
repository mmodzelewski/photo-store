<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { BACKEND_URL } from '$lib/config';

	onMount(async () => {
		let response = await fetch(BACKEND_URL + '/auth/providers/google/complete' + $page.url.search);
        let data = await response.json();
		console.log(data);
        localStorage.setItem('userId', data.user_id);
        localStorage.setItem('authToken', data.auth_token);
		goto('/', { replaceState: true });
	});
</script>

<div>Redirecting...</div>
