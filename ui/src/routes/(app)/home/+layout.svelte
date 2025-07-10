<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import { ShieldAlertIcon } from '@lucide/svelte';

	let { children, data } = $props();
</script>

<Sidebar.Provider>
	<AppSidebar />
	<Sidebar.Inset class="overflow-clip relative">
		<div class="flex flex-col h-full">
			<header class="flex h-16 shrink-0 items-center gap-2 px-4">
				<Sidebar.Trigger />
			</header>
			<main class="flex-1">
				{@render children?.()}
			</main>
		</div>
		{#if data.session?.isAdmin == true}
			<div class="flex flex-row items-center justify-center gap-2 p-2 text-sm text-red-950 bg-red-300 absolute bottom-0 left-0 right-0">
				<ShieldAlertIcon class="h-4 w-4" />
				<span>You are currently acting as an administrator.</span>
			</div>
		{/if}
	</Sidebar.Inset>
</Sidebar.Provider>
