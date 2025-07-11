<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import { ShieldAlertIcon } from '@lucide/svelte';
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
				<Sidebar.Trigger class="-ml-1" />
			</header>
			<main class="flex-1 p-4 pt-0">
				{@render children?.()}
			</main>
			{#if session.isAdmin === true}
				<footer
					class="flex flex-row items-center justify-center gap-2 bg-red-300 p-2 text-sm text-red-950"
				>
					<ShieldAlertIcon class="h-4 w-4" />
					<Tooltip.Provider>
						<Tooltip.Root>
							<Tooltip.Trigger>Administrator mode</Tooltip.Trigger> is enabled.
							<Tooltip.Content>
								Administrator mode lets you manage other users, application SSO configurations, and
								IAM portal settings.
							</Tooltip.Content>
						</Tooltip.Root>
					</Tooltip.Provider>
				</footer>
			{/if}
		</div>
	</Sidebar.Inset>
</Sidebar.Provider>
