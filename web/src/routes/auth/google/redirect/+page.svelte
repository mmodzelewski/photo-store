<script lang="ts">

	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';

	onMount(async () => {
		let response = await fetch('http://localhost:3000/auth/providers/google/complete' + $page.url.search);
        let data = await response.json();
		console.log(data);
        localStorage.setItem('userId', data.user_id);
        localStorage.setItem('authToken', data.auth_token);
		goto('/', { replaceState: true });
	});

</script>
