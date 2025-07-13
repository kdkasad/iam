<script lang="ts">
	import { goto } from '$app/navigation';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { cn } from '$lib/utils';
	import { ArrowRightIcon, Loader2Icon, ShieldOffIcon, ShieldUserIcon } from '@lucide/svelte';
	import { getContext } from 'svelte';
	import type { DropdownContext } from './sidebar-pieces/user.svelte';

	interface Props {
		type: 'upgrade' | 'downgrade';
	}
	let { type }: Props = $props();

	let loading = $state(false);

	let dropdownContext = getContext<() => DropdownContext>('dropdown');

	// Create abort controller to cancel fetch when dropdown is closed.
	let abortController: AbortController | undefined;
	$effect(() => {
		if (!dropdownContext().open) {
			abortController?.abort();
			abortController = new AbortController();
		}
	})

	const onclick: HTMLElement['onclick'] = async (ev) => {
		// ev.preventDefault();
		loading = true;
		let response: Response;
		try {
			if (type === 'upgrade') {
				response = await fetch('/api/v1/auth/upgrade', {
					method: 'POST',
					signal: abortController?.signal,
					headers: {
						'Content-Type': 'application/json'
					},
					body: JSON.stringify({
						target: 'Admin'
					}),
					credentials: 'include'
				});
			} else {
				response = await fetch('/api/v1/auth/downgrade', {
					method: 'POST',
					signal: abortController?.signal,
					headers: {
						'Content-Type': 'application/json'
					},
					credentials: 'include'
				});
			}
		} catch (error) {
			loading = false;
			if (error instanceof DOMException && error.name === 'AbortError') {
				// Operation was canceled; not a problem.
			} else {
				console.error(error);
			}
			return;
		}
		if (!response?.ok) {
			console.error('fetch failed', response);
		}
		// This endpoint doesn't return anything, so we don't need to do anything else with the response.
		loading = false;
		goto(type === 'upgrade' ? '/admin' : '/home', { invalidateAll: true });
		dropdownContext().open = false;
	};
</script>

<DropdownMenu.Item {onclick} variant="destructive">
	{#if type === 'upgrade'}
		<ShieldUserIcon />
	{:else}
		<ShieldOffIcon />
	{/if}
	{type === 'upgrade' ? 'Administrator mode' : 'Leave administrator mode'}
	{#if type === 'upgrade'}
		<ArrowRightIcon class={cn('ml-auto size-4', loading && 'hidden')} />
	{/if}
	<Loader2Icon class={cn('ml-auto size-4 animate-spin', !loading && 'hidden')} />
</DropdownMenu.Item>
