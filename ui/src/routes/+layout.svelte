<script lang="ts">
	import '../app.css';

	import { setContext } from 'svelte';

	let { data, children } = $props();

	setContext('appConfig', data.appConfig)
</script>

{@render children()}
