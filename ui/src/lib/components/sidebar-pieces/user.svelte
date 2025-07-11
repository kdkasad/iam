<script lang="ts" module>
	export interface DropdownContext {
		open: boolean;
	}
</script>

<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { useSidebar } from '$lib/components/ui/sidebar';
	import { ArrowRightIcon, EllipsisVerticalIcon, LogOutIcon, ShieldUserIcon } from '@lucide/svelte';
	import type { Session, User } from '$lib/models';
	import UpDownGradeDropdownItem from '../up-down-grade-dropdown-item.svelte';
	import { getContext, setContext } from 'svelte';
	import { canUpgrade as canUpgradeFn, nameToInitials } from '$lib/logic';
	import * as Avatar from '$lib/components/ui/avatar';

	const user = getContext<() => User>('user');
	const session = getContext<() => Session>('session');

	// Figure out if we can upgrade/downgrade the session
	let canUpgrade = $derived(canUpgradeFn(user(), session()));
	let canDowngrade = $derived(session().isAdmin);

	let isDropdownOpen = $state(false);
	setContext<() => DropdownContext>('dropdown', () => ({ open: isDropdownOpen }));
</script>

{#snippet avatar()}
	<Avatar.Root>
		<Avatar.Fallback>
			{nameToInitials(user().displayName)}
		</Avatar.Fallback>
	</Avatar.Root>
{/snippet}

<Sidebar.Menu>
	<Sidebar.MenuItem>
		<DropdownMenu.Root bind:open={isDropdownOpen}>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Sidebar.MenuButton
						{...props}
						size="lg"
						class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
					>
						{@render avatar()}
						<div class="grid flex-1 text-left text-sm leading-tight">
							<span class="truncate font-medium">{user().displayName}</span>
							<span class="truncate text-xs">{user().email}</span>
						</div>
						<EllipsisVerticalIcon class="ml-auto size-4" />
					</Sidebar.MenuButton>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content
				class="w-(--bits-dropdown-menu-anchor-width) min-w-56 rounded-lg"
				side={useSidebar().isMobile ? 'bottom' : 'right'}
				align="end"
				sideOffset={4}
			>
				<DropdownMenu.Label class="p-0 font-normal">
					<div class="flex items-center gap-2 px-1 py-1.5 text-left text-sm">
						{@render avatar()}
						<div class="grid flex-1 text-left text-sm leading-tight">
							<span class="truncate font-medium">{user().displayName}</span>
							<span class="truncate text-xs">{user().email}</span>
						</div>
					</div>
				</DropdownMenu.Label>
				{#if canUpgrade || canDowngrade}
					<DropdownMenu.Separator />
					<DropdownMenu.Group>
						<UpDownGradeDropdownItem type={canUpgrade ? 'upgrade' : 'downgrade'} />
					</DropdownMenu.Group>
				{/if}
				<DropdownMenu.Separator />
				<DropdownMenu.Group>
					<a href="/logout">
						<DropdownMenu.Item>
							<LogOutIcon />
							Log out
						</DropdownMenu.Item>
					</a>
				</DropdownMenu.Group>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.MenuItem>
</Sidebar.Menu>
