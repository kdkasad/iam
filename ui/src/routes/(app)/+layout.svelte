<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import { ShieldAlertIcon } from '@lucide/svelte';
	import { canUpgrade } from '$lib/logic.js';
	import { setContext } from 'svelte';
	import type { Session, User } from '$lib/models.js';

	let { children, data } = $props();
	let user = $derived(data.user!);
	let session = $derived(data.session!);
	setContext<() => User>('user', () => user);
	setContext<() => Session>('session', () => session);
</script>

<Sidebar.Provider>
	<AppSidebar />
	<Sidebar.Inset class="relative overflow-clip">
		<div class="flex h-full flex-col">
			<header class="flex h-16 shrink-0 items-center gap-2 px-4">
				<Sidebar.Trigger />
			</header>
			<main class="flex-1">
				{@render children?.()}
			</main>
		</div>
		{#if data.session?.isAdmin == true}
			<div
				class="absolute bottom-0 left-0 right-0 flex flex-row items-center justify-center gap-2 bg-red-300 p-2 text-sm text-red-950"
			>
				<ShieldAlertIcon class="h-4 w-4" />
				<span>You are currently acting as an administrator.</span>
			</div>
		{/if}
	</Sidebar.Inset>
</Sidebar.Provider>
