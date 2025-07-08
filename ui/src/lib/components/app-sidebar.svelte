<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { appConfig } from '$lib/app-config';
	import { CommandIcon, FingerprintIcon, LayoutDashboard as LayoutDashboardIcon, Settings as SettingsIcon } from '@lucide/svelte';

	let { ref = $bindable(null), ...restProps } = $props();

	const items = [
		{
			title: 'Applications',
			url: '/home/applications',
			icon: LayoutDashboardIcon
		},
		{
			title: 'Settings',
			url: '#',
			icon: SettingsIcon
		}
	];
</script>

<Sidebar.Root bind:ref variant="inset">
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton variant="disabled" size="lg">
					{#snippet child({ props })}
						<div {...props}>
							<div
								class="bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg"
							>
								<FingerprintIcon class="size-4" />
							</div>
							<div class="grid flex-1 text-left text-sm leading-tight">
								<span class="truncate font-medium">{$appConfig.instanceName}</span>
								<span class="truncate text-xs">IAM portal</span>
							</div>
						</div>
					{/snippet}
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>
	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each items as item}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<item.icon />
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Content>
	<Sidebar.Footer />
</Sidebar.Root>
