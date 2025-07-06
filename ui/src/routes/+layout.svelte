<script lang="ts">
	import '../app.css';

	import { appConfig } from '$lib/app-config';

	let { data, children } = $props();

	$effect(() => {
		if (data.appConfig) {
			appConfig.set(data.appConfig);
		}
	});
</script>

{@render children()}
